use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ValidationStatus {
    Pass,
    Fail,
    Warn,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct ValidationCheck {
    pub id: Uuid,
    pub check_name: String,
    pub status: ValidationStatus,
    pub message: Option<String>,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct ValidationReport {
    pub overall_status: ValidationStatus,
    pub checks: Vec<ValidationCheck>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct BenchmarkSnapshot {
    pub id: Uuid,
    pub wfp_latency_ms: f64,
    pub route_latency_ms: f64,
    pub dns_latency_ms: f64,
    pub transport_startup_ms: f64,
    pub ui_event_throughput: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SecuritySeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct SecurityFinding {
    pub id: Uuid,
    pub severity: SecuritySeverity,
    pub category: String,
    pub title: String,
    pub detail_json: serde_json::Value,
    pub resolved: bool,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StressTestReport {
    pub connections_processed: u64,
    pub duration_ms: u64,
    pub memory_bytes_peak: u64,
    pub event_throughput: f64,
    pub errors: u32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct DriverState {
    pub engine: String,
    pub state: String,
    pub filter_count: u32,
    pub provider_registered: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReleaseManifestEntry {
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReleaseManifest {
    pub version: String,
    pub channel: String,
    pub build_date: DateTime<Utc>,
    pub arch: String,
    pub artifacts: Vec<ReleaseManifestEntry>,
}
