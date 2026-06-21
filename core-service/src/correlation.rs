//! Traffic correlation between DNS queries and network flows.

use shared_types::WireSentinelError;
use std::sync::Arc;
use storage::{CorrelationRepository, DnsLogRepository};
use uuid::Uuid;

use crate::domain_cache::DomainResolverCache;

pub struct TrafficCorrelator {
    domain_cache: Arc<DomainResolverCache>,
    correlations: Arc<dyn CorrelationRepository>,
    dns_logs: Arc<dyn DnsLogRepository>,
}

impl TrafficCorrelator {
    pub fn new(
        domain_cache: Arc<DomainResolverCache>,
        correlations: Arc<dyn CorrelationRepository>,
        dns_logs: Arc<dyn DnsLogRepository>,
    ) -> Self {
        Self {
            domain_cache,
            correlations,
            dns_logs,
        }
    }

    pub async fn on_dns_resolved(
        &self,
        app_id: Option<Uuid>,
        domain: &str,
        ips: &[String],
        correlation_id: Uuid,
    ) -> Result<(), WireSentinelError> {
        let ip_refs: Vec<&str> = ips.iter().map(String::as_str).collect();
        self.domain_cache
            .record_dns(app_id, domain, &ip_refs)
            .await?;

        for ip in ips {
            self.correlations.record_dns(app_id, domain, ip).await?;
        }
        let _ = correlation_id;
        let _ = &self.dns_logs;
        Ok(())
    }

    pub async fn on_traffic(
        &self,
        app_id: Option<Uuid>,
        dest_ip: &str,
    ) -> Result<Option<String>, WireSentinelError> {
        if let Some(domain) = self
            .domain_cache
            .resolve_ip_to_domain(app_id, dest_ip)
            .await?
        {
            return Ok(Some(domain));
        }

        self.correlations.record_traffic(app_id, dest_ip).await
    }
}
