//! ZTNA pre-policy gate — evaluates zero-trust access before the ruleset.

use policy_engine::{ConnectionContext, Decision, ZtnaPolicyLookup};
use shared_types::{TrafficRoute, Verdict};

/// Evaluate ZTNA policy before the main ruleset.
///
/// Returns `Some(Decision)` only when ZTNA denies access; `None` allows the
/// caller to continue with the standard policy engine.
pub fn evaluate_ztna_gate<L: ZtnaPolicyLookup>(
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
