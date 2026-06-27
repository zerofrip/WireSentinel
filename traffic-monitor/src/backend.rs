//! Connection source backends (packet event-driven and iphlpapi polling).

use crate::filter::is_processable_connection;
use crate::handler::ConnectionHandler;
use crate::monitor::{connection_key, TrafficMonitor};
use async_trait::async_trait;
use parking_lot::RwLock;
use shared_types::ConnectionSnapshot;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, watch};
use tracing::warn;

/// Fixed number of workers draining the connection queue. Bounds how many
/// `on_connection` handlers (and their DB work) run concurrently.
const CONN_WORKER_COUNT: usize = 4;
/// Bounded queue depth. Bursts up to this many connections are buffered; beyond
/// that, new connections are dropped to apply backpressure instead of growing memory.
const CONN_QUEUE_CAPACITY: usize = 256;

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
    known_keys: RwLock<HashSet<String>>,
    tx: mpsc::Sender<ConnectionSnapshot>,
    dropped: AtomicU64,
    last_drop_warn: RwLock<Option<Instant>>,
}

impl MonitorConnectionSink {
    pub fn new(monitor: Arc<TrafficMonitor>, handler: Arc<dyn ConnectionHandler>) -> Self {
        Self::with_capacity(monitor, handler, CONN_WORKER_COUNT, CONN_QUEUE_CAPACITY)
    }

    /// Build a sink backed by a bounded queue and a fixed pool of workers that
    /// invoke the handler, so connection processing concurrency stays capped.
    pub fn with_capacity(
        monitor: Arc<TrafficMonitor>,
        handler: Arc<dyn ConnectionHandler>,
        workers: usize,
        capacity: usize,
    ) -> Self {
        let (tx, rx) = mpsc::channel::<ConnectionSnapshot>(capacity.max(1));
        // #region agent log
        shared_types::debug_log::emit_kv(
            "traffic-monitor/src/backend.rs:with_capacity",
            "connection worker pool started",
            &[
                ("hypothesisId", "DEPLOY_D".to_string()),
                ("workers", workers.max(1).to_string()),
                ("queue_capacity", capacity.max(1).to_string()),
            ],
        );
        // #endregion
        let rx = Arc::new(tokio::sync::Mutex::new(rx));
        for _ in 0..workers.max(1) {
            let rx = Arc::clone(&rx);
            let handler = Arc::clone(&handler);
            tokio::spawn(async move {
                loop {
                    let next = {
                        let mut guard = rx.lock().await;
                        guard.recv().await
                    };
                    match next {
                        Some(conn) => handler.on_connection(conn).await,
                        None => break,
                    }
                }
            });
        }
        Self {
            monitor,
            known_keys: RwLock::new(HashSet::new()),
            tx,
            dropped: AtomicU64::new(0),
            last_drop_warn: RwLock::new(None),
        }
    }

    /// Mark connection as known without invoking the handler (startup bootstrap).
    pub fn seed_known(&self, conn: &ConnectionSnapshot) {
        if !is_processable_connection(conn) {
            return;
        }
        self.known_keys.write().insert(connection_key(conn));
    }

    /// Enqueue a connection for processing if newly seen. Non-blocking: when the
    /// worker queue is full the connection is dropped (and may be retried on a
    /// later poll) rather than spawning unbounded work.
    pub fn try_new_connection(&self, conn: ConnectionSnapshot) {
        if !is_processable_connection(&conn) {
            return;
        }
        let key = connection_key(&conn);
        if self.known_keys.read().contains(&key) {
            return;
        }
        if !self.known_keys.write().insert(key.clone()) {
            return;
        }
        self.monitor.track_connection(&conn);
        self.monitor.broadcast_connection(conn.clone());
        if self.tx.try_send(conn).is_err() {
            self.known_keys.write().remove(&key);
            self.note_dropped();
        }
    }

    fn note_dropped(&self) {
        let total = self.dropped.fetch_add(1, Ordering::Relaxed) + 1;
        let now = Instant::now();
        let mut last = self.last_drop_warn.write();
        if last
            .map(|t| now.duration_since(t) >= Duration::from_secs(60))
            .unwrap_or(true)
        {
            warn!(
                dropped_total = total,
                "connection processing queue full; shedding new connections"
            );
            // #region agent log
            shared_types::debug_log::emit_kv(
                "traffic-monitor/src/backend.rs:note_dropped",
                "connection queue full; shedding",
                &[
                    ("hypothesisId", "DEPLOY_D".to_string()),
                    ("dropped_total", total.to_string()),
                ],
            );
            // #endregion
            *last = Some(now);
        }
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
