//! Enforcement backend resolution and persistence.

use shared_types::{EnforcementBackend, EnforcementMapping, Result};
use storage::Storage;

/// Load persisted backend and derive low-level driver mapping.
pub async fn resolve_mapping(storage: &Storage) -> Result<EnforcementMapping> {
    if let Some(raw) = storage.settings.get("enforcement_backend").await? {
        let backend = serde_json::from_str::<String>(&raw).unwrap_or_else(|_| raw);
        let trimmed = backend.trim_matches('"');
        return Ok(EnforcementMapping::from_backend(EnforcementBackend::parse(
            trimmed,
        )));
    }
    // Legacy installs: infer from guardian_mode + wfp_engine_impl.
    let guardian = storage.settings.guardian_mode().await?;
    let impl_name = storage.settings.wfp_engine_impl().await?;
    let backend = if impl_name == "kernel" || guardian == "hybrid" || guardian == "ndis" {
        EnforcementBackend::CustomKernel
    } else {
        EnforcementBackend::Signed
    };
    Ok(EnforcementMapping::from_backend(backend))
}

/// Coordinates userspace WFP policy with the WinDivert datapath in signed mode.
pub struct SignedEnforcementCoordinator;

impl SignedEnforcementCoordinator {
    pub const DESCRIPTION: &'static str =
        "userspace WFP + WinDivert redirect/transform + sing-box TUN (subprocess)";
}

/// Persist backend choice and sync derived `guardian_mode` / `wfp_engine_impl` keys.
pub async fn apply_backend(storage: &Storage, backend: EnforcementBackend) -> Result<()> {
    let mapping = EnforcementMapping::from_backend(backend);
    storage
        .settings
        .set_enforcement_backend(mapping.backend.as_str())
        .await?;
    storage
        .settings
        .set_guardian_mode(mapping.guardian_mode.as_str())
        .await?;
    storage
        .settings
        .set_wfp_engine_impl(mapping.wfp_engine_impl)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_maps_to_userspace_wfp() {
        let m = EnforcementMapping::from_backend(EnforcementBackend::Signed);
        assert_eq!(m.guardian_mode, shared_types::GuardianMode::Wfp);
        assert_eq!(m.wfp_engine_impl, "userspace");
        assert!(m.use_windivert);
    }

    #[test]
    fn custom_kernel_maps_to_hybrid_kernel() {
        let m = EnforcementMapping::from_backend(EnforcementBackend::CustomKernel);
        assert_eq!(m.guardian_mode, shared_types::GuardianMode::Hybrid);
        assert_eq!(m.wfp_engine_impl, "kernel");
        assert!(!m.use_windivert);
    }
}
