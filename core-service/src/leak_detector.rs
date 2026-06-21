//! Leak detection for DNS, routing, and VPN disconnect scenarios.

use chrono::Utc;
use event_bus::EventBus;
use parking_lot::RwLock;
use policy_engine::PolicyEngine;
use shared_types::{
    ConnectionSnapshot, LeakIncident, LeakType, Result, ServiceEventInner, TrafficRoute, VpnStatus,
};
use std::sync::Arc;
use storage::Storage;
use tracing::warn;
use uuid::Uuid;
use vpn_engine::VpnManager;

pub struct LeakDetector {
    storage: Arc<Storage>,
    events: EventBus,
    policy: Arc<RwLock<PolicyEngine>>,
    vpn: Arc<VpnManager>,
}

impl LeakDetector {
    pub fn new(
        storage: Arc<Storage>,
        events: EventBus,
        policy: Arc<RwLock<PolicyEngine>>,
        vpn: Arc<VpnManager>,
    ) -> Self {
        Self {
            storage,
            events,
            policy,
            vpn,
        }
    }

    pub async fn check_connection(
        &self,
        conn: &ConnectionSnapshot,
        app_id: Uuid,
        route: &TrafficRoute,
        vpn_connected: bool,
    ) -> Result<()> {
        if !self
            .storage
            .settings
            .leak_detection_enabled()
            .await
            .unwrap_or(true)
        {
            return Ok(());
        }

        if conn.remote_addr.port() == 53 && matches!(route, TrafficRoute::Direct) && vpn_connected {
            self.report(
                LeakType::Dns,
                Some(app_id),
                serde_json::json!({
                    "remote": conn.remote_addr.to_string(),
                    "reason": "direct_dns_while_vpn_connected",
                })
                .to_string(),
                "warning",
            )
            .await?;
        }

        if vpn_connected
            && self.policy.read().kill_switch_active()
            && matches!(route, TrafficRoute::Direct)
        {
            self.report(
                LeakType::Route,
                Some(app_id),
                serde_json::json!({
                    "remote": conn.remote_addr.to_string(),
                    "reason": "direct_while_kill_switch_active",
                })
                .to_string(),
                "critical",
            )
            .await?;
        }

        if let Some(profile_id) = route.profile_id() {
            let disconnected = self
                .vpn
                .state(profile_id)
                .await
                .map(|s| s.status != VpnStatus::Connected)
                .unwrap_or(true);
            if disconnected {
                self.report(
                    LeakType::VpnDisconnect,
                    Some(app_id),
                    serde_json::json!({
                        "profile_id": profile_id,
                        "remote": conn.remote_addr.to_string(),
                        "reason": "traffic_routed_via_disconnected_vpn",
                    })
                    .to_string(),
                    "critical",
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn report(
        &self,
        leak_type: LeakType,
        app_id: Option<Uuid>,
        detail_json: String,
        severity: &str,
    ) -> Result<()> {
        let incident = LeakIncident {
            id: Uuid::new_v4(),
            leak_type,
            app_id,
            detail_json: Some(detail_json),
            severity: severity.to_string(),
            detected_at: Utc::now(),
            resolved_at: None,
        };

        self.storage.leak_incidents.insert(&incident).await?;
        self.events.publish(
            ServiceEventInner::LeakDetected {
                incident: incident.clone(),
            }
            .with_timestamp(Utc::now()),
        );

        warn!(
            ?leak_type,
            severity,
            app_id = ?app_id,
            "leak detected"
        );
        Ok(())
    }
}
