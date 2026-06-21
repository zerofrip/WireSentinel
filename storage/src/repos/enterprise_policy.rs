use super::traits::{EnterprisePolicyRepository, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{EnterprisePolicy, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteEnterprisePolicyRepository {
    pool: SqlitePool,
}

impl SqliteEnterprisePolicyRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EnterprisePolicyRepository for SqliteEnterprisePolicyRepository {
    async fn get_active(&self) -> Result<Option<EnterprisePolicy>> {
        let row: Option<(String, i64, String, String, String)> = sqlx::query_as(
            "SELECT id, version, policy_json, locked_keys_json, updated_at
             FROM enterprise_policy ORDER BY updated_at DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(parse_row).transpose()
    }

    async fn upsert(&self, policy: &EnterprisePolicy) -> Result<()> {
        let policy_json =
            serde_json::to_string(&policy.policy_json).map_err(WireSentinelError::Serde)?;
        let locked_json =
            serde_json::to_string(&policy.locked_keys).map_err(WireSentinelError::Serde)?;

        sqlx::query(
            "INSERT INTO enterprise_policy (id, version, policy_json, locked_keys_json, updated_at)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
               version = excluded.version,
               policy_json = excluded.policy_json,
               locked_keys_json = excluded.locked_keys_json,
               updated_at = excluded.updated_at",
        )
        .bind(policy.id.to_string())
        .bind(policy.version as i64)
        .bind(policy_json)
        .bind(locked_json)
        .bind(policy.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }
}

fn parse_row(row: (String, i64, String, String, String)) -> Result<EnterprisePolicy> {
    let (id, version, policy_json, locked_keys_json, updated_at) = row;
    Ok(EnterprisePolicy {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        version: version as u32,
        policy_json: serde_json::from_str(&policy_json).map_err(WireSentinelError::Serde)?,
        locked_keys: serde_json::from_str(&locked_keys_json).map_err(WireSentinelError::Serde)?,
        updated_at: DateTime::parse_from_rfc3339(&updated_at)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
    })
}
