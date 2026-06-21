//! Optional cloud usage metering reporter.

use chrono::Utc;
use event_bus::EventBus;
use serde::{Deserialize, Serialize};
use shared_types::{CloudUsagePayload, Result, WireSentinelError};
use std::sync::Arc;
use std::time::Duration;
use storage::Storage;
use tokio::sync::watch;
use tracing::{info, warn};
use uuid::Uuid;

use crate::metrics::MetricsService;

const SETTINGS_KEY: &str = "cloud_usage";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CloudUsageReporterConfig {
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
    300
}

impl Default for CloudUsageReporterConfig {
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

pub struct CloudUsageReporter;

impl CloudUsageReporter {
    pub async fn load_config(storage: &Storage) -> Result<CloudUsageReporterConfig> {
        match storage.settings.get(SETTINGS_KEY).await? {
            Some(json) => serde_json::from_str(&json).map_err(WireSentinelError::Serde),
            None => Ok(CloudUsageReporterConfig::default()),
        }
    }

    pub async fn save_config(storage: &Storage, config: &CloudUsageReporterConfig) -> Result<()> {
        let json = serde_json::to_string(config).map_err(WireSentinelError::Serde)?;
        storage.settings.set(SETTINGS_KEY, &json).await
    }

    pub fn spawn(
        storage: Arc<Storage>,
        metrics: Arc<MetricsService>,
        _events: EventBus,
        config: CloudUsageReporterConfig,
        mut shutdown: watch::Receiver<bool>,
    ) {
        if !config.enabled || config.cloud_url.is_empty() {
            return;
        }

        let Some(tenant_id) = config.tenant_id else {
            warn!("cloud usage reporter enabled but tenant_id is not configured");
            return;
        };

        tokio::spawn(async move {
            let http = reqwest::Client::new();
            let base = config.cloud_url.trim_end_matches('/').to_string();
            let device_id = config.device_id.clone();
            let mut tick = tokio::time::interval(Duration::from_secs(config.interval_secs.max(60)));
            tick.tick().await;

            loop {
                tokio::select! {
                    changed = shutdown.changed() => {
                        if changed.is_ok() && *shutdown.borrow() {
                            break;
                        }
                    }
                    _ = tick.tick() => {
                        let window_end = Utc::now();
                        let window_start = window_end - chrono::Duration::seconds(config.interval_secs as i64);
                        if let Ok(snapshot) = metrics.snapshot().await {
                            let payloads = vec![
                                ("bandwidth_bytes", snapshot.blocked_requests as f64 + snapshot.dns_queries as f64),
                                ("active_devices", 1.0),
                                ("dns_queries", snapshot.dns_queries as f64),
                            ];
                            for (metric, quantity) in payloads {
                                let payload = CloudUsagePayload {
                                    tenant_id: tenant_id.to_string(),
                                    controller_id: None,
                                    device_id: device_id.clone(),
                                    metric: metric.into(),
                                    quantity,
                                    window_start,
                                    window_end,
                                    metadata: None,
                                };
                                if let Err(e) = push_usage(
                                    &http,
                                    &base,
                                    &payload,
                                    config.api_token.as_deref(),
                                ).await {
                                    warn!(err = %e, metric, "cloud usage push failed");
                                }
                            }
                            info!("cloud usage reporter pushed metrics window");
                        }
                        let _ = Self::save_config(&storage, &config).await;
                    }
                }
            }
        });
    }
}

async fn push_usage(
    http: &reqwest::Client,
    base: &str,
    payload: &CloudUsagePayload,
    token: Option<&str>,
) -> Result<()> {
    let url = format!("{base}/api/v1/cloud/usage");
    let mut req = http.post(url).json(payload);
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| WireSentinelError::Config(format!("cloud usage push: {e}")))?;
    if !resp.status().is_success() {
        return Err(WireSentinelError::Config(format!(
            "cloud usage push status {}",
            resp.status()
        )));
    }
    Ok(())
}
