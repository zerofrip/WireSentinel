//! Decoy routing framework wrapper (lab mode).

use anonymity_decoy_routing::{DecoyMode, DecoyRoutingFramework, DecoyRoute};
use event_bus::EventBus;
use shared_types::{Result, ServiceEvent, ServiceEventInner, WireSentinelError};

use crate::anonymity_security::AnonymitySecurityPolicy;

pub struct AnonymityDecoyService {
    framework: parking_lot::Mutex<DecoyRoutingFramework>,
    events: EventBus,
}

impl AnonymityDecoyService {
    pub fn new(security: &AnonymitySecurityPolicy, events: EventBus) -> Self {
        let mode = if security.is_lab_mode() {
            DecoyMode::Lab
        } else {
            DecoyMode::Research
        };
        Self {
            framework: parking_lot::Mutex::new(DecoyRoutingFramework::new(mode)),
            events,
        }
    }

    pub fn refresh_lab_mode(&self, security: &AnonymitySecurityPolicy) {
        let mode = if security.is_lab_mode() {
            DecoyMode::Lab
        } else {
            DecoyMode::Research
        };
        *self.framework.lock() = DecoyRoutingFramework::new(mode);
    }

    pub fn create_route(&self, target: impl Into<String>) -> Result<DecoyRoute> {
        let target = target.into();
        let mut framework = self.framework.lock();
        if !matches!(framework.mode(), DecoyMode::Lab | DecoyMode::Simulation) {
            return Err(WireSentinelError::Policy(
                "decoy routing requires lab or simulation mode".into(),
            ));
        }
        let result = framework.simulate_route(&[target.as_str()], 2);
        self.events
            .publish(ServiceEvent::now(ServiceEventInner::DecoyRouteCreated {
                route_id: result.route.id,
                target,
            }));
        Ok(result.route)
    }

    pub fn simulate(&self, route: &DecoyRoute) -> Result<u32> {
        let framework = self.framework.lock();
        let hops = route.hops.len() as u32;
        self.events
            .publish(ServiceEvent::now(ServiceEventInner::DecoyRouteSimulated {
                route_id: route.id,
                simulated_hops: hops,
            }));
        Ok(hops)
    }
}
