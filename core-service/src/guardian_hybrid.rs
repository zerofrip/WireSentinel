//! Guardian + NDIS hybrid health validation.

use chrono::Utc;
use event_bus::EventBus;
use shared_types::{
    GuardianMode, NdisHealth, Result, ServiceEvent, ServiceEventInner, WireSentinelError,
};
use std::sync::Arc;
use storage::Storage;
use wfp::{kernel_driver_available, ndis_driver_available, NdisEngine};

pub struct GuardianHybridService {
    storage: Arc<Storage>,
    ndis: Arc<dyn NdisEngine>,
    events: EventBus,
}

impl GuardianHybridService {
    pub fn new(storage: Arc<Storage>, ndis: Arc<dyn NdisEngine>, events: EventBus) -> Self {
        Self {
            storage,
            ndis,
            events,
        }
    }

    pub async fn guardian_mode(&self) -> GuardianMode {
        self.storage
            .settings
            .guardian_mode()
            .await
            .map(|s| GuardianMode::parse(&s))
            .unwrap_or_default()
    }

    pub async fn ndis_health(&self) -> NdisHealth {
        self.ndis.health().await
    }

    fn publish_driver_integrity_failure(&self, driver: &str, detail: String) {
        self.events.publish(ServiceEvent::now(ServiceEventInner::DriverIntegrityFailure {
            driver: driver.into(),
            detail: detail.clone(),
        }));
        self.events.publish(ServiceEvent::now(ServiceEventInner::KernelSecurityViolation {
            violation_type: "driver_integrity".into(),
            detail: format!("{driver}: {detail}"),
        }));
    }

    pub async fn validate(&self) -> Result<(GuardianMode, Option<String>, Option<String>)> {
        let mode = self.guardian_mode().await;
        let guardian_msg = if mode == GuardianMode::Hybrid {
            match kernel_driver_available() {
                Ok(info) => Some(info),
                Err(e) => {
                    let detail = format!("hybrid mode requires Guardian driver: {e}");
                    self.publish_driver_integrity_failure("guardian", detail.clone());
                    return Err(WireSentinelError::Config(detail));
                }
            }
        } else {
            None
        };

        let ndis_msg = if mode.uses_ndis() {
            match ndis_driver_available() {
                Ok(info) => Some(info),
                Err(e) if mode == GuardianMode::Ndis => {
                    let detail = format!("ndis mode requires NDIS driver: {e}");
                    self.publish_driver_integrity_failure("ndis", detail.clone());
                    return Err(WireSentinelError::Config(detail));
                }
                Err(e) => Some(format!("ndis unavailable: {e}")),
            }
        } else {
            None
        };

        if mode == GuardianMode::Hybrid {
            let health = self.ndis.health().await;
            if !health.available && health.state != "stub" {
                let detail = "hybrid mode NDIS side unhealthy".to_string();
                self.publish_driver_integrity_failure("ndis", detail.clone());
                return Err(WireSentinelError::Config(detail));
            }
        }

        Ok((mode, guardian_msg, ndis_msg))
    }

    pub async fn summary(&self) -> Result<serde_json::Value> {
        let mode = self.guardian_mode().await;
        let ndis = self.ndis.health().await;
        Ok(serde_json::json!({
            "guardian_mode": mode.as_str(),
            "ndis": ndis,
            "checked_at": Utc::now(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event_bus::EventBus;
    use storage::{init_pool_in_memory, Storage};
    use wfp::StubNdisEngine;

    #[tokio::test]
    async fn publishes_integrity_events_when_ndis_required_but_missing() {
        let pool = init_pool_in_memory().await.unwrap();
        let storage = Arc::new(Storage::new(pool));
        storage
            .settings
            .set("guardian_mode", "\"ndis\"")
            .await
            .unwrap();

        let events = EventBus::new();
        let mut rx = events.subscribe();
        let hybrid = GuardianHybridService::new(
            storage,
            Arc::new(StubNdisEngine::new()) as Arc<dyn NdisEngine>,
            events,
        );

        let result = hybrid.validate().await;
        assert!(result.is_err());

        let event = rx.try_recv().expect("driver integrity event");
        assert!(matches!(
            event,
            ServiceEvent::DriverIntegrityFailure { driver, .. } if driver == "ndis"
        ));

        let event = rx.try_recv().expect("kernel security event");
        assert!(matches!(
            event,
            ServiceEvent::KernelSecurityViolation { violation_type, .. }
                if violation_type == "driver_integrity"
        ));
    }
}
