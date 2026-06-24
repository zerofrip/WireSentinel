use crate::singbox::TorOutboundSpec;
use async_trait::async_trait;
use shared_types::{
    ObfuscationPreset, ProxyProfile, Result, TransportHealth, TransportKind, TransportProfile,
    TransportState, VPNProfile,
};
use std::path::PathBuf;
use uuid::Uuid;

/// Runtime context passed when starting a transport instance.
#[derive(Debug, Clone)]
pub struct TransportContext {
    pub id: Uuid,
    pub name: String,
    pub vpn_profile: Option<VPNProfile>,
    pub transport_profile: Option<TransportProfile>,
    pub config_path: Option<PathBuf>,
    /// Local listen port for proxy transports (sing-box / xray inbounds).
    pub listen_port: Option<u16>,
    /// Upstream SOCKS address from the previous chain hop (e.g. `127.0.0.1:1080`).
    pub upstream_socks: Option<String>,
    /// Obfuscation preset applied to proxy transport configs at start time.
    pub obfuscation_preset: Option<ObfuscationPreset>,
    /// Tor outbound spec when starting `TransportKind::Tor`.
    pub tor_spec: Option<TorOutboundSpec>,
    /// SOCKS5/HTTP proxy profile for `TransportKind::Proxy` hops.
    pub proxy_profile: Option<ProxyProfile>,
    /// Upstream mixnet SOCKS (`host:port`) for `TransportKind::Mixnet` hops.
    pub mixnet_upstream: Option<String>,
}

impl TransportContext {
    pub fn new(id: Uuid, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            vpn_profile: None,
            transport_profile: None,
            config_path: None,
            listen_port: None,
            upstream_socks: None,
            obfuscation_preset: None,
            tor_spec: None,
            proxy_profile: None,
            mixnet_upstream: None,
        }
    }
}

#[async_trait]
pub trait TransportBackend: Send + Sync {
    fn kind(&self) -> TransportKind;
    async fn start(&self, ctx: &TransportContext) -> Result<()>;
    async fn stop(&self) -> Result<()>;
    fn status(&self) -> TransportState;
    async fn health_check(&self) -> TransportHealth;
}
