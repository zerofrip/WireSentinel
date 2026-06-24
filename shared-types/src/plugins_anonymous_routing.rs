use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::TrafficRoute;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginFormat {
    Wasm,
    Native,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    FilterEngine,
    DnsProvider,
    TransportBackend,
    TransformModule,
    PolicyProvider,
    MetricsProvider,
    MixnetBackend,
    AnonymityBackend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginPermission {
    ReadConfig,
    WriteConfig,
    Network,
    DnsResolve,
    SpawnProcess,
    FilterDomains,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginState {
    Installed,
    Loaded,
    Failed,
    Unloaded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct PluginManifest {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub format: PluginFormat,
    pub capabilities: Vec<PluginCapability>,
    pub permissions: Vec<PluginPermission>,
    pub min_core_version: String,
    pub path: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct PluginRecord {
    pub id: Uuid,
    pub manifest: PluginManifest,
    pub state: PluginState,
    pub error_message: Option<String>,
    pub installed_at: DateTime<Utc>,
    pub loaded_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct PluginSecurityPolicy {
    pub require_signature: bool,
    pub allowed_paths: Vec<String>,
    pub max_fuel: u64,
}

impl Default for PluginSecurityPolicy {
    fn default() -> Self {
        Self {
            require_signature: false,
            allowed_paths: vec![],
            max_fuel: 1_000_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum AnonymousRoute {
    Tor(Uuid),
    TorBridge(Uuid),
    MultiHop(Vec<Uuid>),
    FutureMixnet(Uuid),
    Katzenpost(Uuid),
    Loopix(Uuid),
    FederatedMixnet {
        providers: Vec<String>,
        profile_id: Uuid,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct IdentityRoute {
    pub route: TrafficRoute,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BridgeType {
    Obfs4,
    Snowflake,
    Meek,
    Webtunnel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct BridgeProfile {
    pub id: Uuid,
    pub name: String,
    pub bridge_type: BridgeType,
    pub config_json: serde_json::Value,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct TorProfile {
    pub id: Uuid,
    pub name: String,
    pub control_port: u16,
    pub socks_port: u16,
    pub data_dir: String,
    pub bridge_ids: Vec<Uuid>,
    pub enabled: bool,
    pub bootstrap_progress: u8,
    pub circuit_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct TailnetProfile {
    pub id: Uuid,
    pub name: String,
    pub auth_key: Option<String>,
    pub exit_node: Option<String>,
    pub subnet_router: bool,
    pub magic_dns: bool,
    pub hostname: Option<String>,
    pub tailnet_ip: Option<String>,
    pub connected: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProxyKind {
    Socks5,
    Http,
    Https,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct ProxyProfile {
    pub id: Uuid,
    pub name: String,
    pub kind: ProxyKind,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password_encrypted: Option<String>,
    pub enabled: bool,
    pub active: bool,
    pub latency_ms: Option<u64>,
    pub last_health_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct TlsTunnelConfig {
    pub remote_host: String,
    pub remote_port: u16,
    pub sni: Option<String>,
    pub alpn: Option<String>,
    pub verify_cert: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct WebSocketTunnelConfig {
    pub url: String,
    pub path: Option<String>,
    pub host_header: Option<String>,
    pub tls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct TailscaleStatus {
    pub connected: bool,
    pub hostname: Option<String>,
    pub tailnet_ip: Option<String>,
    pub exit_node: Option<String>,
    pub magic_dns: bool,
    pub profiles: Vec<TailnetProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct TorStatus {
    pub running: bool,
    pub bootstrap_progress: u8,
    pub circuit_count: u32,
    pub socks_port: u16,
    pub profile: Option<TorProfile>,
}
