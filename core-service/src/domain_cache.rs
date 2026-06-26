//! In-memory domain resolver cache with TTL and wildcard matching.

use chrono::{Duration, Utc};
use parking_lot::RwLock;
use shared_types::{DomainCacheEntry, Result};
use std::collections::HashMap;
use std::sync::Arc;
use storage::DomainCacheRepository;
use tokio::sync::watch;
use tracing::{debug, warn};
use uuid::Uuid;

const DEFAULT_TTL_SECS: u64 = 300;

#[derive(Debug, Clone)]
struct MemoryEntry {
    domain: String,
    #[allow(dead_code)]
    wildcard: bool,
    expires_at: chrono::DateTime<Utc>,
}

pub struct DomainResolverCache {
    repo: Arc<dyn DomainCacheRepository>,
    ttl_secs: u64,
    memory: RwLock<HashMap<(Option<Uuid>, String), MemoryEntry>>,
}

impl DomainResolverCache {
    pub fn new(repo: Arc<dyn DomainCacheRepository>) -> Self {
        Self::with_ttl(repo, DEFAULT_TTL_SECS)
    }

    pub fn with_ttl(repo: Arc<dyn DomainCacheRepository>, ttl_secs: u64) -> Self {
        Self {
            repo,
            ttl_secs,
            memory: RwLock::new(HashMap::new()),
        }
    }

    pub async fn record_dns(&self, app_id: Option<Uuid>, domain: &str, ips: &[&str]) -> Result<()> {
        let wildcard = domain.starts_with("*.");
        let now = Utc::now();
        let expires_at = now + Duration::seconds(self.ttl_secs as i64);

        for ip in ips {
            let key = (app_id, ip.to_string());
            self.memory.write().insert(
                key,
                MemoryEntry {
                    domain: domain.to_string(),
                    wildcard,
                    expires_at,
                },
            );

            let entry = DomainCacheEntry {
                id: crate::deterministic_id::domain_cache_id(app_id, ip),
                app_id,
                domain: domain.to_string(),
                ip_address: ip.to_string(),
                wildcard,
                expires_at,
                first_seen: now,
                last_seen: now,
                hit_count: 1,
            };
            if let Err(e) = self.repo.upsert(&entry).await {
                warn!(error = %e, ip, domain, "domain cache persist failed");
            }
        }
        Ok(())
    }

    pub async fn resolve_ip_to_domain(
        &self,
        app_id: Option<Uuid>,
        ip: &str,
    ) -> Result<Option<String>> {
        let now = Utc::now();

        if let Some(entry) = self.memory.read().get(&(app_id, ip.to_string())) {
            if entry.expires_at > now {
                return Ok(Some(entry.domain.clone()));
            }
        }

        if let Some(entry) = self.memory.read().get(&(None, ip.to_string())) {
            if entry.expires_at > now {
                return Ok(Some(entry.domain.clone()));
            }
        }

        if let Some(db_entry) = self.repo.lookup_by_ip(app_id, ip).await? {
            self.memory.write().insert(
                (app_id, ip.to_string()),
                MemoryEntry {
                    domain: db_entry.domain.clone(),
                    wildcard: db_entry.wildcard,
                    expires_at: db_entry.expires_at,
                },
            );
            return Ok(Some(db_entry.domain));
        }

        Ok(None)
    }

    fn domain_matches_wildcard(pattern: &str, candidate: &str) -> bool {
        if !pattern.starts_with("*.") {
            return pattern == candidate;
        }
        let suffix = &pattern[1..];
        candidate.ends_with(suffix) || candidate == &pattern[2..]
    }

    pub fn matches_domain(pattern: &str, candidate: &str) -> bool {
        if pattern.starts_with("*.") {
            Self::domain_matches_wildcard(pattern, candidate)
        } else {
            pattern == candidate
        }
    }

    pub async fn purge_expired(&self) -> Result<u64> {
        let now = Utc::now();
        self.memory
            .write()
            .retain(|_, entry| entry.expires_at > now);
        self.repo.purge_expired().await
    }

    pub fn start_purge_task(self: Arc<Self>, mut shutdown: watch::Receiver<bool>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = self.purge_expired().await {
                            warn!(error = %e, "domain cache purge failed");
                        } else {
                            debug!("domain cache purge completed");
                        }
                    }
                    changed = shutdown.changed() => {
                        if changed.is_ok() && *shutdown.borrow() {
                            break;
                        }
                    }
                }
            }
        });
    }
}

impl DomainResolverCache {
    pub async fn record_dns_strings(
        &self,
        app_id: Option<Uuid>,
        domain: &str,
        ips: &[String],
    ) -> Result<()> {
        let refs: Vec<&str> = ips.iter().map(String::as_str).collect();
        self.record_dns(app_id, domain, &refs).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_matches_subdomain() {
        assert!(DomainResolverCache::matches_domain(
            "*.googlevideo.com",
            "r1---sn.googlevideo.com"
        ));
        assert!(!DomainResolverCache::matches_domain(
            "*.googlevideo.com",
            "example.com"
        ));
    }

    #[tokio::test]
    async fn record_and_resolve_ip() {
        let pool = storage::init_pool_in_memory().await.unwrap();
        let storage = Arc::new(storage::Storage::new(pool));
        let cache = DomainResolverCache::with_ttl(
            Arc::clone(&storage.domain_cache) as Arc<dyn storage::DomainCacheRepository>,
            300,
        );
        let app_id = Uuid::new_v4();
        cache
            .record_dns(Some(app_id), "example.com", &["93.184.216.34"])
            .await
            .unwrap();
        let domain = cache
            .resolve_ip_to_domain(Some(app_id), "93.184.216.34")
            .await
            .unwrap();
        assert_eq!(domain.as_deref(), Some("example.com"));
    }
}
