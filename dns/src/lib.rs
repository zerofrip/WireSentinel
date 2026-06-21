//! DNS security layer — encrypted upstream, filtering, and query logging.

mod failover;
mod layer;
mod policy;
mod provider;
mod providers;
mod proxy;

pub use failover::DnsProviderRegistry;
pub use layer::DnsLayer;
pub use policy::{evaluate_domain, DomainDecision};
pub use provider::DnsProvider;
pub use providers::{DohProvider, DotProvider, DoqProvider, from_record, parse_dot_endpoint};
pub use proxy::{spawn_proxy, DnsLogHandler, DnsProxyHandle, UpstreamLookup};

// Backward-compatible aliases
pub use providers::DohProvider as DohResolver;
pub use providers::DotProvider as DotResolver;
