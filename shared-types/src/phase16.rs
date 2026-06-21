//! Phase 16 SSE (Secure Service Edge) shared DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Web access decision action for SWG policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WebAccessAction {
    Allow,
    Block,
    Warn,
    Isolate,
    LogOnly,
}

/// DLP enforcement action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DlpAction {
    Allow,
    Block,
    Quarantine,
    Redact,
    LogOnly,
}

/// Continuous risk tier synthesized from UEBA, threat, DLP, and CASB signals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Minimal,
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn score_floor(self) -> u8 {
        match self {
            Self::Minimal => 0,
            Self::Low => 25,
            Self::Medium => 50,
            Self::High => 75,
            Self::Critical => 90,
        }
    }
}

/// SSE security policy envelope (Phase 16-A).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SseSecurityPolicy {
    pub id: Uuid,
    pub name: String,
    pub enabled: bool,
    pub min_risk_level: RiskLevel,
    pub default_web_action: WebAccessAction,
    pub default_dlp_action: DlpAction,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SseSecurityPolicy {
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            enabled: true,
            min_risk_level: RiskLevel::Medium,
            default_web_action: WebAccessAction::Allow,
            default_dlp_action: DlpAction::Block,
            created_at: now,
            updated_at: now,
        }
    }
}

/// URL content category for SWG filtering.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum UrlCategory {
    Business,
    SocialMedia,
    Gambling,
    Malware,
    Phishing,
    Adult,
    Streaming,
    CloudStorage,
    Uncategorized,
    Custom(String),
}

/// Domain reputation snapshot used by SWG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DomainReputation {
    pub domain: String,
    pub score: u8,
    pub malicious: bool,
    pub categories: Vec<UrlCategory>,
    pub evaluated_at: DateTime<Utc>,
}

/// Web access policy rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct WebPolicy {
    pub id: Uuid,
    pub name: String,
    pub enabled: bool,
    pub blocked_categories: Vec<UrlCategory>,
    pub allowed_domains: Vec<String>,
    pub blocked_domains: Vec<String>,
    pub default_action: WebAccessAction,
}

impl WebPolicy {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            enabled: true,
            blocked_categories: Vec::new(),
            allowed_domains: Vec::new(),
            blocked_domains: Vec::new(),
            default_action: WebAccessAction::Allow,
        }
    }
}

/// Outcome of a web access evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct WebAccessResult {
    pub url: String,
    pub domain: String,
    pub action: WebAccessAction,
    pub category: Option<UrlCategory>,
    pub reputation: Option<DomainReputation>,
    pub reason: String,
    pub evaluated_at: DateTime<Utc>,
}

/// CASB SaaS provider kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CasbProviderKind {
    M365,
    Google,
    Slack,
    GitHub,
    Dropbox,
    Box,
    Salesforce,
    GenericMock,
}

/// Known SaaS application catalog entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SaasApplication {
    pub id: Uuid,
    pub name: String,
    pub provider: CasbProviderKind,
    pub domain: String,
    pub sanctioned: bool,
    pub risk_score: u8,
}

/// Shadow IT discovery record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ShadowItRecord {
    pub id: Uuid,
    pub application: SaasApplication,
    pub user_id: Uuid,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub session_count: u64,
}

/// CASB policy violation finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CasbFinding {
    pub id: Uuid,
    pub provider: CasbProviderKind,
    pub application_id: Uuid,
    pub user_id: Uuid,
    pub violation_type: String,
    pub detail: String,
    pub detected_at: DateTime<Utc>,
}

/// DLP sensitive data pattern kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DlpPatternKind {
    CreditCard,
    ApiKey,
    Ssn,
    Email,
    PhoneNumber,
    Custom,
}

/// DLP policy definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DlpPolicy {
    pub id: Uuid,
    pub name: String,
    pub enabled: bool,
    pub patterns: Vec<DlpPatternKind>,
    pub action: DlpAction,
    pub channels: Vec<String>,
}

impl DlpPolicy {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            enabled: true,
            patterns: Vec::new(),
            action: DlpAction::Block,
            channels: vec!["web".into(), "email".into()],
        }
    }
}

/// DLP incident record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DlpIncident {
    pub id: Uuid,
    pub policy_id: Uuid,
    pub pattern: DlpPatternKind,
    pub action: DlpAction,
    pub channel: String,
    pub user_id: Option<Uuid>,
    pub matched_snippet: String,
    pub detected_at: DateTime<Utc>,
}

/// Browser isolation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IsolationMode {
    Disabled,
    Remote,
    Containerized,
    ReadOnly,
}

/// Browser isolation policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct IsolationPolicy {
    pub id: Uuid,
    pub name: String,
    pub enabled: bool,
    pub mode: IsolationMode,
    pub target_categories: Vec<UrlCategory>,
    pub target_domains: Vec<String>,
}

impl IsolationPolicy {
    pub fn new(name: impl Into<String>, mode: IsolationMode) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            enabled: true,
            mode,
            target_categories: Vec::new(),
            target_domains: Vec::new(),
        }
    }
}

/// Active browser isolation session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct IsolationSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub url: String,
    pub mode: IsolationMode,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

/// Threat indicator from a feed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ThreatIndicator {
    pub id: Uuid,
    pub indicator_type: String,
    pub value: String,
    pub severity: RiskLevel,
    pub source_feed_id: Uuid,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Threat intelligence feed metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ThreatFeed {
    pub id: Uuid,
    pub name: String,
    pub provider: String,
    pub enabled: bool,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub indicator_count: u64,
}

/// Threat match against traffic or content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ThreatMatch {
    pub id: Uuid,
    pub indicator: ThreatIndicator,
    pub matched_value: String,
    pub blocked: bool,
    pub detected_at: DateTime<Utc>,
}

/// UEBA behavior baseline for a subject.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct BehaviorBaseline {
    pub subject_id: Uuid,
    pub avg_daily_sessions: f64,
    pub typical_hours: Vec<u8>,
    pub typical_destinations: Vec<String>,
    pub established_at: DateTime<Utc>,
}

/// UEBA behavior anomaly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct BehaviorAnomaly {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub anomaly_type: String,
    pub severity: RiskLevel,
    pub detail: String,
    pub detected_at: DateTime<Utc>,
}

/// Continuous risk score snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RiskScore {
    pub subject_id: Uuid,
    pub score: u8,
    pub level: RiskLevel,
    pub ueba_contribution: u8,
    pub threat_contribution: u8,
    pub dlp_contribution: u8,
    pub casb_contribution: u8,
    pub computed_at: DateTime<Utc>,
}

/// Data lake retention preset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RetentionPolicy {
    Days30,
    Days90,
    Days180,
    Days365,
    Custom { days: u32 },
}

impl RetentionPolicy {
    pub fn days(&self) -> u32 {
        match self {
            Self::Days30 => 30,
            Self::Days90 => 90,
            Self::Days180 => 180,
            Self::Days365 => 365,
            Self::Custom { days } => *days,
        }
    }
}

/// Stored security event in the data lake.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SecurityEventRecord {
    pub id: Uuid,
    pub event_kind: String,
    pub payload: serde_json::Value,
    pub tenant_id: Uuid,
    pub ingested_at: DateTime<Utc>,
}

/// SIEM exporter destination kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SiemExporterKind {
    Splunk,
    Sentinel,
    Elastic,
    OpenSearch,
    QRadar,
    Syslog,
}

/// SIEM export serialization format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SiemExportFormat {
    Json,
    Cef,
    Leef,
    Syslog,
}

/// SIEM export job descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SiemExportJob {
    pub id: Uuid,
    pub exporter: SiemExporterKind,
    pub format: SiemExportFormat,
    pub endpoint: String,
    pub event_count: u64,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Agent telemetry payload for WireSentinel-Controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SseTelemetryPayload {
    pub agent_id: Uuid,
    pub reported_at: DateTime<Utc>,
    pub swg_active: bool,
    pub casb_providers: u32,
    pub dlp_policies: u32,
    pub active_isolation_sessions: u32,
    pub threat_feeds: u32,
    pub avg_risk_score: f64,
    pub recent_incidents: u32,
}

impl SseTelemetryPayload {
    pub fn empty(agent_id: Uuid) -> Self {
        Self {
            agent_id,
            reported_at: Utc::now(),
            swg_active: false,
            casb_providers: 0,
            dlp_policies: 0,
            active_isolation_sessions: 0,
            threat_feeds: 0,
            avg_risk_score: 0.0,
            recent_incidents: 0,
        }
    }
}

/// Policy bundle pushed from WireSentinel-Controller for local SSE evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SsePolicyBundle {
    pub bundle_id: Uuid,
    pub tenant_id: Uuid,
    pub security_policy: Option<SseSecurityPolicy>,
    pub web_policies: Vec<WebPolicy>,
    pub dlp_policies: Vec<DlpPolicy>,
    pub threat_indicators: Vec<ThreatIndicator>,
    pub issued_at: DateTime<Utc>,
}

/// Incident bundle pushed to WireSentinel-Controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SseIncidentBundle {
    pub bundle_id: Uuid,
    pub tenant_id: Uuid,
    pub dlp_incidents: Vec<DlpIncident>,
    pub casb_findings: Vec<CasbFinding>,
    pub threat_matches: Vec<ThreatMatch>,
    pub anomalies: Vec<BehaviorAnomaly>,
    pub risk_scores: Vec<RiskScore>,
    pub issued_at: DateTime<Utc>,
}
