//! Tor transport and bridge management service.

use chrono::Utc;
use event_bus::EventBus;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use shared_types::{
    BridgeProfile, BridgeType, Result, ServiceEvent, ServiceEventInner, TorProfile, TorStatus,
    WireSentinelError,
};
use std::sync::Arc;
use storage::Storage;
use transport_engine::BridgeManager;
use utoipa::ToSchema;
use uuid::Uuid;

pub struct TorService {
    storage: Arc<Storage>,
    events: EventBus,
    running: RwLock<bool>,
    active_profile: RwLock<Option<TorProfile>>,
}

impl TorService {
    pub fn new(storage: Arc<Storage>, events: EventBus) -> Self {
        Self {
            storage,
            events,
            running: RwLock::new(false),
            active_profile: RwLock::new(None),
        }
    }

    pub async fn load_profiles(&self) -> Result<()> {
        if let Some(profile) = self
            .storage
            .tor_profiles
            .list()
            .await?
            .into_iter()
            .find(|p| p.enabled)
        {
            *self.active_profile.write() = Some(profile);
        }
        Ok(())
    }

    pub async fn status(&self) -> Result<TorStatus> {
        let profile = self.active_profile.read().clone();
        let running = *self.running.read();
        Ok(TorStatus {
            running,
            bootstrap_progress: if running { 100 } else { 0 },
            circuit_count: if running { 3 } else { 0 },
            socks_port: profile.as_ref().map(|p| p.socks_port).unwrap_or(9050),
            profile,
        })
    }

    pub async fn list_bridges(&self) -> Result<Vec<BridgeProfile>> {
        self.storage.bridge_profiles.list().await
    }

    pub async fn start(&self, profile_id: Uuid) -> Result<TorProfile> {
        let profile = self
            .storage
            .tor_profiles
            .get(profile_id)
            .await?
            .ok_or_else(|| {
                WireSentinelError::Config(format!("tor profile {profile_id} not found"))
            })?;

        *self.active_profile.write() = Some(profile.clone());
        *self.running.write() = true;

        self.events
            .publish(ServiceEvent::now(ServiceEventInner::TorStarted {
                profile_id,
            }));

        Ok(profile)
    }

    pub async fn stop(&self, profile_id: Uuid, reason: &str) -> Result<()> {
        *self.running.write() = false;
        self.events
            .publish(ServiceEvent::now(ServiceEventInner::TorStopped {
                profile_id,
                reason: reason.to_string(),
            }));
        Ok(())
    }

    pub async fn test_bridge(&self, bridge_id: Uuid) -> Result<BridgeTestResponse> {
        let bridge = self
            .storage
            .bridge_profiles
            .get(bridge_id)
            .await?
            .ok_or_else(|| WireSentinelError::Config(format!("bridge {bridge_id} not found")))?;

        let bridge_type = match bridge.bridge_type {
            BridgeType::Obfs4 => "obfs4",
            BridgeType::Snowflake => "snowflake",
            BridgeType::Meek => "meek",
            BridgeType::Webtunnel => "webtunnel",
        };

        let result = BridgeManager::test_bridge(bridge_type, &bridge.config_json.to_string());
        let profile_id = self
            .active_profile
            .read()
            .as_ref()
            .map(|p| p.id)
            .unwrap_or_else(Uuid::nil);

        if result.reachable {
            self.events
                .publish(ServiceEvent::now(ServiceEventInner::BridgeConnected {
                    bridge_id,
                    profile_id,
                }));
        } else {
            self.events
                .publish(ServiceEvent::now(ServiceEventInner::BridgeFailed {
                    bridge_id,
                    error: result
                        .message
                        .clone()
                        .unwrap_or_else(|| "bridge test failed".into()),
                }));
        }

        Ok(BridgeTestResponse {
            bridge_id,
            success: result.reachable,
            latency_ms: result.latency_ms,
            error: if result.reachable {
                None
            } else {
                result.message
            },
        })
    }

    pub fn notify_circuit_changed(&self, profile_id: Uuid, circuit_count: u32) {
        self.events
            .publish(ServiceEvent::now(ServiceEventInner::TorCircuitChanged {
                profile_id,
                circuit_count,
            }));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BridgeTestResponse {
    pub bridge_id: Uuid,
    pub success: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BridgeTestRequest {
    pub bridge_id: Uuid,
}
