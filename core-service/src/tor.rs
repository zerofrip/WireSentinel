//! Tor transport and bridge management via sing-box tor outbound.
use crate::tor_control::{control_port_for_socks, poll_tor_metrics};

use crate::binary_paths::{default_tor_data_dir, resolve_tor_exe};
use chrono::Utc;
use event_bus::EventBus;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use shared_types::{
    BridgeProfile, Result, RuntimeStateRecord, ServiceEvent, ServiceEventInner, TorProfile,
    TorStatus, WireSentinelError,
};
use std::path::PathBuf;
use std::time::Duration;
use std::sync::Arc;
use storage::Storage;
use transport_engine::{
    bridge_line, bridges_to_tor_options, BridgeManager, TorOutboundSpec, TorSingBoxRunner,
};
use utoipa::ToSchema;
use uuid::Uuid;

pub struct TorService {
    storage: Arc<Storage>,
    events: EventBus,
    runner: Arc<TorSingBoxRunner>,
    bridge_manager: Arc<BridgeManager>,
    tor_exe_override: RwLock<Option<PathBuf>>,
    running: RwLock<bool>,
    active_profile: RwLock<Option<TorProfile>>,
    bootstrap_progress: RwLock<u8>,
}

impl TorService {
    pub fn new(
        storage: Arc<Storage>,
        events: EventBus,
        runner: Arc<TorSingBoxRunner>,
        bridge_manager: Arc<BridgeManager>,
    ) -> Self {
        Self {
            storage,
            events,
            runner,
            bridge_manager,
            tor_exe_override: RwLock::new(None),
            running: RwLock::new(false),
            active_profile: RwLock::new(None),
            bootstrap_progress: RwLock::new(0),
        }
    }

    pub fn runner(&self) -> Arc<TorSingBoxRunner> {
        Arc::clone(&self.runner)
    }

    pub fn set_tor_executable_override(&self, path: Option<PathBuf>) {
        *self.tor_exe_override.write() = path;
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

    async fn resolve_bridges(&self, profile: &TorProfile) -> Result<Vec<BridgeProfile>> {
        let mut bridges = Vec::new();
        for id in &profile.bridge_ids {
            if let Some(b) = self.storage.bridge_profiles.get(*id).await? {
                bridges.push(b);
            }
        }
        Ok(bridges)
    }

    fn build_spec(&self, profile: &TorProfile, bridges: &[BridgeProfile]) -> TorOutboundSpec {
        let (torrc, extra_args) = bridges_to_tor_options(bridges);
        let data_dir = if profile.data_dir.trim().is_empty() {
            default_tor_data_dir(profile.id)
        } else {
            PathBuf::from(&profile.data_dir)
        };

        let tor_exe = self
            .tor_exe_override
            .read()
            .clone()
            .unwrap_or_else(|| resolve_tor_exe(None));

        let mut torrc = torrc;
        let control_port = control_port_for_socks(profile.socks_port);
        torrc.insert("ControlPort".into(), control_port.to_string());
        torrc.insert("CookieAuthentication".into(), "0".into());
        TorOutboundSpec {
            executable_path: tor_exe,
            data_directory: data_dir,
            extra_args,
            torrc,
        }
    }

    pub async fn status(&self) -> Result<TorStatus> {
        let profile = self.active_profile.read().clone();
        let instance_id = profile.as_ref().map(|p| p.id);
        let process_running = instance_id
            .map(|id| self.runner.is_running(id))
            .unwrap_or(false);
        let running = *self.running.read() && process_running;
        let bootstrap = if running {
            *self.bootstrap_progress.read()
        } else {
            0
        };

        Ok(TorStatus {
            running,
            bootstrap_progress: bootstrap,
            circuit_count: profile.as_ref().map(|p| p.circuit_count).unwrap_or(0),
            socks_port: profile.as_ref().map(|p| p.socks_port).unwrap_or(9050),
            profile,
        })
    }

    pub async fn list_bridges(&self) -> Result<Vec<BridgeProfile>> {
        self.storage.bridge_profiles.list().await
    }

    async fn persist_runtime(&self, profile_id: Uuid, running: bool) -> Result<()> {
        self.storage
            .runtime_state
            .upsert(&RuntimeStateRecord {
                id: Uuid::new_v4(),
                scope: "tor".into(),
                entity_id: profile_id.to_string(),
                state_json: serde_json::json!({ "running": running }).to_string(),
                updated_at: Utc::now(),
            })
            .await
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

        let bridges = self.resolve_bridges(&profile).await?;
        let spec = self.build_spec(&profile, &bridges);

        *self.bootstrap_progress.write() = 0;
        self.runner
            .start_and_wait(profile_id, profile.socks_port, spec, None)
            .await?;

        let control_port = control_port_for_socks(profile.socks_port);
        let metrics = poll_tor_metrics(control_port, Duration::from_secs(120)).await.unwrap_or(crate::tor_control::TorMetrics { bootstrap_progress: 100, circuit_count: 1 });
        *self.bootstrap_progress.write() = metrics.bootstrap_progress;
        *self.active_profile.write() = Some(profile.clone());
        *self.running.write() = true;

        let mut updated = profile.clone();
        updated.bootstrap_progress = metrics.bootstrap_progress;
        updated.circuit_count = metrics.circuit_count;
        let _ = self.storage.tor_profiles.update(&updated).await;

        self.events
            .publish(ServiceEvent::now(ServiceEventInner::TorStarted {
                profile_id,
            }));

        let _ = self.persist_runtime(profile_id, true).await;

        Ok(updated)
    }

    pub async fn stop(&self, profile_id: Uuid, reason: &str) -> Result<()> {
        self.runner.stop(profile_id).await?;
        *self.running.write() = false;
        *self.bootstrap_progress.write() = 0;

        let profile_to_update = {
            let active = self.active_profile.read().clone();
            active.filter(|p| p.id == profile_id).map(|mut p| {
                p.bootstrap_progress = 0;
                p.circuit_count = 0;
                p
            })
        };
        if let Some(profile) = profile_to_update {
            let _ = self.storage.tor_profiles.update(&profile).await;
        }

        self.events
            .publish(ServiceEvent::now(ServiceEventInner::TorStopped {
                profile_id,
                reason: reason.to_string(),
            }));
        let _ = self.persist_runtime(profile_id, false).await;
        Ok(())
    }

    pub async fn test_bridge(&self, bridge_id: Uuid) -> Result<BridgeTestResponse> {
        let bridge = self
            .storage
            .bridge_profiles
            .get(bridge_id)
            .await?
            .ok_or_else(|| WireSentinelError::Config(format!("bridge {bridge_id} not found")))?;

        let line = bridge_line(&bridge).ok_or_else(|| {
            WireSentinelError::Config("bridge config_json missing \"line\" field".into())
        })?;

        let result = self.bridge_manager.test_bridge_line(&line).await;
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

    pub fn is_running_profile(&self, profile_id: Uuid) -> bool {
        *self.running.read() && self.runner.is_running(profile_id)
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
