use crate::backend::{TransportBackend, TransportContext};
use crate::config_store::TransportConfigStore;
use crate::process_manager::ProcessManager;
use async_trait::async_trait;
use parking_lot::RwLock;
use shared_types::{Result, TransportHealth, TransportKind, TransportState};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

/// Tor SOCKS proxy transport (stub supervisor — marks running without spawning tor.exe).
pub struct TorTransport {
    process_manager: Arc<ProcessManager>,
    #[allow(dead_code)]
    config_store: Arc<TransportConfigStore>,
    instance_id: RwLock<Option<Uuid>>,
    state: RwLock<TransportState>,
    socks_port: RwLock<u16>,
}

impl TorTransport {
    pub fn new(
        process_manager: Arc<ProcessManager>,
        config_store: Arc<TransportConfigStore>,
    ) -> Self {
        Self {
            process_manager,
            config_store,
            instance_id: RwLock::new(None),
            state: RwLock::new(TransportState::Stopped),
            socks_port: RwLock::new(9050),
        }
    }
}

#[async_trait]
impl TransportBackend for TorTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::Tor
    }

    async fn start(&self, ctx: &TransportContext) -> Result<()> {
        *self.state.write() = TransportState::Starting;
        let port = ctx.listen_port.unwrap_or(9050);
        *self.socks_port.write() = port;
        *self.instance_id.write() = Some(ctx.id);
        *self.state.write() = TransportState::Running;
        info!(name = %ctx.name, port, "tor transport started (stub)");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        *self.state.write() = TransportState::Stopping;
        self.instance_id.write().take();
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
            message: Some(format!("tor socks 127.0.0.1:{}", *self.socks_port.read())),
        }
    }
}

/// Bridge profile test helper (stub).
pub struct BridgeManager;

impl BridgeManager {
    pub fn test_bridge(_bridge_type: &str, _config_json: &str) -> BridgeTestResult {
        BridgeTestResult {
            reachable: true,
            latency_ms: Some(100),
            message: Some("stub bridge test ok".into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BridgeTestResult {
    pub reachable: bool,
    pub latency_ms: Option<u64>,
    pub message: Option<String>,
}
