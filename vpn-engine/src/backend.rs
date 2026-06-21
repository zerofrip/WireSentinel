use async_trait::async_trait;
use shared_types::{Result, TunnelIface, VPNProfile, VpnState, VpnStats, VpnStatus};
use uuid::Uuid;

#[async_trait]
pub trait VpnBackend: Send + Sync {
    async fn connect(&self, profile: &VPNProfile) -> Result<()>;
    async fn disconnect(&self, profile_id: Uuid) -> Result<()>;
    async fn status(&self, profile_id: Uuid) -> VpnStatus;
    async fn stats(&self, profile_id: Uuid) -> VpnStats;
    async fn list_active(&self) -> Vec<Uuid>;
    async fn tunnel_iface(&self, _profile_id: Uuid) -> Option<TunnelIface> {
        None
    }
}
