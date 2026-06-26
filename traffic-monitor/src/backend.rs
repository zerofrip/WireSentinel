//! Connection source backends (packet event-driven and iphlpapi polling).

use crate::filter::is_processable_connection;
use crate::handler::ConnectionHandler;
use crate::monitor::{connection_key, TrafficMonitor};
use async_trait::async_trait;
use parking_lot::RwLock;
use shared_types::ConnectionSnapshot;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::watch;
use tracing::warn;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendMode {
    Poll,
    Event,
}

pub struct MonitorContext {
    pub monitor: Arc<TrafficMonitor>,
    pub handler: Arc<dyn ConnectionHandler>,
    pub shutdown: watch::Receiver<bool>,
    pub cleaner_interval_ms: u64,
}

/// Handles new connections and lifecycle pruning for all backends.
pub struct MonitorConnectionSink {
    monitor: Arc<TrafficMonitor>,
    handler: Arc<dyn ConnectionHandler>,
    known_keys: RwLock<HashSet<String>>,
}

impl MonitorConnectionSink {
    pub fn new(monitor: Arc<TrafficMonitor>, handler: Arc<dyn ConnectionHandler>) -> Self {
        Self {
            monitor,
            handler,
            known_keys: RwLock::new(HashSet::new()),
        }
    }

    /// Mark connection as known without invoking the handler (startup bootstrap).
    pub fn seed_known(&self, conn: &ConnectionSnapshot) {
        if !is_processable_connection(conn) {
            return;
        }
        self.known_keys.write().insert(connection_key(conn));
    }

    /// Process a connection if newly seen (non-blocking handler dispatch).
    pub fn try_new_connection(&self, conn: ConnectionSnapshot) {
        if !is_processable_connection(&conn) {
            return;
        }
        let key = connection_key(&conn);
        if self.known_keys.read().contains(&key) {
            return;
        }
        if !self.known_keys.write().insert(key) {
            return;
        }
        self.monitor.track_connection(&conn);
        self.monitor.broadcast_connection(conn.clone());
        let handler = Arc::clone(&self.handler);
        tokio::spawn(async move {
            handler.on_connection(conn).await;
        });
    }

    pub fn prune_to_active(&self, connections: &[ConnectionSnapshot]) {
        self.monitor.replace_active_connections(connections);
        let active_keys: HashSet<String> = connections.iter().map(connection_key).collect();
        self.known_keys.write().retain(|k| active_keys.contains(k));
    }
}

#[async_trait]
pub trait ConnectionBackend: Send + Sync {
    fn mode(&self) -> BackendMode;
    fn name(&self) -> &'static str;
    async fn run(&self, ctx: MonitorContext) -> Result<(), String>;
}

/// Resolve backend name to implementation. Packet backends fall back to iphlpapi when WinDivert is unavailable.
pub fn create_connection_backend(backend_name: &str) -> Arc<dyn ConnectionBackend> {
    let normalized = backend_name.to_ascii_lowercase();
    match normalized.as_str() {
        "iphlpapi" | "etw" => {
            if normalized == "etw" {
                warn!("traffic_monitor_backend=etw is deprecated; using iphlpapi polling");
            }
            Arc::new(crate::iphlpapi::IphlpapiBackend)
        }
        "packet" | "windivert" => {
            #[cfg(windows)]
            {
                if windivert_engine::capture_available().is_ok() {
                    return Arc::new(crate::packet::PacketConnectionBackend);
                }
                warn!("WinDivert capture unavailable — falling back to iphlpapi polling");
            }
            Arc::new(crate::iphlpapi::IphlpapiBackend)
        }
        _ => {
            warn!(backend = %backend_name, "unknown traffic backend; using packet default");
            create_connection_backend("packet")
        }
    }
}

/// Spawn the monitor loop for the selected backend.
pub fn spawn_monitor(
    monitor: Arc<TrafficMonitor>,
    handler: Arc<dyn ConnectionHandler>,
    shutdown: watch::Receiver<bool>,
    backend: Arc<dyn ConnectionBackend>,
    cleaner_interval_ms: u64,
) -> tokio::task::JoinHandle<()> {
    let ctx = MonitorContext {
        monitor,
        handler,
        shutdown,
        cleaner_interval_ms,
    };
    let name = backend.name();
    tokio::spawn(async move {
        tracing::info!(backend = name, "traffic monitor backend starting");
        if let Err(e) = backend.run(ctx).await {
            tracing::error!(backend = name, error = %e, "traffic monitor backend exited");
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_backend_selects_iphlpapi_by_name() {
        let backend = create_connection_backend("iphlpapi");
        assert_eq!(backend.name(), "iphlpapi");
        assert_eq!(backend.mode(), BackendMode::Poll);
    }

    #[test]
    fn create_backend_packet_falls_back_without_windivert() {
        let backend = create_connection_backend("packet");
        #[cfg(not(windows))]
        assert_eq!(backend.name(), "iphlpapi");
        #[cfg(windows)]
        {
            let _ = backend.name();
        }
    }

    #[test]
    fn create_backend_maps_etw_to_iphlpapi() {
        let backend = create_connection_backend("etw");
        assert_eq!(backend.name(), "iphlpapi");
    }
}
