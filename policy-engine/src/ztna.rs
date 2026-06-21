//! Optional ZTNA policy gate lookup.

use crate::engine::ConnectionContext;
use uuid::Uuid;

/// Result of a ZTNA pre-policy gate check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZtnaGateResult {
    pub allowed: bool,
    pub reason: String,
    pub matched_policy_id: Option<Uuid>,
}

/// Evaluates ZTNA access before the main ruleset (additive gate).
pub trait ZtnaPolicyLookup: Send + Sync {
    /// Returns `Some` only when ZTNA is active and evaluated; `None` when ZTNA is disabled.
    fn evaluate(&self, ctx: &ConnectionContext) -> Option<ZtnaGateResult>;
}

/// Default no-op lookup when ZTNA is not configured.
pub struct NoOpZtnaPolicyLookup;

impl ZtnaPolicyLookup for NoOpZtnaPolicyLookup {
    fn evaluate(&self, _ctx: &ConnectionContext) -> Option<ZtnaGateResult> {
        None
    }
}
