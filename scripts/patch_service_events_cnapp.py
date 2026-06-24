#!/usr/bin/env python3
"""Add CNAPP service events to shared-types events.rs."""

from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVENTS = ROOT / "shared-types/src/events.rs"
text = EVENTS.read_text()

if "CloudMisconfigurationDetected" in text:
    print("already patched")
    raise SystemExit(0)

IMPORT_MARKER = "    TrafficEvent, TrafficRoute, TrustScoreSnapshot, UserIdentity, WebAccessResult,"
IMPORT_ADDITION = """
    AffectedAsset, CloudAttackPath, CloudResource, CnappSeverity, ComplianceControl, ComplianceScore,
    ContainerFinding, DependencyRecord, IacFinding, KubernetesFinding, PostureFinding, RemediationPlan,
    SbomDocument, SecretFinding, Vulnerability, WorkloadRecord,"""

text = text.replace(IMPORT_MARKER, IMPORT_MARKER + IMPORT_ADDITION, 1)

SERVICE_EVENT_VARIANTS = """
    CloudMisconfigurationDetected {
        finding: PostureFinding,
        timestamp: DateTime<Utc>,
    },
    CloudRiskIncreased {
        tenant_id: Uuid,
        previous_score: f64,
        current_score: f64,
        timestamp: DateTime<Utc>,
    },
    CloudPolicyViolation {
        finding: PostureFinding,
        timestamp: DateTime<Utc>,
    },
    WorkloadThreatDetected {
        workload: WorkloadRecord,
        severity: CnappSeverity,
        timestamp: DateTime<Utc>,
    },
    WorkloadCompromised {
        workload: WorkloadRecord,
        timestamp: DateTime<Utc>,
    },
    KubernetesRiskDetected {
        finding: KubernetesFinding,
        timestamp: DateTime<Utc>,
    },
    ClusterCompromiseSuspected {
        finding: KubernetesFinding,
        timestamp: DateTime<Utc>,
    },
    ContainerRiskDetected {
        finding: ContainerFinding,
        timestamp: DateTime<Utc>,
    },
    ContainerThreatDetected {
        finding: ContainerFinding,
        timestamp: DateTime<Utc>,
    },
    IacFindingDetected {
        finding: IacFinding,
        timestamp: DateTime<Utc>,
    },
    SecretExposed {
        finding: SecretFinding,
        timestamp: DateTime<Utc>,
    },
    DependencyRiskDetected {
        dependency: DependencyRecord,
        severity: CnappSeverity,
        timestamp: DateTime<Utc>,
    },
    SupplyChainThreatDetected {
        dependency: DependencyRecord,
        timestamp: DateTime<Utc>,
    },
    SbomGenerated {
        document: SbomDocument,
        timestamp: DateTime<Utc>,
    },
    SbomImported {
        document: SbomDocument,
        timestamp: DateTime<Utc>,
    },
    CriticalVulnerabilityDetected {
        vulnerability: Vulnerability,
        asset: AffectedAsset,
        timestamp: DateTime<Utc>,
    },
    AttackPathDiscovered {
        path: CloudAttackPath,
        timestamp: DateTime<Utc>,
    },
    ComplianceViolation {
        control: ComplianceControl,
        severity: CnappSeverity,
        timestamp: DateTime<Utc>,
    },
    ComplianceScoreUpdated {
        score: ComplianceScore,
        timestamp: DateTime<Utc>,
    },
    CnappSecurityViolation {
        violation_type: String,
        detail: String,
        timestamp: DateTime<Utc>,
    },"""

INNER_VARIANTS = """
    CloudMisconfigurationDetected { finding: PostureFinding },
    CloudRiskIncreased {
        tenant_id: Uuid,
        previous_score: f64,
        current_score: f64,
    },
    CloudPolicyViolation { finding: PostureFinding },
    WorkloadThreatDetected {
        workload: WorkloadRecord,
        severity: CnappSeverity,
    },
    WorkloadCompromised { workload: WorkloadRecord },
    KubernetesRiskDetected { finding: KubernetesFinding },
    ClusterCompromiseSuspected { finding: KubernetesFinding },
    ContainerRiskDetected { finding: ContainerFinding },
    ContainerThreatDetected { finding: ContainerFinding },
    IacFindingDetected { finding: IacFinding },
    SecretExposed { finding: SecretFinding },
    DependencyRiskDetected {
        dependency: DependencyRecord,
        severity: CnappSeverity,
    },
    SupplyChainThreatDetected { dependency: DependencyRecord },
    SbomGenerated { document: SbomDocument },
    SbomImported { document: SbomDocument },
    CriticalVulnerabilityDetected {
        vulnerability: Vulnerability,
        asset: AffectedAsset,
    },
    AttackPathDiscovered { path: CloudAttackPath },
    ComplianceViolation {
        control: ComplianceControl,
        severity: CnappSeverity,
    },
    ComplianceScoreUpdated { score: ComplianceScore },
    CnappSecurityViolation {
        violation_type: String,
        detail: String,
    },"""

MATCH_ARMS = """
            Self::CloudMisconfigurationDetected { finding } => {
                ServiceEvent::CloudMisconfigurationDetected { finding, timestamp }
            }
            Self::CloudRiskIncreased {
                tenant_id,
                previous_score,
                current_score,
            } => ServiceEvent::CloudRiskIncreased {
                tenant_id,
                previous_score,
                current_score,
                timestamp,
            },
            Self::CloudPolicyViolation { finding } => {
                ServiceEvent::CloudPolicyViolation { finding, timestamp }
            }
            Self::WorkloadThreatDetected { workload, severity } => {
                ServiceEvent::WorkloadThreatDetected {
                    workload,
                    severity,
                    timestamp,
                }
            }
            Self::WorkloadCompromised { workload } => {
                ServiceEvent::WorkloadCompromised { workload, timestamp }
            }
            Self::KubernetesRiskDetected { finding } => {
                ServiceEvent::KubernetesRiskDetected { finding, timestamp }
            }
            Self::ClusterCompromiseSuspected { finding } => {
                ServiceEvent::ClusterCompromiseSuspected { finding, timestamp }
            }
            Self::ContainerRiskDetected { finding } => {
                ServiceEvent::ContainerRiskDetected { finding, timestamp }
            }
            Self::ContainerThreatDetected { finding } => {
                ServiceEvent::ContainerThreatDetected { finding, timestamp }
            }
            Self::IacFindingDetected { finding } => {
                ServiceEvent::IacFindingDetected { finding, timestamp }
            }
            Self::SecretExposed { finding } => {
                ServiceEvent::SecretExposed { finding, timestamp }
            }
            Self::DependencyRiskDetected { dependency, severity } => {
                ServiceEvent::DependencyRiskDetected {
                    dependency,
                    severity,
                    timestamp,
                }
            }
            Self::SupplyChainThreatDetected { dependency } => {
                ServiceEvent::SupplyChainThreatDetected { dependency, timestamp }
            }
            Self::SbomGenerated { document } => {
                ServiceEvent::SbomGenerated { document, timestamp }
            }
            Self::SbomImported { document } => {
                ServiceEvent::SbomImported { document, timestamp }
            }
            Self::CriticalVulnerabilityDetected { vulnerability, asset } => {
                ServiceEvent::CriticalVulnerabilityDetected {
                    vulnerability,
                    asset,
                    timestamp,
                }
            }
            Self::AttackPathDiscovered { path } => {
                ServiceEvent::AttackPathDiscovered { path, timestamp }
            }
            Self::ComplianceViolation { control, severity } => {
                ServiceEvent::ComplianceViolation {
                    control,
                    severity,
                    timestamp,
                }
            }
            Self::ComplianceScoreUpdated { score } => {
                ServiceEvent::ComplianceScoreUpdated { score, timestamp }
            }
            Self::CnappSecurityViolation {
                violation_type,
                detail,
            } => ServiceEvent::CnappSecurityViolation {
                violation_type,
                detail,
                timestamp,
            },"""

text = text.replace(
    "    XdrSecurityViolation {\n        violation_type: String,\n        detail: String,\n        timestamp: DateTime<Utc>,\n    },\n}",
    "    XdrSecurityViolation {\n        violation_type: String,\n        detail: String,\n        timestamp: DateTime<Utc>,\n    },"
    + SERVICE_EVENT_VARIANTS
    + "\n}",
    1,
)

text = text.replace(
    "    XdrSecurityViolation {\n        violation_type: String,\n        detail: String,\n    },\n}",
    "    XdrSecurityViolation {\n        violation_type: String,\n        detail: String,\n    },"
    + INNER_VARIANTS
    + "\n}",
    1,
)

text = text.replace(
    "            Self::XdrSecurityViolation {\n                violation_type,\n                detail,\n            } => ServiceEvent::XdrSecurityViolation {\n                violation_type,\n                detail,\n                timestamp,\n            },\n        }\n    }\n}",
    "            Self::XdrSecurityViolation {\n                violation_type,\n                detail,\n            } => ServiceEvent::XdrSecurityViolation {\n                violation_type,\n                detail,\n                timestamp,\n            },"
    + MATCH_ARMS
    + "\n        }\n    }\n}",
    1,
)

EVENTS.write_text(text)
print("patched")
