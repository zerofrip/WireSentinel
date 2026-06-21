//! SSE agent — SWG/DLP/threat local eval and controller telemetry push.

use chrono::Utc;
use dlp::DlpEngine;
use event_bus::EventBus;
use parking_lot::RwLock;
use policy_engine::{ConnectionContext, SsePolicyLookup};
use risk_engine::ContinuousRiskEngine;
use serde::{Deserialize, Serialize};
use shared_types::{
    DlpAction, Result, ServiceEvent, ServiceEventInner, SsePolicyBundle, SseTelemetryPayload,
    WebAccessAction, WireSentinelError,
};
use std::sync::Arc;
use std::time::Duration;
use storage::Storage;
use swg::SecureWebGateway;
use threat_protection::ThreatProtectionEngine;
use tokio::sync::watch;
use tracing::{info, warn};
use uuid::Uuid;

const SETTINGS_KEY: &str = "sse";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SseAgentConfig {
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

impl Default for SseAgentConfig {
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

pub struct SseAgent {
    config: RwLock<SseAgentConfig>,
    swg: SecureWebGateway,
    dlp: DlpEngine,
    threat: ThreatProtectionEngine,
    risk: ContinuousRiskEngine,
    policy_bundle: RwLock<Option<SsePolicyBundle>>,
    agent_id: RwLock<Option<Uuid>>,
    recent_blocks: RwLock<u32>,
    recent_incidents: RwLock<u32>,
    risk_samples: RwLock<Vec<u8>>,
}

impl SseAgent {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(SseAgentConfig::default()),
            swg: SecureWebGateway::new(),
            dlp: DlpEngine::new(),
            threat: ThreatProtectionEngine::new(),
            risk: ContinuousRiskEngine::default(),
            policy_bundle: RwLock::new(None),
            agent_id: RwLock::new(None),
            recent_blocks: RwLock::new(0),
            recent_incidents: RwLock::new(0),
            risk_samples: RwLock::new(Vec::new()),
        }
    }

    pub async fn load_config(storage: &Storage) -> Result<SseAgentConfig> {
        match storage.settings.get(SETTINGS_KEY).await? {
            Some(json) => serde_json::from_str(&json).map_err(WireSentinelError::Serde),
            None => Ok(SseAgentConfig::default()),
        }
    }

    pub async fn save_config(storage: &Storage, config: &SseAgentConfig) -> Result<()> {
        let json = serde_json::to_string(config).map_err(WireSentinelError::Serde)?;
        storage.settings.set(SETTINGS_KEY, &json).await
    }

    pub fn is_enabled(&self) -> bool {
        self.config.read().enabled
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.config.write().enabled = enabled;
    }

    pub fn apply_policy_bundle(&self, bundle: SsePolicyBundle) {
        for policy in &bundle.web_policies {
            self.swg.add_policy(policy.clone());
        }
        for policy in &bundle.dlp_policies {
            self.dlp.add_policy(policy.clone());
        }
        for indicator in &bundle.threat_indicators {
            self.threat.add_indicator(indicator.clone());
        }
        *self.policy_bundle.write() = Some(bundle);
    }

    pub fn telemetry_payload(&self) -> SseTelemetryPayload {
        let agent_id = self.agent_id.read().unwrap_or_else(Uuid::new_v4);
        let bundle = self.policy_bundle.read();
        let samples = self.risk_samples.read();
        let avg_risk_score = if samples.is_empty() {
            0.0
        } else {
            let sum: u32 = samples.iter().map(|s| u32::from(*s)).sum();
            f64::from(sum) / samples.len() as f64
        };

        SseTelemetryPayload {
            agent_id,
            reported_at: Utc::now(),
            swg_active: bundle
                .as_ref()
                .map(|b| !b.web_policies.is_empty())
                .unwrap_or(false),
            casb_providers: 0,
            dlp_policies: bundle
                .as_ref()
                .map(|b| b.dlp_policies.len() as u32)
                .unwrap_or(0),
            active_isolation_sessions: 0,
            threat_feeds: self.threat.feeds().len() as u32,
            avg_risk_score,
            recent_incidents: *self.recent_incidents.read(),
        }
    }

    fn record_block(&self, risk_score: u8) {
        *self.recent_blocks.write() += 1;
        *self.recent_incidents.write() += 1;
        let mut samples = self.risk_samples.write();
        samples.push(risk_score);
        if samples.len() > 256 {
            let drain = samples.len().saturating_sub(128);
            samples.drain(0..drain);
        }
    }

    fn evaluate_local(&self, ctx: &ConnectionContext) -> Option<policy_engine::SseGateResult> {
        let domain = ctx.domain.as_ref()?;
        let bundle = self.policy_bundle.read();
        let bundle = match bundle.as_ref() {
            None => {
                return Some(policy_engine::SseGateResult {
                    allowed: true,
                    reason: "sse policy inactive".into(),
                    matched_policy_id: None,
                });
            }
            Some(bundle)
                if bundle.web_policies.is_empty()
                    && bundle.dlp_policies.is_empty()
                    && bundle.threat_indicators.is_empty() =>
            {
                return Some(policy_engine::SseGateResult {
                    allowed: true,
                    reason: "no sse policies configured".into(),
                    matched_policy_id: None,
                });
            }
            Some(_) => {}
        };

        let url = normalize_url(domain);

        if let Ok(web) = self.swg.evaluate(&url) {
            if web.action == WebAccessAction::Block {
                self.record_block(90);
                return Some(policy_engine::SseGateResult {
                    allowed: false,
                    reason: web.reason,
                    matched_policy_id: None,
                });
            }
        }

        let host = extract_host(domain);
        if let Ok(Some(threat_match)) = self.threat.check(&host, true) {
            if threat_match.blocked {
                self.record_block(100);
                return Some(policy_engine::SseGateResult {
                    allowed: false,
                    reason: format!("threat indicator: {}", threat_match.matched_value),
                    matched_policy_id: Some(threat_match.indicator.id),
                });
            }
        }

        let user_id = ctx.ztna_subject.as_ref().map(|s| s.id);
        if let Ok(incidents) = self.dlp.scan(&url, "web", user_id) {
            for incident in incidents {
                if incident.action == DlpAction::Block {
                    self.record_block(95);
                    return Some(policy_engine::SseGateResult {
                        allowed: false,
                        reason: format!("dlp pattern {:?}", incident.pattern),
                        matched_policy_id: Some(incident.policy_id),
                    });
                }
            }
        }

        if let Some(subject_id) = user_id {
            if let Ok(score) = self.risk.compute(subject_id, 0, 0, 0, 0) {
                self.risk_samples.write().push(score.score);
            }
        }

        let _ = bundle;
        Some(policy_engine::SseGateResult {
            allowed: true,
            reason: "sse checks passed".into(),
            matched_policy_id: None,
        })
    }

    pub fn spawn(
        storage: Arc<Storage>,
        events: EventBus,
        agent: Arc<SseAgent>,
        mut config: SseAgentConfig,
        mut shutdown: watch::Receiver<bool>,
    ) {
        if !config.enabled || config.controller_url.is_empty() {
            return;
        }

        *agent.config.write() = config.clone();

        tokio::spawn(async move {
            let http = reqwest::Client::new();
            let base = config.controller_url.trim_end_matches('/').to_string();

            if let Some(device_id) = config.device_id.clone() {
                if let Ok(id) = Uuid::parse_str(&device_id) {
                    *agent.agent_id.write() = Some(id);
                }
            } else if let Some(token) = config.enrollment_token.clone() {
                match register_sse_agent(&http, &base, &token).await {
                    Ok(agent_id) => {
                        config.device_id = Some(agent_id.to_string());
                        *agent.agent_id.write() = Some(agent_id);
                        if let Err(e) = SseAgent::save_config(&storage, &config).await {
                            warn!(err = %e, "failed to persist sse agent id");
                        }
                        events.publish(ServiceEvent::now(ServiceEventInner::AgentEnrolled {
                            agent_id,
                            name: "sse-agent".into(),
                        }));
                        info!(agent_id = %agent_id, "sse agent registered");
                    }
                    Err(e) => warn!(err = %e, "sse agent registration failed"),
                }
            } else {
                warn!("sse agent enabled but no enrollment token or device id");
                return;
            }

            let device_id = match config.device_id.clone() {
                Some(id) => id,
                None => return,
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
                        if let Ok(bundle) = pull_policy_bundle(&http, &base, &device_id).await {
                            agent.apply_policy_bundle(bundle);
                        }
                        let payload = agent.telemetry_payload();
                        if let Err(e) = push_sse_telemetry(&http, &base, &device_id, &payload).await {
                            warn!(err = %e, "sse telemetry push failed");
                        }
                    }
                }
            }
        });
    }
}

impl Default for SseAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl SsePolicyLookup for SseAgent {
    fn evaluate(&self, ctx: &ConnectionContext) -> Option<policy_engine::SseGateResult> {
        if !self.is_enabled() {
            return None;
        }
        self.evaluate_local(ctx)
    }
}

fn normalize_url(domain: &str) -> String {
    if domain.starts_with("http://") || domain.starts_with("https://") {
        domain.to_string()
    } else {
        format!("https://{domain}")
    }
}

fn extract_host(domain: &str) -> String {
    domain
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or(domain)
        .split(':')
        .next()
        .unwrap_or(domain)
        .to_string()
}

#[derive(Serialize)]
struct SseRegisterRequest {
    enrollment_token: String,
    agent_version: Option<String>,
}

#[derive(Deserialize)]
struct SseRegisterResponse {
    agent_id: Uuid,
}

async fn register_sse_agent(
    http: &reqwest::Client,
    base: &str,
    token: &str,
) -> Result<Uuid> {
    let url = format!("{base}/api/v1/sse/agents/register");
    let resp = http
        .post(url)
        .json(&SseRegisterRequest {
            enrollment_token: token.to_string(),
            agent_version: Some(env!("CARGO_PKG_VERSION").into()),
        })
        .send()
        .await
        .map_err(|e| WireSentinelError::Config(format!("sse register: {e}")))?;

    if !resp.status().is_success() {
        return Err(WireSentinelError::Config(format!(
            "sse register status {}",
            resp.status()
        )));
    }

    let body: SseRegisterResponse = resp
        .json()
        .await
        .map_err(|e| WireSentinelError::Config(format!("sse register decode: {e}")))?;

    Ok(body.agent_id)
}

async fn pull_policy_bundle(
    http: &reqwest::Client,
    base: &str,
    device_id: &str,
) -> Result<SsePolicyBundle> {
    let url = format!("{base}/api/v1/agents/{device_id}/sse/policy");
    let resp = http
        .get(url)
        .send()
        .await
        .map_err(|e| WireSentinelError::Config(format!("sse policy pull: {e}")))?;

    if !resp.status().is_success() {
        return Err(WireSentinelError::Config(format!(
            "sse policy pull status {}",
            resp.status()
        )));
    }

    resp.json()
        .await
        .map_err(|e| WireSentinelError::Config(format!("sse policy decode: {e}")))
}

async fn push_sse_telemetry(
    http: &reqwest::Client,
    base: &str,
    device_id: &str,
    payload: &SseTelemetryPayload,
) -> Result<()> {
    let url = format!("{base}/api/v1/agents/{device_id}/sse/telemetry");
    let resp = http
        .post(url)
        .json(payload)
        .send()
        .await
        .map_err(|e| WireSentinelError::Config(format!("sse telemetry: {e}")))?;

    if !resp.status().is_success() {
        return Err(WireSentinelError::Config(format!(
            "sse telemetry status {}",
            resp.status()
        )));
    }
    Ok(())
}
