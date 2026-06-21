//! Aggregates per-route traffic stats into rolling windows.

use chrono::{DateTime, Timelike, Utc};
use event_bus::EventBus;
use shared_types::{
    RouteStatisticsQuery, RouteStatisticsRecord, ServiceEventInner, TrafficEvent, TrafficRoute,
};
use std::sync::Arc;
use storage::RouteStatisticsRepository;
use uuid::Uuid;

pub struct RouteStatsAggregator;

impl RouteStatsAggregator {
    fn route_type(route: &TrafficRoute) -> String {
        match route {
            TrafficRoute::Direct => "direct".into(),
            TrafficRoute::WireGuard(_) => "wire_guard".into(),
            TrafficRoute::AmneziaWG(_) => "amnezia_wg".into(),
            TrafficRoute::Tailnet(_) => "tailnet".into(),
            TrafficRoute::Tor(_) => "tor".into(),
            TrafficRoute::Anonymous(_) => "anonymous".into(),
            TrafficRoute::Proxy(_) => "proxy".into(),
            TrafficRoute::ProxyChain(_) => "proxy_chain".into(),
            TrafficRoute::Chain(_) => "chain".into(),
            TrafficRoute::Katzenpost(_) => "katzenpost".into(),
            TrafficRoute::Loopix(_) => "loopix".into(),
            TrafficRoute::FederatedMixnet(_) => "federated_mixnet".into(),
            TrafficRoute::Blocked => "blocked".into(),
        }
    }

    fn window_start(now: DateTime<Utc>) -> DateTime<Utc> {
        now.date_naive()
            .and_hms_opt(now.hour(), 0, 0)
            .map(|ndt| ndt.and_utc())
            .unwrap_or(now)
    }

    fn bucket_id(
        app_id: Option<Uuid>,
        profile_id: Option<Uuid>,
        domain: Option<&str>,
        route_type: &str,
        window_start: DateTime<Utc>,
    ) -> Uuid {
        let _key = format!(
            "{}:{}:{}:{}:{}",
            window_start.timestamp(),
            app_id.map(|id| id.to_string()).unwrap_or_default(),
            profile_id.map(|id| id.to_string()).unwrap_or_default(),
            domain.unwrap_or(""),
            route_type,
        );
        Uuid::new_v4()
    }

    pub async fn record_traffic(
        repo: Arc<dyn RouteStatisticsRepository>,
        events: &EventBus,
        event: &TrafficEvent,
    ) -> shared_types::Result<()> {
        let now = Utc::now();
        let window_start = Self::window_start(now);
        let window_end = window_start + chrono::Duration::hours(1);
        let route_type = Self::route_type(&event.route);
        let app_id = Some(event.app.id());
        let profile_id = event.route.profile_id();
        let domain = event.remote_domain.as_deref();
        let record = RouteStatisticsRecord {
            id: Self::bucket_id(app_id, profile_id, domain, &route_type, window_start),
            app_id,
            profile_id,
            domain: domain.map(str::to_string),
            route_type,
            bytes_in: event.bytes_in,
            bytes_out: event.bytes_out,
            connection_count: 1,
            window_start,
            window_end,
            updated_at: now,
        };
        repo.upsert(&record).await?;
        events.publish(
            ServiceEventInner::RouteUsageUpdated {
                stats: record.clone(),
            }
            .with_timestamp(now),
        );
        Ok(())
    }

    pub async fn list_routes(
        repo: Arc<dyn RouteStatisticsRepository>,
        query: RouteStatisticsQuery,
    ) -> shared_types::Result<Vec<RouteStatisticsRecord>> {
        repo.list(query).await
    }

    pub async fn blocked_summary(
        repo: Arc<dyn RouteStatisticsRepository>,
        limit: u32,
    ) -> shared_types::Result<Vec<RouteStatisticsRecord>> {
        repo.blocked_summary(limit).await
    }
}
