//! Cover traffic generation backed by mixnet-cover-traffic.

use chrono::Utc;
use event_bus::EventBus;
use mixnet_cover_traffic::CoverTrafficEngine;
use shared_types::{
    CoverTrafficProfile, CoverTrafficSettings, Result, ServiceEvent, ServiceEventInner,
    WireSentinelError,
};
use std::sync::Arc;
use storage::Storage;
use uuid::Uuid;
use anonymity_cover_traffic::{AdaptiveCoverProfile, AdaptiveCoverTrafficEngine};

fn to_engine_profile(profile: CoverTrafficProfile) -> mixnet_cover_traffic::CoverTrafficProfile {
    match profile {
        CoverTrafficProfile::Disabled => mixnet_cover_traffic::CoverTrafficProfile::Disabled,
        CoverTrafficProfile::Low => mixnet_cover_traffic::CoverTrafficProfile::Low,
        CoverTrafficProfile::Medium => mixnet_cover_traffic::CoverTrafficProfile::Medium,
        CoverTrafficProfile::High => mixnet_cover_traffic::CoverTrafficProfile::High,
        CoverTrafficProfile::Maximum => mixnet_cover_traffic::CoverTrafficProfile::Maximum,
    }
}

pub struct CoverTrafficService {
    storage: Arc<Storage>,
    events: EventBus,
    engine: Arc<CoverTrafficEngine>,
    adaptive_engine: Arc<AdaptiveCoverTrafficEngine>,
    adaptive_enabled: parking_lot::RwLock<bool>,
    active_profile_id: parking_lot::RwLock<Option<Uuid>>,
}

impl CoverTrafficService {
    pub fn new(storage: Arc<Storage>, events: EventBus) -> Self {
        Self {
            storage,
            events,
            engine: CoverTrafficEngine::new(mixnet_cover_traffic::CoverTrafficProfile::Disabled),
            adaptive_engine: AdaptiveCoverTrafficEngine::new(AdaptiveCoverProfile::Balanced),
            adaptive_enabled: parking_lot::RwLock::new(false),
            active_profile_id: parking_lot::RwLock::new(None),
        }
    }

    pub fn engine(&self) -> Arc<CoverTrafficEngine> {
        Arc::clone(&self.engine)
    }

    pub fn is_running(&self) -> bool {
        self.engine.is_running() || self.adaptive_engine.is_running()
    }

    pub fn is_adaptive(&self) -> bool {
        *self.adaptive_enabled.read()
    }

    pub fn set_adaptive_mode(&self, enabled: bool, events: &EventBus) {
        *self.adaptive_enabled.write() = enabled;
        events.publish(ServiceEvent::now(ServiceEventInner::AdaptiveCoverUpdated {
            adaptive: enabled,
        }));
    }

    pub fn toggle_adaptive_mode(&self, events: &EventBus) -> bool {
        let enabled = !self.is_adaptive();
        self.set_adaptive_mode(enabled, events);
        enabled
    }

    async fn sync_adaptive_engine(&self) -> Result<()> {
        if self.is_adaptive() && self.engine.is_running() {
            self.adaptive_engine.start().await.map_err(|e| {
                WireSentinelError::Other(format!("adaptive cover start failed: {e}"))
            })?;
        } else {
            self.adaptive_engine.stop().await.map_err(|e| {
                WireSentinelError::Other(format!("adaptive cover stop failed: {e}"))
            })?;
        }
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<CoverTrafficSettings>> {
        self.storage.cover_traffic.list().await
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<CoverTrafficSettings>> {
        self.storage.cover_traffic.get(id).await
    }

    pub async fn get_for_mixnet(&self, mixnet_profile_id: Uuid) -> Result<Option<CoverTrafficSettings>> {
        self.storage
            .cover_traffic
            .get_by_mixnet_profile(mixnet_profile_id)
            .await
    }

    pub async fn upsert(&self, mut settings: CoverTrafficSettings) -> Result<CoverTrafficSettings> {
        let now = Utc::now();
        if settings.id.is_nil() {
            settings.id = Uuid::new_v4();
            settings.created_at = now;
        }
        settings.updated_at = now;

        if self.storage.cover_traffic.get(settings.id).await?.is_some() {
            self.storage.cover_traffic.update(&settings).await?;
        } else {
            self.storage.cover_traffic.insert(&settings).await?;
        }
        Ok(settings)
    }

    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let _ = self.stop(id, "settings deleted").await;
        self.storage.cover_traffic.delete(id).await
    }

    pub async fn start(&self, settings_id: Uuid) -> Result<CoverTrafficSettings> {
        let settings = self
            .storage
            .cover_traffic
            .get(settings_id)
            .await?
            .ok_or_else(|| {
                WireSentinelError::Other(format!("cover traffic settings {settings_id} not found"))
            })?;

        self.engine
            .set_profile(to_engine_profile(settings.profile));
        self.engine.start().await.map_err(|e| {
            WireSentinelError::Other(format!("cover traffic start failed: {e}"))
        })?;
        self.sync_adaptive_engine().await?;

        let profile_id = settings
            .mixnet_profile_id
            .unwrap_or(settings_id);
        *self.active_profile_id.write() = Some(profile_id);

        self.events
            .publish(ServiceEvent::now(ServiceEventInner::CoverTrafficStarted { profile_id }));
        Ok(settings)
    }

    pub async fn stop(&self, settings_id: Uuid, reason: &str) -> Result<()> {
        self.engine.stop().await.map_err(|e| {
            WireSentinelError::Other(format!("cover traffic stop failed: {e}"))
        })?;
        self.adaptive_engine.stop().await.map_err(|e| {
            WireSentinelError::Other(format!("adaptive cover stop failed: {e}"))
        })?;

        let profile_id = self
            .active_profile_id
            .read()
            .unwrap_or(settings_id);
        *self.active_profile_id.write() = None;

        self.events.publish(ServiceEvent::now(ServiceEventInner::CoverTrafficStopped {
            profile_id,
            reason: reason.to_string(),
        }));
        Ok(())
    }

    pub async fn start_if_enabled(&self) -> Result<()> {
        let settings = self.storage.cover_traffic.list().await?;
        for entry in settings {
            if entry.enabled && entry.profile != CoverTrafficProfile::Disabled {
                let _ = self.start(entry.id).await?;
                break;
            }
        }
        Ok(())
    }
}
