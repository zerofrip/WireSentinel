//! Anonymity provider orchestration (Katzenpost, Loopix, federated mixnet).

use anonymity_core::{
    AnonymityBackend, AnonymityProfile, AnonymityProvider as CoreProvider, AnonymitySession,
};
use anonymity_federation::MixnetFederationManager;
use anonymity_katzenpost::{KatzenpostBackend, KatzenpostGatewayDiscovery};
use anonymity_loopix::{LoopixBackend, LoopixProviderDiscovery};
use chrono::Utc;
use event_bus::EventBus;
use parking_lot::RwLock;
use proxy_engine::ProxyListenPort;
use shared_types::{
    AnonymityProvider, AnonymityStatus, AnonymousRoute, FederatedMixnetConfig, KatzenpostProfile,
    LoopixProfile, Result, ServiceEvent, ServiceEventInner, TrafficRoute, WireSentinelError,
};
use std::collections::HashMap;
use std::sync::Arc;
use storage::Storage;
use uuid::Uuid;

use crate::anonymity_security::AnonymitySecurityPolicy;

const DEFAULT_KATZENPOST_GATEWAYS: &str = r#"{"gateways":[{"id":"gw1","address":"127.0.0.1:4444","identity_key":"k1","country":"DE","latency_ms":25,"healthy":true,"last_seen":null}]}"#;
const DEFAULT_LOOPIX_PROVIDERS: &str = r#"{"providers":[{"id":"p1","address":"127.0.0.1:5555","public_key":"pk1","layer":1,"latency_ms":30,"healthy":true,"last_seen":null}]}"#;

pub struct AnonymityService {
    storage: Arc<Storage>,
    events: EventBus,
    listen_ports: Arc<ProxyListenPort>,
    security: Arc<AnonymitySecurityPolicy>,
    federation: Arc<MixnetFederationManager>,
    sessions: RwLock<HashMap<Uuid, AnonymitySession>>,
}

impl AnonymityService {
    pub fn new(
        storage: Arc<Storage>,
        events: EventBus,
        listen_ports: Arc<ProxyListenPort>,
        security: Arc<AnonymitySecurityPolicy>,
        federation: Arc<MixnetFederationManager>,
    ) -> Self {
        Self {
            storage,
            events,
            listen_ports,
            security,
            federation,
            sessions: RwLock::new(HashMap::new()),
        }
    }

    pub fn federation(&self) -> Arc<MixnetFederationManager> {
        Arc::clone(&self.federation)
    }

    pub async fn list_katzenpost(&self) -> Result<Vec<KatzenpostProfile>> {
        self.storage.katzenpost_profiles.list().await
    }

    pub async fn list_loopix(&self) -> Result<Vec<LoopixProfile>> {
        self.storage.loopix_profiles.list().await
    }

    pub async fn status(&self) -> Result<AnonymityStatus> {
        let katzenpost = self.storage.katzenpost_profiles.list().await?;
        let loopix = self.storage.loopix_profiles.list().await?;
        let katzenpost_active = katzenpost.iter().any(|p| p.active);
        let loopix_active = loopix.iter().any(|p| p.active);
        let active_providers = katzenpost.iter().filter(|p| p.active).count() as u32
            + loopix.iter().filter(|p| p.active).count() as u32;
        let entropy_score = self
            .sessions
            .read()
            .values()
            .map(|s| s.route.hop_count as f64)
            .sum::<f64>()
            / self.sessions.read().len().max(1) as f64;
        Ok(AnonymityStatus {
            katzenpost_active,
            loopix_active,
            federated_active: self.federation.discover().len() > 1,
            active_providers,
            entropy_score,
        })
    }

    fn katzenpost_backend(&self, profile: &KatzenpostProfile) -> Result<Arc<dyn AnonymityBackend>> {
        let core_profile = AnonymityProfile {
            id: profile.id,
            name: profile.name.clone(),
            provider: CoreProvider::Katzenpost,
            gateway_id: profile.gateway_id.clone(),
            config_json: profile.config_json.clone(),
            enabled: profile.enabled,
            created_at: profile.created_at,
            updated_at: profile.updated_at,
        };
        let discovery = KatzenpostGatewayDiscovery::from_json(DEFAULT_KATZENPOST_GATEWAYS)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(KatzenpostBackend::new(core_profile, discovery))
    }

    fn loopix_backend(&self, profile: &LoopixProfile) -> Result<Arc<dyn AnonymityBackend>> {
        let core_profile = AnonymityProfile {
            id: profile.id,
            name: profile.name.clone(),
            provider: CoreProvider::Loopix,
            gateway_id: profile.provider_id.clone(),
            config_json: profile.config_json.clone(),
            enabled: profile.enabled,
            created_at: profile.created_at,
            updated_at: profile.updated_at,
        };
        let discovery = LoopixProviderDiscovery::from_json(DEFAULT_LOOPIX_PROVIDERS)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(LoopixBackend::new(core_profile, discovery))
    }

    pub async fn start_katzenpost(&self, profile_id: Uuid) -> Result<KatzenpostProfile> {
        let mut profile = self
            .storage
            .katzenpost_profiles
            .get(profile_id)
            .await?
            .ok_or_else(|| {
                WireSentinelError::Other(format!("katzenpost profile {profile_id} not found"))
            })?;

        self.security.validate_katzenpost(&profile)?;
        let backend = self.katzenpost_backend(&profile)?;

        match backend.start().await {
            Ok(session) => {
                self.listen_ports.set(profile_id, session.socks_port);
                self.sessions.write().insert(profile_id, session);
                profile.active = true;
                profile.last_error = None;
                profile.updated_at = Utc::now();
                self.storage.katzenpost_profiles.update(&profile).await?;
                self.federation.register(Arc::clone(&backend));
                self.events.publish(ServiceEvent::now(ServiceEventInner::KatzenpostStarted {
                    profile_id,
                }));
                Ok(profile)
            }
            Err(e) => {
                profile.active = false;
                profile.last_error = Some(e.to_string());
                profile.updated_at = Utc::now();
                let _ = self.storage.katzenpost_profiles.update(&profile).await;
                self.events.publish(ServiceEvent::now(ServiceEventInner::KatzenpostFailed {
                    profile_id,
                    error: e.to_string(),
                }));
                Err(e)
            }
        }
    }

    pub async fn stop_katzenpost(&self, profile_id: Uuid, reason: &str) -> Result<()> {
        self.sessions.write().remove(&profile_id);
        self.listen_ports.remove(profile_id);
        if let Some(mut profile) = self.storage.katzenpost_profiles.get(profile_id).await? {
            profile.active = false;
            profile.updated_at = Utc::now();
            self.storage.katzenpost_profiles.update(&profile).await?;
        }
        self.events.publish(ServiceEvent::now(ServiceEventInner::KatzenpostStopped {
            profile_id,
            reason: reason.to_string(),
        }));
        Ok(())
    }

    pub async fn start_loopix(&self, profile_id: Uuid) -> Result<LoopixProfile> {
        let mut profile = self
            .storage
            .loopix_profiles
            .get(profile_id)
            .await?
            .ok_or_else(|| WireSentinelError::Other(format!("loopix profile {profile_id} not found")))?;

        self.security.validate_loopix(&profile)?;
        let backend = self.loopix_backend(&profile)?;

        match backend.start().await {
            Ok(session) => {
                self.listen_ports.set(profile_id, session.socks_port);
                self.sessions.write().insert(profile_id, session);
                profile.active = true;
                profile.last_error = None;
                profile.updated_at = Utc::now();
                self.storage.loopix_profiles.update(&profile).await?;
                self.federation.register(Arc::clone(&backend));
                self.events
                    .publish(ServiceEvent::now(ServiceEventInner::LoopixStarted { profile_id }));
                Ok(profile)
            }
            Err(e) => {
                profile.active = false;
                profile.last_error = Some(e.to_string());
                profile.updated_at = Utc::now();
                let _ = self.storage.loopix_profiles.update(&profile).await;
                self.events.publish(ServiceEvent::now(ServiceEventInner::LoopixFailed {
                    profile_id,
                    error: e.to_string(),
                }));
                Err(e)
            }
        }
    }

    pub async fn stop_loopix(&self, profile_id: Uuid, reason: &str) -> Result<()> {
        self.sessions.write().remove(&profile_id);
        self.listen_ports.remove(profile_id);
        if let Some(mut profile) = self.storage.loopix_profiles.get(profile_id).await? {
            profile.active = false;
            profile.updated_at = Utc::now();
            self.storage.loopix_profiles.update(&profile).await?;
        }
        self.events.publish(ServiceEvent::now(ServiceEventInner::LoopixStopped {
            profile_id,
            reason: reason.to_string(),
        }));
        Ok(())
    }

    pub async fn start_federated(&self, config: FederatedMixnetConfig) -> Result<u16> {
        self.security.validate_federation(&config)?;
        let route = self
            .federation
            .optimize_route()
            .ok_or_else(|| WireSentinelError::Other("no federated providers registered".into()))?;
        let port = route.socks_port.unwrap_or(9360);
        self.listen_ports.set(config.profile_id, port);
        self.events
            .publish(ServiceEvent::now(ServiceEventInner::MixnetFederationUpdated {
                profile_id: config.profile_id,
                providers: config.providers,
            }));
        Ok(port)
    }

    pub async fn ensure_route_ready(&self, route: &TrafficRoute) -> Result<u16> {
        match route {
            TrafficRoute::Katzenpost(id) => self.ensure_katzenpost_ready(*id).await,
            TrafficRoute::Loopix(id) => self.ensure_loopix_ready(*id).await,
            TrafficRoute::FederatedMixnet(id) => self.ensure_federated_ready(*id).await,
            TrafficRoute::Anonymous(anon) => self.ensure_anonymous_ready(anon).await,
            _ => Err(WireSentinelError::Policy(
                "route does not require anonymity transport".into(),
            )),
        }
    }

    async fn ensure_anonymous_ready(&self, route: &AnonymousRoute) -> Result<u16> {
        match route {
            AnonymousRoute::Katzenpost(id) => self.ensure_katzenpost_ready(*id).await,
            AnonymousRoute::Loopix(id) => self.ensure_loopix_ready(*id).await,
            AnonymousRoute::FederatedMixnet { profile_id, .. } => {
                self.ensure_federated_ready(*profile_id).await
            }
            _ => Err(WireSentinelError::Policy(
                "anonymous route variant not handled by anonymity service".into(),
            )),
        }
    }

    async fn ensure_katzenpost_ready(&self, profile_id: Uuid) -> Result<u16> {
        if let Some(port) = self.listen_ports.get(profile_id) {
            return Ok(port);
        }
        self.start_katzenpost(profile_id).await?;
        self.listen_ports.get(profile_id).ok_or_else(|| {
            WireSentinelError::Other("katzenpost listen port unavailable".into())
        })
    }

    async fn ensure_loopix_ready(&self, profile_id: Uuid) -> Result<u16> {
        if let Some(port) = self.listen_ports.get(profile_id) {
            return Ok(port);
        }
        self.start_loopix(profile_id).await?;
        self.listen_ports.get(profile_id).ok_or_else(|| {
            WireSentinelError::Other("loopix listen port unavailable".into())
        })
    }

    async fn ensure_federated_ready(&self, profile_id: Uuid) -> Result<u16> {
        if let Some(port) = self.listen_ports.get(profile_id) {
            return Ok(port);
        }
        let config = FederatedMixnetConfig {
            providers: self
                .federation
                .discover()
                .into_iter()
                .map(|p| p.provider_id())
                .collect(),
            profile_id,
            quorum: 1,
            enabled: true,
        };
        self.start_federated(config).await
    }

    pub fn provider_for_route(route: &TrafficRoute) -> Option<AnonymityProvider> {
        match route {
            TrafficRoute::Katzenpost(_) => Some(AnonymityProvider::Katzenpost),
            TrafficRoute::Loopix(_) => Some(AnonymityProvider::Loopix),
            TrafficRoute::FederatedMixnet(_) => Some(AnonymityProvider::FederatedMixnet),
            TrafficRoute::Anonymous(AnonymousRoute::Katzenpost(_)) => {
                Some(AnonymityProvider::Katzenpost)
            }
            TrafficRoute::Anonymous(AnonymousRoute::Loopix(_)) => Some(AnonymityProvider::Loopix),
            TrafficRoute::Anonymous(AnonymousRoute::FederatedMixnet { .. }) => {
                Some(AnonymityProvider::FederatedMixnet)
            }
            _ => None,
        }
    }

    pub fn active_routes(&self) -> Vec<anonymity_core::AnonymityRoute> {
        self.sessions
            .read()
            .values()
            .map(|s| s.route.clone())
            .collect()
    }

    pub fn federation_profiles(&self) -> Vec<AnonymityProfile> {
        self.federation.discover()
    }
}
