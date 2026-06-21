//! Mixnet gateway trust and credential validation policy.

use event_bus::EventBus;
use parking_lot::RwLock;
use shared_types::{
    MixnetProfile, MixnetProvider, Result, ServiceEvent, ServiceEventInner, WireSentinelError,
};
use std::collections::HashSet;
use uuid::Uuid;

/// Security policy for mixnet gateway trust and credential checks.
pub struct MixnetSecurityPolicy {
    events: EventBus,
    trusted_gateways: RwLock<HashSet<String>>,
    require_credentials: RwLock<bool>,
}

impl MixnetSecurityPolicy {
    pub fn new(events: EventBus) -> Self {
        Self {
            events,
            trusted_gateways: RwLock::new(HashSet::new()),
            require_credentials: RwLock::new(false),
        }
    }

    pub fn set_trusted_gateways(&self, gateways: impl IntoIterator<Item = String>) {
        *self.trusted_gateways.write() = gateways.into_iter().collect();
    }

    pub fn trusted_gateways(&self) -> Vec<String> {
        self.trusted_gateways.read().iter().cloned().collect()
    }

    pub fn set_require_credentials(&self, require: bool) {
        *self.require_credentials.write() = require;
    }

    pub fn validate_profile(&self, profile: &MixnetProfile) -> Result<()> {
        if let Some(gateway_id) = &profile.gateway_id {
            self.validate_gateway(profile.id, gateway_id)?;
        }
        self.validate_credentials(profile)?;
        Ok(())
    }

    pub fn validate_gateway(&self, profile_id: Uuid, gateway_id: &str) -> Result<()> {
        let trusted = self.trusted_gateways.read();
        if trusted.is_empty() {
            return Ok(());
        }
        if trusted.contains(gateway_id) {
            return Ok(());
        }
        let detail = format!("gateway '{gateway_id}' is not in the trust list");
        self.publish_violation(profile_id, "gateway_trust", &detail);
        Err(WireSentinelError::Policy(detail))
    }

    pub fn validate_credentials(&self, profile: &MixnetProfile) -> Result<()> {
        if !*self.require_credentials.read() {
            return Ok(());
        }
        match &profile.provider {
            MixnetProvider::Nym => {
                let has_creds = profile
                    .config_json
                    .as_ref()
                    .and_then(|v| v.get("mnemonic"))
                    .or_else(|| {
                        profile
                            .config_json
                            .as_ref()
                            .and_then(|v| v.get("credential"))
                    })
                    .is_some();
                if !has_creds {
                    let detail = "nym profile missing mnemonic or credential".to_string();
                    self.publish_violation(profile.id, "credential", &detail);
                    return Err(WireSentinelError::Policy(detail));
                }
            }
            MixnetProvider::Plugin(plugin_id) => {
                if plugin_id.is_nil() {
                    let detail = "plugin mixnet provider has nil plugin id".to_string();
                    self.publish_violation(profile.id, "credential", &detail);
                    return Err(WireSentinelError::Policy(detail));
                }
            }
        }
        Ok(())
    }

    pub fn to_core_policy(&self) -> mixnet_core::MixnetSecurityPolicy {
        mixnet_core::MixnetSecurityPolicy {
            require_binaries: false,
            allowed_providers: vec![],
            max_restarts: 3,
            loopback_only: true,
        }
    }

    fn publish_violation(&self, profile_id: Uuid, violation_type: &str, detail: &str) {
        self.events.publish(ServiceEvent::now(
            ServiceEventInner::MixnetSecurityViolation {
                profile_id,
                violation_type: violation_type.to_string(),
                detail: detail.to_string(),
            },
        ));
    }
}
