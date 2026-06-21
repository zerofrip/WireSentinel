use super::traits::{DomainCacheRepository, Result};
use async_trait::async_trait;
use chrono::Utc;
use shared_types::{DomainCacheEntry, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteDomainCacheRepository {
    pool: SqlitePool,
}

impl SqliteDomainCacheRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DomainCacheRepository for SqliteDomainCacheRepository {
    async fn upsert(&self, entry: &DomainCacheEntry) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO domain_resolver_cache (id, app_id, domain, ip_address, wildcard, expires_at, first_seen, last_seen, hit_count)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(id) DO UPDATE SET
                 last_seen = excluded.last_seen,
                 hit_count = hit_count + 1,
                 expires_at = excluded.expires_at"#,
        )
        .bind(entry.id.to_string())
        .bind(entry.app_id.map(|id| id.to_string()))
        .bind(&entry.domain)
        .bind(&entry.ip_address)
        .bind(entry.wildcard as i32)
        .bind(entry.expires_at.to_rfc3339())
        .bind(entry.first_seen.to_rfc3339())
        .bind(entry.last_seen.to_rfc3339())
        .bind(entry.hit_count as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn lookup_by_ip(
        &self,
        app_id: Option<Uuid>,
        ip: &str,
    ) -> Result<Option<DomainCacheEntry>> {
        let row = sqlx::query_as::<
            _,
            (
                String,
                Option<String>,
                String,
                String,
                i32,
                String,
                String,
                String,
                i64,
            ),
        >(
            "SELECT id, app_id, domain, ip_address, wildcard, expires_at, first_seen, last_seen, hit_count
             FROM domain_resolver_cache WHERE ip_address = ? AND expires_at > ? AND (app_id IS ? OR app_id = ?)
             ORDER BY last_seen DESC LIMIT 1",
        )
        .bind(ip)
        .bind(Utc::now().to_rfc3339())
        .bind(app_id.map(|id| id.to_string()))
        .bind(app_id.map(|id| id.to_string()))
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(parse_row).transpose()
    }

    async fn purge_expired(&self) -> Result<u64> {
        let r = sqlx::query("DELETE FROM domain_resolver_cache WHERE expires_at <= ?")
            .bind(Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(r.rows_affected())
    }
}

fn parse_row(
    (id, app_id, domain, ip_address, wildcard, expires_at, first_seen, last_seen, hit_count): (
        String,
        Option<String>,
        String,
        String,
        i32,
        String,
        String,
        String,
        i64,
    ),
) -> Result<DomainCacheEntry> {
    Ok(DomainCacheEntry {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        app_id: app_id.and_then(|s| Uuid::parse_str(&s).ok()),
        domain,
        ip_address,
        wildcard: wildcard != 0,
        expires_at: chrono::DateTime::parse_from_rfc3339(&expires_at)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
        first_seen: chrono::DateTime::parse_from_rfc3339(&first_seen)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
        last_seen: chrono::DateTime::parse_from_rfc3339(&last_seen)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
        hit_count: hit_count as u32,
    })
}
