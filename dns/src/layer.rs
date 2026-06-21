use crate::failover::DnsProviderRegistry;
use crate::policy::{evaluate_domain, DomainDecision};
use crate::providers::{from_record, parse_dot_endpoint, DohProvider};
use crate::proxy::{spawn_proxy, DnsLogHandler, DnsProxyHandle, ProxyContext, UpstreamLookup};
use async_trait::async_trait;
use filter_lists::FilterListProvider;
use parking_lot::RwLock;
use shared_types::{
    DnsProviderRecord, DnsSettings, DnsTransport, DNSQueryLog, Result, WireSentinelError,
};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn};
use uuid::Uuid;

struct RegistryUpstream(Arc<DnsProviderRegistry>);

#[async_trait]
impl UpstreamLookup for RegistryUpstream {
    async fn resolve_a(&self, name: &str) -> Result<Vec<String>> {
        self.0.resolve_a(name).await
    }

    fn provider_name(&self) -> String {
        self.0.provider_name()
    }
}

/// DNS proxy engine with encrypted upstream, optional filtering, and query logging.
pub struct DnsLayer {
    settings: RwLock<DnsSettings>,
    registry: RwLock<Option<Arc<DnsProviderRegistry>>>,
    upstream: RwLock<Option<Arc<dyn UpstreamLookup>>>,
    filter: RwLock<Option<Arc<dyn FilterListProvider>>>,
    log_handler: RwLock<Option<DnsLogHandler>>,
    query_tx: broadcast::Sender<DNSQueryLog>,
    logs: RwLock<Vec<DNSQueryLog>>,
    proxy: RwLock<Option<DnsProxyHandle>>,
}

impl DnsLayer {
    pub fn new(settings: DnsSettings) -> Self {
        let (query_tx, _) = broadcast::channel(256);
        Self {
            settings: RwLock::new(settings),
            registry: RwLock::new(None),
            upstream: RwLock::new(None),
            filter: RwLock::new(None),
            log_handler: RwLock::new(None),
            query_tx,
            logs: RwLock::new(Vec::new()),
            proxy: RwLock::new(None),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DNSQueryLog> {
        self.query_tx.subscribe()
    }

    pub fn settings(&self) -> DnsSettings {
        self.settings.read().clone()
    }

    pub fn registry(&self) -> Option<Arc<DnsProviderRegistry>> {
        self.registry.read().clone()
    }

    pub fn set_registry(&self, registry: Arc<DnsProviderRegistry>) {
        *self.registry.write() = Some(Arc::clone(&registry));
        let upstream: Arc<dyn UpstreamLookup> = Arc::new(RegistryUpstream(Arc::clone(&registry)));
        *self.upstream.write() = Some(upstream);
    }

    pub fn set_log_handler(&self, handler: Option<DnsLogHandler>) {
        *self.log_handler.write() = handler;
    }

    pub fn set_filter_provider(&self, provider: Arc<dyn FilterListProvider>) {
        *self.filter.write() = Some(provider);
    }

    pub fn clear_filter_provider(&self) {
        *self.filter.write() = None;
    }

    pub fn update_settings(&self, settings: DnsSettings) -> Result<()> {
        self.stop();
        *self.settings.write() = settings.clone();
        if settings.enabled {
            self.start()?;
        }
        Ok(())
    }

    pub fn load_providers_from_records(&self, records: &[DnsProviderRecord]) -> Result<()> {
        let mut sorted: Vec<_> = records.iter().filter(|r| r.enabled).cloned().collect();
        sorted.sort_by_key(|r| r.priority);

        let mut providers = Vec::new();
        for record in &sorted {
            providers.push(from_record(&record.name, record.transport, &record.endpoint)?);
        }

        if providers.is_empty() {
            providers.push(Arc::new(DohProvider::cloudflare()?));
        }

        self.set_registry(Arc::new(DnsProviderRegistry::new(providers)));
        Ok(())
    }

    pub fn start(&self) -> Result<()> {
        let settings = self.settings.read().clone();
        if !settings.enabled {
            return Ok(());
        }

        if self.upstream.read().is_none() {
            let provider = match settings.transport {
                DnsTransport::Doh => from_record(
                    &settings.provider,
                    DnsTransport::Doh,
                    &settings.upstream_url,
                )?,
                DnsTransport::Dot => {
                    let (host, port) = parse_dot_endpoint(&settings.upstream_url)?;
                    from_record(&settings.provider, DnsTransport::Dot, &format!("{host}:{port}"))?
                }
                DnsTransport::Doq => from_record(
                    &settings.provider,
                    DnsTransport::Doq,
                    &settings.upstream_url,
                )?,
                DnsTransport::Plain => {
                    return Err(WireSentinelError::Dns(
                        "plain DNS upstream is not supported in MVP".into(),
                    ));
                }
            };
            let registry = Arc::new(DnsProviderRegistry::new(vec![provider]));
            self.set_registry(registry);
        }

        let upstream = self
            .upstream
            .read()
            .clone()
            .ok_or_else(|| WireSentinelError::Dns("DNS upstream not configured".into()))?;

        let log_handler = self.log_handler.read().clone();
        let ctx = Arc::new(ProxyContext {
            settings: settings.clone(),
            filter: self.filter.read().clone(),
            upstream,
            log_tx: self.query_tx.clone(),
            log_handler,
        });
        match spawn_proxy(ctx) {
            Ok(handle) => {
                *self.proxy.write() = Some(handle);
            }
            Err(e) => {
                warn!(error = %e, "DNS UDP proxy failed to start");
            }
        }

        info!(
            provider = %settings.provider,
            listen = %settings.listen_addr,
            filter_mode = ?settings.filter_mode,
            block_mode = ?settings.dns_block_mode,
            "DNS engine started"
        );
        Ok(())
    }

    pub fn stop(&self) {
        if let Some(handle) = self.proxy.write().take() {
            handle.stop();
        }
        *self.upstream.write() = None;
        info!("DNS engine stopped");
    }

    pub async fn resolve(&self, qname: &str, qtype: &str) -> Result<DNSQueryLog> {
        let settings = self.settings.read().clone();
        let filter_provider = self.filter.read().clone();
        let correlation_id = Uuid::new_v4();

        let decision = evaluate_domain(
            qname,
            settings.filter_mode,
            filter_provider.as_deref(),
        );

        if decision == DomainDecision::Block {
            let log = DNSQueryLog {
                id: Uuid::new_v4(),
                timestamp: chrono::Utc::now(),
                app_id: None,
                pid: None,
                qname: qname.to_string(),
                qtype: qtype.to_string(),
                upstream: settings.provider.clone(),
                blocked: true,
                latency_ms: 0,
                answers: vec![],
                response: Some(format!("blocked:{:?}", settings.dns_block_mode)),
                correlation_id: Some(correlation_id),
            };
            warn!(qname, mode = ?settings.filter_mode, "DNS query blocked by filter");
            self.record(log.clone());
            return Ok(log);
        }

        let registry = self
            .registry
            .read()
            .clone()
            .ok_or_else(|| WireSentinelError::Dns("DNS engine not running".into()))?;
        let (answers, latency, upstream_name) = registry.resolve(qname, qtype).await?;

        let log = DNSQueryLog {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            app_id: None,
            pid: None,
            qname: qname.to_string(),
            qtype: qtype.to_string(),
            upstream: upstream_name,
            blocked: false,
            latency_ms: latency,
            answers,
            response: None,
            correlation_id: Some(correlation_id),
        };
        self.record(log.clone());
        Ok(log)
    }

    fn record(&self, log: DNSQueryLog) {
        if self.settings.read().enabled {
            self.logs.write().push(log.clone());
            if self.logs.read().len() > 10_000 {
                self.logs.write().drain(0..1000);
            }
            let _ = self.query_tx.send(log.clone());
            if let Some(handler) = self.log_handler.read().clone() {
                handler(log);
            }
        }
    }

    pub fn recent_logs(&self, limit: usize) -> Vec<DNSQueryLog> {
        let logs = self.logs.read();
        let start = logs.len().saturating_sub(limit);
        logs[start..].to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_dot_host_and_port() {
        assert_eq!(
            parse_dot_endpoint("tls://dns.example.com:853").unwrap(),
            ("dns.example.com".into(), 853)
        );
        assert_eq!(
            parse_dot_endpoint("dns.example.com").unwrap(),
            ("dns.example.com".into(), 853)
        );
    }
}
