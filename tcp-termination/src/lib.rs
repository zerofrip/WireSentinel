//! TCP socket termination engine for VPN gateway compatibility reconnect policies.

mod engine;
mod platform;

pub use engine::TcpTerminationEngine;
pub use platform::{default_terminator, TcpSessionTerminator};
