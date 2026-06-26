use parking_lot::RwLock;
use shared_types::{AppIdentity, BandwidthSnapshot, ConnectionSnapshot, TrafficEvent};
use std::collections::{HashMap, HashSet};
use tokio::sync::broadcast;
use uuid::Uuid;

pub fn connection_key(snapshot: &ConnectionSnapshot) -> String {
    format!(
        "{}:{}:{}",
        snapshot.pid, snapshot.local_addr, snapshot.remote_addr
    )
}

/// Tracks live connections and emits traffic events.
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

    pub fn track_connection(&self, conn: &ConnectionSnapshot) {
        self.seen_connections
            .write()
            .insert(connection_key(conn), ());
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
        let active_app_ids: HashSet<Uuid> = self.apps.read().values().map(|a| a.id()).collect();
        self.bandwidth
            .write()
            .retain(|id, _| active_app_ids.contains(id));
    }

    pub fn broadcast_connection(&self, snapshot: ConnectionSnapshot) {
        let _ = self.connection_tx.send(snapshot);
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
