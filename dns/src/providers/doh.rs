use async_trait::async_trait;
use shared_types::{DnsTransport, ProviderHealth, Result, WireSentinelError};
use std::time::Instant;
use tracing::debug;

use crate::provider::DnsProvider;

#[derive(serde::Deserialize)]
struct DohResponse {
    #[serde(default, rename = "Answer")]
    answer: Vec<DohAnswer>,
}

#[derive(serde::Deserialize)]
struct DohAnswer {
    #[serde(default)]
    data: String,
}

/// DNS-over-HTTPS provider using JSON API (Cloudflare-compatible).
pub struct DohProvider {
    client: reqwest::Client,
    endpoint: String,
    provider_name: String,
}

impl DohProvider {
    pub fn cloudflare() -> Result<Self> {
        Self::from_url("cloudflare", "https://cloudflare-dns.com/dns-query")
    }

    pub fn quad9() -> Result<Self> {
        Self::from_url("quad9", "https://dns.quad9.net/dns-query")
    }

    pub fn from_url(provider_name: &str, url: &str) -> Result<Self> {
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| WireSentinelError::Dns(format!("HTTP client: {e}")))?;
        Ok(Self {
            client,
            endpoint: url.to_string(),
            provider_name: provider_name.to_string(),
        })
    }
}

#[async_trait]
impl DnsProvider for DohProvider {
    fn name(&self) -> &str {
        &self.provider_name
    }

    fn transport(&self) -> DnsTransport {
        DnsTransport::Doh
    }

    async fn resolve(&self, qname: &str, qtype: &str) -> Result<(Vec<String>, u64)> {
        let start = Instant::now();
        let url = format!(
            "{}?name={}&type={}",
            self.endpoint.trim_end_matches('/'),
            urlencoding::encode(qname),
            urlencoding::encode(qtype)
        );

        let response = self
            .client
            .get(&url)
            .header("Accept", "application/dns-json")
            .send()
            .await
            .map_err(|e| WireSentinelError::Dns(format!("DoH request failed: {e}")))?;

        if !response.status().is_success() {
            return Err(WireSentinelError::Dns(format!(
                "DoH HTTP {}",
                response.status()
            )));
        }

        let body: DohResponse = response
            .json()
            .await
            .map_err(|e| WireSentinelError::Dns(format!("DoH parse failed: {e}")))?;

        let answers: Vec<String> = body.answer.into_iter().map(|a| a.data).collect();
        let latency = start.elapsed().as_millis() as u64;
        debug!(name = qname, ?answers, latency_ms = latency, "DoH resolved");
        Ok((answers, latency))
    }

    async fn health_check(&self) -> ProviderHealth {
        match self.resolve("example.com", "A").await {
            Ok((answers, latency)) if !answers.is_empty() => ProviderHealth {
                available: true,
                latency_ms: Some(latency),
                message: None,
            },
            Ok(_) => ProviderHealth {
                available: false,
                latency_ms: None,
                message: Some("empty response".into()),
            },
            Err(e) => ProviderHealth {
                available: false,
                latency_ms: None,
                message: Some(e.to_string()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires network"]
    async fn resolve_cloudflare() {
        let provider = DohProvider::cloudflare().unwrap();
        let (answers, _) = provider.resolve("example.com", "A").await.unwrap();
        assert!(!answers.is_empty());
    }
}
