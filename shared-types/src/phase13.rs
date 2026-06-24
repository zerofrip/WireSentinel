use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Phase 13 anonymity transport providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnonymityProvider {
    Katzenpost,
    Loopix,
    FederatedMixnet,
    Plugin(Uuid),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct KatzenpostGateway {
    pub id: String,
    pub address: String,
    pub identity_key: Option<String>,
    pub latency_ms: Option<u64>,
    pub last_seen: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct KatzenpostRoute {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub entry_gateway: String,
    pub exit_gateway: String,
    pub hop_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct KatzenpostProfile {
    pub id: Uuid,
    pub name: String,
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
pub struct LoopixProvider {
    pub id: String,
    pub address: String,
    pub region: Option<String>,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct LoopixRoute {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub provider_id: String,
    pub hop_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct LoopixProfile {
    pub id: Uuid,
    pub name: String,
    pub provider_id: Option<String>,
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
pub struct AnonymousService {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub provider: AnonymityProvider,
    pub profile_id: Uuid,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct AnonymousServiceEndpoint {
    pub id: Uuid,
    pub service_id: Uuid,
    pub host: String,
    pub port: u16,
    pub protocol: String,
    pub path: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct FederatedMixnetConfig {
    pub providers: Vec<String>,
    pub profile_id: Uuid,
    pub quorum: u32,
    pub enabled: bool,
}

/// Phase 13 advanced privacy analytics fields (additive to phase 10 snapshot).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct PrivacyAnalyticsPhase13Fields {
    #[serde(default)]
    pub anonymity_set_estimate: Option<f64>,
    #[serde(default)]
    pub cover_traffic_efficiency: Option<f64>,
    #[serde(default)]
    pub mixnet_diversity: Option<f64>,
    #[serde(default)]
    pub federation_diversity: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct AnonymityStatus {
    pub katzenpost_active: bool,
    pub loopix_active: bool,
    pub federated_active: bool,
    pub active_providers: u32,
    pub entropy_score: f64,
    #[serde(default)]
    pub katzenpost_stub_mode: bool,
    #[serde(default)]
    pub loopix_stub_mode: bool,
    #[serde(default)]
    pub lab_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct EntropySnapshot {
    pub score: f64,
    pub anonymity_set_estimate: f64,
    pub captured_at: DateTime<Utc>,
}
