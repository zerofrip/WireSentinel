//! Proxy profile and chain management backed by proxy-engine.

use chrono::Utc;
use event_bus::EventBus;
use proxy_engine::{ProxyHealth, ProxyManager, ProxyStatus};
use shared_types::{
    ProxyChain, ProxyProfile, Result, ServiceEvent, ServiceEventInner, WireSentinelError,
};
use std::sync::Arc;
use storage::Storage;
use uuid::Uuid;

pub struct ProxyService {
    storage: Arc<Storage>,
    events: EventBus,
    manager: Arc<ProxyManager>,
}

impl ProxyService {
    pub fn new(storage: Arc<Storage>, events: EventBus, manager: Arc<ProxyManager>) -> Self {
        Self {
            storage,
            events,
            manager,
        }
    }

    pub fn manager(&self) -> Arc<ProxyManager> {
        Arc::clone(&self.manager)
    }

    pub async fn list(&self) -> Result<Vec<ProxyProfile>> {
        self.storage.proxy_profiles.list().await
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<ProxyProfile>> {
        self.storage.proxy_profiles.get(id).await
    }

    pub async fn create(&self, mut profile: ProxyProfile) -> Result<ProxyProfile> {
        let now = Utc::now();
        if profile.id.is_nil() {
            profile.id = Uuid::new_v4();
        }
        if profile.created_at.timestamp() == 0 {
            profile.created_at = now;
        }
        profile.updated_at = now;
        self.storage.proxy_profiles.insert(&profile).await?;
        self.events.publish(ServiceEvent::now(ServiceEventInner::ProxyProfileCreated {
            profile: profile.clone(),
        }));
        Ok(profile)
    }

    pub async fn update(&self, profile: ProxyProfile) -> Result<ProxyProfile> {
        let mut profile = profile;
        profile.updated_at = Utc::now();
        self.storage.proxy_profiles.update(&profile).await?;
        self.events.publish(ServiceEvent::now(ServiceEventInner::ProxyProfileUpdated {
            profile: profile.clone(),
        }));
        Ok(profile)
    }

    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let _ = self.disconnect(id).await;
        self.storage.proxy_profiles.delete(id).await
    }

    pub async fn connect(&self, profile_id: Uuid) -> Result<ProxyProfile> {
        let mut profile = self
            .storage
            .proxy_profiles
            .get(profile_id)
            .await?
            .ok_or_else(|| WireSentinelError::Proxy(format!("proxy {profile_id} not found")))?;

        match self.manager.connect(&profile).await {
            Ok(port) => {
                profile.active = true;
                profile.last_error = None;
                profile.updated_at = Utc::now();
                self.storage.proxy_profiles.update(&profile).await?;
                self.events
                    .publish(ServiceEvent::now(ServiceEventInner::ProxyConnected {
                        profile_id,
                        listen_port: port,
                    }));
                Ok(profile)
            }
            Err(e) => {
                profile.active = false;
                profile.last_error = Some(e.to_string());
                profile.updated_at = Utc::now();
                let _ = self.storage.proxy_profiles.update(&profile).await;
                self.events.publish(ServiceEvent::now(ServiceEventInner::ProxyFailed {
                    profile_id,
                    error: e.to_string(),
                }));
                Err(e)
            }
        }
    }

    pub async fn disconnect(&self, profile_id: Uuid) -> Result<()> {
        self.manager.disconnect(profile_id).await?;
        if let Some(mut profile) = self.storage.proxy_profiles.get(profile_id).await? {
            profile.active = false;
            profile.updated_at = Utc::now();
            self.storage.proxy_profiles.update(&profile).await?;
        }
        self.events
            .publish(ServiceEvent::now(ServiceEventInner::ProxyDisconnected {
                profile_id,
                reason: "user requested".into(),
            }));
        Ok(())
    }

    pub async fn health_check(&self, profile_id: Uuid) -> Result<ProxyHealth> {
        let health = self.manager.health_check(profile_id).await?;
        if let Some(mut profile) = self.storage.proxy_profiles.get(profile_id).await? {
            profile.last_health_at = Some(Utc::now());
            profile.last_error = health.message.clone();
            profile.updated_at = Utc::now();
            self.storage.proxy_profiles.update(&profile).await?;
        }
        if !health.healthy {
            self.events.publish(ServiceEvent::now(ServiceEventInner::ProxyFailed {
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
            .proxy_profiles
            .get(profile_id)
            .await?
            .ok_or_else(|| WireSentinelError::Proxy(format!("proxy {profile_id} not found")))?;
        let latency_ms = self.manager.measure_latency(&profile).await?;
        let mut profile = profile;
        profile.latency_ms = Some(latency_ms);
        profile.updated_at = Utc::now();
        self.storage.proxy_profiles.update(&profile).await?;
        self.events
            .publish(ServiceEvent::now(ServiceEventInner::ProxyLatencyMeasured {
                profile_id,
                latency_ms,
            }));
        Ok(latency_ms)
    }

    pub fn status(&self, profile_id: Uuid) -> Option<ProxyStatus> {
        self.manager.status(profile_id)
    }

    pub async fn list_chains(&self) -> Result<Vec<ProxyChain>> {
        self.storage.proxy_chains.list().await
    }

    pub async fn get_chain(&self, id: Uuid) -> Result<Option<ProxyChain>> {
        self.storage.proxy_chains.get(id).await
    }

    pub async fn create_chain(&self, mut chain: ProxyChain) -> Result<ProxyChain> {
        let now = Utc::now();
        if chain.id.is_nil() {
            chain.id = Uuid::new_v4();
        }
        if chain.created_at.timestamp() == 0 {
            chain.created_at = now;
        }
        chain.updated_at = now;
        self.storage.proxy_chains.insert(&chain).await?;
        Ok(chain)
    }

    pub async fn update_chain(&self, chain: ProxyChain) -> Result<ProxyChain> {
        let mut chain = chain;
        chain.updated_at = Utc::now();
        self.storage.proxy_chains.update(&chain).await?;
        Ok(chain)
    }

    pub async fn delete_chain(&self, id: Uuid) -> Result<bool> {
        let _ = self.stop_chain(id).await;
        self.storage.proxy_chains.delete(id).await
    }

    pub async fn start_chain(&self, chain_id: Uuid) -> Result<ProxyChain> {
        let chain = self
            .storage
            .proxy_chains
            .get(chain_id)
            .await?
            .ok_or_else(|| WireSentinelError::Proxy(format!("proxy chain {chain_id} not found")))?;

        let mut ordered = chain.hops.clone();
        ordered.sort_by_key(|h| h.order);
        let mut profiles = Vec::new();
        for hop in ordered {
            if let Some(profile) = self.storage.proxy_profiles.get(hop.profile_id).await? {
                profiles.push(profile);
            }
        }

        let port = self
            .manager
            .start_chain_with_profiles(chain_id, &profiles)
            .await?;

        self.events.publish(ServiceEvent::now(ServiceEventInner::ProxyChainStarted {
            chain_id,
            name: chain.name.clone(),
        }));
        let _ = port;
        Ok(chain)
    }

    pub async fn stop_chain(&self, chain_id: Uuid) -> Result<()> {
        self.manager.stop_chain(chain_id).await?;
        self.events
            .publish(ServiceEvent::now(ServiceEventInner::ProxyChainStopped {
                chain_id,
                reason: "user requested".into(),
            }));
        Ok(())
    }

    pub async fn ensure_route_ready(&self, route: &shared_types::TrafficRoute) -> Result<u16> {
        match route {
            shared_types::TrafficRoute::Proxy(id) => {
                let profile = self
                    .storage
                    .proxy_profiles
                    .get(*id)
                    .await?
                    .ok_or_else(|| WireSentinelError::Proxy(format!("proxy {id} not found")))?;
                if profile.active {
                    if let Some(port) = self.manager.socks_port_for(*id) {
                        return Ok(port);
                    }
                }
                let connected = self.connect(*id).await?;
                let _ = connected;
                self.manager
                    .socks_port_for(*id)
                    .ok_or_else(|| WireSentinelError::Proxy("proxy listen port unavailable".into()))
            }
            shared_types::TrafficRoute::ProxyChain(id) => {
                if self.manager.is_chain_active(*id) {
                    if let Some(port) = self.manager.socks_port_for(*id) {
                        return Ok(port);
                    }
                }
                self.start_chain(*id).await?;
                self.manager
                    .socks_port_for(*id)
                    .ok_or_else(|| WireSentinelError::Proxy("proxy chain listen port unavailable".into()))
            }
            _ => Err(WireSentinelError::Proxy("not a proxy route".into())),
        }
    }
}
