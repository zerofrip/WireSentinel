use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Cloud billing plan identifiers (Phase 14).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BillingPlanId {
    Free,
    Starter,
    Pro,
    Enterprise,
    EnterprisePlus,
}

impl BillingPlanId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Starter => "starter",
            Self::Pro => "pro",
            Self::Enterprise => "enterprise",
            Self::EnterprisePlus => "enterprise_plus",
        }
    }
}

/// Usage metering payload pushed from controller or core agents to cloud.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct CloudUsagePayload {
    pub tenant_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    pub metric: String,
    pub quantity: f64,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Fleet health rollup pushed to cloud.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct CloudHealthPayload {
    pub tenant_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller_id: Option<String>,
    pub healthy: bool,
    pub reporting_devices: u32,
    pub healthy_devices: u32,
    #[serde(default)]
    pub kernel_devices: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anonymity_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct CloudLogEntry {
    pub level: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<serde_json::Value>,
}

/// Batched log lines forwarded to cloud aggregation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct CloudLogBatch {
    pub tenant_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    pub entries: Vec<CloudLogEntry>,
    pub ingested_at: DateTime<Utc>,
}

/// Recovery lifecycle event payload for cloud DR coordination.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct RecoveryEventPayload {
    pub tenant_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<String>,
    pub scope: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

/// Quota / plan threshold crossing notification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct UsageThresholdReached {
    pub tenant_id: Uuid,
    pub metric: String,
    pub threshold: f64,
    pub current: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<BillingPlanId>,
}
