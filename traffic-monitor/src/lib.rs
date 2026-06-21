//! Network traffic monitoring via iphlpapi polling and bandwidth aggregation.

mod etw;
mod handler;
mod monitor;
mod stub;

pub use etw::{backend_from_settings, TrafficBackend};

#[cfg(windows)]
mod windows;

pub use handler::ConnectionHandler;
pub use monitor::{spawn_monitor, TrafficMonitor};
