//! Deterministic, priority-based rule evaluation engine.

mod engine;
mod lookup;
mod sse;
mod xdr;
mod ztna;

pub use engine::{ConnectionContext, Decision, PolicyEngine};
pub use lookup::ProfileLookup;
pub use sse::{NoOpSsePolicyLookup, SseGateResult, SsePolicyLookup};
pub use xdr::{NoOpXdrPolicyLookup, XdrPolicyLookup};
pub use ztna::{NoOpZtnaPolicyLookup, ZtnaGateResult, ZtnaPolicyLookup};
