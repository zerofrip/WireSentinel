use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{TrafficRoute, Verdict};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelIface {
    pub profile_id: Uuid,
    pub name: String,
    pub luid: u64,
    #[serde(default)]
    pub socks_port: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DnsBlockMode {
    Null,
    #[default]
    Nxdomain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteStatisticsRecord {
    pub id: Uuid,
    pub app_id: Option<Uuid>,
    pub profile_id: Option<Uuid>,
    pub domain: Option<String>,
    pub route_type: String,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub connection_count: u32,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub event_type: String,
    pub actor: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub detail_json: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainCacheEntry {
    pub id: Uuid,
    pub app_id: Option<Uuid>,
    pub domain: String,
    pub ip_address: String,
    pub wildcard: bool,
    pub expires_at: DateTime<Utc>,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub hit_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WfpFilterStateRecord {
    pub id: Uuid,
    pub scope_type: String,
    pub scope_value: Option<String>,
    pub filter_id: u64,
    pub route: TrafficRoute,
    pub rule_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallDecisionRecord {
    pub id: Uuid,
    pub app_id: Option<Uuid>,
    pub domain: Option<String>,
    pub dest_ip: Option<String>,
    pub route: TrafficRoute,
    pub verdict: Verdict,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RouteStatisticsQuery {
    pub app_id: Option<Uuid>,
    pub domain: Option<String>,
    pub route_type: Option<String>,
    pub limit: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditLogQuery {
    pub event_type: Option<String>,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpnConfigFileRecord {
    pub profile_id: Uuid,
    pub disk_path: String,
    pub materialized_at: DateTime<Utc>,
}
