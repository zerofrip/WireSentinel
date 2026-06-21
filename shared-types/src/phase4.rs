use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::HandshakeProxySettings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum TransportKind {
    #[default]
    Direct,
    WireGuard,
    AmneziaWg,
    SingBox,
    Xray,
    Tor,
    TlsTunnel,
    WebSocketTunnel,
    Mixnet,
    Proxy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransportProfileKind {
    SingBox,
    Xray,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ObfuscationPreset {
    Disabled,
    Basic,
    Balanced,
    Aggressive,
    Lwo,
    Socks5Handshake,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LeakType {
    Dns,
    Route,
    VpnDisconnect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransportState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChainHop {
    pub kind: TransportKind,
    pub profile_id: Option<Uuid>,
    pub transport_profile_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChainProfile {
    pub id: Uuid,
    pub name: String,
    pub hops: Vec<ChainHop>,
    pub obfuscation_profile_id: Option<Uuid>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ObfuscationProfile {
    pub id: Uuid,
    pub name: String,
    pub preset: ObfuscationPreset,
    pub modules_json: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handshake_proxy: Option<HandshakeProxySettings>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TransportProfile {
    pub id: Uuid,
    pub name: String,
    pub transport_kind: TransportProfileKind,
    pub config_json: Option<String>,
    #[schema(value_type = String)]
    pub config_path: Option<PathBuf>,
    #[schema(value_type = String)]
    pub binary_path: Option<PathBuf>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DnsProviderRecord {
    pub id: Uuid,
    pub name: String,
    pub transport: crate::DnsTransport,
    pub endpoint: String,
    pub priority: i32,
    pub enabled: bool,
    pub latency_ms: Option<u64>,
    pub last_check: Option<DateTime<Utc>>,
    pub failure_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LeakIncident {
    pub id: Uuid,
    pub leak_type: LeakType,
    pub app_id: Option<Uuid>,
    pub detail_json: Option<String>,
    pub severity: String,
    pub detected_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct PrivacyScoreComponents {
    pub encrypted_dns: u8,
    pub blocked_trackers: u8,
    pub vpn_coverage: u8,
    pub route_leakage: u8,
    pub dns_leakage: u8,
    #[serde(default)]
    pub anonymity_score: Option<u8>,
    #[serde(default)]
    pub route_entropy: Option<f64>,
    #[serde(default)]
    pub path_diversity: Option<f64>,
    #[serde(default)]
    pub cover_traffic_effectiveness: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PrivacyScoreSnapshot {
    pub id: Uuid,
    pub score: u8,
    pub components: PrivacyScoreComponents,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TransportStatusRecord {
    pub id: Uuid,
    pub name: String,
    pub kind: TransportProfileKind,
    pub state: TransportState,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TransportHealth {
    pub healthy: bool,
    pub latency_ms: Option<u64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProviderHealth {
    pub available: bool,
    pub latency_ms: Option<u64>,
    pub message: Option<String>,
}
