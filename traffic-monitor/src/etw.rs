//! ETW-based traffic monitoring (Windows). Falls back to iphlpapi polling bridge.

use shared_types::ConnectionSnapshot;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficBackend {
    Iphlpapi,
    Etw,
}

pub struct EtwMonitor {
    tx: Sender<ConnectionSnapshot>,
}

impl EtwMonitor {
    pub fn new() -> (Self, Receiver<ConnectionSnapshot>) {
        let (tx, rx) = mpsc::channel();
        (Self { tx }, rx)
    }

    pub fn spawn(self) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            #[cfg(windows)]
            etw_loop(self.tx);
            #[cfg(not(windows))]
            let _ = self.tx;
        })
    }
}

/// Poll connections via the iphlpapi bridge (used when ETW backend is selected).
pub fn poll_connections() -> Vec<ConnectionSnapshot> {
    #[cfg(windows)]
    {
        crate::windows::enumerate_tcp_connections()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[cfg(windows)]
fn etw_loop(tx: Sender<ConnectionSnapshot>) {
    loop {
        for conn in poll_connections() {
            if tx.send(conn).is_err() {
                return;
            }
        }
        thread::sleep(Duration::from_millis(500));
    }
}

pub fn backend_from_settings(s: &str) -> TrafficBackend {
    match s.to_ascii_lowercase().as_str() {
        "etw" => TrafficBackend::Etw,
        _ => TrafficBackend::Iphlpapi,
    }
}
