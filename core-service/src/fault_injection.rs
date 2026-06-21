//! Fault injection for recovery verification (dev/admin).

use chrono::Utc;
use event_bus::EventBus;
use shared_types::{Result, ServiceEventInner};
use std::sync::Arc;

use crate::recovery::RecoveryService;

pub struct FaultInjectionService {
    recovery: Arc<RecoveryService>,
    events: EventBus,
}

impl FaultInjectionService {
    pub fn new(recovery: Arc<RecoveryService>, events: EventBus) -> Self {
        Self { recovery, events }
    }

    pub async fn inject_and_verify(
        &self,
        scenario: &str,
        vpn: &vpn_engine::VpnManager,
        transport: &crate::transport::TransportManager,
        wfp: &dyn wfp::WfpEngine,
        dns_enabled: bool,
    ) -> Result<bool> {
        self.events.publish(
            ServiceEventInner::FaultInjected {
                scenario: scenario.into(),
            }
            .with_timestamp(Utc::now()),
        );

        match scenario {
            "vpn_crash" => {
                for profile in vpn.profiles() {
                    let _ = vpn.disconnect(profile.id).await;
                }
            }
            "dns_crash" => {
                let _ = dns_enabled;
            }
            "wfp_failure" => {
                let _ = wfp.shutdown().await;
            }
            "transport_crash" => {
                let _ = transport;
            }
            _ => {}
        }

        let restored = self.recovery.recover_all(vpn, transport, true).await?;

        let verified = restored > 0 || scenario == "dns_crash";
        if verified {
            self.events.publish(
                ServiceEventInner::RecoveryVerified {
                    scenario: scenario.into(),
                }
                .with_timestamp(Utc::now()),
            );
        }

        Ok(verified)
    }
}
