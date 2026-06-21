use async_trait::async_trait;
use shared_types::{DnsTransport, ProviderHealth, Result};

/// Pluggable upstream DNS resolver (DoH, DoT, DoQ, …).
#[async_trait]
pub trait DnsProvider: Send + Sync {
    fn name(&self) -> &str;
    fn transport(&self) -> DnsTransport;
    async fn resolve(&self, qname: &str, qtype: &str) -> Result<(Vec<String>, u64)>;
    async fn health_check(&self) -> ProviderHealth;
}
