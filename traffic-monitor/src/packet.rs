//! WinDivert packet-event connection backend.

use crate::backend::{BackendMode, ConnectionBackend, MonitorConnectionSink, MonitorContext};
use async_trait::async_trait;
use shared_types::ConnectionSnapshot;
use std::collections::HashSet;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use windivert_engine::{parse_packet, WinDivertCapture};

const PACKET_FILTER: &str = "(outbound and tcp.Syn == 1) or (outbound and udp)";

pub struct PacketConnectionBackend;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FlowKey {
    pid: u32,
    local: String,
    remote: String,
}

impl FlowKey {
    fn from_snapshot(conn: &ConnectionSnapshot) -> Self {
        Self {
            pid: conn.pid,
            local: conn.local_addr.to_string(),
            remote: conn.remote_addr.to_string(),
        }
    }
}

struct ConnectionTracker {
    flows: HashSet<FlowKey>,
}

impl ConnectionTracker {
    fn new() -> Self {
        Self {
            flows: HashSet::new(),
        }
    }

    fn insert_snapshot(&mut self, conn: &ConnectionSnapshot) -> bool {
        self.flows.insert(FlowKey::from_snapshot(conn))
    }

    fn retain_active(&mut self, connections: &[ConnectionSnapshot]) {
        let active: HashSet<FlowKey> = connections.iter().map(FlowKey::from_snapshot).collect();
        self.flows.retain(|k| active.contains(k));
    }
}

#[async_trait]
impl ConnectionBackend for PacketConnectionBackend {
    fn mode(&self) -> BackendMode {
        BackendMode::Event
    }

    fn name(&self) -> &'static str {
        "packet"
    }

    async fn run(&self, ctx: MonitorContext) -> Result<(), String> {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<ConnectionSnapshot>(1024);
        let capture_shutdown = ctx.shutdown.clone();

        let capture_handle = thread::spawn(move || packet_capture_loop(tx, capture_shutdown));

        let sink = Arc::new(MonitorConnectionSink::new(
            ctx.monitor.clone(),
            ctx.handler.clone(),
        ));
        let mut tracker = ConnectionTracker::new();
        let cleaner_ms = ctx.cleaner_interval_ms.max(1000);
        let mut cleaner = tokio::time::interval(Duration::from_millis(cleaner_ms));
        cleaner.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut shutdown = ctx.shutdown;

        loop {
            tokio::select! {
                _ = cleaner.tick() => {
                    let active = collect_tcp_connections();
                    sink.prune_to_active(&active);
                    tracker.retain_active(&active);
                }
                conn = rx.recv() => {
                    match conn {
                        Some(conn) => {
                            if tracker.insert_snapshot(&conn) {
                                sink.on_new_connection(conn).await;
                            }
                        }
                        None => break,
                    }
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        break;
                    }
                }
            }
        }

        let _ = capture_handle.join();
        Ok(())
    }
}

fn packet_capture_loop(
    tx: tokio::sync::mpsc::Sender<ConnectionSnapshot>,
    shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let capture = match WinDivertCapture::open_sniff(PACKET_FILTER) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "WinDivert packet capture failed to open");
            return;
        }
    };

    loop {
        if *shutdown.borrow() {
            break;
        }
        match capture.recv_blocking() {
            Ok(pkt) => {
                let Some(flow) = parse_packet(&pkt.data, pkt.meta.outbound, pkt.meta.ipv6) else {
                    continue;
                };
                let conn = ConnectionSnapshot {
                    pid: pkt.meta.process_id,
                    app_id: None,
                    exe_name: format!("pid:{}", pkt.meta.process_id),
                    protocol: flow.protocol,
                    local_addr: flow.local,
                    remote_addr: flow.remote,
                    state: "packet:new".into(),
                    remote_domain: None,
                    bytes_sent: 0,
                    bytes_received: 0,
                };
                if tx.blocking_send(conn).is_err() {
                    break;
                }
            }
            Err(_) => {
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

fn collect_tcp_connections() -> Vec<ConnectionSnapshot> {
    crate::windows::enumerate_tcp_connections()
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::Protocol;
    use std::net::SocketAddr;

    #[test]
    fn connection_tracker_dedup() {
        let mut t = ConnectionTracker::new();
        let conn = ConnectionSnapshot {
            pid: 1,
            app_id: None,
            exe_name: "pid:1".into(),
            protocol: Protocol::Tcp,
            local_addr: "127.0.0.1:1234".parse().unwrap(),
            remote_addr: "93.184.216.34:443".parse().unwrap(),
            state: "new".into(),
            remote_domain: None,
            bytes_sent: 0,
            bytes_received: 0,
        };
        assert!(t.insert_snapshot(&conn));
        assert!(!t.insert_snapshot(&conn));
    }
}
