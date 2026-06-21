//! Optional cloud backup / log batch reporter.

use chrono::Utc;
use event_bus::EventBus;
use serde::{Deserialize, Serialize};
use shared_types::{CloudLogBatch, CloudLogEntry, Result, WireSentinelError};
use std::sync::Arc;
use std::time::Duration;
use storage::Storage;
use tokio::sync::watch;
use tracing::{info, warn};
use uuid::Uuid;

use crate::backup::BackupService;

const SETTINGS_KEY: &str = "cloud_backup";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CloudBackupReporterConfig {
    #[serde(default)]
    pub enabled: bool,
    pub cloud_url: String,
    #[serde(default)]
    pub tenant_id: Option<Uuid>,
    #[serde(default)]
    pub device_id: Option<String>,
    #[serde(default = "default_interval_secs")]
    pub interval_secs: u64,
    #[serde(default)]
    pub api_token: Option<String>,
}

fn default_interval_secs() -> u64 {
    600
}

impl Default for CloudBackupReporterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cloud_url: String::new(),
            tenant_id: None,
            device_id: None,
            interval_secs: default_interval_secs(),
            api_token: None,
        }
    }
}

pub struct CloudBackupReporter;

impl CloudBackupReporter {
    pub async fn load_config(storage: &Storage) -> Result<CloudBackupReporterConfig> {
        match storage.settings.get(SETTINGS_KEY).await? {
            Some(json) => serde_json::from_str(&json).map_err(WireSentinelError::Serde),
            None => Ok(CloudBackupReporterConfig::default()),
        }
    }

    pub async fn save_config(storage: &Storage, config: &CloudBackupReporterConfig) -> Result<()> {
        let json = serde_json::to_string(config).map_err(WireSentinelError::Serde)?;
        storage.settings.set(SETTINGS_KEY, &json).await
    }

    pub fn spawn(
        storage: Arc<Storage>,
        backup: Arc<BackupService>,
        _events: EventBus,
        config: CloudBackupReporterConfig,
        mut shutdown: watch::Receiver<bool>,
    ) {
        if !config.enabled || config.cloud_url.is_empty() {
            return;
        }

        let Some(tenant_id) = config.tenant_id else {
            warn!("cloud backup reporter enabled but tenant_id is not configured");
            return;
        };

        tokio::spawn(async move {
            let http = reqwest::Client::new();
            let base = config.cloud_url.trim_end_matches('/').to_string();
            let device_id = config
                .device_id
                .clone()
                .unwrap_or_else(|| Uuid::new_v4().to_string());
            let mut tick =
                tokio::time::interval(Duration::from_secs(config.interval_secs.max(120)));
            tick.tick().await;

            loop {
                tokio::select! {
                    changed = shutdown.changed() => {
                        if changed.is_ok() && *shutdown.borrow() {
                            break;
                        }
                    }
                    _ = tick.tick() => {
                        let bundle_meta = match backup.export_json().await {
                            Ok((bundle, _)) => serde_json::json!({
                                "version": bundle.version,
                                "exported_at": bundle.exported_at,
                            }),
                            Err(e) => {
                                warn!(err = %e, "cloud backup reporter metadata export failed");
                                continue;
                            }
                        };
                        let batch = CloudLogBatch {
                            tenant_id: tenant_id.to_string(),
                            controller_id: None,
                            device_id: Some(device_id.clone()),
                            entries: vec![CloudLogEntry {
                                level: "info".into(),
                                message: "backup snapshot".into(),
                                source: Some("cloud_backup_reporter".into()),
                                fields: Some(bundle_meta),
                            }],
                            ingested_at: Utc::now(),
                        };
                        if let Err(e) = push_logs(
                            &http,
                            &base,
                            &batch,
                            config.api_token.as_deref(),
                        ).await {
                            warn!(err = %e, "cloud backup log push failed");
                        } else {
                            info!("cloud backup reporter pushed log batch");
                        }
                        let _ = storage.settings.get(SETTINGS_KEY).await;
                    }
                }
            }
        });
    }
}

async fn push_logs(
    http: &reqwest::Client,
    base: &str,
    batch: &CloudLogBatch,
    token: Option<&str>,
) -> Result<()> {
    let url = format!("{base}/api/v1/cloud/logs");
    let mut req = http.post(url).json(batch);
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| WireSentinelError::Config(format!("cloud backup push: {e}")))?;
    if !resp.status().is_success() {
        return Err(WireSentinelError::Config(format!(
            "cloud backup push status {}",
            resp.status()
        )));
    }
    Ok(())
}
