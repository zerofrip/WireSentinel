use crate::backend::VpnBackend;
use async_trait::async_trait;
use parking_lot::RwLock;
use shared_types::{Result, VPNProfile, VpnStats, VpnStatus, WireSentinelError};
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

/// Tailscale VPN backend (stub — invokes tailscale CLI on Windows when available).
pub struct TailscaleBackend {
    active: RwLock<HashMap<Uuid, VpnStatus>>,
}

impl TailscaleBackend {
    pub fn new() -> Self {
        Self {
            active: RwLock::new(HashMap::new()),
        }
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
        info!(name = %profile.name, id = %profile.id, "tailscale connect");
        #[cfg(windows)]
        {
            let _ = tokio::process::Command::new("tailscale")
                .args(["up", "--accept-routes"])
                .status()
                .await;
        }
        self.active.write().insert(profile.id, VpnStatus::Connected);
        Ok(())
    }

    async fn disconnect(&self, profile_id: Uuid) -> Result<()> {
        info!(id = %profile_id, "tailscale disconnect");
        #[cfg(windows)]
        {
            let _ = tokio::process::Command::new("tailscale")
                .arg("down")
                .status()
                .await;
        }
        self.active.write().remove(&profile_id);
        Ok(())
    }

    async fn status(&self, profile_id: Uuid) -> VpnStatus {
        self.active
            .read()
            .get(&profile_id)
            .copied()
            .unwrap_or(VpnStatus::Disconnected)
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
        let connected = !self.list_active().await.is_empty();
        TailscaleRuntimeStatus {
            connected,
            ..TailscaleRuntimeStatus::default()
        }
    }

    pub async fn set_exit_node(&self, node: Option<&str>) -> Result<()> {
        let mut cmd = tokio::process::Command::new("tailscale");
        cmd.arg("up");
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
