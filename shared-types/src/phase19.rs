//! Phase 19 AI (Artificial Intelligence) shared DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::PlaybookKind;

/// LLM / embedding provider identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiProvider {
    OpenAi,
    Anthropic,
    AzureOpenAi,
    Local,
    Mock,
}

/// Copilot query intent classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiIntentKind {
    Investigate,
    Detect,
    Policy,
    ThreatIntel,
    Playbook,
    Report,
    General,
    Unknown,
}

/// AI recommendation category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiRecommendationKind {
    Detection,
    Policy,
    Playbook,
    Remediation,
    Risk,
    General,
}

/// Executive report output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiReportFormat {
    Json,
    Markdown,
    Pdf,
}

/// AI finding severity tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Tenant AI security policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AiSecurityPolicy {
    pub tenant_id: Uuid,
    pub max_prompt_length: u32,
    pub max_queries_per_hour: u32,
    pub allowed_providers: Vec<AiProvider>,
    pub blocked_prompt_patterns: Vec<String>,
    pub require_human_review: bool,
}

impl Default for AiSecurityPolicy {
    fn default() -> Self {
        Self {
            tenant_id: Uuid::nil(),
            max_prompt_length: 8_000,
            max_queries_per_hour: 500,
            allowed_providers: vec![AiProvider::Mock, AiProvider::Local],
            blocked_prompt_patterns: vec!["ignore previous instructions".into()],
            require_human_review: false,
        }
    }
}

/// AI security violation summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AiSecurityViolationDetail {
    pub violation_type: String,
    pub detail: String,
    pub resource: String,
}

/// Security context entry aggregated for LLM prompts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SecurityContextEntry {
    pub key: String,
    pub value: String,
    pub source: String,
    pub observed_at: DateTime<Utc>,
}

/// Copilot natural-language query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CopilotQuery {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub prompt: String,
    pub context_ids: Vec<Uuid>,
    pub submitted_at: DateTime<Utc>,
}

/// Copilot response payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CopilotResponse {
    pub query_id: Uuid,
    pub intent: AiIntentKind,
    pub answer: String,
    pub confidence: f64,
    pub provider: AiProvider,
    pub generated_at: DateTime<Utc>,
}

/// Investigation report produced by AI analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct InvestigationReport {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub incident_id: Option<Uuid>,
    pub title: String,
    pub summary: String,
    pub severity: AiSeverity,
    pub narrative: AttackNarrative,
    pub root_cause: RootCauseAnalysis,
    pub recommendations: Vec<AiRecommendation>,
    pub generated_at: DateTime<Utc>,
}

/// Attack narrative timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AttackNarrative {
    pub stages: Vec<String>,
    pub techniques: Vec<String>,
    pub affected_assets: Vec<String>,
}

/// Root cause analysis section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RootCauseAnalysis {
    pub primary_cause: String,
    pub contributing_factors: Vec<String>,
    pub confidence: f64,
}

/// Correlated multi-source threat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CorrelatedThreat {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub title: String,
    pub severity: AiSeverity,
    pub source_events: Vec<Uuid>,
    pub correlation_score: f64,
    pub detected_at: DateTime<Utc>,
}

/// Knowledge graph node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct KnowledgeGraphNode {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub label: String,
    pub node_kind: String,
    pub properties: serde_json::Value,
}

/// Knowledge graph edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct KnowledgeGraphEdge {
    pub id: Uuid,
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub relation: String,
    pub weight: f64,
}

/// RAG document metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RagDocument {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub title: String,
    pub source: String,
    pub indexed_at: DateTime<Utc>,
}

/// RAG text chunk with embedding reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RagChunk {
    pub id: Uuid,
    pub document_id: Uuid,
    pub content: String,
    pub embedding_id: Option<Uuid>,
    pub chunk_index: u32,
}

/// RAG retrieval result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RagRetrievalResult {
    pub chunk: RagChunk,
    pub score: f64,
}

/// AI-generated detection rule suggestion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DetectionSuggestion {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub title: String,
    pub rule_body: String,
    pub severity: AiSeverity,
    pub confidence: f64,
    pub rationale: String,
    pub generated_at: DateTime<Utc>,
}

/// AI-generated policy suggestion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PolicySuggestion {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub title: String,
    pub policy_body: String,
    pub severity: AiSeverity,
    pub confidence: f64,
    pub rationale: String,
    pub generated_at: DateTime<Utc>,
}

/// XDR-compatible playbook step suggestion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PlaybookStepSuggestion {
    pub step_order: u32,
    pub kind: PlaybookKind,
    pub description: String,
    pub automated: bool,
}

/// AI-generated playbook suggestion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PlaybookSuggestion {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub title: String,
    pub steps: Vec<PlaybookStepSuggestion>,
    pub confidence: f64,
    pub generated_at: DateTime<Utc>,
}

/// Threat intelligence report from AI enrichment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ThreatIntelligenceReport {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub indicator: String,
    pub indicator_kind: String,
    pub summary: String,
    pub severity: AiSeverity,
    pub sources: Vec<String>,
    pub generated_at: DateTime<Utc>,
}

/// Generic AI recommendation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AiRecommendation {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub kind: AiRecommendationKind,
    pub title: String,
    pub body: String,
    pub confidence: f64,
    pub generated_at: DateTime<Utc>,
}

/// Executive summary report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ExecutiveReport {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub title: String,
    pub format: AiReportFormat,
    pub content: String,
    pub risk_score: f64,
    pub generated_at: DateTime<Utc>,
}

/// AI-computed tenant risk score (distinct from SSE RiskScore).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AiRiskScore {
    pub tenant_id: Uuid,
    pub score: f64,
    pub previous_score: f64,
    pub factors: Vec<String>,
    pub computed_at: DateTime<Utc>,
}

/// Embedding vector record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct EmbeddingRecord {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub model: String,
    pub dimensions: u32,
    pub vector: Vec<f32>,
    pub created_at: DateTime<Utc>,
}

/// Agent telemetry payload for WireSentinel-Controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AiTelemetryPayload {
    pub agent_id: Uuid,
    pub tenant_id: Uuid,
    pub reported_at: DateTime<Utc>,
    pub copilot_queries: u32,
    pub investigations: u32,
    pub correlations: u32,
    pub rag_documents: u32,
    pub recommendations: u32,
    pub ai_risk_score: f64,
}

impl AiTelemetryPayload {
    pub fn empty(agent_id: Uuid, tenant_id: Uuid) -> Self {
        Self {
            agent_id,
            tenant_id,
            reported_at: Utc::now(),
            copilot_queries: 0,
            investigations: 0,
            correlations: 0,
            rag_documents: 0,
            recommendations: 0,
            ai_risk_score: 0.0,
        }
    }
}

/// Context bundle pushed from WireSentinel-Controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AiContextBundle {
    pub bundle_id: Uuid,
    pub tenant_id: Uuid,
    pub security_policy: Option<AiSecurityPolicy>,
    pub context_entries: Vec<SecurityContextEntry>,
    pub issued_at: DateTime<Utc>,
}

/// Multi-domain AI analytics summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AiAnalyticsSummary {
    pub tenant_id: Uuid,
    pub total_queries: u64,
    pub investigations_completed: u64,
    pub threats_correlated: u64,
    pub recommendations_generated: u64,
    pub average_risk_score: f64,
    pub computed_at: DateTime<Utc>,
}
