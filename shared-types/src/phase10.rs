use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::TransportState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum MixnetProvider {
    Nym,
    Plugin(Uuid),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct MixnetProfile {
    pub id: Uuid,
    pub name: String,
    pub provider: MixnetProvider,
    pub gateway_id: Option<String>,
    pub config_json: Option<serde_json::Value>,
    pub enabled: bool,
    pub active: bool,
    pub latency_ms: Option<u64>,
    pub last_health_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct MixnetNode {
    pub id: String,
    pub address: String,
    pub identity_key: String,
    pub layer: u8,
    pub latency_ms: Option<u64>,
    pub last_seen: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct MixnetRoute {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub entry_gateway: String,
    pub exit_gateway: String,
    pub hop_count: u32,
    pub nodes: Vec<MixnetNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct MixnetSession {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub route: Option<MixnetRoute>,
    pub state: TransportState,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct MixnetStatus {
    pub running: bool,
    pub profile_id: Option<Uuid>,
    pub gateway_id: Option<String>,
    pub latency_ms: Option<u64>,
    pub active_sessions: u32,
    pub profile: Option<MixnetProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct MixnetHealth {
    pub healthy: bool,
    pub latency_ms: Option<u64>,
    pub message: Option<String>,
    pub last_check: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum CoverTrafficProfile {
    #[default]
    Disabled,
    Low,
    Medium,
    High,
    Maximum,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct CoverTrafficSettings {
    pub id: Uuid,
    pub mixnet_profile_id: Option<Uuid>,
    pub profile: CoverTrafficProfile,
    pub enabled: bool,
    pub rate_bps: Option<u64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnonymousChainHopKind {
    Tor,
    Vpn,
    Mixnet,
    Proxy,
    TlsTunnel,
    WebSocket,
    Katzenpost,
    Loopix,
    FederatedMixnet,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct AnonymousChainHop {
    pub kind: AnonymousChainHopKind,
    pub profile_id: Uuid,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct AnonymousChain {
    pub id: Uuid,
    pub name: String,
    pub hops: Vec<AnonymousChainHop>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct PrivacyAnalyticsSnapshot {
    pub id: Uuid,
    pub anonymity_score: u8,
    pub route_entropy: f64,
    pub path_diversity: f64,
    pub cover_traffic_effectiveness: f64,
    #[serde(default)]
    pub anonymity_set_estimate: Option<f64>,
    #[serde(default)]
    pub cover_traffic_efficiency: Option<f64>,
    #[serde(default)]
    pub mixnet_diversity: Option<f64>,
    #[serde(default)]
    pub federation_diversity: Option<f64>,
    pub timestamp: DateTime<Utc>,
}
