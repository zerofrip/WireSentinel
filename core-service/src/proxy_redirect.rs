//! Proxy redirect integration for NDIS datapath.

use crate::kernel_route_bridge::KernelRouteBridge;
use proxy_engine::ProxyListenPort;
use shared_types::{AppIdentity, Result, TrafficRoute, TunnelIface};
use std::sync::Arc;

pub struct ProxyRedirectEngine {
    bridge: Arc<KernelRouteBridge>,
    listen_ports: Arc<ProxyListenPort>,
}

impl ProxyRedirectEngine {
    pub fn new(bridge: Arc<KernelRouteBridge>, listen_ports: Arc<ProxyListenPort>) -> Self {
        Self {
            bridge,
            listen_ports,
        }
    }

    pub fn bridge(&self) -> Arc<KernelRouteBridge> {
        Arc::clone(&self.bridge)
    }

    pub async fn apply(
        &self,
        app: &AppIdentity,
        route: &TrafficRoute,
        tunnel: Option<TunnelIface>,
    ) -> Result<()> {
        if !matches!(route, TrafficRoute::Proxy(_) | TrafficRoute::ProxyChain(_)) {
            return Ok(());
        }
        let socks_port = route
            .profile_id()
            .and_then(|id| self.listen_ports.get(id))
            .or_else(|| tunnel.as_ref().and_then(|t| t.socks_port));
        let proxy_tunnel = route.profile_id().map(|profile_id| TunnelIface {
            profile_id,
            name: "proxy".into(),
            luid: 0,
            socks_port,
        });
        self.bridge
            .sync_enforce(app, route, proxy_tunnel.or(tunnel))
            .await
    }
}
