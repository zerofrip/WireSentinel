use super::traits::{ProxyChainRepository, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{ProxyChain, ProxyChainHop, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteProxyChainRepository {
    pool: SqlitePool,
}

impl SqliteProxyChainRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn parse_hops(json: &str) -> Result<Vec<ProxyChainHop>> {
    serde_json::from_str(json).map_err(|e| WireSentinelError::Config(e.to_string()))
}

fn parse_row(
    id: String,
    name: String,
    hops_json: String,
    enabled: i32,
    created_at: String,
    updated_at: String,
) -> Result<ProxyChain> {
    Ok(ProxyChain {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        name,
        hops: parse_hops(&hops_json)?,
        enabled: enabled != 0,
        created_at: DateTime::parse_from_rfc3339(&created_at)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
    })
}

#[async_trait]
impl ProxyChainRepository for SqliteProxyChainRepository {
    async fn list(&self) -> Result<Vec<ProxyChain>> {
        let rows = sqlx::query_as::<_, (String, String, String, i32, String, String)>(
            "SELECT id, name, hops_json, enabled, created_at, updated_at FROM proxy_chains ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(|r| parse_row(r.0, r.1, r.2, r.3, r.4, r.5))
            .collect()
    }

    async fn get(&self, id: Uuid) -> Result<Option<ProxyChain>> {
        let row = sqlx::query_as::<_, (String, String, String, i32, String, String)>(
            "SELECT id, name, hops_json, enabled, created_at, updated_at FROM proxy_chains WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(|r| parse_row(r.0, r.1, r.2, r.3, r.4, r.5))
            .transpose()
    }

    async fn insert(&self, chain: &ProxyChain) -> Result<()> {
        let hops_json = serde_json::to_string(&chain.hops)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        sqlx::query(
            "INSERT INTO proxy_chains (id, name, hops_json, enabled, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(chain.id.to_string())
        .bind(&chain.name)
        .bind(hops_json)
        .bind(chain.enabled as i32)
        .bind(chain.created_at.to_rfc3339())
        .bind(chain.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn update(&self, chain: &ProxyChain) -> Result<()> {
        let hops_json = serde_json::to_string(&chain.hops)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        sqlx::query(
            "UPDATE proxy_chains SET name = ?, hops_json = ?, enabled = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&chain.name)
        .bind(hops_json)
        .bind(chain.enabled as i32)
        .bind(chain.updated_at.to_rfc3339())
        .bind(chain.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let r = sqlx::query("DELETE FROM proxy_chains WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(r.rows_affected() > 0)
    }
}
