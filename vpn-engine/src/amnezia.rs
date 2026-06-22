//! AmneziaWG backend skeleton — config validation and backend registration.

use crate::backend::VpnBackend;
use crate::conf::{parse_conf, write_conf};
use async_trait::async_trait;
use shared_types::{Result, VPNProfile, VpnStats, VpnStatus, WireSentinelError};
use uuid::Uuid;

pub struct AmneziaWgBackend;

impl Default for AmneziaWgBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl AmneziaWgBackend {
    pub fn new() -> Self {
        Self
    }

    pub fn validate_config(content: &str) -> Result<()> {
        let config = parse_conf(content);
        if config.interface.private_key.is_none() {
            return Err(WireSentinelError::Vpn(
                "AmneziaWG config missing PrivateKey".into(),
            ));
        }
        let has_awg = config.interface.jc.is_some()
            || config.interface.h1.is_some()
            || config.interface.i1.is_some();
        if !has_awg {
            return Err(WireSentinelError::Vpn(
                "AmneziaWG config missing obfuscation parameters (Jc/H1/I1)".into(),
            ));
        }
        if config.peers.is_empty() {
            return Err(WireSentinelError::Vpn(
                "AmneziaWG config missing [Peer] section".into(),
            ));
        }
        let _ = write_conf(&config);
        Ok(())
    }
}

#[async_trait]
impl VpnBackend for AmneziaWgBackend {
    async fn connect(&self, profile: &VPNProfile) -> Result<()> {
        Err(WireSentinelError::Vpn(format!(
            "native AmneziaWG connect not yet implemented — use SCM backend for profile {}",
            profile.name
        )))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_awg_config() {
        let conf = "[Interface]\nPrivateKey = x\nJc = 4\n\n[Peer]\nPublicKey = y\nAllowedIPs = 0.0.0.0/0\n";
        assert!(AmneziaWgBackend::validate_config(conf).is_ok());
    }

    #[test]
    fn rejects_plain_wg_as_awg() {
        let conf = "[Interface]\nPrivateKey = x\n\n[Peer]\nPublicKey = y\n";
        assert!(AmneziaWgBackend::validate_config(conf).is_err());
    }
}
