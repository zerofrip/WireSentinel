use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProxyChainHopKind {
    Socks5,
    Http,
    Https,
    Tor,
    TlsTunnel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct ProxyChainHop {
    pub kind: ProxyChainHopKind,
    pub profile_id: Uuid,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct ProxyChain {
    pub id: Uuid,
    pub name: String,
    pub hops: Vec<ProxyChainHop>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct AgentRecord {
    pub id: Uuid,
    pub name: String,
    pub enrolled_at: DateTime<Utc>,
}
