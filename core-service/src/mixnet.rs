//! Mixnet profile management backed by mixnet-core and Nym transport.

use chrono::Utc;
use event_bus::EventBus;
use mixnet_core::{MixnetManager, MixnetState};
use mixnet_transports::{NymBackend, PluginMixnetBackend};
use proxy_engine::ProxyListenPort;
use shared_types::{
    AnonymousRoute, MixnetHealth, MixnetProfile, MixnetProvider, MixnetRoute, MixnetSession,
    MixnetStatus, Result, ServiceEvent, ServiceEventInner, TransportState, WireSentinelError,
};
use std::sync::Arc;
use storage::Storage;
use uuid::Uuid;

use crate::mixnet_security::MixnetSecurityPolicy;

fn to_core_profile(profile: &MixnetProfile) -> mixnet_core::MixnetProfile {
    let provider = match &profile.provider {
        MixnetProvider::Nym => "nym".to_string(),
        MixnetProvider::Plugin(id) => format!("plugin:{id}"),
    };
    mixnet_core::MixnetProfile {
        id: profile.id,
        name: profile.name.clone(),
        provider,
        enabled: profile.enabled,
        created_at: profile.created_at,
        updated_at: profile.updated_at,
    }
}

pub struct MixnetService {
    storage: Arc<Storage>,
    events: EventBus,
    manager: Arc<MixnetManager>,
    listen_ports: Arc<ProxyListenPort>,
    security: Arc<MixnetSecurityPolicy>,
}

impl MixnetService {
    pub fn new(
        storage: Arc<Storage>,
        events: EventBus,
        manager: Arc<MixnetManager>,
        listen_ports: Arc<ProxyListenPort>,
        security: Arc<MixnetSecurityPolicy>,
    ) -> Self {
        Self {
            storage,
            events,
            manager,
            listen_ports,
            security,
        }
    }

    pub fn manager(&self) -> Arc<MixnetManager> {
        Arc::clone(&self.manager)
    }

    pub fn listen_ports(&self) -> Arc<ProxyListenPort> {
        Arc::clone(&self.listen_ports)
    }

    pub async fn list(&self) -> Result<Vec<MixnetProfile>> {
        self.storage.mixnet_profiles.list().await
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<MixnetProfile>> {
        self.storage.mixnet_profiles.get(id).await
    }

    pub async fn create(&self, mut profile: MixnetProfile) -> Result<MixnetProfile> {
        let now = Utc::now();
        if profile.id.is_nil() {
            profile.id = Uuid::new_v4();
        }
        if profile.created_at.timestamp() == 0 {
            profile.created_at = now;
        }
        profile.updated_at = now;
        self.security.validate_profile(&profile)?;
        self.storage.mixnet_profiles.insert(&profile).await?;
        Ok(profile)
    }

    pub async fn update(&self, profile: MixnetProfile) -> Result<MixnetProfile> {
        let old = self
            .storage
            .mixnet_profiles
            .get(profile.id)
            .await?
            .ok_or_else(|| {
                WireSentinelError::Other(format!("mixnet profile {} not found", profile.id))
            })?;

        self.security.validate_profile(&profile)?;
        let mut profile = profile;
        profile.updated_at = Utc::now();
        self.storage.mixnet_profiles.update(&profile).await?;

        if old.gateway_id != profile.gateway_id {
            if let Some(gateway_id) = &profile.gateway_id {
                self.events
                    .publish(ServiceEvent::now(ServiceEventInner::GatewayChanged {
                        profile_id: profile.id,
                        gateway_id: gateway_id.clone(),
                    }));
            }
        }

        Ok(profile)
    }

    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let _ = self.stop(id, "profile deleted").await;
        self.storage.mixnet_profiles.delete(id).await
    }

    pub async fn start(&self, profile_id: Uuid) -> Result<MixnetProfile> {
        let mut profile = self
            .storage
            .mixnet_profiles
            .get(profile_id)
            .await?
            .ok_or_else(|| {
                WireSentinelError::Other(format!("mixnet profile {profile_id} not found"))
            })?;

        self.security.validate_profile(&profile)?;

        let core_profile = to_core_profile(&profile);
        let backend: Arc<dyn mixnet_core::MixnetBackend> = match &profile.provider {
            MixnetProvider::Nym => NymBackend::new(core_profile),
            MixnetProvider::Plugin(plugin_id) => {
                PluginMixnetBackend::new(core_profile, format!("plugin:{plugin_id}"))
            }
        };

        match self.manager.start(backend).await {
            Ok(port) => {
                self.listen_ports.set(profile_id, port);
                profile.active = true;
                profile.last_error = None;
                profile.updated_at = Utc::now();
                self.storage.mixnet_profiles.update(&profile).await?;

                let session = MixnetSession {
                    id: Uuid::new_v4(),
                    profile_id,
                    route: None,
                    state: TransportState::Running,
                    started_at: Utc::now(),
                    ended_at: None,
                    rx_bytes: 0,
                    tx_bytes: 0,
                };
                let _ = self.storage.mixnet_sessions.insert(&session).await;

                self.events
                    .publish(ServiceEvent::now(ServiceEventInner::MixnetStarted {
                        profile_id,
                    }));
                Ok(profile)
            }
            Err(e) => {
                profile.active = false;
                profile.last_error = Some(e.to_string());
                profile.updated_at = Utc::now();
                let _ = self.storage.mixnet_profiles.update(&profile).await;
                self.events
                    .publish(ServiceEvent::now(ServiceEventInner::MixnetFailed {
                        profile_id,
                        error: e.to_string(),
                    }));
                Err(e)
            }
        }
    }

    pub async fn stop(&self, profile_id: Uuid, reason: &str) -> Result<()> {
        self.manager.stop(profile_id).await?;
        self.listen_ports.remove(profile_id);

        if let Some(mut profile) = self.storage.mixnet_profiles.get(profile_id).await? {
            profile.active = false;
            profile.updated_at = Utc::now();
            self.storage.mixnet_profiles.update(&profile).await?;
        }

        self.events
            .publish(ServiceEvent::now(ServiceEventInner::MixnetStopped {
                profile_id,
                reason: reason.to_string(),
            }));
        Ok(())
    }

    pub async fn health_check(&self, profile_id: Uuid) -> Result<MixnetHealth> {
        let core_health = self.manager.health_check(profile_id).await?;
        let health = MixnetHealth {
            healthy: core_health.healthy,
            latency_ms: None,
            message: core_health.message,
            last_check: Some(core_health.checked_at),
        };

        if let Some(mut profile) = self.storage.mixnet_profiles.get(profile_id).await? {
            profile.last_health_at = health.last_check;
            profile.last_error = if health.healthy {
                None
            } else {
                health.message.clone()
            };
            profile.updated_at = Utc::now();
            self.storage.mixnet_profiles.update(&profile).await?;
        }

        if !health.healthy {
            self.events
                .publish(ServiceEvent::now(ServiceEventInner::MixnetFailed {
                    profile_id,
                    error: health
                        .message
                        .clone()
                        .unwrap_or_else(|| "health check failed".into()),
                }));
        }

        Ok(health)
    }

    pub async fn measure_latency(&self, profile_id: Uuid) -> Result<u64> {
        let profile = self
            .storage
            .mixnet_profiles
            .get(profile_id)
            .await?
            .ok_or_else(|| {
                WireSentinelError::Other(format!("mixnet profile {profile_id} not found"))
            })?;

        let core_profile = to_core_profile(&profile);
        let latency_ms = self.manager.measure_latency(&core_profile).await?;

        let mut profile = profile;
        profile.latency_ms = Some(latency_ms);
        profile.updated_at = Utc::now();
        self.storage.mixnet_profiles.update(&profile).await?;
        Ok(latency_ms)
    }

    pub async fn status(&self) -> Result<MixnetStatus> {
        let profiles = self.storage.mixnet_profiles.list().await?;
        let active = profiles.iter().find(|p| p.active).cloned();
        let profile_id = active.as_ref().map(|p| p.id);
        let gateway_id = active.as_ref().and_then(|p| p.gateway_id.clone());
        let latency_ms = active.as_ref().and_then(|p| p.latency_ms);

        let running = profile_id
            .and_then(|id| self.manager.status(id))
            .map(|s| s.state == MixnetState::Running)
            .unwrap_or(false);

        let sessions = if let Some(id) = profile_id {
            self.storage
                .mixnet_sessions
                .list_by_profile(id, 100)
                .await?
        } else {
            vec![]
        };
        let active_sessions = sessions
            .iter()
            .filter(|s| s.state == TransportState::Running && s.ended_at.is_none())
            .count() as u32;

        Ok(MixnetStatus {
            running,
            profile_id,
            gateway_id,
            latency_ms,
            active_sessions,
            profile: active,
        })
    }

    pub async fn list_routes(&self, profile_id: Uuid) -> Result<Vec<MixnetRoute>> {
        let sessions = self
            .storage
            .mixnet_sessions
            .list_by_profile(profile_id, 50)
            .await?;
        Ok(sessions.into_iter().filter_map(|s| s.route).collect())
    }

    pub async fn ensure_route_ready(&self, route: &AnonymousRoute) -> Result<u16> {
        match route {
            AnonymousRoute::FutureMixnet(id) => self.ensure_profile_ready(*id).await,
            _ => Err(WireSentinelError::Other(
                "ensure_route_ready only supports FutureMixnet".into(),
            )),
        }
    }

    pub async fn ensure_profile_ready(&self, profile_id: Uuid) -> Result<u16> {
        if let Some(port) = self.listen_ports.get(profile_id) {
            if self
                .manager
                .status(profile_id)
                .map(|s| s.state == MixnetState::Running)
                .unwrap_or(false)
            {
                return Ok(port);
            }
        }

        if let Some(profile) = self.storage.mixnet_profiles.get(profile_id).await? {
            if profile.active {
                if let Some(port) = self.listen_ports.get(profile_id) {
                    return Ok(port);
                }
            }
        }

        self.start(profile_id).await?;
        self.listen_ports
            .get(profile_id)
            .ok_or_else(|| WireSentinelError::Other("mixnet listen port unavailable".into()))
    }
}
