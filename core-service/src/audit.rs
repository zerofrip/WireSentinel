//! Audit log helper — persists entries and publishes policy/route events.

use chrono::Utc;
use event_bus::EventBus;
use shared_types::{AuditLogEntry, AuditLogQuery, ServiceEventInner, TrafficRoute};
use std::sync::Arc;
use storage::AuditLogRepository;
use uuid::Uuid;

pub struct AuditRecorder {
    repo: Arc<dyn AuditLogRepository>,
    events: EventBus,
}

impl AuditRecorder {
    pub fn new(repo: Arc<dyn AuditLogRepository>, events: EventBus) -> Self {
        Self { repo, events }
    }

    pub async fn record_policy_changed(
        &self,
        field: &str,
        old_value: Option<String>,
        new_value: Option<String>,
        actor: Option<String>,
    ) -> shared_types::Result<()> {
        let now = Utc::now();
        let detail = serde_json::json!({
            "field": field,
            "old": old_value,
            "new": new_value,
        });
        let entry = AuditLogEntry {
            id: Uuid::new_v4(),
            event_type: "policy_changed".into(),
            actor,
            target_type: Some("policy".into()),
            target_id: Some(field.to_string()),
            detail_json: Some(detail.to_string()),
            timestamp: now,
        };
        self.repo.insert(&entry).await?;
        self.events.publish(
            ServiceEventInner::PolicyChanged {
                field: field.to_string(),
                old_value,
                new_value,
            }
            .with_timestamp(now),
        );
        Ok(())
    }

    pub async fn record_route_changed(
        &self,
        app_id: Uuid,
        old_route: Option<TrafficRoute>,
        new_route: Option<TrafficRoute>,
        actor: Option<String>,
    ) -> shared_types::Result<()> {
        let now = Utc::now();
        let detail = serde_json::json!({
            "old_route": old_route,
            "new_route": new_route,
        });
        let entry = AuditLogEntry {
            id: Uuid::new_v4(),
            event_type: "route_changed".into(),
            actor,
            target_type: Some("app".into()),
            target_id: Some(app_id.to_string()),
            detail_json: Some(detail.to_string()),
            timestamp: now,
        };
        self.repo.insert(&entry).await?;
        self.events.publish(
            ServiceEventInner::RouteChanged {
                app_id,
                old_route,
                new_route,
            }
            .with_timestamp(now),
        );
        Ok(())
    }

    pub async fn list(&self, query: AuditLogQuery) -> shared_types::Result<Vec<AuditLogEntry>> {
        self.repo.list(query).await
    }
}
