//! SSE pre-policy gate — evaluates secure edge controls before the ruleset.

use policy_engine::{ConnectionContext, Decision, SsePolicyLookup};
use shared_types::{TrafficRoute, Verdict};

/// Evaluate SSE policy after ZTNA and before the main ruleset.
///
/// Returns `Some(Decision)` only when SSE denies access; `None` allows the
/// caller to continue with the standard policy engine.
pub fn evaluate_sse_gate<L: SsePolicyLookup>(
    lookup: &L,
    ctx: &ConnectionContext,
) -> Option<Decision> {
    let result = lookup.evaluate(ctx)?;
    if result.allowed {
        return None;
    }
    Some(Decision {
        route: TrafficRoute::Blocked,
        verdict: Verdict::block(result.reason),
        matched_rule_id: result.matched_policy_id,
    })
}
