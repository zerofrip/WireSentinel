use crate::discovery::exe_path_for_pid;
use crate::identity::{file_sha256, icon_path_for_exe, publisher_for_exe};
use chrono::Utc;
use event_bus::EventBus;
use shared_types::{AppIdentity, AppRecord, ServiceEventInner, WireSentinelError};
use std::sync::Arc;
use storage::{AppFilter, AppRepository};
use tracing::debug;
use uuid::Uuid;

pub struct AppRegistryService {
    apps: Arc<dyn AppRepository>,
    events: EventBus,
}

impl AppRegistryService {
    pub fn new(apps: Arc<dyn AppRepository>, events: EventBus) -> Self {
        Self { apps, events }
    }

    pub async fn resolve_or_register(
        &self,
        pid: u32,
    ) -> Result<(AppIdentity, bool), WireSentinelError> {
        let exe_path = exe_path_for_pid(pid)?;
        let sha256 = file_sha256(&exe_path).ok();

        let existing = if let Some(ref hash) = sha256 {
            self.apps.find_by_sha256(hash).await?
        } else {
            None
        }
        .or(self.apps.find_by_exe_path(&exe_path).await?);

        if let Some(mut record) = existing {
            let updated = record.sha256.is_none() && sha256.is_some();
            if updated {
                record.sha256 = sha256.clone();
                record.publisher = publisher_for_exe(&exe_path);
                record.icon_path = icon_path_for_exe(&exe_path);
            }
            record.touch();
            self.apps.upsert(&record).await?;
            if updated {
                self.events.publish(
                    ServiceEventInner::AppUpdated {
                        app: record.clone(),
                    }
                    .with_timestamp(Utc::now()),
                );
            }
            return Ok((AppIdentity::new(pid, record), false));
        }

        let mut record = AppRecord::new(exe_path.clone());
        record.sha256 = sha256;
        record.publisher = publisher_for_exe(&exe_path);
        record.icon_path = icon_path_for_exe(&exe_path);
        self.apps.upsert(&record).await?;
        debug!(app = %record.display_name, pid, "app discovered");
        self.events.publish(
            ServiceEventInner::AppDiscovered {
                app: record.clone(),
            }
            .with_timestamp(Utc::now()),
        );
        Ok((AppIdentity::new(pid, record), true))
    }

    pub async fn list(
        &self,
        search: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<AppRecord>, WireSentinelError> {
        self.apps.list(AppFilter { search, limit }).await
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<AppRecord>, WireSentinelError> {
        self.apps.find_by_id(id).await
    }

    pub async fn set_default_route(
        &self,
        app_id: Uuid,
        route: Option<shared_types::TrafficRoute>,
    ) -> Result<Option<AppRecord>, WireSentinelError> {
        let exit_config = route.map(shared_types::AppExitConfig::from_single);
        self.set_exit_config(app_id, exit_config).await
    }

    pub async fn set_exit_config(
        &self,
        app_id: Uuid,
        exit_config: Option<shared_types::AppExitConfig>,
    ) -> Result<Option<AppRecord>, WireSentinelError> {
        let mut record = self
            .apps
            .find_by_id(app_id)
            .await?
            .ok_or_else(|| WireSentinelError::Config("app not found".into()))?;
        record.exit_config = exit_config;
        record.sync_legacy_default_route();
        record.touch();
        self.apps.upsert(&record).await?;
        self.events.publish(
            ServiceEventInner::AppUpdated {
                app: record.clone(),
            }
            .with_timestamp(Utc::now()),
        );
        Ok(Some(record))
    }
}
