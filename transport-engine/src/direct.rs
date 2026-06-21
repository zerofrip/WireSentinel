use crate::backend::{TransportBackend, TransportContext};
use async_trait::async_trait;
use parking_lot::RwLock;
use shared_types::{Result, TransportHealth, TransportKind, TransportState};
use tracing::info;

/// Passthrough transport — no tunnel, immediately running.
pub struct DirectTransport {
    state: RwLock<TransportState>,
}

impl DirectTransport {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(TransportState::Stopped),
        }
    }
}

impl Default for DirectTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TransportBackend for DirectTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::Direct
    }

    async fn start(&self, ctx: &TransportContext) -> Result<()> {
        info!(name = %ctx.name, "direct transport start (passthrough)");
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
            message: Some("direct passthrough".into()),
        }
    }
}
