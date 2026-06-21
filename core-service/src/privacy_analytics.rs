//! Privacy analytics derived from route statistics and mixnet state.

use chrono::Utc;
use event_bus::EventBus;
use shared_types::{PrivacyAnalyticsSnapshot, Result, ServiceEvent, ServiceEventInner};
use std::sync::Arc;
use storage::Storage;
use tokio::sync::watch;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::anonymity::AnonymityService;
use crate::anonymity_entropy::RouteEntropyBridge;
use crate::cover_traffic::CoverTrafficService;
use crate::mixnet::MixnetService;
use anonymity_analytics::AnonymityAnalytics;

pub struct PrivacyAnalyticsService {
    storage: Arc<Storage>,
    events: EventBus,
    mixnet: Arc<MixnetService>,
    cover_traffic: Arc<CoverTrafficService>,
    anonymity: Arc<AnonymityService>,
    entropy: Arc<RouteEntropyBridge>,
    analytics: AnonymityAnalytics,
}

impl PrivacyAnalyticsService {
    pub fn new(
        storage: Arc<Storage>,
        events: EventBus,
        mixnet: Arc<MixnetService>,
        cover_traffic: Arc<CoverTrafficService>,
        anonymity: Arc<AnonymityService>,
        entropy: Arc<RouteEntropyBridge>,
    ) -> Self {
        Self {
            storage,
            events,
            mixnet,
            cover_traffic,
            anonymity,
            entropy,
            analytics: AnonymityAnalytics::new(),
        }
    }

    pub async fn calculate(&self) -> Result<PrivacyAnalyticsSnapshot> {
        let route_stats = self
            .storage
            .route_statistics
            .list(shared_types::RouteStatisticsQuery {
                app_id: None,
                domain: None,
                route_type: None,
                limit: 500,
            })
            .await?;

        let total_bytes: u64 = route_stats.iter().map(|r| r.bytes_in + r.bytes_out).sum();
        let anonymous_types = [
            "tor",
            "anonymous",
            "chain",
            "proxy",
            "proxy_chain",
            "katzenpost",
            "loopix",
            "federated_mixnet",
        ];
        let anonymous_bytes: u64 = route_stats
            .iter()
            .filter(|r| anonymous_types.contains(&r.route_type.as_str()))
            .map(|r| r.bytes_in + r.bytes_out)
            .sum();

        let route_types: Vec<&str> = route_stats.iter().map(|r| r.route_type.as_str()).collect();
        let route_entropy = self.entropy.score_route_types(&route_types);

        let path_diversity = if route_stats.is_empty() {
            0.0
        } else {
            let distinct_routes = route_types
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len();
            (distinct_routes as f64 / 10.0).min(1.0)
        };

        let mixnet_status = self.mixnet.status().await?;
        let anonymity_status = self.anonymity.status().await?;
        let mixnet_bonus = if mixnet_status.running { 20.0 } else { 0.0 };
        let anonymity_bonus = if anonymity_status.active_providers > 0 {
            10.0
        } else {
            0.0
        };
        let cover_bonus = if self.cover_traffic.is_running() {
            15.0
        } else {
            0.0
        };

        let anonymous_ratio = if total_bytes == 0 {
            0.0
        } else {
            anonymous_bytes as f64 / total_bytes as f64
        };

        let anonymity_score =
            ((anonymous_ratio * 65.0) + mixnet_bonus + anonymity_bonus + cover_bonus)
                .min(100.0)
                .max(0.0) as u8;

        let cover_traffic_effectiveness = if self.cover_traffic.is_running() {
            match self.cover_traffic.engine().profile() {
                mixnet_cover_traffic::CoverTrafficProfile::Disabled => 0.0,
                mixnet_cover_traffic::CoverTrafficProfile::Low => 0.25,
                mixnet_cover_traffic::CoverTrafficProfile::Medium => 0.5,
                mixnet_cover_traffic::CoverTrafficProfile::High => 0.75,
                mixnet_cover_traffic::CoverTrafficProfile::Maximum => 1.0,
            }
        } else {
            0.0
        };

        let advanced = self.analytics.compute(
            anonymity_status.active_providers,
            if anonymity_status.federated_active {
                1
            } else {
                0
            },
            self.cover_traffic.is_adaptive(),
            route_types
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len() as u32,
        );
        let anonymity_set_estimate = Some(self.entropy.estimate_from_counts(
            anonymity_status.active_providers,
            anonymity_status.federated_active,
        ));
        let cover_traffic_efficiency = Some(advanced.cover_traffic_efficiency);
        let mixnet_diversity = Some(advanced.mixnet_diversity);
        let federation_diversity = Some(advanced.federation_diversity);

        let snapshot = PrivacyAnalyticsSnapshot {
            id: Uuid::new_v4(),
            anonymity_score,
            route_entropy,
            path_diversity,
            cover_traffic_effectiveness,
            anonymity_set_estimate,
            cover_traffic_efficiency,
            mixnet_diversity,
            federation_diversity,
            timestamp: Utc::now(),
        };

        self.storage.privacy_analytics.insert(&snapshot).await?;
        self.events.publish(ServiceEvent::now(
            ServiceEventInner::PrivacyAnalyticsUpdated {
                snapshot: snapshot.clone(),
            },
        ));
        self.events.publish(ServiceEvent::now(
            ServiceEventInner::AnonymityAnalyticsUpdated {
                snapshot: snapshot.clone(),
            },
        ));

        debug!(
            anonymity_score,
            route_entropy, path_diversity, "privacy analytics updated"
        );
        Ok(snapshot)
    }

    pub fn start_periodic(self: Arc<Self>, mut shutdown: watch::Receiver<bool>) {
        let service = Arc::clone(&self);
        tokio::spawn(async move {
            let interval_secs = service
                .storage
                .settings
                .privacy_score_interval_secs()
                .await
                .unwrap_or(300)
                .max(60);
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
            interval.tick().await;

            loop {
                tokio::select! {
                    _ = shutdown.changed() => {
                        if *shutdown.borrow() {
                            break;
                        }
                    }
                    _ = interval.tick() => {
                        if let Err(e) = service.calculate().await {
                            warn!(error = %e, "privacy analytics calculation failed");
                        }
                    }
                }
            }
        });
    }
}
