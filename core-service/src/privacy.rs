//! Privacy score calculation from DNS, filters, routing, and leak history.

use chrono::Utc;
use dns::DnsLayer;
use event_bus::EventBus;
use shared_types::{
    DnsTransport, LeakType, PrivacyScoreComponents, PrivacyScoreSnapshot, Result, ServiceEventInner,
};
use std::sync::Arc;
use storage::Storage;
use tokio::sync::watch;
use tracing::{debug, warn};
use uuid::Uuid;
use vpn_engine::VpnManager;

pub struct PrivacyScoreService {
    storage: Arc<Storage>,
    events: EventBus,
    dns: Arc<DnsLayer>,
    vpn: Arc<VpnManager>,
}

impl PrivacyScoreService {
    pub fn new(
        storage: Arc<Storage>,
        events: EventBus,
        dns: Arc<DnsLayer>,
        vpn: Arc<VpnManager>,
    ) -> Self {
        Self {
            storage,
            events,
            dns,
            vpn,
        }
    }

    pub async fn calculate(&self) -> Result<PrivacyScoreSnapshot> {
        let dns_settings = self.dns.settings();

        let encrypted_dns = match (dns_settings.enabled, dns_settings.transport) {
            (true, DnsTransport::Doh | DnsTransport::Dot | DnsTransport::Doq) => 100,
            (true, DnsTransport::Plain) => 30,
            (false, _) => 0,
        };

        let filter_records = self.storage.filter_lists.list().await?;
        let enabled_count = filter_records.iter().filter(|f| f.enabled).count();
        let blocked_trackers = ((enabled_count.min(5) as u8) * 20).min(100);

        let vpn_coverage = if self.vpn.any_connected().await {
            100
        } else {
            0
        };

        let route_stats = self
            .storage
            .route_statistics
            .list(shared_types::RouteStatisticsQuery {
                app_id: None,
                domain: None,
                route_type: None,
                limit: 200,
            })
            .await?;
        let direct_bytes: u64 = route_stats
            .iter()
            .filter(|r| r.route_type == "direct")
            .map(|r| r.bytes_out + r.bytes_in)
            .sum();
        let total_bytes: u64 = route_stats.iter().map(|r| r.bytes_out + r.bytes_in).sum();
        let route_leakage = if total_bytes == 0 {
            100
        } else {
            let direct_pct = (direct_bytes * 100 / total_bytes) as u8;
            100u8.saturating_sub(direct_pct.min(80))
        };

        let leaks = self.storage.leak_incidents.list_recent(100).await?;
        let open_dns = leaks
            .iter()
            .filter(|l| l.leak_type == LeakType::Dns && l.resolved_at.is_none())
            .count();
        let dns_leakage = 100u8.saturating_sub((open_dns * 25) as u8);

        let components = PrivacyScoreComponents {
            encrypted_dns,
            blocked_trackers,
            vpn_coverage,
            route_leakage,
            dns_leakage,
            anonymity_score: None,
            route_entropy: None,
            path_diversity: None,
            cover_traffic_effectiveness: None,
        };

        let score = ((encrypted_dns as u16
            + blocked_trackers as u16
            + vpn_coverage as u16
            + route_leakage as u16
            + dns_leakage as u16)
            / 5) as u8;

        let snapshot = PrivacyScoreSnapshot {
            id: Uuid::new_v4(),
            score,
            components,
            timestamp: Utc::now(),
        };

        self.storage.privacy_snapshots.insert(&snapshot).await?;
        self.events.publish(
            ServiceEventInner::PrivacyScoreUpdated {
                snapshot: snapshot.clone(),
            }
            .with_timestamp(Utc::now()),
        );

        debug!(score, "privacy score updated");
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
                .max(30);
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
                            warn!(error = %e, "privacy score calculation failed");
                        }
                    }
                }
            }
        });
    }
}
