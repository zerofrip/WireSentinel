use crate::etw::{backend_from_settings, poll_connections, TrafficBackend};
use crate::handler::ConnectionHandler;
use shared_types::{AppIdentity, BandwidthSnapshot, ConnectionSnapshot, TrafficEvent};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

/// Polls connection tables and emits traffic events.
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

    pub fn emit_traffic(&self, event: TrafficEvent) {
        let _ = self.traffic_tx.send(event);
    }

    pub fn emit_connection(&self, snapshot: ConnectionSnapshot) {
        let key = format!(
            "{}:{}:{}",
            snapshot.pid,
            snapshot.local_addr,
            snapshot.remote_addr
        );
        self.seen_connections.write().insert(key, ());
        let _ = self.connection_tx.send(snapshot);
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
                    for conn in connections {
                        monitor.emit_connection(conn.clone());
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
