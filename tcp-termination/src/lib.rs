//! TCP socket termination engine for WireSock-style reconnect policies.

mod engine;
mod platform;

pub use engine::TcpTerminationEngine;
pub use platform::{default_terminator, TcpSessionTerminator};
