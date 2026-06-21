use crate::backend::VpnBackend;
use crate::factory::VpnBackendFactory;
use async_trait::async_trait;
use chrono::Utc;
use event_bus::EventBus;
use parking_lot::RwLock;
use shared_types::{Result, TunnelIface, VPNProfile, VpnState, VpnStatus};
use shared_types::{ServiceEventInner, WireSentinelError};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Manages VPN profiles and active connections.
pub struct VpnManager {
    factory: Arc<VpnBackendFactory>,
    profiles: RwLock<Vec<VPNProfile>>,
    active_backend: RwLock<HashMap<Uuid, Arc<dyn VpnBackend>>>,
    events: Option<EventBus>,
}

impl VpnManager {
    pub fn new(factory: Arc<VpnBackendFactory>) -> Self {
        Self {
            factory,
            profiles: RwLock::new(Vec::new()),
            active_backend: RwLock::new(HashMap::new()),
            events: None,
        }
    }

    pub fn with_events(mut self, events: EventBus) -> Self {
        self.events = Some(events);
        self
    }

    pub fn set_profiles(&self, profiles: Vec<VPNProfile>) {
        *self.profiles.write() = profiles;
    }

    pub fn profiles(&self) -> Vec<VPNProfile> {
        self.profiles.read().clone()
    }

    pub fn add_profile(&self, profile: VPNProfile) {
        self.profiles.write().push(profile);
    }

    pub fn remove_profile(&self, id: Uuid) -> bool {
        let before = self.profiles.read().len();
        self.profiles.write().retain(|p| p.id != id);
        self.profiles.read().len() < before
    }

    pub fn get_profile(&self, id: Uuid) -> Option<VPNProfile> {
        self.profiles.read().iter().find(|p| p.id == id).cloned()
    }

    fn backend_for(&self, profile: &VPNProfile) -> Arc<dyn VpnBackend> {
        self.factory.for_profile(profile)
    }

    pub async fn connect_with_config(
        &self,
        profile_id: Uuid,
        config_path: std::path::PathBuf,
    ) -> Result<()> {
        let mut profile = self
            .get_profile(profile_id)
            .ok_or_else(|| WireSentinelError::Vpn("profile not found".into()))?;
        profile.config_path = config_path;
        let backend = self.backend_for(&profile);
        if let Err(e) = backend.connect(&profile).await {
            if let Some(events) = &self.events {
                events.publish(
                    ServiceEventInner::VpnError {
                        profile_id,
                        message: e.to_string(),
                    }
                    .with_timestamp(Utc::now()),
                );
            }
            return Err(e);
        }
        self.active_backend.write().insert(profile_id, backend);
        if let Some(events) = &self.events {
            events.publish(
                ServiceEventInner::VpnConnected {
                    profile_id,
                    profile_name: profile.name.clone(),
                }
                .with_timestamp(Utc::now()),
            );
        }
        Ok(())
    }

    pub async fn connect(&self, profile_id: Uuid) -> Result<()> {
        let profile = self
            .get_profile(profile_id)
            .ok_or_else(|| WireSentinelError::Vpn("profile not found".into()))?;
        self.connect_with_config(profile_id, profile.config_path.clone())
            .await
    }

    pub async fn disconnect(&self, profile_id: Uuid) -> Result<()> {
        let profile_name = self
            .get_profile(profile_id)
            .map(|p| p.name)
            .unwrap_or_else(|| "unknown".into());
        let backend = self.active_backend.write().remove(&profile_id);
        if let Some(backend) = backend {
            backend.disconnect(profile_id).await?;
        }
        if let Some(events) = &self.events {
            events.publish(
                ServiceEventInner::VpnDisconnected {
                    profile_id,
                    reason: "user requested".into(),
                }
                .with_timestamp(Utc::now()),
            );
        }
        let _ = profile_name;
        Ok(())
    }

    pub async fn state(&self, profile_id: Uuid) -> Option<VpnState> {
        let profile = self.get_profile(profile_id)?;
        let backend = self
            .active_backend
            .read()
            .get(&profile_id)
            .cloned()
            .unwrap_or_else(|| self.backend_for(&profile));
        let status = backend.status(profile_id).await;
        let stats = backend.stats(profile_id).await;
        Some(VpnState {
            profile_id,
            profile_name: profile.name,
            status,
            stats,
        })
    }

    pub async fn all_states(&self) -> Vec<VpnState> {
        let profile_ids: Vec<Uuid> = self.profiles.read().iter().map(|p| p.id).collect();
        let mut states = Vec::new();
        for id in profile_ids {
            if let Some(state) = self.state(id).await {
                states.push(state);
            }
        }
        states
    }

    pub async fn active_profile(&self) -> Option<Uuid> {
        let entries: Vec<(Uuid, Arc<dyn VpnBackend>)> = self
            .active_backend
            .read()
            .iter()
            .map(|(id, backend)| (*id, Arc::clone(backend)))
            .collect();
        for (id, backend) in entries {
            if backend.status(id).await == VpnStatus::Connected {
                return Some(id);
            }
        }
        None
    }

    pub fn active_count(&self) -> u32 {
        self.active_backend.read().len() as u32
    }

    pub async fn any_connected(&self) -> bool {
        self.active_profile().await.is_some()
    }

    pub async fn tunnel_iface(&self, profile_id: Uuid) -> Option<TunnelIface> {
        let backend = self.active_backend.read().get(&profile_id).cloned()?;
        backend.tunnel_iface(profile_id).await
    }
}
