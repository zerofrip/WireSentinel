use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ── Tenant & organization ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TenantStatus {
    Active,
    Suspended,
    Pending,
    Deprovisioned,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub status: TenantStatus,
    pub organization_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub billing_email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionPlan {
    Free,
    Pro,
    Enterprise,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct Subscription {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub plan: SubscriptionPlan,
    pub seats: u32,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema, Default)]
pub struct TenantConfig {
    pub tenant_id: Uuid,
    pub settings: serde_json::Value,
    pub feature_flags: serde_json::Value,
    pub updated_at: DateTime<Utc>,
}

// ── Teams & RBAC ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TeamRole {
    Owner,
    Administrator,
    Operator,
    Viewer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct Team {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct TeamMembership {
    pub id: Uuid,
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub role: TeamRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── Federated controllers ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FederationStatus {
    Registered,
    Connected,
    Syncing,
    Disconnected,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct FederatedController {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub url: String,
    pub device_id: Option<String>,
    pub status: FederationStatus,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── Cloud sync ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum SyncMode {
    Push,
    #[default]
    Pull,
    Bidirectional,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct SyncConflict {
    pub id: Uuid,
    pub entity_type: String,
    pub entity_id: String,
    pub local_version: serde_json::Value,
    pub remote_version: serde_json::Value,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    KeepLocal,
    KeepRemote,
    Merge,
    Manual,
}

// ── Compliance ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceStatus {
    Passed,
    Failed,
    Warning,
    Pending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceCheckKind {
    PolicyEnforcement,
    EncryptionAtRest,
    AuditLogRetention,
    AccessControl,
    DataResidency,
    KillSwitch,
    DnsLeakProtection,
    PluginSandbox,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct ComplianceReport {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub device_id: Option<String>,
    pub status: ComplianceStatus,
    pub checks: Vec<ComplianceCheckResult>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct ComplianceCheckResult {
    pub kind: ComplianceCheckKind,
    pub status: ComplianceStatus,
    pub detail: Option<String>,
}

// ── Cloud plans & quotas ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CloudPlan {
    Starter,
    Business,
    Enterprise,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct Quota {
    pub name: String,
    pub limit: u64,
    pub used: u64,
    pub unit: String,
}

// ── Identity (OIDC) ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct IdentityClaims {
    /// OIDC `sub` claim.
    pub subject: String,
    pub email: Option<String>,
    pub roles: Vec<String>,
    pub tenant_id: Option<Uuid>,
    pub issued_at: Option<DateTime<Utc>>,
}
