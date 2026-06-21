//! Phase 9 proxy engine — SOCKS/HTTP/HTTPS backends with local relay and failover.

mod backend;
mod chain;
mod context;
mod http;
mod https;
mod manager;
mod socks5;

pub use backend::{ProxyBackend, ProxyHealth, ProxyState, ProxyStatus};
pub use chain::{validate_hop_sequence, ChainValidationError};
pub use context::ProxyListenPort;
pub use manager::ProxyManager;
