//! Optional controller enrollment, heartbeat, policy pull, and telemetry push.

use chrono::Utc;
use event_bus::EventBus;
use serde::{Deserialize, Serialize};
use shared_types::{Result, ServiceEvent, ServiceEventInner, WireSentinelError};
use std::sync::Arc;
use std::time::Duration;
use storage::Storage;
use tokio::sync::watch;
use tracing::{info, warn};
use uuid::Uuid;

use crate::audit::AuditRecorder;
use crate::enterprise::{LocalPolicyProvider, RemotePolicyProvider};
use crate::metrics::MetricsService;
use crate::ztna_agent::ZtnaAgent;
use crate::sse_agent::SseAgent;

const SETTINGS_KEY: &str = "controller_agent";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ControllerAgentConfig {
    #[serde(default)]
    pub enabled: bool,
    pub controller_url: String,
    #[serde(default)]
    pub enrollment_token: Option<String>,
    #[serde(default)]
    pub device_id: Option<String>,
    #[serde(default = "default_device_name")]
    pub device_name: String,
    #[serde(default = "default_heartbeat_secs")]
    pub heartbeat_interval_secs: u64,
    #[serde(default = "default_policy_pull_secs")]
    pub policy_pull_interval_secs: u64,
    #[serde(default)]
    pub push_kernel_heartbeat: bool,
    #[serde(default)]
    pub push_mixnet_heartbeat: bool,
    #[serde(default)]
    pub push_anonymity_heartbeat: bool,
    #[serde(default)]
    pub push_ztna_heartbeat: bool,
    #[serde(default)]
    pub push_sse_telemetry: bool,
}

fn default_device_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "wiresentinel-core".into())
}

fn default_heartbeat_secs() -> u64 {
    60
}

fn default_policy_pull_secs() -> u64 {
    300
}

impl Default for ControllerAgentConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            controller_url: String::new(),
            enrollment_token: None,
            device_id: None,
            device_name: default_device_name(),
            heartbeat_interval_secs: default_heartbeat_secs(),
            policy_pull_interval_secs: default_policy_pull_secs(),
            push_kernel_heartbeat: false,
            push_mixnet_heartbeat: false,
            push_anonymity_heartbeat: false,
            push_ztna_heartbeat: false,
            push_sse_telemetry: false,
        }
    }
}

pub struct ControllerAgent;

impl ControllerAgent {
    pub async fn load_config(storage: &Storage) -> Result<ControllerAgentConfig> {
        match storage.settings.get(SETTINGS_KEY).await? {
            Some(json) => serde_json::from_str(&json).map_err(WireSentinelError::Serde),
            None => Ok(ControllerAgentConfig::default()),
        }
    }

    pub async fn save_config(storage: &Storage, config: &ControllerAgentConfig) -> Result<()> {
        let json = serde_json::to_string(config).map_err(WireSentinelError::Serde)?;
        storage.settings.set(SETTINGS_KEY, &json).await
    }

    pub fn spawn(
        storage: Arc<Storage>,
        events: EventBus,
        enterprise: Arc<LocalPolicyProvider>,
        metrics: Arc<MetricsService>,
        audit: Arc<AuditRecorder>,
        ztna: Option<Arc<ZtnaAgent>>,
        sse: Option<Arc<SseAgent>>,
        mut config: ControllerAgentConfig,
        mut shutdown: watch::Receiver<bool>,
    ) {
        if !config.enabled || config.controller_url.is_empty() {
            return;
        }

        tokio::spawn(async move {
            let http = reqwest::Client::new();
            let base = config.controller_url.trim_end_matches('/').to_string();

            if config.device_id.is_none() {
                if let Some(token) = config.enrollment_token.clone() {
                    match register_device(&http, &base, &token, &config.device_name).await {
                        Ok(device_id) => {
                            config.device_id = Some(device_id.clone());
                            if let Err(e) = Self::save_config(&storage, &config).await {
                                warn!(error = %e, "failed to persist controller device id");
                            }
                            if let Ok(agent_id) = Uuid::parse_str(&device_id) {
                                events.publish(ServiceEvent::now(ServiceEventInner::AgentEnrolled {
                                    agent_id,
                                    name: config.device_name.clone(),
                                }));
                            }
                            info!(device_id, "controller agent enrolled");
                        }
                        Err(e) => warn!(error = %e, "controller enrollment failed"),
                    }
                } else {
                    warn!("controller agent enabled but no enrollment token or device id");
                    return;
                }
            }

            let Some(device_id) = config.device_id.clone() else {
                return;
            };

            let remote = RemotePolicyProvider::new(
                base.clone(),
                device_id.clone(),
                http.clone(),
                Arc::clone(&enterprise),
            );

            let mut heartbeat_tick =
                tokio::time::interval(Duration::from_secs(config.heartbeat_interval_secs.max(15)));
            let mut policy_tick =
                tokio::time::interval(Duration::from_secs(config.policy_pull_interval_secs.max(60)));
            policy_tick.tick().await;

            loop {
                tokio::select! {
                    changed = shutdown.changed() => {
                        if changed.is_ok() && *shutdown.borrow() {
                            break;
                        }
                    }
                    _ = heartbeat_tick.tick() => {
                        if let Err(e) = send_heartbeat(&http, &base, &device_id).await {
                            warn!(error = %e, "controller heartbeat failed");
                        }
                        if let Ok(snapshot) = metrics.snapshot().await {
                            if let Err(e) = push_metrics(&http, &base, &device_id, &snapshot).await {
                                warn!(error = %e, "controller metrics push failed");
                            }
                        }
                        if config.push_kernel_heartbeat {
                            let payload = serde_json::json!({
                                "routes": [],
                                "telemetry": { "captured_at": Utc::now() },
                            });
                            if let Err(e) = push_kernel_heartbeat(&http, &base, &device_id, &payload).await {
                                warn!(error = %e, "controller kernel heartbeat failed");
                            }
                        }
                        if config.push_mixnet_heartbeat {
                            let payload = serde_json::json!({
                                "nodes": [],
                                "routes": [],
                            });
                            if let Err(e) = push_mixnet_heartbeat(&http, &base, &device_id, &payload).await {
                                warn!(error = %e, "controller mixnet heartbeat failed");
                            }
                        }
                        if config.push_anonymity_heartbeat {
                            let payload = serde_json::json!({
                                "routes": [],
                                "entropy_score": 0.0,
                            });
                            if let Err(e) = push_anonymity_heartbeat(&http, &base, &device_id, &payload).await {
                                warn!(error = %e, "controller anonymity heartbeat failed");
                            }
                        }
                        if config.push_ztna_heartbeat {
                            if let Some(ztna) = ztna.as_ref() {
                                let payload = ztna.heartbeat_payload();
                                if let Err(e) =
                                    push_ztna_heartbeat(&http, &base, &device_id, &payload).await
                                {
                                    warn!(error = %e, "controller ztna heartbeat failed");
                                }
                            }
                        }
                        if config.push_sse_telemetry {
                            if let Some(sse) = sse.as_ref() {
                                let payload = sse.telemetry_payload();
                                if let Err(e) =
                                    push_sse_telemetry(&http, &base, &device_id, &payload).await
                                {
                                    warn!(error = %e, "controller sse telemetry failed");
                                }
                            }
                        }
                    }
                    _ = policy_tick.tick() => {
                        match remote.pull_and_apply().await {
                            Ok(policy) => {
                                info!(version = policy.version, "controller policy applied");
                                let _ = audit
                                    .record_policy_changed(
                                        "enterprise_remote",
                                        None,
                                        Some(format!("v{}", policy.version)),
                                        Some("controller-agent".into()),
                                    )
                                    .await;
                            }
                            Err(e) => warn!(error = %e, "controller policy pull failed"),
                        }
                    }
                }
            }
        });
    }
}

#[derive(Serialize)]
struct RegisterRequest {
    enrollment_token: String,
    name: String,
    hostname: Option<String>,
    os: Option<String>,
    agent_version: Option<String>,
}

#[derive(Deserialize)]
struct RegisterResponse {
    id: String,
}

async fn register_device(
    http: &reqwest::Client,
    base: &str,
    token: &str,
    name: &str,
) -> Result<String> {
    let url = format!("{base}/api/v1/devices/register");
    let resp = http
        .post(url)
        .json(&RegisterRequest {
            enrollment_token: token.to_string(),
            name: name.to_string(),
            hostname: Some(default_device_name()),
            os: Some(std::env::consts::OS.into()),
            agent_version: Some(env!("CARGO_PKG_VERSION").into()),
        })
        .send()
        .await
        .map_err(|e| WireSentinelError::Config(format!("controller register: {e}")))?;
    if !resp.status().is_success() {
        return Err(WireSentinelError::Config(format!(
            "controller register status {}",
            resp.status()
        )));
    }
    let body: RegisterResponse = resp
        .json()
        .await
        .map_err(|e| WireSentinelError::Config(format!("controller register decode: {e}")))?;
    Ok(body.id)
}

#[derive(Serialize)]
struct HeartbeatRequest {
    agent_version: Option<String>,
    metadata: Option<serde_json::Value>,
}

async fn send_heartbeat(http: &reqwest::Client, base: &str, device_id: &str) -> Result<()> {
    let url = format!("{base}/api/v1/devices/{device_id}/heartbeat");
    let resp = http
        .post(url)
        .json(&HeartbeatRequest {
            agent_version: Some(env!("CARGO_PKG_VERSION").into()),
            metadata: Some(serde_json::json!({ "ts": Utc::now() })),
        })
        .send()
        .await
        .map_err(|e| WireSentinelError::Config(format!("controller heartbeat: {e}")))?;
    if !resp.status().is_success() {
        return Err(WireSentinelError::Config(format!(
            "controller heartbeat status {}",
            resp.status()
        )));
    }
    Ok(())
}

async fn push_metrics(
    http: &reqwest::Client,
    base: &str,
    device_id: &str,
    snapshot: &shared_types::MetricsSnapshot,
) -> Result<()> {
    let url = format!("{base}/api/v1/agents/{device_id}/metrics");
    let resp = http
        .post(url)
        .json(snapshot)
        .send()
        .await
        .map_err(|e| WireSentinelError::Config(format!("controller metrics: {e}")))?;
    if !resp.status().is_success() {
        return Err(WireSentinelError::Config(format!(
            "controller metrics status {}",
            resp.status()
        )));
    }
    Ok(())
}

async fn push_kernel_heartbeat(
    http: &reqwest::Client,
    base: &str,
    device_id: &str,
    payload: &serde_json::Value,
) -> Result<()> {
    let url = format!("{base}/api/v1/agents/{device_id}/kernel/heartbeat");
    let resp = http
        .post(url)
        .json(payload)
        .send()
        .await
        .map_err(|e| WireSentinelError::Config(format!("controller kernel heartbeat: {e}")))?;
    if !resp.status().is_success() {
        return Err(WireSentinelError::Config(format!(
            "controller kernel heartbeat status {}",
            resp.status()
        )));
    }
    Ok(())
}

async fn push_mixnet_heartbeat(
    http: &reqwest::Client,
    base: &str,
    device_id: &str,
    payload: &serde_json::Value,
) -> Result<()> {
    let url = format!("{base}/api/v1/agents/{device_id}/mixnet/heartbeat");
    let resp = http
        .post(url)
        .json(payload)
        .send()
        .await
        .map_err(|e| WireSentinelError::Config(format!("controller mixnet heartbeat: {e}")))?;
    if !resp.status().is_success() {
        return Err(WireSentinelError::Config(format!(
            "controller mixnet heartbeat status {}",
            resp.status()
        )));
    }
    Ok(())
}

async fn push_anonymity_heartbeat(
    http: &reqwest::Client,
    base: &str,
    device_id: &str,
    payload: &serde_json::Value,
) -> Result<()> {
    let url = format!("{base}/api/v1/agents/{device_id}/anonymity/heartbeat");
    let resp = http
        .post(url)
        .json(payload)
        .send()
        .await
        .map_err(|e| WireSentinelError::Config(format!("controller anonymity heartbeat: {e}")))?;
    if !resp.status().is_success() {
        return Err(WireSentinelError::Config(format!(
            "controller anonymity heartbeat status {}",
            resp.status()
        )));
    }
    Ok(())
}

async fn push_ztna_heartbeat(
    http: &reqwest::Client,
    base: &str,
    device_id: &str,
    payload: &shared_types::ZtnaHeartbeatPayload,
) -> Result<()> {
    let url = format!("{base}/api/v1/agents/{device_id}/ztna/heartbeat");
    let resp = http
        .post(url)
        .json(payload)
        .send()
        .await
        .map_err(|e| WireSentinelError::Config(format!("controller ztna heartbeat: {e}")))?;
    if !resp.status().is_success() {
        return Err(WireSentinelError::Config(format!(
            "controller ztna heartbeat status {}",
            resp.status()
        )));
    }
    Ok(())
}

async fn push_sse_telemetry(
    http: &reqwest::Client,
    base: &str,
    device_id: &str,
    payload: &shared_types::SseTelemetryPayload,
) -> Result<()> {
    let url = format!("{base}/api/v1/agents/{device_id}/sse/telemetry");
    let resp = http
        .post(url)
        .json(payload)
        .send()
        .await
        .map_err(|e| WireSentinelError::Config(format!("controller sse telemetry: {e}")))?;
    if !resp.status().is_success() {
        return Err(WireSentinelError::Config(format!(
            "controller sse telemetry status {}",
            resp.status()
        )));
    }
    Ok(())
}
