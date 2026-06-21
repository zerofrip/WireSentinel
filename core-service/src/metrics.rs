//! Observability metrics aggregation.

use chrono::{Duration, Utc};
use shared_types::{MetricsSnapshot, Result, TransportState};
use std::sync::Arc;
use storage::Storage;
use vpn_engine::VpnManager;

use crate::transport::TransportManager;

pub struct MetricsService {
    storage: Arc<Storage>,
    vpn: Arc<VpnManager>,
    transport: Arc<TransportManager>,
}

impl MetricsService {
    pub fn new(storage: Arc<Storage>, vpn: Arc<VpnManager>, transport: Arc<TransportManager>) -> Self {
        Self {
            storage,
            vpn,
            transport,
        }
    }

    pub async fn snapshot(&self) -> Result<MetricsSnapshot> {
        let blocked = self.storage.firewall_decisions.count().await? as u64;
        let dns_queries = self.storage.dns_logs.count().await? as u64;
        let open_leaks = self
            .storage
            .leak_incidents
            .list_recent(100)
            .await?
            .into_iter()
            .filter(|i| i.resolved_at.is_none())
            .count() as u32;

        let since = Utc::now() - Duration::hours(24);
        let route_changes = self
            .storage
            .audit_log
            .count_since(Some("route_changed"), since)
            .await?;

        let active_transports = self
            .transport
            .status()
            .await?
            .into_iter()
            .filter(|r| r.state == TransportState::Running)
            .count() as u32;

        Ok(MetricsSnapshot {
            active_tunnels: self.vpn.active_count() as u32,
            active_transports,
            blocked_requests: blocked,
            dns_queries,
            open_leak_incidents: open_leaks,
            route_changes_24h: route_changes,
            timestamp: Utc::now(),
        })
    }

    pub fn to_prometheus(snapshot: &MetricsSnapshot) -> String {
        format!(
            "# HELP wiresentinel_active_tunnels Active VPN tunnels\n\
             # TYPE wiresentinel_active_tunnels gauge\n\
             wiresentinel_active_tunnels {}\n\
             # HELP wiresentinel_active_transports Active transport processes\n\
             # TYPE wiresentinel_active_transports gauge\n\
             wiresentinel_active_transports {}\n\
             # HELP wiresentinel_blocked_requests Blocked firewall decisions\n\
             # TYPE wiresentinel_blocked_requests counter\n\
             wiresentinel_blocked_requests {}\n\
             # HELP wiresentinel_dns_queries DNS query count\n\
             # TYPE wiresentinel_dns_queries counter\n\
             wiresentinel_dns_queries {}\n\
             # HELP wiresentinel_open_leak_incidents Open leak incidents\n\
             # TYPE wiresentinel_open_leak_incidents gauge\n\
             wiresentinel_open_leak_incidents {}\n\
             # HELP wiresentinel_route_changes_24h Route changes in 24h\n\
             # TYPE wiresentinel_route_changes_24h counter\n\
             wiresentinel_route_changes_24h {}\n",
            snapshot.active_tunnels,
            snapshot.active_transports,
            snapshot.blocked_requests,
            snapshot.dns_queries,
            snapshot.open_leak_incidents,
            snapshot.route_changes_24h,
        )
    }
}
