//! Phase 18 CNAPP (Cloud-Native Application Protection Platform) shared DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Cloud provider identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CloudProvider {
    Aws,
    Azure,
    Gcp,
    Oracle,
    Alibaba,
    Other,
}

/// CNAPP finding severity tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CnappSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Cloud posture finding from CSPM scans.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PostureFinding {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub provider: CloudProvider,
    pub resource_id: String,
    pub control_id: String,
    pub title: String,
    pub severity: CnappSeverity,
    pub description: Option<String>,
    pub detected_at: DateTime<Utc>,
}

/// Cloud resource inventory record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CloudResource {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub provider: CloudProvider,
    pub resource_type: String,
    pub resource_id: String,
    pub region: String,
    pub tags: serde_json::Value,
    pub observed_at: DateTime<Utc>,
}

/// Workload protection record for CWPP.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct WorkloadRecord {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub provider: CloudProvider,
    pub workload_kind: String,
    pub name: String,
    pub image: Option<String>,
    pub namespace: Option<String>,
    pub observed_at: DateTime<Utc>,
}

/// Kubernetes security finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct KubernetesFinding {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub cluster_id: String,
    pub namespace: String,
    pub resource_kind: String,
    pub resource_name: String,
    pub finding_kind: String,
    pub severity: CnappSeverity,
    pub detected_at: DateTime<Utc>,
}

/// Container runtime security finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ContainerFinding {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub container_id: String,
    pub image: String,
    pub finding_kind: String,
    pub severity: CnappSeverity,
    pub detected_at: DateTime<Utc>,
}

/// Infrastructure-as-Code security finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct IacFinding {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub file_path: String,
    pub iac_kind: String,
    pub rule_id: String,
    pub severity: CnappSeverity,
    pub message: String,
    pub detected_at: DateTime<Utc>,
}

/// Secret exposure finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SecretFinding {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub location: String,
    pub secret_kind: String,
    pub severity: CnappSeverity,
    pub redacted_preview: String,
    pub detected_at: DateTime<Utc>,
}

/// Supply-chain dependency record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DependencyRecord {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub package_name: String,
    pub version: String,
    pub ecosystem: String,
    pub direct: bool,
    pub observed_at: DateTime<Utc>,
}

/// SBOM document metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SbomDocument {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub format: String,
    pub artifact_name: String,
    pub artifact_version: String,
    pub component_count: u32,
    pub generated_at: DateTime<Utc>,
}

/// Vulnerability record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Vulnerability {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub cve_id: String,
    pub severity: CnappSeverity,
    pub cvss_score: f64,
    pub package_name: String,
    pub package_version: String,
    pub fixed_version: Option<String>,
    pub detected_at: DateTime<Utc>,
}

/// Asset affected by a vulnerability or attack path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AffectedAsset {
    pub id: Uuid,
    pub asset_kind: String,
    pub identifier: String,
    pub provider: CloudProvider,
}

/// Remediation plan for a finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RemediationPlan {
    pub id: Uuid,
    pub finding_id: Uuid,
    pub title: String,
    pub steps: Vec<String>,
    pub automated: bool,
    pub created_at: DateTime<Utc>,
}

/// Cloud attack path summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CloudAttackPath {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub source_resource: String,
    pub target_resource: String,
    pub risk_score: f64,
    pub node_ids: Vec<Uuid>,
    pub edge_ids: Vec<Uuid>,
    pub discovered_at: DateTime<Utc>,
}

/// Compliance control definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ComplianceControl {
    pub id: Uuid,
    pub framework: String,
    pub control_id: String,
    pub title: String,
    pub description: Option<String>,
}

/// Compliance score for a tenant/framework.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ComplianceScore {
    pub tenant_id: Uuid,
    pub framework: String,
    pub score_pct: f64,
    pub passing_controls: u32,
    pub failing_controls: u32,
    pub computed_at: DateTime<Utc>,
}

/// CNAPP security policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CnappSecurityPolicy {
    pub tenant_id: Uuid,
    pub max_iac_findings_per_scan: u32,
    pub max_secrets_per_repo: u32,
    pub allowed_sbom_formats: Vec<String>,
    pub required_compliance_frameworks: Vec<String>,
    pub block_critical_vulnerabilities: bool,
}

impl Default for CnappSecurityPolicy {
    fn default() -> Self {
        Self {
            tenant_id: Uuid::nil(),
            max_iac_findings_per_scan: 500,
            max_secrets_per_repo: 100,
            allowed_sbom_formats: vec!["cyclonedx".into(), "spdx".into()],
            required_compliance_frameworks: vec!["cis".into()],
            block_critical_vulnerabilities: true,
        }
    }
}

/// CNAPP security violation summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CnappSecurityViolationDetail {
    pub violation_type: String,
    pub detail: String,
    pub resource: String,
}

/// Agent telemetry payload for WireSentinel-Controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CnappTelemetryPayload {
    pub agent_id: Uuid,
    pub tenant_id: Uuid,
    pub reported_at: DateTime<Utc>,
    pub posture_findings: u32,
    pub workload_records: u32,
    pub kubernetes_findings: u32,
    pub container_findings: u32,
    pub iac_findings: u32,
    pub secret_findings: u32,
    pub vulnerabilities: u32,
    pub compliance_score_pct: f64,
}

impl CnappTelemetryPayload {
    pub fn empty(agent_id: Uuid, tenant_id: Uuid) -> Self {
        Self {
            agent_id,
            tenant_id,
            reported_at: Utc::now(),
            posture_findings: 0,
            workload_records: 0,
            kubernetes_findings: 0,
            container_findings: 0,
            iac_findings: 0,
            secret_findings: 0,
            vulnerabilities: 0,
            compliance_score_pct: 100.0,
        }
    }
}

/// Scan bundle pushed from WireSentinel-Controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CnappScanBundle {
    pub bundle_id: Uuid,
    pub tenant_id: Uuid,
    pub security_policy: Option<CnappSecurityPolicy>,
    pub resources: Vec<CloudResource>,
    pub workloads: Vec<WorkloadRecord>,
    pub issued_at: DateTime<Utc>,
}

/// Multi-cloud analytics summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CnappAnalyticsSummary {
    pub tenant_id: Uuid,
    pub total_posture_findings: u64,
    pub critical_findings: u64,
    pub open_vulnerabilities: u64,
    pub exposed_secrets: u64,
    pub compliance_score_pct: f64,
    pub attack_paths_discovered: u64,
    pub cloud_risk_score: f64,
    pub computed_at: DateTime<Utc>,
}
