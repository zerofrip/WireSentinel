use super::traits::{ChainProfileRepository, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{ChainHop, ChainProfile, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteChainProfileRepository {
    pool: SqlitePool,
}

impl SqliteChainProfileRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn parse_hops(json: &str) -> Result<Vec<ChainHop>> {
    serde_json::from_str(json).map_err(|e| WireSentinelError::Config(e.to_string()))
}

#[async_trait]
impl ChainProfileRepository for SqliteChainProfileRepository {
    async fn list(&self) -> Result<Vec<ChainProfile>> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                Option<String>,
                i32,
                String,
                String,
            ),
        >(
            "SELECT id, name, hops_json, obfuscation_profile_id, enabled, created_at, updated_at FROM chain_profiles ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(
                |(id, name, hops_json, obfuscation_profile_id, enabled, created_at, updated_at)| {
                    Ok(ChainProfile {
                        id: Uuid::parse_str(&id)
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?,
                        name,
                        hops: parse_hops(&hops_json)?,
                        obfuscation_profile_id: obfuscation_profile_id
                            .map(|s| Uuid::parse_str(&s))
                            .transpose()
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?,
                        enabled: enabled != 0,
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

    async fn get(&self, id: Uuid) -> Result<Option<ChainProfile>> {
        Ok(self.list().await?.into_iter().find(|p| p.id == id))
    }

    async fn insert(&self, profile: &ChainProfile) -> Result<()> {
        let hops_json = serde_json::to_string(&profile.hops)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        sqlx::query(
            "INSERT INTO chain_profiles (id, name, hops_json, obfuscation_profile_id, enabled, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(profile.id.to_string())
        .bind(&profile.name)
        .bind(hops_json)
        .bind(profile.obfuscation_profile_id.map(|id| id.to_string()))
        .bind(profile.enabled as i32)
        .bind(profile.created_at.to_rfc3339())
        .bind(profile.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn update(&self, profile: &ChainProfile) -> Result<()> {
        let hops_json = serde_json::to_string(&profile.hops)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        sqlx::query(
            "UPDATE chain_profiles SET name = ?, hops_json = ?, obfuscation_profile_id = ?, enabled = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&profile.name)
        .bind(hops_json)
        .bind(profile.obfuscation_profile_id.map(|id| id.to_string()))
        .bind(profile.enabled as i32)
        .bind(profile.updated_at.to_rfc3339())
        .bind(profile.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let r = sqlx::query("DELETE FROM chain_profiles WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(r.rows_affected() > 0)
    }
}
