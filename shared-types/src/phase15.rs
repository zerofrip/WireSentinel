//! Phase 15 ZTNA (Zero Trust Network Access) shared DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Device / session trust tier used in policy evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    Untrusted,
    Low,
    Medium,
    High,
    Full,
}

impl TrustLevel {
    pub fn score_floor(self) -> u8 {
        match self {
            Self::Untrusted => 0,
            Self::Low => 25,
            Self::Medium => 50,
            Self::High => 75,
            Self::Full => 90,
        }
    }
}

/// Outcome of a ZTNA access evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ZtnaDecision {
    Allow,
    Deny,
    Challenge,
    StepUp,
}

/// Subject requesting access (user, device, service account).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Subject {
    pub id: Uuid,
    pub kind: SubjectKind,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default)]
    pub group_ids: Vec<Uuid>,
    #[serde(default)]
    pub role_ids: Vec<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubjectKind {
    User,
    Device,
    ServiceAccount,
}

/// Protected resource reachable via ZTNA gateway.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Resource {
    pub id: Uuid,
    pub name: String,
    pub resource_type: ResourceType,
    pub host: String,
    pub port: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_prefix: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Http,
    Https,
    Tcp,
    Ssh,
    Rdp,
    Database,
    Custom,
}

/// Policy condition evaluated against subject, device, and context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Condition {
    TrustLevelAtLeast { level: TrustLevel },
    TrustScoreAtLeast { score: u8 },
    GroupMembership { group_id: Uuid },
    RoleAssignment { role_id: Uuid },
    GeoAllowed { countries: Vec<String> },
    TimeWindow { start_hour: u8, end_hour: u8 },
    DevicePosture { requirement: String },
    Custom { expression: String },
}

/// Action applied when conditions match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Allow,
    Deny,
    RequireMfa,
    RequireStepUp,
    LogOnly,
}

/// Security policy envelope for ZTNA (Phase 15-O basics).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ZtnaSecurityPolicy {
    pub id: Uuid,
    pub name: String,
    pub enabled: bool,
    pub min_trust_level: TrustLevel,
    pub min_trust_score: u8,
    pub conditions: Vec<Condition>,
    pub default_action: Action,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ZtnaSecurityPolicy {
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            enabled: true,
            min_trust_level: TrustLevel::Medium,
            min_trust_score: 50,
            conditions: Vec::new(),
            default_action: Action::Deny,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Identity provider kind supported by WireSentinel ZTNA.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IdentityProviderKind {
    Local,
    GenericOidc,
    OAuth2,
    SamlMock,
    LdapMock,
    AzureAd,
    GoogleWorkspace,
    Okta,
    Keycloak,
}

/// End-user identity resolved from an IdP.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct UserIdentity {
    pub id: Uuid,
    pub subject: String,
    pub email: Option<String>,
    pub display_name: String,
    pub provider: IdentityProviderKind,
    pub authenticated_at: DateTime<Utc>,
}

/// Group membership identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct GroupIdentity {
    pub id: Uuid,
    pub name: String,
    pub external_id: Option<String>,
    pub provider: IdentityProviderKind,
}

/// Role assignment identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RoleIdentity {
    pub id: Uuid,
    pub name: String,
    pub permissions: Vec<String>,
    pub provider: IdentityProviderKind,
}

/// Result of an identity authentication attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct IdentityAuthResult {
    pub success: bool,
    pub user: Option<UserIdentity>,
    pub groups: Vec<GroupIdentity>,
    pub roles: Vec<RoleIdentity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Device trust record maintained by the trust engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DeviceTrustRecord {
    pub device_id: Uuid,
    pub trust_level: TrustLevel,
    pub trust_score: u8,
    pub posture: DevicePosture,
    pub last_evaluated_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificate_fingerprint: Option<String>,
}

/// Device posture signals used in trust scoring.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DevicePosture {
    pub os_version: Option<String>,
    pub disk_encrypted: bool,
    pub firewall_enabled: bool,
    pub antivirus_running: bool,
    pub jailbroken_or_rooted: bool,
    pub compliant: bool,
}

impl Default for DevicePosture {
    fn default() -> Self {
        Self {
            os_version: None,
            disk_encrypted: false,
            firewall_enabled: false,
            antivirus_running: false,
            jailbroken_or_rooted: false,
            compliant: false,
        }
    }
}

/// Trust score snapshot emitted on updates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TrustScoreSnapshot {
    pub device_id: Uuid,
    pub score: u8,
    pub trust_level: TrustLevel,
    pub captured_at: DateTime<Utc>,
}

/// Conditional access evaluation result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ConditionalAccessResult {
    pub decision: ZtnaDecision,
    pub subject_id: Uuid,
    pub resource_id: Uuid,
    pub matched_policy_id: Option<Uuid>,
    pub reason: String,
    pub evaluated_at: DateTime<Utc>,
}

/// Gateway connection request metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct GatewayConnectionRequest {
    pub gateway_id: Uuid,
    pub subject_id: Uuid,
    pub resource_id: Uuid,
    pub client_ip: Option<String>,
}

/// Gateway connection outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct GatewayConnectionResult {
    pub allowed: bool,
    pub gateway_id: Uuid,
    pub resource_id: Uuid,
    pub subject_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listen_port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Published application/resource catalog entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PublishedResource {
    pub id: Uuid,
    pub name: String,
    pub resource: Resource,
    pub published: bool,
    pub access_policy_id: Option<Uuid>,
    pub published_at: DateTime<Utc>,
}

/// Access policy attached to a published resource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ResourceAccessPolicy {
    pub id: Uuid,
    pub resource_id: Uuid,
    pub policy: ZtnaSecurityPolicy,
    pub allowed_group_ids: Vec<Uuid>,
    pub allowed_role_ids: Vec<Uuid>,
}

/// Micro-segment definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct MicroSegment {
    pub id: Uuid,
    pub name: String,
    pub segment_type: SegmentType,
    pub member_resource_ids: Vec<Uuid>,
    pub isolation_level: IsolationLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SegmentType {
    Application,
    Network,
    Workload,
    Data,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IsolationLevel {
    Open,
    Restricted,
    Isolated,
}

/// Segment policy evaluation outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SegmentPolicyResult {
    pub segment_id: Uuid,
    pub subject_id: Uuid,
    pub allowed: bool,
    pub reason: String,
}

/// Connector health snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ConnectorHealth {
    pub connector_id: Uuid,
    pub healthy: bool,
    pub latency_ms: Option<u64>,
    pub last_check_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Outbound connector registration payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ConnectorRegistration {
    pub connector_id: Uuid,
    pub name: String,
    pub endpoint: String,
    pub resource_ids: Vec<Uuid>,
    pub registered_at: DateTime<Utc>,
}

/// Recorded ZTNA access decision for analytics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ZtnaAccessDecisionRecord {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub resource_id: Uuid,
    pub decision: ZtnaDecision,
    pub trust_score: u8,
    pub reason: String,
    pub recorded_at: DateTime<Utc>,
}

/// Aggregated ZTNA analytics snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ZtnaAnalyticsSnapshot {
    pub total_decisions: u64,
    pub allow_count: u64,
    pub deny_count: u64,
    pub challenge_count: u64,
    pub avg_trust_score: f64,
    pub captured_at: DateTime<Utc>,
}

/// Policy bundle pushed from WireSentinel-Controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ZtnaPolicyBundle {
    pub bundle_id: Uuid,
    pub tenant_id: Uuid,
    pub policies: Vec<ZtnaSecurityPolicy>,
    pub published_resources: Vec<PublishedResource>,
    pub segments: Vec<MicroSegment>,
    pub issued_at: DateTime<Utc>,
}

/// Agent heartbeat payload for ZTNA status reporting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ZtnaHeartbeatPayload {
    pub agent_id: Uuid,
    pub reported_at: DateTime<Utc>,
    pub identity_connected: bool,
    pub active_provider: Option<IdentityProviderKind>,
    pub gateway_active: bool,
    pub connector_count: u32,
    pub healthy_connectors: u32,
    pub avg_trust_score: f64,
    pub published_resource_count: u32,
    pub recent_denials: u32,
}

impl ZtnaHeartbeatPayload {
    pub fn empty(agent_id: Uuid) -> Self {
        Self {
            agent_id,
            reported_at: Utc::now(),
            identity_connected: false,
            active_provider: None,
            gateway_active: false,
            connector_count: 0,
            healthy_connectors: 0,
            avg_trust_score: 0.0,
            published_resource_count: 0,
            recent_denials: 0,
        }
    }
}
