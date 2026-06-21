//! Optional cloud fleet telemetry (health) reporter.

use chrono::Utc;
use event_bus::EventBus;
use serde::{Deserialize, Serialize};
use shared_types::{CloudHealthPayload, Result, WireSentinelError};
use std::sync::Arc;
use std::time::Duration;
use storage::Storage;
use tokio::sync::watch;
use tracing::{info, warn};
use uuid::Uuid;

use crate::metrics::MetricsService;

const SETTINGS_KEY: &str = "cloud_telemetry";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CloudTelemetryReporterConfig {
    #[serde(default)]
    pub enabled: bool,
    pub cloud_url: String,
    #[serde(default)]
    pub tenant_id: Option<Uuid>,
    #[serde(default)]
    pub controller_id: Option<String>,
    #[serde(default = "default_interval_secs")]
    pub interval_secs: u64,
    #[serde(default)]
    pub api_token: Option<String>,
}

fn default_interval_secs() -> u64 {
    120
}

impl Default for CloudTelemetryReporterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cloud_url: String::new(),
            tenant_id: None,
            controller_id: None,
            interval_secs: default_interval_secs(),
            api_token: None,
        }
    }
}

pub struct CloudTelemetryReporter;

impl CloudTelemetryReporter {
    pub async fn load_config(storage: &Storage) -> Result<CloudTelemetryReporterConfig> {
        match storage.settings.get(SETTINGS_KEY).await? {
            Some(json) => serde_json::from_str(&json).map_err(WireSentinelError::Serde),
            None => Ok(CloudTelemetryReporterConfig::default()),
        }
    }

    pub async fn save_config(storage: &Storage, config: &CloudTelemetryReporterConfig) -> Result<()> {
        let json = serde_json::to_string(config).map_err(WireSentinelError::Serde)?;
        storage.settings.set(SETTINGS_KEY, &json).await
    }

    pub fn spawn(
        _storage: Arc<Storage>,
        metrics: Arc<MetricsService>,
        _events: EventBus,
        config: CloudTelemetryReporterConfig,
        mut shutdown: watch::Receiver<bool>,
    ) {
        if !config.enabled || config.cloud_url.is_empty() {
            return;
        }

        let Some(tenant_id) = config.tenant_id else {
            warn!("cloud telemetry reporter enabled but tenant_id is not configured");
            return;
        };

        tokio::spawn(async move {
            let http = reqwest::Client::new();
            let base = config.cloud_url.trim_end_matches('/').to_string();
            let mut tick =
                tokio::time::interval(Duration::from_secs(config.interval_secs.max(30)));
            tick.tick().await;

            loop {
                tokio::select! {
                    changed = shutdown.changed() => {
                        if changed.is_ok() && *shutdown.borrow() {
                            break;
                        }
                    }
                    _ = tick.tick() => {
                        let healthy = match metrics.snapshot().await {
                            Ok(s) => s.open_leak_incidents == 0,
                            Err(_) => false,
                        };
                        let reporting = if metrics.snapshot().await.is_ok() { 1 } else { 0 };
                        let payload = CloudHealthPayload {
                            tenant_id: tenant_id.to_string(),
                            controller_id: config.controller_id.clone(),
                            healthy,
                            reporting_devices: reporting,
                            healthy_devices: if healthy { reporting } else { 0 },
                            kernel_devices: 0,
                            anonymity_score: None,
                            notes: None,
                            captured_at: Utc::now(),
                        };
                        if let Err(e) = push_health(
                            &http,
                            &base,
                            &payload,
                            config.api_token.as_deref(),
                        ).await {
                            warn!(err = %e, "cloud telemetry push failed");
                        } else {
                            info!(healthy, "cloud telemetry reporter pushed health");
                        }
                    }
                }
            }
        });
    }
}

async fn push_health(
    http: &reqwest::Client,
    base: &str,
    payload: &CloudHealthPayload,
    token: Option<&str>,
) -> Result<()> {
    let url = format!("{base}/api/v1/cloud/health");
    let mut req = http.post(url).json(payload);
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| WireSentinelError::Config(format!("cloud telemetry push: {e}")))?;
    if !resp.status().is_success() {
        return Err(WireSentinelError::Config(format!(
            "cloud telemetry push status {}",
            resp.status()
        )));
    }
    Ok(())
}
