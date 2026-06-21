//! VPN backend factory and profile-driven backend selection.

mod amnezia;
mod backend;
mod conf;
mod factory;
mod handshake_proxy;
mod manager;
mod materialize;
mod native;
mod obfuscation;
mod stub;

#[cfg(windows)]
mod scm;
mod tailscale;

pub use amnezia::AmneziaWgBackend;
pub use backend::VpnBackend;
pub use conf::{detect_backend, encode_awg_config, parse_conf, write_conf, WireGuardConfig};
pub use factory::{default_dll_path, default_factory, VpnBackendFactory, VpnBackendKind};
pub use handshake_proxy::{apply_handshake_proxy, HandshakeProxySession};
pub use manager::VpnManager;
pub use materialize::{is_db_path, materialize_profile_config, vpn_config_dir};
pub use native::{NativeAmneziaWgBackend, NativeWireGuardBackend};
pub use obfuscation::{PreparedHandshake, Socks5HandshakeBackend};
pub use tailscale::{TailscaleBackend, TailscaleRuntimeStatus};

#[cfg(windows)]
pub use scm::ScmTunnelDllBackend;

#[cfg(not(windows))]
pub use stub::StubVpnBackend as ScmTunnelDllBackend;
