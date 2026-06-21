//! Plugin lifecycle service wrapping the WASM runtime.

use chrono::Utc;
use event_bus::EventBus;
use plugin_runtime::{plugins_data_dir, PluginManager, PluginSecurityPolicy};
use shared_types::{PluginCapability, PluginRecord, Result, ServiceEventInner, WireSentinelError};
use std::sync::Arc;
use storage::{PluginRepository, Storage};
use tracing::info;
use uuid::Uuid;

pub struct PluginService {
    storage: Arc<Storage>,
    events: EventBus,
    manager: parking_lot::Mutex<PluginManager>,
}

impl PluginService {
    pub fn new(storage: Arc<Storage>, events: EventBus) -> Result<Self> {
        let policy = PluginSecurityPolicy::default();
        let manager = PluginManager::new(plugins_data_dir(), policy)?;
        Ok(Self {
            storage,
            events,
            manager: parking_lot::Mutex::new(manager),
        })
    }

    pub async fn discover(&self) -> Result<Vec<PluginRecord>> {
        let discovered = {
            let mut manager = self.manager.lock();
            manager.discover()?
        };

        for record in &discovered {
            let is_new = self.storage.plugins.get(record.id).await?.is_none();
            self.storage.plugins.upsert(record).await?;
            if is_new {
                self.events.publish(
                    ServiceEventInner::PluginInstalled {
                        plugin: record.clone(),
                    }
                    .with_timestamp(Utc::now()),
                );
            }
        }

        info!(count = discovered.len(), "plugins discovered and persisted");
        Ok(discovered)
    }

    pub async fn list(&self) -> Result<Vec<PluginRecord>> {
        let persisted = self.storage.plugins.list().await?;
        if !persisted.is_empty() {
            self.manager.lock().seed_records(persisted.clone());
            return Ok(persisted);
        }
        Ok(self.manager.lock().list())
    }

    pub async fn load(&self, id: Uuid) -> Result<PluginRecord> {
        if self.storage.plugins.get(id).await?.is_none() {
            return Err(WireSentinelError::Other(format!("plugin not found: {id}")));
        }

        let load_result = {
            let mut manager = self.manager.lock();
            manager.load(id)
        };

        let loaded = match load_result {
            Ok(record) => record,
            Err(e) => {
                let failed = {
                    let mut manager = self.manager.lock();
                    manager.mark_failed(id, e.to_string())?
                };
                self.storage.plugins.upsert(&failed).await?;
                self.events.publish(
                    ServiceEventInner::PluginFailed {
                        plugin_id: id,
                        error: failed.error_message.clone().unwrap_or_default(),
                    }
                    .with_timestamp(Utc::now()),
                );
                return Err(e);
            }
        };

        self.storage.plugins.upsert(&loaded).await?;
        self.events.publish(
            ServiceEventInner::PluginLoaded {
                plugin_id: loaded.id,
                name: loaded.manifest.name.clone(),
            }
            .with_timestamp(Utc::now()),
        );
        Ok(loaded)
    }

    pub async fn unload(&self, id: Uuid) -> Result<PluginRecord> {
        let unloaded = {
            let mut manager = self.manager.lock();
            manager.unload(id)?
        };
        self.storage.plugins.upsert(&unloaded).await?;
        self.events.publish(
            ServiceEventInner::PluginUnloaded {
                plugin_id: unloaded.id,
                reason: "api unload".into(),
            }
            .with_timestamp(Utc::now()),
        );
        Ok(unloaded)
    }

    pub fn emit_security_violation(
        &self,
        plugin_id: Uuid,
        violation_type: impl Into<String>,
        detail: impl Into<String>,
    ) {
        self.events.publish(
            ServiceEventInner::PluginSecurityViolation {
                plugin_id,
                violation_type: violation_type.into(),
                detail: detail.into(),
            }
            .with_timestamp(Utc::now()),
        );
    }

    pub async fn list_mixnet_providers(&self) -> Result<Vec<PluginRecord>> {
        let all = self.list().await?;
        Ok(all
            .into_iter()
            .filter(|r| {
                r.manifest
                    .capabilities
                    .iter()
                    .any(|c| *c == PluginCapability::MixnetBackend)
            })
            .collect())
    }
}
