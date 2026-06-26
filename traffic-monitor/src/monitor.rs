use crate::etw::{backend_from_settings, poll_connections, TrafficBackend};
use crate::handler::ConnectionHandler;
use parking_lot::RwLock;
use shared_types::{AppIdentity, BandwidthSnapshot, ConnectionSnapshot, TrafficEvent};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

static POLL_TICK: AtomicU64 = AtomicU64::new(0);

fn connection_key(snapshot: &ConnectionSnapshot) -> String {
    format!(
        "{}:{}:{}",
        snapshot.pid, snapshot.local_addr, snapshot.remote_addr
    )
}

// #region agent log
fn agent_debug_log(hypothesis_id: &str, location: &str, message: &str, data: &str) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let line = format!(
        r#"{{"sessionId":"28de1e","hypothesisId":"{hypothesis_id}","location":"{location}","message":"{message}","data":{data},"timestamp":{timestamp}}}"#
    );
    tracing::warn!(
        target: "agent_debug",
        session_id = "28de1e",
        hypothesis_id,
        location,
        message,
        data,
        "agent debug"
    );
    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(path) = std::env::var("WIRESENTINEL_DEBUG_LOG") {
        paths.push(path.into());
    }
    #[cfg(windows)]
    paths.push(std::path::PathBuf::from(
        r"C:\ProgramData\WireSentinel\debug-28de1e.log",
    ));
    #[cfg(not(windows))]
    paths.push(std::path::PathBuf::from(
        "/home/zero/github/WireSentinel/.cursor/debug-28de1e.log",
    ));
    for path in paths {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(file, "{line}");
        }
    }
}
// #endregion

/// Polls conn tables and emits traffic events.
pub struct TrafficMonitor {
    apps: RwLock<HashMap<u32, AppIdentity>>,
    bandwidth: RwLock<HashMap<Uuid, BandwidthSnapshot>>,
    traffic_tx: broadcast::Sender<TrafficEvent>,
    connection_tx: broadcast::Sender<ConnectionSnapshot>,
    poll_interval_ms: u64,
    seen_connections: RwLock<HashMap<String, ()>>,
}

impl TrafficMonitor {
    pub fn new(poll_interval_ms: u64) -> Self {
        let (traffic_tx, _) = broadcast::channel(1024);
        let (connection_tx, _) = broadcast::channel(1024);
        Self {
            apps: RwLock::new(HashMap::new()),
            bandwidth: RwLock::new(HashMap::new()),
            traffic_tx,
            connection_tx,
            poll_interval_ms,
            seen_connections: RwLock::new(HashMap::new()),
        }
    }

    pub fn subscribe_traffic(&self) -> broadcast::Receiver<TrafficEvent> {
        self.traffic_tx.subscribe()
    }

    pub fn subscribe_connections(&self) -> broadcast::Receiver<ConnectionSnapshot> {
        self.connection_tx.subscribe()
    }

    pub fn register_app(&self, app: AppIdentity) {
        self.apps.write().insert(app.pid, app);
    }

    pub fn apps(&self) -> Vec<AppIdentity> {
        self.apps.read().values().cloned().collect()
    }

    pub fn bandwidth_snapshots(&self) -> Vec<BandwidthSnapshot> {
        self.bandwidth.read().values().cloned().collect()
    }

    pub fn connection_count(&self) -> u32 {
        self.seen_connections.read().len() as u32
    }

    /// Replace tracked connections with the current poll snapshot (prevents unbounded growth).
    pub fn replace_active_connections(&self, connections: &[ConnectionSnapshot]) {
        let mut active = HashMap::with_capacity(connections.len());
        let mut active_pids = HashSet::with_capacity(connections.len());
        for conn in connections {
            active.insert(connection_key(conn), ());
            active_pids.insert(conn.pid);
        }
        *self.seen_connections.write() = active;
        self.apps.write().retain(|pid, _| active_pids.contains(pid));
    }

    pub fn broadcast_connection(&self, snapshot: ConnectionSnapshot) {
        let _ = self.connection_tx.send(snapshot);
    }

    pub fn memory_stats(&self) -> (usize, usize, usize) {
        (
            self.seen_connections.read().len(),
            self.apps.read().len(),
            self.bandwidth.read().len(),
        )
    }

    pub fn emit_traffic(&self, event: TrafficEvent) {
        let _ = self.traffic_tx.send(event);
    }

    pub fn emit_connection(&self, snapshot: ConnectionSnapshot) {
        self.replace_active_connections(std::slice::from_ref(&snapshot));
        self.broadcast_connection(snapshot);
    }

    pub fn update_bandwidth(&self, app_id: Uuid, exe_name: &str, bytes_in: u64, bytes_out: u64) {
        let mut map = self.bandwidth.write();
        let entry = map.entry(app_id).or_insert_with(|| BandwidthSnapshot {
            app_id,
            exe_name: exe_name.to_string(),
            bytes_in_per_sec: 0,
            bytes_out_per_sec: 0,
            total_bytes_in: 0,
            total_bytes_out: 0,
        });
        entry.bytes_in_per_sec = bytes_in;
        entry.bytes_out_per_sec = bytes_out;
        entry.total_bytes_in += bytes_in;
        entry.total_bytes_out += bytes_out;
    }

    pub fn poll_interval_ms(&self) -> u64 {
        self.poll_interval_ms
    }

    pub fn get_app_by_pid(&self, pid: u32) -> Option<AppIdentity> {
        self.apps.read().get(&pid).cloned()
    }
}

fn collect_connections(backend: TrafficBackend) -> Vec<ConnectionSnapshot> {
    match backend {
        TrafficBackend::Etw => poll_connections(),
        TrafficBackend::Iphlpapi => {
            #[cfg(windows)]
            {
                crate::windows::enumerate_tcp_connections()
            }
            #[cfg(not(windows))]
            {
                Vec::new()
            }
        }
    }
}

/// Spawn the polling loop with connection handler pipeline.
pub fn spawn_monitor(
    monitor: Arc<TrafficMonitor>,
    handler: Arc<dyn ConnectionHandler>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
    backend: &str,
) -> tokio::task::JoinHandle<()> {
    let backend = backend_from_settings(backend);
    tokio::spawn(async move {
        let interval = monitor.poll_interval_ms();
        let mut ticker = tokio::time::interval(std::time::Duration::from_millis(interval));
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let connections = collect_connections(backend);
                    let poll_conn_count = connections.len();
                    monitor.replace_active_connections(&connections);

                    // #region agent log
                    let tick = POLL_TICK.fetch_add(1, Ordering::Relaxed) + 1;
                    if tick == 1 || tick % 30 == 0 {
                        let (seen_len, apps_len, bandwidth_len) = monitor.memory_stats();
                        agent_debug_log(
                            "A",
                            "monitor.rs:spawn_monitor",
                            "traffic monitor memory stats",
                            &format!(
                                r#"{{"tick":{tick},"poll_conn_count":{poll_conn_count},"seen_connections":{seen_len},"apps":{apps_len},"bandwidth":{bandwidth_len}}}"#
                            ),
                        );
                    }
                    // #endregion

                    for conn in connections {
                        monitor.broadcast_connection(conn.clone());
                        let h = Arc::clone(&handler);
                        tokio::spawn(async move {
                            h.on_connection(conn).await;
                        });
                    }
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        break;
                    }
                }
            }
        }
    })
}
