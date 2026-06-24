//! Phase 4 transport engine — backends, chains, and external proxy supervision.

mod amnezia;
mod backend;
mod chain;
mod config_store;
mod direct;
mod factory;
mod mixnet;
mod process_manager;
mod proxy;
mod singbox;
mod tls_tunnel;
mod tor;
mod wireguard;
mod ws_tunnel;
mod xray;

pub use amnezia::AmneziaWGTransport;
pub use backend::{TransportBackend, TransportContext};
pub use chain::{validate_chain, ChainOrchestrator};
pub use config_store::{transports_dir, TransportConfigStore};
pub use direct::DirectTransport;
pub use factory::TransportBackendFactory;
pub use mixnet::MixnetTransport;
pub use process_manager::ProcessManager;
pub use singbox::{
    bridge_line, bridges_to_tor_options, build_config as build_singbox_config,
    build_split_tunnel_route_rules, build_tor_config, build_tun_config, SingBoxOutboundSpec,
    SingBoxProtocol, SingBoxTransport, TorOutboundSpec, TorSingBoxRunner,
};
pub use tls_tunnel::TlsTunnelTransport;
pub use tor::{BridgeManager, BridgeTestResult, TorTransport};
pub use wireguard::WireGuardTransport;
pub use ws_tunnel::WebSocketTunnelTransport;
pub use xray::{build_config as build_xray_config, XrayOutboundSpec, XrayProtocol, XrayTransport};
