use super::traits::{DnsProviderRepository, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{DnsProviderRecord, DnsTransport, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteDnsProviderRepository {
    pool: SqlitePool,
}

impl SqliteDnsProviderRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn transport_str(t: DnsTransport) -> &'static str {
    match t {
        DnsTransport::Plain => "plain",
        DnsTransport::Doh => "doh",
        DnsTransport::Dot => "dot",
        DnsTransport::Doq => "doq",
    }
}

fn transport_from_str(s: &str) -> Result<DnsTransport> {
    match s {
        "plain" => Ok(DnsTransport::Plain),
        "doh" => Ok(DnsTransport::Doh),
        "dot" => Ok(DnsTransport::Dot),
        "doq" => Ok(DnsTransport::Doq),
        other => Err(WireSentinelError::Config(format!(
            "unknown dns transport: {other}"
        ))),
    }
}

#[async_trait]
impl DnsProviderRepository for SqliteDnsProviderRepository {
    async fn list(&self) -> Result<Vec<DnsProviderRecord>> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                i32,
                i32,
                Option<i64>,
                Option<String>,
                i32,
                String,
                String,
            ),
        >(
            "SELECT id, name, transport, endpoint, priority, enabled, latency_ms, last_check, failure_count, created_at, updated_at FROM dns_providers ORDER BY priority ASC, name ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(
                |(
                    id,
                    name,
                    transport,
                    endpoint,
                    priority,
                    enabled,
                    latency_ms,
                    last_check,
                    failure_count,
                    created_at,
                    updated_at,
                )| {
                    Ok(DnsProviderRecord {
                        id: Uuid::parse_str(&id)
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?,
                        name,
                        transport: transport_from_str(&transport)?,
                        endpoint,
                        priority,
                        enabled: enabled != 0,
                        latency_ms: latency_ms.map(|v| v as u64),
                        last_check: last_check
                            .map(|s| DateTime::parse_from_rfc3339(&s))
                            .transpose()
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?
                            .map(|d| d.with_timezone(&Utc)),
                        failure_count: failure_count as u32,
                        created_at: DateTime::parse_from_rfc3339(&created_at)
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?
                            .with_timezone(&Utc),
                        updated_at: DateTime::parse_from_rfc3339(&updated_at)
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?
                            .with_timezone(&Utc),
                    })
                },
            )
            .collect()
    }

    async fn get(&self, id: Uuid) -> Result<Option<DnsProviderRecord>> {
        Ok(self.list().await?.into_iter().find(|p| p.id == id))
    }

    async fn upsert(&self, provider: &DnsProviderRecord) -> Result<()> {
        sqlx::query(
            "INSERT INTO dns_providers (id, name, transport, endpoint, priority, enabled, latency_ms, last_check, failure_count, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET name=excluded.name, transport=excluded.transport, endpoint=excluded.endpoint, priority=excluded.priority, enabled=excluded.enabled, latency_ms=excluded.latency_ms, last_check=excluded.last_check, failure_count=excluded.failure_count, updated_at=excluded.updated_at",
        )
        .bind(provider.id.to_string())
        .bind(&provider.name)
        .bind(transport_str(provider.transport))
        .bind(&provider.endpoint)
        .bind(provider.priority)
        .bind(provider.enabled as i32)
        .bind(provider.latency_ms.map(|v| v as i64))
        .bind(provider.last_check.map(|t| t.to_rfc3339()))
        .bind(provider.failure_count as i32)
        .bind(provider.created_at.to_rfc3339())
        .bind(provider.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let r = sqlx::query("DELETE FROM dns_providers WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(r.rows_affected() > 0)
    }
}
