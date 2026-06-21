use async_trait::async_trait;
use shared_types::{DnsTransport, ProviderHealth, Result, WireSentinelError};

use crate::provider::DnsProvider;

/// DNS-over-QUIC stub — returns unavailable until QUIC transport is implemented.
pub struct DoqProvider {
    name: String,
    endpoint: String,
}

impl DoqProvider {
    pub fn new(name: &str, endpoint: &str) -> Self {
        Self {
            name: name.to_string(),
            endpoint: endpoint.to_string(),
        }
    }
}

#[async_trait]
impl DnsProvider for DoqProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn transport(&self) -> DnsTransport {
        DnsTransport::Doq
    }

    async fn resolve(&self, _qname: &str, _qtype: &str) -> Result<(Vec<String>, u64)> {
        Err(WireSentinelError::Dns(format!(
            "DoQ provider '{}' ({}) unavailable",
            self.name, self.endpoint
        )))
    }

    async fn health_check(&self) -> ProviderHealth {
        ProviderHealth {
            available: false,
            latency_ms: None,
            message: Some("DoQ not implemented".into()),
        }
    }
}
