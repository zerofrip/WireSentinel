//! Route entropy bridge to anonymity-entropy crate.

use anonymity_analytics::{anonymity_set_estimate, federation_diversity, path_diversity};
use anonymity_core::AnonymityRoute;
use anonymity_entropy::RouteEntropyEngine;
use event_bus::EventBus;
use shared_types::{Result, ServiceEvent, ServiceEventInner};

pub struct RouteEntropyBridge {
    engine: RouteEntropyEngine,
    events: EventBus,
}

impl RouteEntropyBridge {
    pub fn new(events: EventBus) -> Self {
        Self {
            engine: RouteEntropyEngine::new(),
            events,
        }
    }

    pub fn score(&self, routes: &[AnonymityRoute]) -> f64 {
        self.engine.score_paths(routes).mean_entropy
    }

    pub fn score_route_types(&self, route_types: &[&str]) -> f64 {
        if route_types.is_empty() {
            return 0.0;
        }
        let unique = route_types
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        unique as f64 / route_types.len() as f64
    }

    pub fn anonymity_set_estimate(
        &self,
        routes: &[AnonymityRoute],
        profiles: &[anonymity_core::AnonymityProfile],
    ) -> f64 {
        anonymity_set_estimate(routes, profiles)
    }

    pub fn path_diversity(&self, routes: &[AnonymityRoute]) -> f64 {
        path_diversity(routes)
    }

    pub fn federation_diversity(&self, profiles: &[anonymity_core::AnonymityProfile]) -> f64 {
        federation_diversity(profiles)
    }

    pub fn estimate_from_counts(&self, active_providers: u32, federated_active: bool) -> f64 {
        let base = active_providers.max(1) as f64 * 2.0;
        if federated_active {
            base * 1.5
        } else {
            base
        }
    }

    pub fn publish_score(
        &self,
        routes: &[AnonymityRoute],
        profiles: &[anonymity_core::AnonymityProfile],
    ) -> Result<f64> {
        let score = self.score(routes);
        let estimate = self.anonymity_set_estimate(routes, profiles);
        self.events
            .publish(ServiceEvent::now(ServiceEventInner::EntropyScoreUpdated {
                score,
                anonymity_set_estimate: estimate,
            }));
        Ok(score)
    }
}
