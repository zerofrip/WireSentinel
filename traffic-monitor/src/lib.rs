//! Network traffic monitoring via packet events or iphlpapi polling.

mod backend;
mod filter;
mod handler;
mod iphlpapi;
mod monitor;
mod stub;

#[cfg(windows)]
mod packet;

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::enumerate_tcp_connections;

pub use backend::{
    create_connection_backend, spawn_monitor, BackendMode, ConnectionBackend, MonitorContext,
};
pub use handler::ConnectionHandler;
pub use monitor::{connection_key, TrafficMonitor};
