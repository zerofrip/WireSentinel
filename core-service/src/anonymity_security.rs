//! Anonymity provider security policy wrapper.

use event_bus::EventBus;
use parking_lot::RwLock;
use shared_types::{
    AnonymityProvider, FederatedMixnetConfig, KatzenpostProfile, LoopixProfile, Result,
    ServiceEvent, ServiceEventInner, WireSentinelError,
};
use std::collections::HashSet;
use uuid::Uuid;

pub struct AnonymitySecurityPolicy {
    events: EventBus,
    trusted_gateways: RwLock<HashSet<String>>,
    lab_mode: RwLock<bool>,
}

impl AnonymitySecurityPolicy {
    pub fn new(events: EventBus) -> Self {
        Self {
            events,
            trusted_gateways: RwLock::new(HashSet::new()),
            lab_mode: RwLock::new(false),
        }
    }

    pub fn set_trusted_gateways(&self, gateways: impl IntoIterator<Item = String>) {
        *self.trusted_gateways.write() = gateways.into_iter().collect();
    }

    pub fn set_lab_mode(&self, enabled: bool) {
        *self.lab_mode.write() = enabled;
    }

    pub fn is_lab_mode(&self) -> bool {
        *self.lab_mode.read()
    }

    pub fn validate_katzenpost(&self, profile: &KatzenpostProfile) -> Result<()> {
        if let Some(gateway_id) = &profile.gateway_id {
            self.validate_gateway(profile.id, gateway_id)?;
        }
        Ok(())
    }

    pub fn validate_loopix(&self, profile: &LoopixProfile) -> Result<()> {
        if profile.provider_id.as_deref().unwrap_or("").is_empty() && profile.enabled {
            let detail = "loopix profile missing provider_id".to_string();
            self.publish_violation(profile.id, "provider", &detail);
            return Err(WireSentinelError::Policy(detail));
        }
        Ok(())
    }

    pub fn validate_federation(&self, config: &FederatedMixnetConfig) -> Result<()> {
        if config.providers.is_empty() {
            let detail = "federated mixnet requires at least one provider".to_string();
            self.publish_violation(config.profile_id, "federation", &detail);
            return Err(WireSentinelError::Policy(detail));
        }
        Ok(())
    }

    pub fn validate_provider(&self, provider: &AnonymityProvider, profile_id: Uuid) -> Result<()> {
        match provider {
            AnonymityProvider::Plugin(id) if id.is_nil() => {
                let detail = "plugin anonymity provider has nil id".to_string();
                self.publish_violation(profile_id, "provider", &detail);
                Err(WireSentinelError::Policy(detail))
            }
            _ => Ok(()),
        }
    }

    fn validate_gateway(&self, profile_id: Uuid, gateway_id: &str) -> Result<()> {
        let trusted = self.trusted_gateways.read();
        if trusted.is_empty() || trusted.contains(gateway_id) {
            return Ok(());
        }
        let detail = format!("gateway '{gateway_id}' is not in the trust list");
        self.publish_violation(profile_id, "gateway_trust", &detail);
        Err(WireSentinelError::Policy(detail))
    }

    fn publish_violation(&self, profile_id: Uuid, violation_type: &str, detail: &str) {
        self.events
            .publish(ServiceEvent::now(ServiceEventInner::AnonymitySecurityViolation {
                profile_id,
                violation_type: violation_type.to_string(),
                detail: detail.to_string(),
            }));
    }
}
