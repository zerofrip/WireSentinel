use crate::backend::{TransportBackend, TransportContext};
use crate::config_store::TransportConfigStore;
use crate::process_manager::ProcessManager;
use crate::singbox::config::{build_config, SingBoxOutboundSpec};
use async_trait::async_trait;
use parking_lot::RwLock;
use shared_types::{Result, TransportHealth, TransportKind, TransportState, WireSentinelError};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

const DEFAULT_LISTEN_PORT: u16 = 1080;

/// sing-box transport: generates JSON config, writes to disk, supervises process.
pub struct SingBoxTransport {
    process_manager: Arc<ProcessManager>,
    config_store: Arc<TransportConfigStore>,
    instance_id: RwLock<Option<Uuid>>,
    listen_port: RwLock<u16>,
    state: RwLock<TransportState>,
    default_binary: PathBuf,
}

impl SingBoxTransport {
    pub fn new(
        process_manager: Arc<ProcessManager>,
        config_store: Arc<TransportConfigStore>,
    ) -> Self {
        Self {
            process_manager,
            config_store,
            instance_id: RwLock::new(None),
            listen_port: RwLock::new(DEFAULT_LISTEN_PORT),
            state: RwLock::new(TransportState::Stopped),
            default_binary: PathBuf::from("sing-box.exe"),
        }
    }

    pub fn with_binary(mut self, path: PathBuf) -> Self {
        self.default_binary = path;
        self
    }

    fn resolve_outbound(ctx: &TransportContext) -> Result<SingBoxOutboundSpec> {
        if let Some(profile) = &ctx.transport_profile {
            if let Some(json) = &profile.config_json {
                return SingBoxOutboundSpec::from_json(json);
            }
        }
        Ok(SingBoxOutboundSpec {
            protocol: crate::singbox::config::SingBoxProtocol::Socks,
            server: "127.0.0.1".into(),
            server_port: 1080,
            uuid: None,
            password: None,
            method: None,
            flow: None,
            tls: false,
            sni: None,
            network: None,
            ws_path: None,
            ws_host: None,
        })
    }
}

#[async_trait]
impl TransportBackend for SingBoxTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::SingBox
    }

    async fn start(&self, ctx: &TransportContext) -> Result<()> {
        *self.state.write() = TransportState::Starting;

        let listen_port = ctx.listen_port.unwrap_or(DEFAULT_LISTEN_PORT);
        *self.listen_port.write() = listen_port;

        let outbound = Self::resolve_outbound(ctx)?;
        let config = build_config(listen_port, &outbound, ctx.upstream_socks.as_deref());
        let config_path = self.config_store.write_json(ctx.id, &config)?;

        let binary = ctx
            .transport_profile
            .as_ref()
            .and_then(|p| p.binary_path.clone())
            .unwrap_or_else(|| self.default_binary.clone());

        self.process_manager
            .spawn(ctx.id, &binary, &["run", "-c"], &config_path)
            .await?;

        *self.instance_id.write() = Some(ctx.id);
        *self.state.write() = TransportState::Running;
        info!(name = %ctx.name, port = listen_port, "sing-box transport started");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        *self.state.write() = TransportState::Stopping;
        let instance_id = self.instance_id.write().take();
        if let Some(id) = instance_id {
            self.process_manager.kill(id).await?;
            let _ = self.config_store.delete(id);
        }
        *self.state.write() = TransportState::Stopped;
        Ok(())
    }

    fn status(&self) -> TransportState {
        *self.state.read()
    }

    async fn health_check(&self) -> TransportHealth {
        let id = match *self.instance_id.read() {
            Some(id) => id,
            None => {
                return TransportHealth {
                    healthy: false,
                    latency_ms: None,
                    message: Some("not started".into()),
                };
            }
        };

        let running = self.process_manager.is_running(id);
        TransportHealth {
            healthy: running,
            latency_ms: if running { Some(1) } else { None },
            message: Some(if running {
                format!("sing-box listening on 127.0.0.1:{}", *self.listen_port.read())
            } else {
                "process not running".into()
            }),
        }
    }
}
