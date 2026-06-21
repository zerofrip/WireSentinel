//! ZTNA agent — connector registration, trust evaluation, controller heartbeat.

use chrono::Utc;
use event_bus::EventBus;
use parking_lot::RwLock;
use policy_engine::{ConnectionContext, ZtnaGateResult, ZtnaPolicyLookup};
use serde::{Deserialize, Serialize};
use shared_types::{
    ConnectorRegistration, DevicePosture, Resource, ResourceType, Result, ServiceEvent,
    ServiceEventInner, Subject, SubjectKind, WireSentinelError, ZtnaDecision, ZtnaHeartbeatPayload,
    ZtnaPolicyBundle,
};
use std::sync::Arc;
use std::time::Duration;
use storage::Storage;
use tokio::sync::watch;
use tracing::{info, warn};
use uuid::Uuid;
use ztna_connectors::{ApplicationConnector, ConnectorHealthMonitor};
use ztna_policy::ZtnaPolicyEngine;
use ztna_trust::DeviceTrustEngine;

const SETTINGS_KEY: &str = "ztna";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZtnaAgentConfig {
    #[serde(default)]
    pub enabled: bool,
    pub controller_url: String,
    #[serde(default)]
    pub enrollment_token: Option<String>,
    #[serde(default)]
    pub device_id: Option<String>,
    #[serde(default = "default_connector_name")]
    pub connector_name: String,
    #[serde(default)]
    pub connector_endpoint: String,
    #[serde(default = "default_heartbeat_secs")]
    pub heartbeat_interval_secs: u64,
}

fn default_connector_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "wiresentinel-ztna".into())
}

fn default_heartbeat_secs() -> u64 {
    60
}

impl Default for ZtnaAgentConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            controller_url: String::new(),
            enrollment_token: None,
            device_id: None,
            connector_name: default_connector_name(),
            connector_endpoint: String::new(),
            heartbeat_interval_secs: default_heartbeat_secs(),
        }
    }
}

pub struct ZtnaAgent {
    config: RwLock<ZtnaAgentConfig>,
    trust: DeviceTrustEngine,
    connectors: ApplicationConnector,
    policy: ZtnaPolicyEngine,
    policy_bundle: RwLock<Option<ZtnaPolicyBundle>>,
    subject: RwLock<Option<Subject>>,
    agent_id: RwLock<Option<Uuid>>,
    recent_denials: RwLock<u32>,
    gateway_active: RwLock<bool>,
}

impl ZtnaAgent {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(ZtnaAgentConfig::default()),
            trust: DeviceTrustEngine::new(),
            connectors: ApplicationConnector::new(),
            policy: ZtnaPolicyEngine::new(),
            policy_bundle: RwLock::new(None),
            subject: RwLock::new(None),
            agent_id: RwLock::new(None),
            recent_denials: RwLock::new(0),
            gateway_active: RwLock::new(false),
        }
    }

    pub async fn load_config(storage: &Storage) -> Result<ZtnaAgentConfig> {
        match storage.settings.get(SETTINGS_KEY).await? {
            Some(json) => serde_json::from_str(&json).map_err(WireSentinelError::Serde),
            None => Ok(ZtnaAgentConfig::default()),
        }
    }

    pub async fn save_config(storage: &Storage, config: &ZtnaAgentConfig) -> Result<()> {
        let json = serde_json::to_string(config).map_err(WireSentinelError::Serde)?;
        storage.settings.set(SETTINGS_KEY, &json).await
    }

    pub fn is_enabled(&self) -> bool {
        self.config.read().enabled
    }

    pub fn current_subject(&self) -> Option<Subject> {
        self.subject.read().clone()
    }

    pub fn set_subject(&self, subject: Option<Subject>) {
        *self.subject.write() = subject;
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.config.write().enabled = enabled;
    }

    pub fn apply_policy_bundle(&self, bundle: ZtnaPolicyBundle) {
        *self.gateway_active.write() = true;
        *self.policy_bundle.write() = Some(bundle);
    }

    pub fn heartbeat_payload(&self) -> ZtnaHeartbeatPayload {
        let agent_id = self.agent_id.read().unwrap_or_else(Uuid::new_v4);
        let connectors = self.connectors.list();
        let healthy_connectors = connectors
            .iter()
            .filter(|c| ConnectorHealthMonitor::check(c.id, &c.endpoint).healthy)
            .count() as u32;
        let published_resource_count = self
            .policy_bundle
            .read()
            .as_ref()
            .map(|b| b.published_resources.len() as u32)
            .unwrap_or(0);
        let avg_trust_score = self
            .policy_bundle
            .read()
            .as_ref()
            .map(|_| {
                let subject = self.subject.read();
                subject
                    .as_ref()
                    .and_then(|s| s.device_id)
                    .and_then(|id| self.trust.get(id))
                    .map(|r| f64::from(r.trust_score))
                    .unwrap_or(0.0)
            })
            .unwrap_or(0.0);

        ZtnaHeartbeatPayload {
            agent_id,
            reported_at: Utc::now(),
            identity_connected: self.subject.read().is_some(),
            active_provider: None,
            gateway_active: *self.gateway_active.read(),
            connector_count: connectors.len() as u32,
            healthy_connectors,
            avg_trust_score,
            published_resource_count,
            recent_denials: *self.recent_denials.read(),
        }
    }

    pub fn spawn(
        storage: Arc<Storage>,
        events: EventBus,
        agent: Arc<ZtnaAgent>,
        mut config: ZtnaAgentConfig,
        mut shutdown: watch::Receiver<bool>,
    ) {
        if !config.enabled || config.controller_url.is_empty() {
            return;
        }

        *agent.config.write() = config.clone();

        tokio::spawn(async move {
            let http = reqwest::Client::new();
            let base = config.controller_url.trim_end_matches('/').to_string();

            if config.device_id.is_none() {
                if let Some(token) = config.enrollment_token.clone() {
                    match register_connector(&http, &base, &token, &config).await {
                        Ok(registration) => {
                            config.device_id = Some(registration.connector_id.to_string());
                            *agent.agent_id.write() = Some(registration.connector_id);
                            agent.connectors.register(
                                registration.name.clone(),
                                registration.endpoint.clone(),
                                registration.resource_ids.clone(),
                            );
                            if let Err(e) = ZtnaAgent::save_config(&storage, &config).await {
                                warn!(err = %e, "failed to persist ztna connector id");
                            }
                            events.publish(ServiceEvent::now(ServiceEventInner::AgentEnrolled {
                                agent_id: registration.connector_id,
                                name: registration.name,
                            }));
                            info!(connector_id = %registration.connector_id, "ztna connector registered");
                        }
                        Err(e) => warn!(err = %e, "ztna connector registration failed"),
                    }
                } else {
                    warn!("ztna agent enabled but no enrollment token or device id");
                    return;
                }
            } else if let Ok(id) = Uuid::parse_str(&config.device_id.clone().unwrap_or_default()) {
                *agent.agent_id.write() = Some(id);
                if !config.connector_endpoint.is_empty() {
                    agent.connectors.register(
                        config.connector_name.clone(),
                        config.connector_endpoint.clone(),
                        Vec::new(),
                    );
                }
            }

            let device_id = match config.device_id.clone() {
                Some(id) => id,
                None => return,
            };

            if let Ok(uuid) = Uuid::parse_str(&device_id) {
                let _ = agent.trust.evaluate(uuid, DevicePosture::default());
            }

            let mut heartbeat_tick =
                tokio::time::interval(Duration::from_secs(config.heartbeat_interval_secs.max(15)));

            loop {
                tokio::select! {
                    changed = shutdown.changed() => {
                        if changed.is_ok() && *shutdown.borrow() {
                            break;
                        }
                    }
                    _ = heartbeat_tick.tick() => {
                        let payload = agent.heartbeat_payload();
                        if let Err(e) = send_ztna_heartbeat(&http, &base, &device_id, &payload).await {
                            warn!(err = %e, "ztna heartbeat failed");
                        }
                        if let Ok(bundle) = pull_policy_bundle(&http, &base, &device_id).await {
                            agent.apply_policy_bundle(bundle);
                        }
                    }
                }
            }
        });
    }

    fn evaluate_subject_access(
        &self,
        subject: &Subject,
        resource: &Resource,
    ) -> Option<ZtnaGateResult> {
        let bundle = self.policy_bundle.read();
        let policies = match bundle.as_ref() {
            None => {
                return Some(ZtnaGateResult {
                    allowed: true,
                    reason: "ztna policy inactive".into(),
                    matched_policy_id: None,
                });
            }
            Some(bundle) if bundle.policies.is_empty() => {
                return Some(ZtnaGateResult {
                    allowed: true,
                    reason: "no ztna policies configured".into(),
                    matched_policy_id: None,
                });
            }
            Some(bundle) => bundle.policies.as_slice(),
        };

        let device_id = subject.device_id.unwrap_or(subject.id);
        let trust = self
            .trust
            .evaluate(device_id, DevicePosture::default())
            .ok()
            .or_else(|| self.trust.get(device_id));

        let result = self
            .policy
            .evaluate_policies(subject, resource.id, policies, trust.as_ref())
            .ok()?;

        match result.decision {
            ZtnaDecision::Deny => {
                *self.recent_denials.write() += 1;
                Some(ZtnaGateResult {
                    allowed: false,
                    reason: result.reason,
                    matched_policy_id: result.matched_policy_id,
                })
            }
            _ => Some(ZtnaGateResult {
                allowed: true,
                reason: result.reason,
                matched_policy_id: result.matched_policy_id,
            }),
        }
    }
}

impl Default for ZtnaAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl ZtnaPolicyLookup for ZtnaAgent {
    fn evaluate(&self, ctx: &ConnectionContext) -> Option<ZtnaGateResult> {
        if !self.is_enabled() {
            return None;
        }
        let subject = ctx.ztna_subject.as_ref()?;
        let resource = connection_resource(ctx);
        self.evaluate_subject_access(subject, &resource)
    }
}

fn connection_resource(ctx: &ConnectionContext) -> Resource {
    let name = ctx.domain.clone().unwrap_or_else(|| "connection".into());
    Resource {
        id: Uuid::new_v5(&Uuid::NAMESPACE_DNS, name.as_bytes()),
        name: name.clone(),
        resource_type: if ctx.domain.is_some() {
            ResourceType::Https
        } else {
            ResourceType::Tcp
        },
        host: name,
        port: 0,
        path_prefix: None,
        tags: Vec::new(),
    }
}
#[derive(Serialize)]
struct ConnectorRegisterRequest {
    enrollment_token: String,
    name: String,
    endpoint: String,
    hostname: Option<String>,
    agent_version: Option<String>,
}

#[derive(Deserialize)]
struct ConnectorRegisterResponse {
    connector_id: Uuid,
    name: String,
    endpoint: String,
    #[serde(default)]
    resource_ids: Vec<Uuid>,
}

async fn register_connector(
    http: &reqwest::Client,
    base: &str,
    token: &str,
    config: &ZtnaAgentConfig,
) -> Result<ConnectorRegistration> {
    let url = format!("{base}/api/v1/ztna/connectors/register");
    let resp = http
        .post(url)
        .json(&ConnectorRegisterRequest {
            enrollment_token: token.to_string(),
            name: config.connector_name.clone(),
            endpoint: config.connector_endpoint.clone(),
            hostname: Some(default_connector_name()),
            agent_version: Some(env!("CARGO_PKG_VERSION").into()),
        })
        .send()
        .await
        .map_err(|e| WireSentinelError::Config(format!("ztna register: {e}")))?;

    if !resp.status().is_success() {
        return Err(WireSentinelError::Config(format!(
            "ztna register status {}",
            resp.status()
        )));
    }

    let body: ConnectorRegisterResponse = resp
        .json()
        .await
        .map_err(|e| WireSentinelError::Config(format!("ztna register decode: {e}")))?;

    Ok(ConnectorRegistration {
        connector_id: body.connector_id,
        name: body.name,
        endpoint: body.endpoint,
        resource_ids: body.resource_ids,
        registered_at: Utc::now(),
    })
}

async fn send_ztna_heartbeat(
    http: &reqwest::Client,
    base: &str,
    device_id: &str,
    payload: &ZtnaHeartbeatPayload,
) -> Result<()> {
    let url = format!("{base}/api/v1/agents/{device_id}/ztna/heartbeat");
    let resp = http
        .post(url)
        .json(payload)
        .send()
        .await
        .map_err(|e| WireSentinelError::Config(format!("ztna heartbeat: {e}")))?;

    if !resp.status().is_success() {
        return Err(WireSentinelError::Config(format!(
            "ztna heartbeat status {}",
            resp.status()
        )));
    }
    Ok(())
}

async fn pull_policy_bundle(
    http: &reqwest::Client,
    base: &str,
    device_id: &str,
) -> Result<ZtnaPolicyBundle> {
    let url = format!("{base}/api/v1/agents/{device_id}/ztna/policy");
    let resp = http
        .get(url)
        .send()
        .await
        .map_err(|e| WireSentinelError::Config(format!("ztna policy pull: {e}")))?;

    if !resp.status().is_success() {
        return Err(WireSentinelError::Config(format!(
            "ztna policy pull status {}",
            resp.status()
        )));
    }

    resp.json()
        .await
        .map_err(|e| WireSentinelError::Config(format!("ztna policy decode: {e}")))
}
