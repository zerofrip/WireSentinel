use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeStateRecord {
    pub id: Uuid,
    pub scope: String,
    pub entity_id: String,
    pub state_json: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PerformanceSnapshot {
    pub id: Uuid,
    pub cpu_percent: f64,
    pub memory_bytes: u64,
    pub api_latency_ms: f64,
    pub wfp_latency_ms: f64,
    pub event_throughput: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct EnterprisePolicy {
    pub id: Uuid,
    pub version: u32,
    pub policy_json: serde_json::Value,
    pub locked_keys: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackupManifestEntry {
    pub id: Uuid,
    pub operation: String,
    pub format: String,
    pub checksum: String,
    pub created_at: DateTime<Utc>,
    pub detail_json: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackupBundle {
    pub version: u32,
    pub exported_at: DateTime<Utc>,
    pub settings: serde_json::Value,
    pub vpn_profiles: Vec<serde_json::Value>,
    pub rules: Vec<serde_json::Value>,
    pub apps: Vec<serde_json::Value>,
    pub dns_settings: serde_json::Value,
    pub dns_providers: Vec<serde_json::Value>,
    pub filter_lists: Vec<serde_json::Value>,
    pub transport_profiles: Vec<serde_json::Value>,
    pub chain_profiles: Vec<serde_json::Value>,
    pub obfuscation_profiles: Vec<serde_json::Value>,
    #[serde(default)]
    pub proxy_profiles: Vec<serde_json::Value>,
    #[serde(default)]
    pub proxy_chains: Vec<serde_json::Value>,
    #[serde(default)]
    pub mixnet_profiles: Vec<serde_json::Value>,
    #[serde(default)]
    pub anonymous_chains: Vec<serde_json::Value>,
    #[serde(default)]
    pub cover_traffic_settings: Vec<serde_json::Value>,
    pub enterprise_policy: Option<EnterprisePolicy>,
    /// Backup bundle schema revision (1 = original, 2 = adds cloud/agent fields).
    #[serde(default = "default_backup_bundle_version")]
    pub bundle_version: u32,
    #[serde(default)]
    pub plugins: Vec<serde_json::Value>,
    #[serde(default)]
    pub tailnet_profiles: Vec<serde_json::Value>,
    #[serde(default)]
    pub tor_profiles: Vec<serde_json::Value>,
    #[serde(default)]
    pub controller_agent_config: Option<serde_json::Value>,
    #[serde(default)]
    pub cloud_sync_config: Option<serde_json::Value>,
}

fn default_backup_bundle_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiagnosticsBundle {
    pub generated_at: DateTime<Utc>,
    pub version: String,
    pub health: DiagnosticsHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DiagnosticsHealth {
    pub wfp: SubsystemHealth,
    pub vpn: SubsystemHealth,
    pub dns: SubsystemHealth,
    pub transport: SubsystemHealth,
    pub database: SubsystemHealth,
    pub disk: SubsystemHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SubsystemHealth {
    pub status: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub channel: String,
    pub staged_percent: u8,
    pub download_url: Option<String>,
    pub update_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricsSnapshot {
    pub active_tunnels: u32,
    pub active_transports: u32,
    pub blocked_requests: u64,
    pub dns_queries: u64,
    pub open_leak_incidents: u32,
    pub route_changes_24h: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    #[default]
    Info,
    Warn,
    Error,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn as_filter(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecurityAuditEntry {
    pub action: String,
    pub actor: Option<String>,
    pub detail: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub target: String,
    pub message: String,
}
