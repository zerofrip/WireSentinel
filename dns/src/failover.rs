use async_trait::async_trait;
use parking_lot::RwLock;
use shared_types::{ProviderHealth, Result, WireSentinelError};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::warn;

use crate::provider::DnsProvider;
use crate::proxy::UpstreamLookup;

/// Priority-ordered DNS upstream with automatic failover and latency tracking.
pub struct DnsProviderRegistry {
    providers: Vec<Arc<dyn DnsProvider>>,
    latencies: RwLock<HashMap<String, u64>>,
    active_name: RwLock<Option<String>>,
}

impl DnsProviderRegistry {
    pub fn new(providers: Vec<Arc<dyn DnsProvider>>) -> Self {
        Self {
            providers,
            latencies: RwLock::new(HashMap::new()),
            active_name: RwLock::new(None),
        }
    }

    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    pub fn active_provider_name(&self) -> String {
        self.active_name
            .read()
            .clone()
            .unwrap_or_else(|| "none".into())
    }

    pub fn latency_for(&self, name: &str) -> Option<u64> {
        self.latencies.read().get(name).copied()
    }

    pub fn latencies(&self) -> HashMap<String, u64> {
        self.latencies.read().clone()
    }

    pub async fn resolve(&self, qname: &str, qtype: &str) -> Result<(Vec<String>, u64, String)> {
        if self.providers.is_empty() {
            return Err(WireSentinelError::Dns("no DNS providers configured".into()));
        }

        let mut last_err = None;
        for provider in &self.providers {
            match provider.resolve(qname, qtype).await {
                Ok((answers, latency)) => {
                    self.latencies
                        .write()
                        .insert(provider.name().to_string(), latency);
                    *self.active_name.write() = Some(provider.name().to_string());
                    return Ok((answers, latency, provider.name().to_string()));
                }
                Err(e) => {
                    warn!(
                        provider = provider.name(),
                        error = %e,
                        "DNS provider failed, trying next"
                    );
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| WireSentinelError::Dns("all DNS providers failed".into())))
    }

    pub async fn health_check_all(&self) -> Vec<(String, ProviderHealth)> {
        let mut out = Vec::new();
        for provider in &self.providers {
            let health = provider.health_check().await;
            if let Some(latency) = health.latency_ms {
                self.latencies
                    .write()
                    .insert(provider.name().to_string(), latency);
            }
            out.push((provider.name().to_string(), health));
        }
        out
    }
}

#[async_trait]
impl UpstreamLookup for DnsProviderRegistry {
    async fn resolve_a(&self, name: &str) -> Result<Vec<String>> {
        let (answers, _, _) = self.resolve(name, "A").await?;
        Ok(answers)
    }

    fn provider_name(&self) -> String {
        self.active_provider_name()
    }
}
