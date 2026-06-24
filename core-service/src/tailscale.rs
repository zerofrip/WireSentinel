//! Tailscale / tailnet profile service.

use chrono::Utc;
use event_bus::EventBus;
use shared_types::{
    Result, ServiceEvent, ServiceEventInner, TailnetProfile, TailscaleStatus, WireSentinelError,
};
use std::sync::Arc;
use storage::Storage;
use uuid::Uuid;
use vpn_engine::{TailscaleBackend, TailscaleRuntimeStatus, VpnBackendFactory};

pub struct TailscaleService {
    storage: Arc<Storage>,
    events: EventBus,
    backend: Arc<TailscaleBackend>,
}

impl TailscaleService {
    pub fn new(storage: Arc<Storage>, events: EventBus, factory: &VpnBackendFactory) -> Self {
        Self {
            storage,
            events,
            backend: factory.tailscale_backend(),
        }
    }

    pub async fn status(&self) -> Result<TailscaleStatus> {
        let profiles = self.storage.tailnet_profiles.list().await?;
        let runtime = self.backend.query_status().await;
        Ok(TailscaleStatus {
            connected: runtime.connected || profiles.iter().any(|p| p.connected),
            hostname: runtime
                .hostname
                .or_else(|| profiles.iter().find_map(|p| p.hostname.clone())),
            tailnet_ip: runtime
                .tailnet_ip
                .or_else(|| profiles.iter().find_map(|p| p.tailnet_ip.clone())),
            exit_node: runtime
                .exit_node
                .or_else(|| profiles.iter().find_map(|p| p.exit_node.clone())),
            magic_dns: profiles.first().map(|p| p.magic_dns).unwrap_or(true),
            profiles,
        })
    }

    pub async fn join(&self, profile_id: Uuid) -> Result<TailnetProfile> {
        let mut profile = self
            .storage
            .tailnet_profiles
            .get(profile_id)
            .await?
            .ok_or_else(|| {
                WireSentinelError::Config(format!("tailnet profile {profile_id} not found"))
            })?;

        self.backend.connect_tailnet(&profile).await?;

        let runtime = self.backend.query_status().await;
        profile.connected = runtime.connected;
        if runtime.hostname.is_some() {
            profile.hostname = runtime.hostname;
        }
        if runtime.tailnet_ip.is_some() {
            profile.tailnet_ip = runtime.tailnet_ip;
        }
        profile.updated_at = Utc::now();
        self.storage.tailnet_profiles.update(&profile).await?;

        self.events
            .publish(ServiceEvent::now(ServiceEventInner::TailnetJoined {
                profile_id,
                hostname: profile.hostname.clone(),
            }));

        Ok(profile)
    }

    pub async fn leave(&self, profile_id: Uuid, reason: &str) -> Result<()> {
        self.backend.disconnect_tailnet(profile_id).await?;

        if let Some(mut profile) = self.storage.tailnet_profiles.get(profile_id).await? {
            profile.connected = false;
            profile.updated_at = Utc::now();
            self.storage.tailnet_profiles.update(&profile).await?;
        }

        self.events
            .publish(ServiceEvent::now(ServiceEventInner::TailnetLeft {
                profile_id,
                reason: reason.to_string(),
            }));
        Ok(())
    }

    pub async fn set_exit_node(&self, profile_id: Uuid, exit_node: Option<String>) -> Result<()> {
        self.backend.set_exit_node(exit_node.as_deref()).await?;

        if let Some(mut profile) = self.storage.tailnet_profiles.get(profile_id).await? {
            profile.exit_node = exit_node.clone();
            profile.updated_at = Utc::now();
            self.storage.tailnet_profiles.update(&profile).await?;
        }

        self.events
            .publish(ServiceEvent::now(ServiceEventInner::ExitNodeChanged {
                profile_id,
                exit_node,
            }));
        Ok(())
    }

    pub fn runtime_status(
        &self,
    ) -> impl std::future::Future<Output = TailscaleRuntimeStatus> + Send {
        let backend = Arc::clone(&self.backend);
        async move { backend.query_status().await }
    }
}
