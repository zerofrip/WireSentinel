//! Phase 4 transport engine — backends, chains, and external proxy supervision.

mod amnezia;
mod backend;
mod chain;
mod config_store;
mod direct;
mod mixnet;
mod factory;
mod process_manager;
mod singbox;
mod tls_tunnel;
mod wireguard;
mod ws_tunnel;
mod xray;
mod tor;

pub use amnezia::AmneziaWGTransport;
pub use backend::{TransportBackend, TransportContext};
pub use chain::{validate_chain, ChainOrchestrator};
pub use config_store::{transports_dir, TransportConfigStore};
pub use direct::DirectTransport;
pub use mixnet::MixnetTransport;
pub use factory::TransportBackendFactory;
pub use process_manager::ProcessManager;
pub use singbox::{build_config as build_singbox_config, SingBoxOutboundSpec, SingBoxProtocol, SingBoxTransport};
pub use tls_tunnel::TlsTunnelTransport;
pub use tor::{BridgeManager, BridgeTestResult, TorTransport};
pub use wireguard::WireGuardTransport;
pub use ws_tunnel::WebSocketTunnelTransport;
pub use xray::{build_config as build_xray_config, XrayOutboundSpec, XrayProtocol, XrayTransport};
