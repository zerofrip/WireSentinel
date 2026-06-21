use crate::amnezia::AmneziaWGTransport;
use crate::backend::TransportBackend;
use crate::config_store::TransportConfigStore;
use crate::direct::DirectTransport;
use crate::mixnet::MixnetTransport;
use crate::process_manager::ProcessManager;
use crate::singbox::SingBoxTransport;
use crate::tls_tunnel::TlsTunnelTransport;
use crate::tor::TorTransport;
use crate::wireguard::WireGuardTransport;
use crate::ws_tunnel::WebSocketTunnelTransport;
use crate::xray::XrayTransport;
use shared_types::TransportKind;
use std::sync::Arc;
use vpn_engine::VpnBackend;

/// Creates transport backend instances by kind.
pub struct TransportBackendFactory {
    wireguard: Arc<dyn VpnBackend>,
    amnezia: Arc<dyn VpnBackend>,
    process_manager: Arc<ProcessManager>,
    config_store: Arc<TransportConfigStore>,
}

impl TransportBackendFactory {
    pub fn new(
        wireguard: Arc<dyn VpnBackend>,
        amnezia: Arc<dyn VpnBackend>,
        process_manager: Arc<ProcessManager>,
        config_store: Arc<TransportConfigStore>,
    ) -> Self {
        Self {
            wireguard,
            amnezia,
            process_manager,
            config_store,
        }
    }

    pub fn process_manager(&self) -> Arc<ProcessManager> {
        Arc::clone(&self.process_manager)
    }

    pub fn config_store(&self) -> Arc<TransportConfigStore> {
        Arc::clone(&self.config_store)
    }

    pub fn create(&self, kind: TransportKind) -> Arc<dyn TransportBackend> {
        match kind {
            TransportKind::Direct => Arc::new(DirectTransport::new()),
            TransportKind::WireGuard => {
                Arc::new(WireGuardTransport::new(Arc::clone(&self.wireguard)))
            }
            TransportKind::AmneziaWg => {
                Arc::new(AmneziaWGTransport::new(Arc::clone(&self.amnezia)))
            }
            TransportKind::SingBox => Arc::new(SingBoxTransport::new(
                Arc::clone(&self.process_manager),
                Arc::clone(&self.config_store),
            )),
            TransportKind::Xray => Arc::new(XrayTransport::new(
                Arc::clone(&self.process_manager),
                Arc::clone(&self.config_store),
            )),
            TransportKind::Tor => Arc::new(TorTransport::new(
                Arc::clone(&self.process_manager),
                Arc::clone(&self.config_store),
            )),
            TransportKind::TlsTunnel => Arc::new(TlsTunnelTransport::new(
                Arc::clone(&self.process_manager),
                Arc::clone(&self.config_store),
            )),
            TransportKind::WebSocketTunnel => Arc::new(WebSocketTunnelTransport::new(
                Arc::clone(&self.process_manager),
                Arc::clone(&self.config_store),
            )),
            TransportKind::Mixnet => Arc::new(MixnetTransport::new()),
            TransportKind::Proxy => Arc::new(DirectTransport::new()),
        }
    }
}
