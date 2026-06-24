//! Crash recovery via persisted runtime state snapshots.

use chrono::Utc;
use event_bus::EventBus;
use shared_types::{Result, RuntimeStateRecord, ServiceEventInner, VpnStatus};
use std::sync::Arc;
use storage::Storage;
use tracing::{info, warn};
use uuid::Uuid;
use vpn_engine::{materialize_profile_config, VpnManager};

use crate::tor::TorService;
use crate::transport::TransportManager;

pub struct RecoveryService {
    storage: Arc<Storage>,
    events: EventBus,
}

impl RecoveryService {
    pub fn new(storage: Arc<Storage>, events: EventBus) -> Self {
        Self { storage, events }
    }

    pub async fn persist_vpn(&self, profile_id: Uuid, connected: bool) -> Result<()> {
        let state_json = serde_json::json!({ "connected": connected }).to_string();
        self.storage
            .runtime_state
            .upsert(&RuntimeStateRecord {
                id: Uuid::new_v4(),
                scope: "vpn".into(),
                entity_id: profile_id.to_string(),
                state_json,
                updated_at: Utc::now(),
            })
            .await
    }

    pub async fn persist_transport(&self, transport_id: Uuid, running: bool) -> Result<()> {
        let state_json = serde_json::json!({ "running": running }).to_string();
        self.storage
            .runtime_state
            .upsert(&RuntimeStateRecord {
                id: Uuid::new_v4(),
                scope: "transport".into(),
                entity_id: transport_id.to_string(),
                state_json,
                updated_at: Utc::now(),
            })
            .await
    }

    pub async fn persist_chain(&self, chain_id: Uuid, running: bool) -> Result<()> {
        let state_json = serde_json::json!({ "running": running }).to_string();
        self.storage
            .runtime_state
            .upsert(&RuntimeStateRecord {
                id: Uuid::new_v4(),
                scope: "chain".into(),
                entity_id: chain_id.to_string(),
                state_json,
                updated_at: Utc::now(),
            })
            .await
    }

    pub async fn persist_dns_active(&self, provider_id: Option<Uuid>) -> Result<()> {
        let state_json = serde_json::json!({ "active_provider_id": provider_id }).to_string();
        self.storage
            .runtime_state
            .upsert(&RuntimeStateRecord {
                id: Uuid::new_v4(),
                scope: "dns".into(),
                entity_id: "active".into(),
                state_json,
                updated_at: Utc::now(),
            })
            .await
    }

    pub async fn persist_tor(&self, profile_id: Uuid, running: bool) -> Result<()> {
        let state_json = serde_json::json!({ "running": running }).to_string();
        self.storage
            .runtime_state
            .upsert(&RuntimeStateRecord {
                id: Uuid::new_v4(),
                scope: "tor".into(),
                entity_id: profile_id.to_string(),
                state_json,
                updated_at: Utc::now(),
            })
            .await
    }

    pub async fn recover_all(
        &self,
        vpn: &VpnManager,
        transport: &TransportManager,
        tor: &TorService,
        enabled: bool,
    ) -> Result<u32> {
        if !enabled {
            return Ok(0);
        }

        self.events.publish(
            ServiceEventInner::RecoveryStarted {
                scope: "all".into(),
            }
            .with_timestamp(Utc::now()),
        );

        let mut restored = 0u32;

        restored += self.recover_vpn(vpn).await;
        restored += self.recover_chains(transport).await;
        restored += self.recover_tor(tor).await;

        self.events.publish(
            ServiceEventInner::RecoveryCompleted {
                restored_count: restored,
            }
            .with_timestamp(Utc::now()),
        );

        info!(restored, "recovery completed");
        Ok(restored)
    }

    async fn recover_vpn(&self, vpn: &VpnManager) -> u32 {
        let mut count = 0u32;
        let records = match self.storage.runtime_state.list_by_scope("vpn").await {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, "failed to load vpn runtime state");
                return 0;
            }
        };

        for record in records {
            let Ok(state) = serde_json::from_str::<serde_json::Value>(&record.state_json) else {
                continue;
            };
            let connected = state
                .get("connected")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !connected {
                continue;
            }
            let Ok(profile_id) = Uuid::parse_str(&record.entity_id) else {
                continue;
            };
            let Some(profile) = vpn.get_profile(profile_id) else {
                self.emit_failed("vpn", format!("profile {profile_id} missing"));
                continue;
            };
            let blob = match self.storage.vpn_profiles.get_config_blob(profile_id).await {
                Ok(b) => b,
                Err(e) => {
                    self.emit_failed("vpn", e.to_string());
                    continue;
                }
            };
            let config_path = match materialize_profile_config(&profile, blob.as_deref()) {
                Ok(p) => p,
                Err(e) => {
                    self.emit_failed("vpn", e.to_string());
                    continue;
                }
            };
            if vpn
                .connect_with_config(profile_id, config_path)
                .await
                .is_ok()
            {
                count += 1;
            } else {
                self.emit_failed("vpn", format!("connect failed for {profile_id}"));
            }
        }
        count
    }

    async fn recover_chains(&self, transport: &TransportManager) -> u32 {
        let mut count = 0u32;
        let records = match self.storage.runtime_state.list_by_scope("chain").await {
            Ok(r) => r,
            Err(_) => return 0,
        };

        for record in records {
            let Ok(state) = serde_json::from_str::<serde_json::Value>(&record.state_json) else {
                continue;
            };
            if !state
                .get("running")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                continue;
            }
            let Ok(chain_id) = Uuid::parse_str(&record.entity_id) else {
                continue;
            };
            let chain = match self.storage.chain_profiles.get(chain_id).await {
                Ok(Some(c)) => c,
                _ => {
                    self.emit_failed("chain", format!("chain {chain_id} missing"));
                    continue;
                }
            };
            if transport.start_chain(&chain).await.is_ok() {
                count += 1;
            } else {
                self.emit_failed("chain", format!("start failed for {chain_id}"));
            }
        }
        count
    }

    async fn recover_tor(&self, tor: &TorService) -> u32 {
        let mut count = 0u32;
        let records = match self.storage.runtime_state.list_by_scope("tor").await {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, "failed to load tor runtime state");
                return 0;
            }
        };

        for record in records {
            let Ok(state) = serde_json::from_str::<serde_json::Value>(&record.state_json) else {
                continue;
            };
            if !state
                .get("running")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                continue;
            }
            let Ok(profile_id) = Uuid::parse_str(&record.entity_id) else {
                continue;
            };
            if tor.start(profile_id).await.is_ok() {
                count += 1;
            } else {
                self.emit_failed("tor", format!("start failed for {profile_id}"));
            }
        }
        count
    }

    fn emit_failed(&self, scope: &str, error: String) {
        warn!(scope, error, "recovery partial failure");
        self.events.publish(
            ServiceEventInner::RecoveryFailed {
                scope: scope.into(),
                error,
            }
            .with_timestamp(Utc::now()),
        );
    }

    pub async fn flush_before_stop(
        &self,
        vpn: &VpnManager,
        transport: &TransportManager,
        tor: &TorService,
    ) -> Result<()> {
        for profile in vpn.profiles() {
            let connected = vpn
                .state(profile.id)
                .await
                .map(|s| matches!(s.status, VpnStatus::Connected))
                .unwrap_or(false);
            self.persist_vpn(profile.id, connected).await?;
        }
        for chain in self.storage.chain_profiles.list().await? {
            let running = transport
                .status()
                .await
                .map(|rows| {
                    rows.iter().any(|r| {
                        r.id == chain.id && r.state == shared_types::TransportState::Running
                    })
                })
                .unwrap_or(false);
            self.persist_chain(chain.id, running).await?;
        }
        for profile in self.storage.tor_profiles.list().await? {
            let running = tor.is_running_profile(profile.id);
            self.persist_tor(profile.id, running).await?;
        }
        Ok(())
    }
}
