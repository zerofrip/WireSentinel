//! Phase 17 XDR (Extended Detection and Response) shared DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Incident severity tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum XdrSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Incident lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IncidentStatus {
    Open,
    Investigating,
    Contained,
    Resolved,
    Closed,
}

/// Threat hunt status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HuntStatus {
    Draft,
    Running,
    Completed,
    Failed,
}

/// Hunt query kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HuntQueryKind {
    Historical,
    Ioc,
    Behavioral,
    Correlation,
}

/// Detection rule kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DetectionRuleKind {
    SigmaInspired,
    Behavioral,
    Correlation,
    Scheduled,
}

/// SOAR playbook kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlaybookKind {
    BlockHost,
    BlockDomain,
    DisableIdentity,
    QuarantineDevice,
    EscalateIncident,
    NotifyTeam,
}

/// Playbook execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlaybookStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Response action kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResponseActionKind {
    KillProcess,
    BlockHash,
    BlockDomain,
    BlockIp,
    DisableUser,
    QuarantineDevice,
    DisconnectVpn,
    ForceReauthentication,
}

/// Response action outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResponseActionStatus {
    Pending,
    Executed,
    Failed,
}

/// Case workflow state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CaseWorkflowState {
    New,
    Assigned,
    InReview,
    Closed,
}

/// Attack graph node kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AttackGraphNodeKind {
    User,
    Device,
    Service,
    Identity,
    Connector,
    Resource,
}

/// Attack graph edge kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AttackGraphEdgeKind {
    Access,
    Trust,
    Authentication,
    NetworkReachability,
}

/// Identity threat kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IdentityThreatKind {
    ImpossibleTravel,
    CredentialAbuse,
    TokenTheft,
    MfaBypass,
    PrivilegeEscalation,
    ExcessiveAuthFailures,
}

/// EDR process event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ProcessEvent {
    pub id: Uuid,
    pub device_id: Uuid,
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub process_name: String,
    pub command_line: Option<String>,
    pub user: Option<String>,
    pub observed_at: DateTime<Utc>,
}

/// EDR file event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct FileEvent {
    pub id: Uuid,
    pub device_id: Uuid,
    pub path: String,
    pub operation: String,
    pub hash_sha256: Option<String>,
    pub observed_at: DateTime<Utc>,
}

/// EDR registry event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RegistryEvent {
    pub id: Uuid,
    pub device_id: Uuid,
    pub key_path: String,
    pub value_name: Option<String>,
    pub operation: String,
    pub observed_at: DateTime<Utc>,
}

/// EDR Windows service event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct EdrServiceEvent {
    pub id: Uuid,
    pub device_id: Uuid,
    pub service_name: String,
    pub operation: String,
    pub observed_at: DateTime<Utc>,
}

/// EDR driver event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DriverEvent {
    pub id: Uuid,
    pub device_id: Uuid,
    pub driver_name: String,
    pub operation: String,
    pub observed_at: DateTime<Utc>,
}

/// Process anomaly summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ProcessAnomaly {
    pub id: Uuid,
    pub device_id: Uuid,
    pub process_event_id: Uuid,
    pub anomaly_kind: String,
    pub severity: XdrSeverity,
    pub description: String,
    pub detected_at: DateTime<Utc>,
}

/// Persistence detection summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PersistenceFinding {
    pub id: Uuid,
    pub device_id: Uuid,
    pub persistence_kind: String,
    pub target: String,
    pub severity: XdrSeverity,
    pub detected_at: DateTime<Utc>,
}

/// Malicious execution finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct MaliciousExecution {
    pub id: Uuid,
    pub device_id: Uuid,
    pub process_event_id: Option<Uuid>,
    pub file_event_id: Option<Uuid>,
    pub indicator: String,
    pub severity: XdrSeverity,
    pub detected_at: DateTime<Utc>,
}

/// Network threat summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct NetworkThreat {
    pub id: Uuid,
    pub device_id: Uuid,
    pub threat_kind: String,
    pub source_ip: Option<String>,
    pub dest_ip: Option<String>,
    pub dest_port: Option<u16>,
    pub protocol: Option<String>,
    pub severity: XdrSeverity,
    pub detected_at: DateTime<Utc>,
}

/// Beaconing detection summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct BeaconingFinding {
    pub id: Uuid,
    pub device_id: Uuid,
    pub dest_ip: String,
    pub dest_port: u16,
    pub interval_secs: f64,
    pub connection_count: u32,
    pub detected_at: DateTime<Utc>,
}

/// Lateral movement finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct LateralMovementFinding {
    pub id: Uuid,
    pub device_id: Uuid,
    pub source_host: String,
    pub target_host: String,
    pub protocol: String,
    pub severity: XdrSeverity,
    pub detected_at: DateTime<Utc>,
}

/// Identity threat record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct IdentityThreat {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: String,
    pub threat_kind: IdentityThreatKind,
    pub severity: XdrSeverity,
    pub description: String,
    pub source_ip: Option<String>,
    pub geo_location: Option<String>,
    pub detected_at: DateTime<Utc>,
}

/// Identity risk record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct IdentityRiskRecord {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: String,
    pub risk_score: u8,
    pub factors: Vec<String>,
    pub evaluated_at: DateTime<Utc>,
}

/// Threat hunt definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Hunt {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub query_kind: HuntQueryKind,
    pub query: String,
    pub status: HuntStatus,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Hunt result entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct HuntResult {
    pub id: Uuid,
    pub hunt_id: Uuid,
    pub event_kind: String,
    pub summary: String,
    pub matched_at: DateTime<Utc>,
}

/// Hunt timeline entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct HuntTimelineEntry {
    pub timestamp: DateTime<Utc>,
    pub event_kind: String,
    pub summary: String,
    pub device_id: Option<Uuid>,
}

/// Hunt timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct HuntTimeline {
    pub hunt_id: Uuid,
    pub entries: Vec<HuntTimelineEntry>,
    pub generated_at: DateTime<Utc>,
}

/// Detection rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DetectionRule {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub rule_kind: DetectionRuleKind,
    pub enabled: bool,
    pub conditions: serde_json::Value,
    pub severity: XdrSeverity,
    pub mitre_technique_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Detection match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DetectionMatch {
    pub id: Uuid,
    pub rule_id: Uuid,
    pub device_id: Option<Uuid>,
    pub user_id: Option<String>,
    pub summary: String,
    pub matched_at: DateTime<Utc>,
}

/// Detection trigger summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DetectionTrigger {
    pub id: Uuid,
    pub rule_id: Uuid,
    pub match_id: Uuid,
    pub severity: XdrSeverity,
    pub title: String,
    pub triggered_at: DateTime<Utc>,
}

/// Security incident.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Incident {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub severity: XdrSeverity,
    pub status: IncidentStatus,
    pub detection_id: Option<Uuid>,
    pub assigned_to: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

/// Incident artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct IncidentArtifact {
    pub id: Uuid,
    pub incident_id: Uuid,
    pub artifact_kind: String,
    pub content: String,
    pub collected_at: DateTime<Utc>,
}

/// Incident timeline entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct IncidentTimelineEntry {
    pub id: Uuid,
    pub incident_id: Uuid,
    pub entry_kind: String,
    pub summary: String,
    pub actor: Option<String>,
    pub recorded_at: DateTime<Utc>,
}

/// Investigation case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Case {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub title: String,
    pub linked_incident_id: Option<Uuid>,
    pub investigator: Option<String>,
    pub workflow_state: CaseWorkflowState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Case comment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CaseComment {
    pub id: Uuid,
    pub case_id: Uuid,
    pub author: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

/// Case evidence item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CaseEvidence {
    pub id: Uuid,
    pub case_id: Uuid,
    pub evidence_kind: String,
    pub description: String,
    pub uri: Option<String>,
    pub collected_at: DateTime<Utc>,
}

/// SOAR playbook definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Playbook {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub playbook_kind: PlaybookKind,
    pub enabled: bool,
    pub steps: Vec<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Playbook execution record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PlaybookExecution {
    pub id: Uuid,
    pub playbook_id: Uuid,
    pub incident_id: Option<Uuid>,
    pub status: PlaybookStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// Attack graph node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AttackGraphNode {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub node_kind: AttackGraphNodeKind,
    pub label: String,
    pub metadata: serde_json::Value,
}

/// Attack graph edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AttackGraphEdge {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub edge_kind: AttackGraphEdgeKind,
    pub weight: f64,
}

/// Attack path result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AttackPath {
    pub nodes: Vec<Uuid>,
    pub edges: Vec<Uuid>,
    pub risk_score: f64,
}

/// MITRE ATT&CK technique.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct MitreTechnique {
    pub technique_id: String,
    pub name: String,
    pub tactic: String,
    pub description: String,
}

/// MITRE detection mapping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct MitreDetectionMapping {
    pub id: Uuid,
    pub detection_kind: String,
    pub technique_id: String,
    pub rule_id: Option<Uuid>,
}

/// Technique detection summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TechniqueDetection {
    pub id: Uuid,
    pub technique_id: String,
    pub technique_name: String,
    pub tactic: String,
    pub source_detection: String,
    pub severity: XdrSeverity,
    pub detected_at: DateTime<Utc>,
}

/// Response action request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ResponseActionRequest {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub action_kind: ResponseActionKind,
    pub target: String,
    pub initiated_by: String,
    pub incident_id: Option<Uuid>,
    pub requested_at: DateTime<Utc>,
}

/// Response action result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ResponseActionResult {
    pub request_id: Uuid,
    pub status: ResponseActionStatus,
    pub detail: String,
    pub executed_at: DateTime<Utc>,
}

/// XDR security policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct XdrSecurityPolicy {
    pub tenant_id: Uuid,
    pub allow_dangerous_hunt_queries: bool,
    pub allowed_response_actions: Vec<ResponseActionKind>,
    pub allowed_playbook_kinds: Vec<PlaybookKind>,
    pub max_detection_rule_conditions: u32,
    pub require_mfa_for_response: bool,
}

impl Default for XdrSecurityPolicy {
    fn default() -> Self {
        Self {
            tenant_id: Uuid::nil(),
            allow_dangerous_hunt_queries: false,
            allowed_response_actions: vec![
                ResponseActionKind::BlockDomain,
                ResponseActionKind::BlockIp,
                ResponseActionKind::QuarantineDevice,
            ],
            allowed_playbook_kinds: vec![PlaybookKind::NotifyTeam, PlaybookKind::EscalateIncident],
            max_detection_rule_conditions: 50,
            require_mfa_for_response: true,
        }
    }
}

/// XDR security violation summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct XdrSecurityViolationDetail {
    pub violation_type: String,
    pub detail: String,
    pub resource: String,
}

/// Agent telemetry payload for WireSentinel-Controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct XdrTelemetryPayload {
    pub agent_id: Uuid,
    pub device_id: Uuid,
    pub reported_at: DateTime<Utc>,
    pub process_events: u32,
    pub file_events: u32,
    pub network_events: u32,
    pub identity_threats: u32,
    pub active_incidents: u32,
    pub detection_matches: u32,
}

impl XdrTelemetryPayload {
    pub fn empty(agent_id: Uuid, device_id: Uuid) -> Self {
        Self {
            agent_id,
            device_id,
            reported_at: Utc::now(),
            process_events: 0,
            file_events: 0,
            network_events: 0,
            identity_threats: 0,
            active_incidents: 0,
            detection_matches: 0,
        }
    }
}

/// Policy bundle pushed from WireSentinel-Controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct XdrPolicyBundle {
    pub bundle_id: Uuid,
    pub tenant_id: Uuid,
    pub security_policy: Option<XdrSecurityPolicy>,
    pub detection_rules: Vec<DetectionRule>,
    pub playbooks: Vec<Playbook>,
    pub issued_at: DateTime<Utc>,
}

/// Incident bundle pushed to WireSentinel-Controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct XdrIncidentBundle {
    pub bundle_id: Uuid,
    pub tenant_id: Uuid,
    pub incidents: Vec<Incident>,
    pub detections: Vec<DetectionTrigger>,
    pub identity_threats: Vec<IdentityThreat>,
    pub network_threats: Vec<NetworkThreat>,
    pub issued_at: DateTime<Utc>,
}

/// Fleet XDR analytics summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct XdrAnalyticsSummary {
    pub tenant_id: Uuid,
    pub total_incidents: u64,
    pub open_incidents: u64,
    pub critical_incidents: u64,
    pub total_detections: u64,
    pub mitre_techniques_detected: u32,
    pub mitre_coverage_pct: f64,
    pub avg_incident_mttr_hours: f64,
    pub fleet_threat_score: f64,
    pub computed_at: DateTime<Utc>,
}
