//! Non-Windows stub for NativeWireGuardBackend.

use crate::backend::VpnBackend;
use async_trait::async_trait;
use shared_types::{Result, VPNProfile, VpnStats, VpnStatus, WireSentinelError};
use std::path::PathBuf;
use uuid::Uuid;

pub struct NativeWireGuardBackend {
    _dll_path: PathBuf,
}

impl NativeWireGuardBackend {
    pub fn new(dll_path: PathBuf) -> Self {
        Self {
            _dll_path: dll_path,
        }
    }
}

#[async_trait]
impl VpnBackend for NativeWireGuardBackend {
    async fn connect(&self, _profile: &VPNProfile) -> Result<()> {
        Err(WireSentinelError::Vpn(
            "NativeWireGuardBackend requires Windows".into(),
        ))
    }

    async fn disconnect(&self, _profile_id: Uuid) -> Result<()> {
        Ok(())
    }

    async fn status(&self, _profile_id: Uuid) -> VpnStatus {
        VpnStatus::Disconnected
    }

    async fn stats(&self, _profile_id: Uuid) -> VpnStats {
        VpnStats::default()
    }

    async fn list_active(&self) -> Vec<Uuid> {
        Vec::new()
    }
}
