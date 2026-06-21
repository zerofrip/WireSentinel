//! Optional cloud configuration sync via backup bundle push/pull.

use event_bus::EventBus;
use serde::{Deserialize, Serialize};
use shared_types::{
    Result, ServiceEvent, ServiceEventInner, SyncMode, WireSentinelError,
};
use std::sync::Arc;
use std::time::Duration;
use storage::Storage;
use tokio::sync::watch;
use tracing::{info, warn};
use uuid::Uuid;

use crate::backup::BackupService;

const SETTINGS_KEY: &str = "cloud_sync";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CloudSyncConfig {
    #[serde(default)]
    pub enabled: bool,
    pub cloud_url: String,
    #[serde(default)]
    pub tenant_id: Option<Uuid>,
    #[serde(default)]
    pub device_id: Option<String>,
    #[serde(default)]
    pub sync_mode: SyncMode,
    #[serde(default = "default_sync_interval_secs")]
    pub interval_secs: u64,
    /// Bearer token for cloud API (admin or service account).
    #[serde(default)]
    pub api_token: Option<String>,
}

fn default_sync_interval_secs() -> u64 {
    300
}

impl Default for CloudSyncConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cloud_url: String::new(),
            tenant_id: None,
            device_id: None,
            sync_mode: SyncMode::default(),
            interval_secs: default_sync_interval_secs(),
            api_token: None,
        }
    }
}

pub struct CloudSyncAgent;

impl CloudSyncAgent {
    pub async fn load_config(storage: &Storage) -> Result<CloudSyncConfig> {
        match storage.settings.get(SETTINGS_KEY).await? {
            Some(json) => serde_json::from_str(&json).map_err(WireSentinelError::Serde),
            None => Ok(CloudSyncConfig::default()),
        }
    }

    pub async fn save_config(storage: &Storage, config: &CloudSyncConfig) -> Result<()> {
        let json = serde_json::to_string(config).map_err(WireSentinelError::Serde)?;
        storage.settings.set(SETTINGS_KEY, &json).await
    }

    pub fn spawn(
        storage: Arc<Storage>,
        backup: Arc<BackupService>,
        events: EventBus,
        mut config: CloudSyncConfig,
        mut shutdown: watch::Receiver<bool>,
    ) {
        if !config.enabled || config.cloud_url.is_empty() {
            return;
        }

        tokio::spawn(async move {
            let http = reqwest::Client::new();
            let base = config.cloud_url.trim_end_matches('/').to_string();

            let Some(tenant_id) = config.tenant_id else {
                warn!("cloud sync enabled but tenant_id is not configured");
                return;
            };

            if config.device_id.is_none() {
                let device_id = Uuid::new_v4().to_string();
                config.device_id = Some(device_id.clone());
                if let Err(e) = Self::save_config(&storage, &config).await {
                    warn!(err = %e, "failed to persist cloud sync device id");
                }
                info!(device_id, "cloud sync device id assigned");
            }

            let Some(device_id) = config.device_id.clone() else {
                return;
            };

            let controller_id = tenant_id;
            events.publish(ServiceEvent::now(ServiceEventInner::ControllerRegistered {
                controller_id,
                url: base.clone(),
            }));

            let mut sync_tick =
                tokio::time::interval(Duration::from_secs(config.interval_secs.max(60)));
            sync_tick.tick().await;

            loop {
                tokio::select! {
                    changed = shutdown.changed() => {
                        if changed.is_ok() && *shutdown.borrow() {
                            break;
                        }
                    }
                    _ = sync_tick.tick() => {
                        let mode = config.sync_mode;
                        let push = matches!(mode, SyncMode::Push | SyncMode::Bidirectional);
                        let pull = matches!(mode, SyncMode::Pull | SyncMode::Bidirectional);

                        if push {
                            match push_backup(
                                &http,
                                &base,
                                tenant_id,
                                &device_id,
                                &backup,
                                config.api_token.as_deref(),
                            )
                            .await
                            {
                                Ok(()) => {
                                    events.publish(ServiceEvent::now(
                                        ServiceEventInner::ControllerSynced {
                                            controller_id,
                                            sync_mode: mode,
                                        },
                                    ));
                                }
                                Err(e) => {
                                    warn!(err = %e, "cloud sync push failed");
                                    events.publish(ServiceEvent::now(
                                        ServiceEventInner::ControllerDisconnected {
                                            controller_id,
                                            reason: e.to_string(),
                                        },
                                    ));
                                }
                            }
                        }

                        if pull {
                            match pull_backup(
                                &http,
                                &base,
                                tenant_id,
                                &device_id,
                                &backup,
                                config.api_token.as_deref(),
                            )
                            .await
                            {
                                Ok(()) => {
                                    events.publish(ServiceEvent::now(
                                        ServiceEventInner::ControllerSynced {
                                            controller_id,
                                            sync_mode: mode,
                                        },
                                    ));
                                }
                                Err(e) => {
                                    warn!(err = %e, "cloud sync pull failed");
                                    events.publish(ServiceEvent::now(
                                        ServiceEventInner::ControllerDisconnected {
                                            controller_id,
                                            reason: e.to_string(),
                                        },
                                    ));
                                }
                            }
                        }
                    }
                }
            }

            events.publish(ServiceEvent::now(ServiceEventInner::ControllerDisconnected {
                controller_id,
                reason: "shutdown".into(),
            }));
        });
    }
}

fn apply_cloud_auth(
    builder: reqwest::RequestBuilder,
    tenant_id: Uuid,
    api_token: Option<&str>,
) -> reqwest::RequestBuilder {
    let builder = builder.header("X-Tenant-Id", tenant_id.to_string());
    if let Some(token) = api_token.filter(|t| !t.is_empty()) {
        builder.header("Authorization", format!("Bearer {token}"))
    } else {
        builder
    }
}

async fn push_backup(
    http: &reqwest::Client,
    base: &str,
    tenant_id: Uuid,
    device_id: &str,
    backup: &BackupService,
    api_token: Option<&str>,
) -> Result<()> {
    let (_, json) = backup.export_json().await?;
    let url = format!("{base}/api/v1/tenants/{tenant_id}/devices/{device_id}/sync/push");
    let req = apply_cloud_auth(http.post(&url), tenant_id, api_token)
        .header("content-type", "application/json")
        .body(json);
    let resp = req
        .send()
        .await
        .map_err(|e| WireSentinelError::Config(format!("cloud sync push: {e}")))?;

    if !resp.status().is_success() {
        return Err(WireSentinelError::Config(format!(
            "cloud sync push status {}",
            resp.status()
        )));
    }
    Ok(())
}

async fn pull_backup(
    http: &reqwest::Client,
    base: &str,
    tenant_id: Uuid,
    device_id: &str,
    backup: &BackupService,
    api_token: Option<&str>,
) -> Result<()> {
    let url = format!("{base}/api/v1/tenants/{tenant_id}/devices/{device_id}/sync/pull");
    let req = apply_cloud_auth(http.get(&url), tenant_id, api_token);
    let resp = req
        .send()
        .await
        .map_err(|e| WireSentinelError::Config(format!("cloud sync pull: {e}")))?;

    if !resp.status().is_success() {
        return Err(WireSentinelError::Config(format!(
            "cloud sync pull status {}",
            resp.status()
        )));
    }

    let json = resp
        .text()
        .await
        .map_err(|e| WireSentinelError::Config(format!("cloud sync pull body: {e}")))?;

    if json.trim().is_empty() {
        return Ok(());
    }

    backup.import_json(&json).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_sync_config_defaults() {
        let cfg = CloudSyncConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.sync_mode, SyncMode::Pull);
        assert_eq!(cfg.interval_secs, 300);
    }
}
