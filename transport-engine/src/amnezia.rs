use crate::backend::{TransportBackend, TransportContext};
use async_trait::async_trait;
use parking_lot::RwLock;
use shared_types::{
    Result, TransportHealth, TransportKind, TransportState, VpnStatus, WireSentinelError,
};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;
use vpn_engine::VpnBackend;

/// AmneziaWG transport delegating to the Amnezia VPN backend.
pub struct AmneziaWGTransport {
    backend: Arc<dyn VpnBackend>,
    active_profile: RwLock<Option<Uuid>>,
    state: RwLock<TransportState>,
}

impl AmneziaWGTransport {
    pub fn new(backend: Arc<dyn VpnBackend>) -> Self {
        Self {
            backend,
            active_profile: RwLock::new(None),
            state: RwLock::new(TransportState::Stopped),
        }
    }
}

#[async_trait]
impl TransportBackend for AmneziaWGTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::AmneziaWg
    }

    async fn start(&self, ctx: &TransportContext) -> Result<()> {
        let profile = ctx.vpn_profile.as_ref().ok_or_else(|| {
            WireSentinelError::Other("amnezia transport requires vpn_profile".into())
        })?;

        *self.state.write() = TransportState::Starting;
        info!(name = %ctx.name, profile = %profile.name, "starting amnezia transport");

        self.backend.connect(profile).await?;
        *self.active_profile.write() = Some(profile.id);
        *self.state.write() = TransportState::Running;
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        *self.state.write() = TransportState::Stopping;
        let profile_id = self.active_profile.write().take();
        if let Some(id) = profile_id {
            self.backend.disconnect(id).await?;
        }
        *self.state.write() = TransportState::Stopped;
        Ok(())
    }

    fn status(&self) -> TransportState {
        *self.state.read()
    }

    async fn health_check(&self) -> TransportHealth {
        let Some(id) = *self.active_profile.read() else {
            return TransportHealth {
                healthy: false,
                latency_ms: None,
                message: Some("no active profile".into()),
            };
        };
        let connected = self.backend.status(id).await == VpnStatus::Connected;
        TransportHealth {
            healthy: connected,
            latency_ms: None,
            message: if connected {
                Some("amnezia connected".into())
            } else {
                Some("amnezia disconnected".into())
            },
        }
    }
}
