use crate::backend::VpnBackend;
use async_trait::async_trait;
use parking_lot::RwLock;
use shared_types::{Result, TailnetProfile, VPNProfile, VpnStats, VpnStatus, WireSentinelError};
use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;

/// Tailscale VPN backend — invokes `tailscale` CLI when available.
pub struct TailscaleBackend {
    active: RwLock<HashMap<Uuid, VpnStatus>>,
}

impl TailscaleBackend {
    pub fn new() -> Self {
        Self {
            active: RwLock::new(HashMap::new()),
        }
    }

    pub async fn connect_tailnet(&self, profile: &TailnetProfile) -> Result<()> {
        info!(name = %profile.name, id = %profile.id, "tailscale up");
        let mut cmd = tokio::process::Command::new("tailscale");
        cmd.arg("up").arg("--accept-routes");
        if profile.magic_dns {
            cmd.args(["--accept-dns=true"]);
        } else {
            cmd.args(["--accept-dns=false"]);
        }
        if let Some(key) = profile.auth_key.as_deref().filter(|k| !k.is_empty()) {
            cmd.args(["--auth-key", key]);
        }
        if let Some(node) = profile.exit_node.as_deref().filter(|n| !n.is_empty()) {
            cmd.args(["--exit-node", node, "--exit-node-allow-lan-access"]);
        }
        let status = cmd
            .status()
            .await
            .map_err(|e| WireSentinelError::Config(format!("tailscale up: {e}")))?;
        if !status.success() {
            return Err(WireSentinelError::Config(
                "tailscale up failed — is tailscale installed?".into(),
            ));
        }
        self.active
            .write()
            .insert(profile.id, VpnStatus::Connected);
        Ok(())
    }

    pub async fn disconnect_tailnet(&self, profile_id: Uuid) -> Result<()> {
        info!(id = %profile_id, "tailscale down");
        let status = tokio::process::Command::new("tailscale")
            .arg("down")
            .status()
            .await
            .map_err(|e| WireSentinelError::Config(format!("tailscale down: {e}")))?;
        if !status.success() {
            warn!(%profile_id, "tailscale down returned non-zero exit");
        }
        self.active.write().remove(&profile_id);
        Ok(())
    }
}

impl Default for TailscaleBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VpnBackend for TailscaleBackend {
    async fn connect(&self, profile: &VPNProfile) -> Result<()> {
        let tailnet = TailnetProfile {
            id: profile.id,
            name: profile.name.clone(),
            auth_key: None,
            exit_node: None,
            subnet_router: false,
            magic_dns: true,
            hostname: None,
            tailnet_ip: None,
            connected: false,
            created_at: profile.created_at,
            updated_at: profile.created_at,
        };
        self.connect_tailnet(&tailnet).await
    }

    async fn disconnect(&self, profile_id: Uuid) -> Result<()> {
        self.disconnect_tailnet(profile_id).await
    }

    async fn status(&self, profile_id: Uuid) -> VpnStatus {
        let runtime = self.query_status().await;
        if runtime.connected {
            VpnStatus::Connected
        } else {
            self.active
                .read()
                .get(&profile_id)
                .copied()
                .unwrap_or(VpnStatus::Disconnected)
        }
    }

    async fn stats(&self, _profile_id: Uuid) -> VpnStats {
        VpnStats::default()
    }

    async fn list_active(&self) -> Vec<Uuid> {
        self.active
            .read()
            .iter()
            .filter(|(_, s)| **s == VpnStatus::Connected)
            .map(|(id, _)| *id)
            .collect()
    }
}

impl TailscaleBackend {
    pub async fn query_status(&self) -> TailscaleRuntimeStatus {
        let output = tokio::process::Command::new("tailscale")
            .args(["status", "--json"])
            .output()
            .await;

        if let Ok(output) = output {
            if output.status.success() {
                if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                    let connected = v
                        .get("BackendState")
                        .and_then(|s| s.as_str())
                        .is_some_and(|s| s == "Running");
                    let tailnet_ip = v
                        .get("TailscaleIPs")
                        .and_then(|a| a.as_array())
                        .and_then(|a| a.first())
                        .and_then(|ip| ip.as_str())
                        .map(str::to_string);
                    let hostname = v
                        .get("Self")
                        .and_then(|s| s.get("HostName"))
                        .and_then(|h| h.as_str())
                        .map(str::to_string);
                    return TailscaleRuntimeStatus {
                        connected,
                        hostname,
                        tailnet_ip,
                        exit_node: None,
                    };
                }
            }
        }

        let connected = !self.list_active().await.is_empty();
        TailscaleRuntimeStatus {
            connected,
            ..TailscaleRuntimeStatus::default()
        }
    }

    pub async fn set_exit_node(&self, node: Option<&str>) -> Result<()> {
        let mut cmd = tokio::process::Command::new("tailscale");
        cmd.arg("up").arg("--accept-routes");
        match node {
            Some(n) if !n.is_empty() => {
                cmd.args(["--exit-node", n, "--exit-node-allow-lan-access"]);
            }
            _ => {
                cmd.arg("--exit-node=");
            }
        }
        let status = cmd
            .status()
            .await
            .map_err(|e| WireSentinelError::Config(format!("tailscale up: {e}")))?;
        if !status.success() {
            return Err(WireSentinelError::Config(
                "tailscale exit node change failed".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct TailscaleRuntimeStatus {
    pub connected: bool,
    pub hostname: Option<String>,
    pub tailnet_ip: Option<String>,
    pub exit_node: Option<String>,
}
