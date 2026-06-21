//! XDR agent — EDR telemetry collection and controller push.

use chrono::Utc;
use edr::EdrEngine;
use event_bus::EventBus;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use shared_types::{
    ProcessEvent, Result, ServiceEvent, ServiceEventInner, XdrPolicyBundle, XdrTelemetryPayload,
    WireSentinelError,
};
use std::sync::Arc;
use std::time::Duration;
use storage::Storage;
use tokio::sync::watch;
use tracing::{info, warn};
use uuid::Uuid;
use xdr_core::XdrEventEmitter;

const SETTINGS_KEY: &str = "xdr";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct XdrAgentConfig {
    #[serde(default)]
    pub enabled: bool,
    pub controller_url: String,
    #[serde(default)]
    pub enrollment_token: Option<String>,
    #[serde(default)]
    pub device_id: Option<String>,
    #[serde(default = "default_telemetry_secs")]
    pub telemetry_interval_secs: u64,
}

fn default_telemetry_secs() -> u64 {
    60
}

impl Default for XdrAgentConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            controller_url: String::new(),
            enrollment_token: None,
            device_id: None,
            telemetry_interval_secs: default_telemetry_secs(),
        }
    }
}

/// Bridges XDR engines to the core service and controller telemetry pipeline.
pub struct XdrAgent {
    config: RwLock<XdrAgentConfig>,
    edr: EdrEngine<EventBusEmitter>,
    policy_bundle: RwLock<Option<XdrPolicyBundle>>,
    agent_id: RwLock<Option<Uuid>>,
    process_events: RwLock<u32>,
    file_events: RwLock<u32>,
    network_events: RwLock<u32>,
    identity_threats: RwLock<u32>,
    detection_matches: RwLock<u32>,
}

#[derive(Clone)]
struct EventBusEmitter(EventBus);

impl XdrEventEmitter for EventBusEmitter {
    fn emit(&self, event: ServiceEvent) {
        self.0.publish(event);
    }
}

impl XdrAgent {
    pub fn new(events: EventBus) -> Self {
        let emitter = EventBusEmitter(events);
        Self {
            config: RwLock::new(XdrAgentConfig::default()),
            edr: EdrEngine::new(emitter),
            policy_bundle: RwLock::new(None),
            agent_id: RwLock::new(None),
            process_events: RwLock::new(0),
            file_events: RwLock::new(0),
            network_events: RwLock::new(0),
            identity_threats: RwLock::new(0),
            detection_matches: RwLock::new(0),
        }
    }

    pub async fn load_config(storage: &Storage) -> Result<XdrAgentConfig> {
        match storage.settings.get(SETTINGS_KEY).await? {
            Some(json) => serde_json::from_str(&json).map_err(WireSentinelError::Serde),
            None => Ok(XdrAgentConfig::default()),
        }
    }

    pub async fn save_config(storage: &Storage, config: &XdrAgentConfig) -> Result<()> {
        let json = serde_json::to_string(config).map_err(WireSentinelError::Serde)?;
        storage.settings.set(SETTINGS_KEY, &json).await
    }

    pub fn is_enabled(&self) -> bool {
        self.config.read().enabled
    }

    pub fn apply_policy_bundle(&self, bundle: XdrPolicyBundle) {
        *self.policy_bundle.write() = Some(bundle);
    }

    pub fn ingest_process(&self, event: ProcessEvent) -> Result<()> {
        self.edr
            .ingest_process(event)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        *self.process_events.write() += 1;
        Ok(())
    }

    pub fn telemetry_payload(&self) -> XdrTelemetryPayload {
        let agent_id = self.agent_id.read().unwrap_or_else(Uuid::new_v4);
        let device_id = self
            .config
            .read()
            .device_id
            .as_ref()
            .and_then(|id| Uuid::parse_str(id).ok())
            .unwrap_or(agent_id);

        XdrTelemetryPayload {
            agent_id,
            device_id,
            reported_at: Utc::now(),
            process_events: *self.process_events.read(),
            file_events: *self.file_events.read(),
            network_events: *self.network_events.read(),
            identity_threats: *self.identity_threats.read(),
            active_incidents: 0,
            detection_matches: *self.detection_matches.read(),
        }
    }

    pub fn spawn(
        storage: Arc<Storage>,
        events: EventBus,
        agent: Arc<XdrAgent>,
        mut config: XdrAgentConfig,
        mut shutdown: watch::Receiver<bool>,
    ) {
        if !config.enabled || config.controller_url.is_empty() {
            return;
        }

        *agent.config.write() = config.clone();

        tokio::spawn(async move {
            let http = reqwest::Client::new();
            let base = config.controller_url.trim_end_matches('/').to_string();

            let device_id = if let Some(id) = config.device_id.clone() {
                if let Ok(parsed) = Uuid::parse_str(&id) {
                    *agent.agent_id.write() = Some(parsed);
                }
                id
            } else if let Some(token) = config.enrollment_token.clone() {
                match register_xdr_agent(&http, &base, &token).await {
                    Ok(agent_id) => {
                        config.device_id = Some(agent_id.to_string());
                        *agent.agent_id.write() = Some(agent_id);
                        if let Err(e) = XdrAgent::save_config(&storage, &config).await {
                            warn!(err = %e, "failed to persist xdr agent id");
                        }
                        events.publish(ServiceEvent::now(ServiceEventInner::AgentEnrolled {
                            agent_id,
                            name: "xdr-agent".into(),
                        }));
                        info!(agent_id = %agent_id, "xdr agent registered");
                        agent_id.to_string()
                    }
                    Err(e) => {
                        warn!(err = %e, "xdr agent registration failed");
                        return;
                    }
                }
            } else {
                warn!("xdr agent enabled but no enrollment token or device id");
                return;
            };

            let mut telemetry_tick =
                tokio::time::interval(Duration::from_secs(config.telemetry_interval_secs.max(15)));

            loop {
                tokio::select! {
                    changed = shutdown.changed() => {
                        if changed.is_ok() && *shutdown.borrow() {
                            break;
                        }
                    }
                    _ = telemetry_tick.tick() => {
                        if let Ok(bundle) = pull_xdr_policy_bundle(&http, &base, &device_id).await {
                            agent.apply_policy_bundle(bundle);
                        }
                        let payload = agent.telemetry_payload();
                        if let Err(e) = push_xdr_telemetry(&http, &base, &device_id, &payload).await {
                            warn!(err = %e, "xdr telemetry push failed");
                        }
                    }
                }
            }
        });
    }
}

#[derive(Serialize)]
struct XdrRegisterRequest {
    enrollment_token: String,
    agent_version: Option<String>,
}

async fn register_xdr_agent(
    http: &reqwest::Client,
    base: &str,
    token: &str,
) -> Result<Uuid> {
    let url = format!("{base}/api/v1/agents/register");
    let body = XdrRegisterRequest {
        enrollment_token: token.to_string(),
        agent_version: Some(env!("CARGO_PKG_VERSION").to_string()),
    };
    let resp = http
        .post(url)
        .json(&body)
        .send()
        .await
        .map_err(|e| WireSentinelError::Config(format!("xdr register: {e}")))?;

    if !resp.status().is_success() {
        return Err(WireSentinelError::Config(format!(
            "xdr register status {}",
            resp.status()
        )));
    }

    #[derive(Deserialize)]
    struct RegisterResponse {
        agent_id: Uuid,
    }

    let parsed: RegisterResponse = resp
        .json()
        .await
        .map_err(|e| WireSentinelError::Config(format!("xdr register decode: {e}")))?;
    Ok(parsed.agent_id)
}

async fn pull_xdr_policy_bundle(
    http: &reqwest::Client,
    base: &str,
    device_id: &str,
) -> Result<XdrPolicyBundle> {
    let url = format!("{base}/api/v1/agents/{device_id}/xdr/policy");
    let resp = http
        .get(url)
        .send()
        .await
        .map_err(|e| WireSentinelError::Config(format!("xdr policy pull: {e}")))?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(WireSentinelError::Config("no xdr policy bundle".into()));
    }

    if !resp.status().is_success() {
        return Err(WireSentinelError::Config(format!(
            "xdr policy status {}",
            resp.status()
        )));
    }

    resp.json()
        .await
        .map_err(|e| WireSentinelError::Config(format!("xdr policy decode: {e}")))
}

async fn push_xdr_telemetry(
    http: &reqwest::Client,
    base: &str,
    device_id: &str,
    payload: &XdrTelemetryPayload,
) -> Result<()> {
    let url = format!("{base}/api/v1/agents/{device_id}/xdr/telemetry");
    let resp = http
        .post(url)
        .json(payload)
        .send()
        .await
        .map_err(|e| WireSentinelError::Config(format!("xdr telemetry: {e}")))?;

    if !resp.status().is_success() {
        return Err(WireSentinelError::Config(format!(
            "xdr telemetry status {}",
            resp.status()
        )));
    }
    Ok(())
}
