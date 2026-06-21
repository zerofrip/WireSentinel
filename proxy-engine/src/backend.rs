use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use shared_types::{ProxyProfile, Result};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxyHealth {
    pub healthy: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyState {
    Disconnected,
    Connected,
    Failed,
}

#[derive(Debug, Clone)]
pub struct ProxyStatus {
    pub state: ProxyState,
    pub listen_port: Option<u16>,
    pub last_error: Option<String>,
}

#[async_trait]
pub trait ProxyBackend: Send + Sync {
    fn profile_id(&self) -> Uuid;
    fn profile(&self) -> &ProxyProfile;
    async fn connect(&self) -> Result<u16>;
    async fn disconnect(&self) -> Result<()>;
    async fn health_check(&self) -> ProxyHealth;
    async fn measure_latency(&self) -> Result<u64>;
    fn status(&self) -> ProxyStatus;
}
