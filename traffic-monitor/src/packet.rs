//! WinDivert packet-event connection backend.

use crate::backend::{BackendMode, ConnectionBackend, MonitorConnectionSink, MonitorContext};
use crate::filter::is_valid_pid;
use async_trait::async_trait;
use shared_types::ConnectionSnapshot;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use windivert_engine::{parse_packet, WinDivertCapture};

const PACKET_FILTER: &str = "outbound and tcp.Syn == 1";

pub struct PacketConnectionBackend;

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
        let cleaner_ms = ctx.cleaner_interval_ms.max(1000);
        let mut cleaner = tokio::time::interval(Duration::from_millis(cleaner_ms));
        cleaner.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut shutdown = ctx.shutdown;

        loop {
            tokio::select! {
                _ = cleaner.tick() => {
                    let active = collect_tcp_connections();
                    sink.prune_to_active(&active);
                }
                conn = rx.recv() => {
                    match conn {
                        Some(conn) => sink.try_new_connection(conn),
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
                if !is_valid_pid(pkt.meta.process_id) {
                    continue;
                }
                let Some(flow) = parse_packet(&pkt.data, pkt.meta.outbound, pkt.meta.ipv6) else {
                    continue;
                };
                if flow.remote.port() == 0 {
                    continue;
                }
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
