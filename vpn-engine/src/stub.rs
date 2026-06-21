use crate::backend::VpnBackend;
use async_trait::async_trait;
use parking_lot::RwLock;
use shared_types::{Result, VPNProfile, VpnState, VpnStats, VpnStatus, WireSentinelError};
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

/// Non-Windows stub VPN backend for development.
pub struct StubVpnBackend {
    active: RwLock<HashMap<Uuid, VpnStatus>>,
}

impl StubVpnBackend {
    pub fn new() -> Self {
        Self {
            active: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for StubVpnBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VpnBackend for StubVpnBackend {
    async fn connect(&self, profile: &VPNProfile) -> Result<()> {
        info!(name = %profile.name, "stub VPN connect");
        self.active.write().insert(profile.id, VpnStatus::Connected);
        Ok(())
    }

    async fn disconnect(&self, profile_id: Uuid) -> Result<()> {
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
