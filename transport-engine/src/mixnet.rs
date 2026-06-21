use crate::backend::{TransportBackend, TransportContext};
use async_trait::async_trait;
use parking_lot::RwLock;
use shared_types::{Result, TransportHealth, TransportKind, TransportState};
use tracing::info;

/// Stub mixnet transport — marks running without an external mixnet process.
pub struct MixnetTransport {
    state: RwLock<TransportState>,
}

impl MixnetTransport {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(TransportState::Stopped),
        }
    }
}

impl Default for MixnetTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TransportBackend for MixnetTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::Mixnet
    }

    async fn start(&self, ctx: &TransportContext) -> Result<()> {
        info!(name = %ctx.name, "mixnet transport start (stub)");
        *self.state.write() = TransportState::Running;
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        *self.state.write() = TransportState::Stopped;
        Ok(())
    }

    fn status(&self) -> TransportState {
        *self.state.read()
    }

    async fn health_check(&self) -> TransportHealth {
        TransportHealth {
            healthy: self.status() == TransportState::Running,
            latency_ms: Some(0),
            message: Some("mixnet stub".into()),
        }
    }
}
