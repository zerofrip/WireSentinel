use crate::backend::{TransportBackend, TransportContext};
use crate::config_store::TransportConfigStore;
use crate::process_manager::ProcessManager;
use crate::singbox::config::{build_config, SingBoxOutboundSpec, SingBoxProtocol};
use async_trait::async_trait;
use parking_lot::RwLock;
use shared_types::{
    Result, TransportHealth, TransportKind, TransportState, WebSocketTunnelConfig,
    WireSentinelError,
};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

const DEFAULT_LISTEN_PORT: u16 = 1083;

/// WebSocket transport tunnel via sing-box (local mixed inbound → WS outbound).
pub struct WebSocketTunnelTransport {
    process_manager: Arc<ProcessManager>,
    config_store: Arc<TransportConfigStore>,
    instance_id: RwLock<Option<Uuid>>,
    listen_port: RwLock<u16>,
    state: RwLock<TransportState>,
    default_binary: PathBuf,
}

impl WebSocketTunnelTransport {
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

    fn resolve_config(ctx: &TransportContext) -> Result<WebSocketTunnelConfig> {
        if let Some(profile) = &ctx.transport_profile {
            if let Some(json) = &profile.config_json {
                return serde_json::from_str(json).map_err(WireSentinelError::Serde);
            }
        }
        Err(WireSentinelError::Other(
            "websocket tunnel requires transport_profile.config_json".into(),
        ))
    }

    fn build_ws_outbound(cfg: &WebSocketTunnelConfig) -> SingBoxOutboundSpec {
        let (host, port, path) = parse_ws_url(&cfg.url);
        SingBoxOutboundSpec {
            protocol: SingBoxProtocol::Vless,
            server: host,
            server_port: port.unwrap_or(if cfg.tls { 443 } else { 80 }),
            uuid: None,
            password: None,
            method: None,
            flow: None,
            tls: cfg.tls,
            sni: cfg.host_header.clone(),
            network: Some("ws".into()),
            ws_path: cfg.path.clone().or(path),
            ws_host: cfg.host_header.clone(),
        }
    }
}

fn parse_ws_url(raw: &str) -> (String, Option<u16>, Option<String>) {
    let stripped = raw
        .trim()
        .strip_prefix("wss://")
        .or_else(|| raw.trim().strip_prefix("ws://"))
        .or_else(|| raw.trim().strip_prefix("https://"))
        .or_else(|| raw.trim().strip_prefix("http://"))
        .unwrap_or(raw.trim());
    let (host_port, path) = match stripped.split_once('/') {
        Some((hp, rest)) => (hp, Some(format!("/{rest}"))),
        None => (stripped, None),
    };
    if let Some((host, port)) = host_port.rsplit_once(':') {
        if let Ok(p) = port.parse() {
            return (host.to_string(), Some(p), path);
        }
    }
    (host_port.to_string(), None, path)
}

#[async_trait]
impl TransportBackend for WebSocketTunnelTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::WebSocketTunnel
    }

    async fn start(&self, ctx: &TransportContext) -> Result<()> {
        *self.state.write() = TransportState::Starting;
        let listen_port = ctx.listen_port.unwrap_or(DEFAULT_LISTEN_PORT);
        *self.listen_port.write() = listen_port;

        let ws_cfg = Self::resolve_config(ctx)?;
        let outbound = Self::build_ws_outbound(&ws_cfg);
        let mut config = build_config(listen_port, &outbound, ctx.upstream_socks.as_deref());

        if let Some(preset) = ctx.obfuscation_preset {
            dpi_transforms::TransformPipeline::from_preset(preset).apply_to_config(&mut config);
        }

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
        info!(name = %ctx.name, port = listen_port, url = %ws_cfg.url, "websocket tunnel started");
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
                format!("ws tunnel on 127.0.0.1:{}", *self.listen_port.read())
            } else {
                "process not running".into()
            }),
        }
    }
}
