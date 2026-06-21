use crate::TransportKind;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum VpnBackendKind {
    WireGuardNt,
    AmneziaWg,
    Tailscale,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VPNProfile {
    pub id: Uuid,
    pub name: String,
    pub backend: VpnBackendKind,
    /// Path to DPAPI-encrypted .conf file.
    #[schema(value_type = String)]
    pub config_path: PathBuf,
    pub auto_connect: bool,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub transport_kind: TransportKind,
    #[serde(default)]
    pub chain_id: Option<Uuid>,
    #[serde(default)]
    pub obfuscation_profile_id: Option<Uuid>,
}

impl VPNProfile {
    pub fn new(name: String, backend: VpnBackendKind, config_path: PathBuf) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            backend,
            config_path,
            auto_connect: false,
            created_at: Utc::now(),
            transport_kind: TransportKind::Direct,
            chain_id: None,
            obfuscation_profile_id: None,
            handshake_proxy: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VpnStatus {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
    Error,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VpnStats {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub last_handshake: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpnState {
    pub profile_id: Uuid,
    pub profile_name: String,
    pub status: VpnStatus,
    pub stats: VpnStats,
}
