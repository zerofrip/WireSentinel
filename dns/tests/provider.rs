use async_trait::async_trait;
use dns::{DnsProvider, DnsProviderRegistry};
use shared_types::{DnsTransport, ProviderHealth, Result, WireSentinelError};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct FailingProvider {
    name: String,
    calls: AtomicUsize,
    fail_until: usize,
}

impl FailingProvider {
    fn new(name: &str, fail_until: usize) -> Self {
        Self {
            name: name.to_string(),
            calls: AtomicUsize::new(0),
            fail_until,
        }
    }
}

#[async_trait]
impl DnsProvider for FailingProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn transport(&self) -> DnsTransport {
        DnsTransport::Doh
    }

    async fn resolve(&self, _qname: &str, _qtype: &str) -> Result<(Vec<String>, u64)> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if n < self.fail_until {
            Err(WireSentinelError::Dns(format!("{} simulated failure", self.name)))
        } else {
            Ok((vec!["93.184.216.34".into()], 12))
        }
    }

    async fn health_check(&self) -> ProviderHealth {
        ProviderHealth {
            available: self.fail_until == 0,
            latency_ms: Some(12),
            message: None,
        }
    }
}

struct LatencyProvider {
    name: String,
    latency: u64,
}

#[async_trait]
impl DnsProvider for LatencyProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn transport(&self) -> DnsTransport {
        DnsTransport::Doh
    }

    async fn resolve(&self, _qname: &str, _qtype: &str) -> Result<(Vec<String>, u64)> {
        Ok((vec!["1.1.1.1".into()], self.latency))
    }

    async fn health_check(&self) -> ProviderHealth {
        ProviderHealth {
            available: true,
            latency_ms: Some(self.latency),
            message: None,
        }
    }
}

#[tokio::test]
async fn registry_failover_to_second_provider() {
    let primary = Arc::new(FailingProvider::new("primary", 1));
    let secondary = Arc::new(LatencyProvider {
        name: "secondary".into(),
        latency: 25,
    });

    let registry = DnsProviderRegistry::new(vec![primary, secondary]);

    let (answers, latency, name) = registry.resolve("example.com", "A").await.unwrap();
    assert_eq!(name, "secondary");
    assert_eq!(answers, vec!["1.1.1.1"]);
    assert_eq!(latency, 25);

    let (answers, latency, name) = registry.resolve("example.com", "A").await.unwrap();
    assert_eq!(name, "primary");
    assert_eq!(answers, vec!["93.184.216.34"]);
    assert_eq!(latency, 12);
    assert_eq!(registry.latency_for("primary"), Some(12));
}

#[tokio::test]
async fn registry_returns_error_when_all_fail() {
    let registry = DnsProviderRegistry::new(vec![Arc::new(FailingProvider::new(
        "always-fail",
        usize::MAX,
    ))]);

    assert!(registry.resolve("example.com", "A").await.is_err());
}
