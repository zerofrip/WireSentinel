//! Optional XDR response policy lookup.

use shared_types::ResponseActionKind;

/// Evaluates whether a response action is permitted before execution.
pub trait XdrPolicyLookup: Send + Sync {
    fn is_action_allowed(&self, kind: ResponseActionKind) -> bool;
}

/// Default permissive lookup when XDR policy is not configured.
pub struct NoOpXdrPolicyLookup;

impl XdrPolicyLookup for NoOpXdrPolicyLookup {
    fn is_action_allowed(&self, _kind: ResponseActionKind) -> bool {
        true
    }
}
