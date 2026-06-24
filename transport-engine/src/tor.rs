use crate::backend::{TransportBackend, TransportContext};
use crate::config_store::TransportConfigStore;
use crate::process_manager::ProcessManager;
use crate::singbox::{TorOutboundSpec, TorSingBoxRunner};
use async_trait::async_trait;
use parking_lot::RwLock;
use shared_types::{Result, TransportHealth, TransportKind, TransportState};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

/// Tor transport via sing-box `tor` outbound (supervises sing-box.exe + tor.exe).
pub struct TorTransport {
    runner: Arc<TorSingBoxRunner>,
    instance_id: RwLock<Option<Uuid>>,
    state: RwLock<TransportState>,
    socks_port: RwLock<u16>,
}

impl TorTransport {
    pub fn new(
        process_manager: Arc<ProcessManager>,
        config_store: Arc<TransportConfigStore>,
    ) -> Self {
        Self::with_binaries(
            process_manager,
            config_store,
            PathBuf::from("sing-box.exe"),
            PathBuf::from("tor.exe"),
        )
    }

    pub fn with_binaries(
        process_manager: Arc<ProcessManager>,
        config_store: Arc<TransportConfigStore>,
        singbox_binary: PathBuf,
        tor_binary: PathBuf,
    ) -> Self {
        let runner = Arc::new(TorSingBoxRunner::new(
            process_manager,
            config_store,
            singbox_binary,
            tor_binary,
        ));
        Self {
            runner,
            instance_id: RwLock::new(None),
            state: RwLock::new(TransportState::Stopped),
            socks_port: RwLock::new(9050),
        }
    }

    pub fn runner(&self) -> Arc<TorSingBoxRunner> {
        Arc::clone(&self.runner)
    }

    fn resolve_spec(ctx: &TransportContext) -> Result<TorOutboundSpec> {
        if let Some(spec) = &ctx.tor_spec {
            return Ok(spec.clone());
        }
        if let Some(profile) = &ctx.transport_profile {
            if let Some(json) = &profile.config_json {
                return TorOutboundSpec::from_json(json);
            }
        }
        let data_dir = std::env::temp_dir()
            .join("WireSentinel")
            .join("tor")
            .join(ctx.id.to_string());
        Ok(TorOutboundSpec {
            executable_path: PathBuf::from("tor.exe"),
            data_directory: data_dir,
            extra_args: vec![],
            torrc: [("ClientOnly".into(), "1".into())].into(),
        })
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

        let spec = Self::resolve_spec(ctx)?;
        self.runner
            .start(ctx.id, port, spec, ctx.upstream_socks.as_deref())
            .await?;

        *self.instance_id.write() = Some(ctx.id);
        *self.state.write() = TransportState::Running;
        info!(name = %ctx.name, port, "tor transport started via sing-box");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        *self.state.write() = TransportState::Stopping;
        let instance_id = self.instance_id.write().take();
        if let Some(id) = instance_id {
            self.runner.stop(id).await?;
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

        let port = *self.socks_port.read();
        let running = self.runner.is_running(id);
        let socks_ok = running
            && self
                .runner
                .wait_socks_ready(port, std::time::Duration::from_secs(3))
                .await
                .is_ok();

        TransportHealth {
            healthy: socks_ok,
            latency_ms: if socks_ok { Some(1) } else { None },
            message: Some(if socks_ok {
                format!("tor socks 127.0.0.1:{port}")
            } else if running {
                "tor process running, socks not ready".into()
            } else {
                "process not running".into()
            }),
        }
    }
}

/// Bridge profile test via short-lived sing-box tor instance.
pub struct BridgeManager {
    runner: Arc<TorSingBoxRunner>,
}

impl BridgeManager {
    pub fn new(runner: Arc<TorSingBoxRunner>) -> Self {
        Self { runner }
    }

    pub async fn test_bridge_line(&self, bridge_line: &str) -> BridgeTestResult {
        let data_dir = std::env::temp_dir()
            .join("WireSentinel")
            .join("bridge-test")
            .join(Uuid::new_v4().to_string());
        self.runner.test_bridge_line(bridge_line, data_dir).await
    }

    pub async fn test_bridge_config(
        &self,
        _bridge_type: &str,
        config_json: &str,
    ) -> BridgeTestResult {
        let line = serde_json::from_str::<serde_json::Value>(config_json)
            .ok()
            .and_then(|v| {
                v.get("line")
                    .and_then(|l| l.as_str())
                    .map(|s| s.to_string())
            });
        match line {
            Some(l) => self.test_bridge_line(&l).await,
            None => BridgeTestResult {
                reachable: false,
                latency_ms: None,
                message: Some("bridge config_json missing \"line\" field".into()),
            },
        }
    }
}

pub use crate::singbox::BridgeTestResult;
