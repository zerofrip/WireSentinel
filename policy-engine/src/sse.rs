//! Optional SSE policy gate lookup.

use crate::engine::ConnectionContext;
use uuid::Uuid;

/// Result of an SSE pre-policy gate check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseGateResult {
    pub allowed: bool,
    pub reason: String,
    pub matched_policy_id: Option<Uuid>,
}

/// Evaluates SSE access before the main ruleset (additive gate).
pub trait SsePolicyLookup: Send + Sync {
    /// Returns `Some` only when SSE is active and evaluated; `None` when SSE is disabled.
    fn evaluate(&self, ctx: &ConnectionContext) -> Option<SseGateResult>;
}

/// Default no-op lookup when SSE is not configured.
pub struct NoOpSsePolicyLookup;

impl SsePolicyLookup for NoOpSsePolicyLookup {
    fn evaluate(&self, _ctx: &ConnectionContext) -> Option<SseGateResult> {
        None
    }
}
