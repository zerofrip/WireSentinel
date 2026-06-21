//! SOCKS5-based WireGuard handshake obfuscation.

mod socks5_handshake;

pub use socks5_handshake::{PreparedHandshake, Socks5HandshakeBackend};
