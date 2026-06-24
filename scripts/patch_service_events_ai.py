#!/usr/bin/env python3
"""Add AI service events to shared-types events.rs."""

from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVENTS = ROOT / "shared-types/src/events.rs"
text = EVENTS.read_text()

if "CopilotQueryExecuted" in text:
    print("already patched")
    raise SystemExit(0)

IMPORT_MARKER = "    SbomDocument, SecretFinding, Vulnerability, WorkloadRecord,"
IMPORT_ADDITION = """
    AiRecommendation, AiRiskScore, CopilotResponse, CorrelatedThreat,
    ExecutiveReport, InvestigationReport,"""

text = text.replace(IMPORT_MARKER, IMPORT_MARKER + IMPORT_ADDITION, 1)

SERVICE_EVENT_VARIANTS = """
    CopilotQueryExecuted {
        response: CopilotResponse,
        timestamp: DateTime<Utc>,
    },
    InvestigationCompleted {
        report: InvestigationReport,
        timestamp: DateTime<Utc>,
    },
    ThreatCorrelated {
        threat: CorrelatedThreat,
        timestamp: DateTime<Utc>,
    },
    AiRiskScoreUpdated {
        score: AiRiskScore,
        timestamp: DateTime<Utc>,
    },
    AiRecommendationGenerated {
        recommendation: AiRecommendation,
        timestamp: DateTime<Utc>,
    },
    ExecutiveReportGenerated {
        report: ExecutiveReport,
        timestamp: DateTime<Utc>,
    },
    AiSecurityViolation {
        violation_type: String,
        detail: String,
        timestamp: DateTime<Utc>,
    },
    PromptBlocked {
        tenant_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    ProviderAccessDenied {
        tenant_id: Uuid,
        provider: String,
        timestamp: DateTime<Utc>,
    },"""

INNER_VARIANTS = """
    CopilotQueryExecuted { response: CopilotResponse },
    InvestigationCompleted { report: InvestigationReport },
    ThreatCorrelated { threat: CorrelatedThreat },
    AiRiskScoreUpdated { score: AiRiskScore },
    AiRecommendationGenerated { recommendation: AiRecommendation },
    ExecutiveReportGenerated { report: ExecutiveReport },
    AiSecurityViolation {
        violation_type: String,
        detail: String,
    },
    PromptBlocked {
        tenant_id: Uuid,
        reason: String,
    },
    ProviderAccessDenied {
        tenant_id: Uuid,
        provider: String,
    },"""

MATCH_ARMS = """
            Self::CopilotQueryExecuted { response } => {
                ServiceEvent::CopilotQueryExecuted { response, timestamp }
            }
            Self::InvestigationCompleted { report } => {
                ServiceEvent::InvestigationCompleted { report, timestamp }
            }
            Self::ThreatCorrelated { threat } => {
                ServiceEvent::ThreatCorrelated { threat, timestamp }
            }
            Self::AiRiskScoreUpdated { score } => {
                ServiceEvent::AiRiskScoreUpdated { score, timestamp }
            }
            Self::AiRecommendationGenerated { recommendation } => {
                ServiceEvent::AiRecommendationGenerated { recommendation, timestamp }
            }
            Self::ExecutiveReportGenerated { report } => {
                ServiceEvent::ExecutiveReportGenerated { report, timestamp }
            }
            Self::AiSecurityViolation {
                violation_type,
                detail,
            } => ServiceEvent::AiSecurityViolation {
                violation_type,
                detail,
                timestamp,
            },
            Self::PromptBlocked {
                tenant_id,
                reason,
            } => ServiceEvent::PromptBlocked {
                tenant_id,
                reason,
                timestamp,
            },
            Self::ProviderAccessDenied {
                tenant_id,
                provider,
            } => ServiceEvent::ProviderAccessDenied {
                tenant_id,
                provider,
                timestamp,
            },"""

text = text.replace(
    "    CnappSecurityViolation {\n        violation_type: String,\n        detail: String,\n        timestamp: DateTime<Utc>,\n    },\n}",
    "    CnappSecurityViolation {\n        violation_type: String,\n        detail: String,\n        timestamp: DateTime<Utc>,\n    },"
    + SERVICE_EVENT_VARIANTS
    + "\n}",
    1,
)

text = text.replace(
    "    CnappSecurityViolation {\n        violation_type: String,\n        detail: String,\n    },\n}",
    "    CnappSecurityViolation {\n        violation_type: String,\n        detail: String,\n    },"
    + INNER_VARIANTS
    + "\n}",
    1,
)

text = text.replace(
    "            Self::CnappSecurityViolation {\n                violation_type,\n                detail,\n            } => ServiceEvent::CnappSecurityViolation {\n                violation_type,\n                detail,\n                timestamp,\n            },\n        }\n    }\n}",
    "            Self::CnappSecurityViolation {\n                violation_type,\n                detail,\n            } => ServiceEvent::CnappSecurityViolation {\n                violation_type,\n                detail,\n                timestamp,\n            },"
    + MATCH_ARMS
    + "\n        }\n    }\n}",
    1,
)

EVENTS.write_text(text)
print("patched")
