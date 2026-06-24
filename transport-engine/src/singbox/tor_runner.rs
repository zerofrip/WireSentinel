use crate::config_store::TransportConfigStore;
use crate::process_manager::ProcessManager;
use crate::singbox::tor_config::{build_tor_config, TorOutboundSpec};
use shared_types::{Result, WireSentinelError};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct BridgeTestResult {
    pub reachable: bool,
    pub latency_ms: Option<u64>,
    pub message: Option<String>,
}

const DEFAULT_SOCKS_WAIT: Duration = Duration::from_secs(120);
const BRIDGE_TEST_WAIT: Duration = Duration::from_secs(90);

/// Spawns sing-box with a tor outbound and supervises the process.
pub struct TorSingBoxRunner {
    process_manager: Arc<ProcessManager>,
    config_store: Arc<TransportConfigStore>,
    singbox_binary: PathBuf,
    tor_binary: PathBuf,
}

impl TorSingBoxRunner {
    pub fn new(
        process_manager: Arc<ProcessManager>,
        config_store: Arc<TransportConfigStore>,
        singbox_binary: PathBuf,
        tor_binary: PathBuf,
    ) -> Self {
        Self {
            process_manager,
            config_store,
            singbox_binary,
            tor_binary,
        }
    }

    pub fn singbox_binary(&self) -> &PathBuf {
        &self.singbox_binary
    }

    pub fn tor_binary(&self) -> &PathBuf {
        &self.tor_binary
    }

    pub fn with_tor_binary(mut self, path: PathBuf) -> Self {
        self.tor_binary = path;
        self
    }

    pub fn with_singbox_binary(mut self, path: PathBuf) -> Self {
        self.singbox_binary = path;
        self
    }

    /// Ensure spec uses configured tor executable path.
    pub fn normalize_spec(&self, mut spec: TorOutboundSpec) -> TorOutboundSpec {
        if spec.executable_path.as_os_str().is_empty() {
            spec.executable_path = self.tor_binary.clone();
        }
        spec
    }

    pub async fn start(
        &self,
        instance_id: Uuid,
        listen_port: u16,
        spec: TorOutboundSpec,
        upstream_socks: Option<&str>,
    ) -> Result<()> {
        if self.process_manager.is_running(instance_id) {
            return Ok(());
        }

        let spec = self.normalize_spec(spec);
        std::fs::create_dir_all(&spec.data_directory).map_err(WireSentinelError::Io)?;

        if !self.singbox_binary.exists() {
            return Err(WireSentinelError::Config(format!(
                "sing-box binary not found: {}",
                self.singbox_binary.display()
            )));
        }
        if !spec.executable_path.exists() {
            return Err(WireSentinelError::Config(format!(
                "tor binary not found: {}",
                spec.executable_path.display()
            )));
        }

        let config = build_tor_config(listen_port, &spec, upstream_socks);
        let config_path = self.config_store.write_json(instance_id, &config)?;

        self.process_manager
            .spawn(
                instance_id,
                &self.singbox_binary,
                &["run", "-c"],
                &config_path,
            )
            .await?;

        info!(
            %instance_id,
            port = listen_port,
            tor = %spec.executable_path.display(),
            "tor sing-box transport started"
        );
        Ok(())
    }

    pub async fn stop(&self, instance_id: Uuid) -> Result<()> {
        if self.process_manager.is_running(instance_id) {
            self.process_manager.kill(instance_id).await?;
        }
        let _ = self.config_store.delete(instance_id);
        Ok(())
    }

    pub fn is_running(&self, instance_id: Uuid) -> bool {
        self.process_manager.is_running(instance_id)
    }

    /// TCP connect to local mixed inbound as bootstrap readiness proxy.
    pub async fn wait_socks_ready(&self, port: u16, wait: Duration) -> Result<()> {
        let addr = format!("127.0.0.1:{port}");
        let started = std::time::Instant::now();
        let step = Duration::from_millis(500);

        while started.elapsed() < wait {
            match timeout(Duration::from_secs(2), TcpStream::connect(&addr)).await {
                Ok(Ok(_)) => return Ok(()),
                Ok(Err(e)) => {
                    if started.elapsed() + step >= wait {
                        return Err(WireSentinelError::Other(format!(
                            "tor socks not ready on {addr}: {e}"
                        )));
                    }
                }
                Err(_) => {
                    if started.elapsed() + step >= wait {
                        return Err(WireSentinelError::Other(format!(
                            "tor socks connect timed out on {addr}"
                        )));
                    }
                }
            }
            tokio::time::sleep(step).await;
        }

        Err(WireSentinelError::Other(format!(
            "tor socks not ready on {addr} within {:?}",
            wait
        )))
    }

    pub async fn start_and_wait(
        &self,
        instance_id: Uuid,
        listen_port: u16,
        spec: TorOutboundSpec,
        upstream_socks: Option<&str>,
    ) -> Result<()> {
        self.start(instance_id, listen_port, spec, upstream_socks)
            .await?;
        self.wait_socks_ready(listen_port, DEFAULT_SOCKS_WAIT).await
    }

    /// Short-lived sing-box instance to verify a bridge line reaches SOCKS readiness.
    pub async fn test_bridge_line(
        &self,
        bridge_line: &str,
        data_directory: PathBuf,
    ) -> BridgeTestResult {
        let test_id = Uuid::new_v4();
        let listen_port = 19050u16;

        let mut torrc = std::collections::HashMap::new();
        torrc.insert("ClientOnly".into(), "1".into());
        torrc.insert("UseBridges".into(), "1".into());

        let spec = TorOutboundSpec {
            executable_path: self.tor_binary.clone(),
            data_directory,
            extra_args: vec!["--Bridge".into(), bridge_line.to_string()],
            torrc,
        };

        let started = std::time::Instant::now();
        if let Err(e) = self.start(test_id, listen_port, spec, None).await {
            let _ = self.stop(test_id).await;
            return BridgeTestResult {
                reachable: false,
                latency_ms: None,
                message: Some(e.to_string()),
            };
        }

        let result = match self.wait_socks_ready(listen_port, BRIDGE_TEST_WAIT).await {
            Ok(()) => BridgeTestResult {
                reachable: true,
                latency_ms: Some(started.elapsed().as_millis() as u64),
                message: Some("bridge test ok".into()),
            },
            Err(e) => {
                warn!(error = %e, "bridge test failed");
                BridgeTestResult {
                    reachable: false,
                    latency_ms: None,
                    message: Some(e.to_string()),
                }
            }
        };

        if let Err(e) = self.stop(test_id).await {
            warn!(error = %e, "bridge test cleanup failed");
        }
        result
    }
}
