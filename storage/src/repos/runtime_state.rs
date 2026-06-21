use super::traits::{Result, RuntimeStateRepository};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{RuntimeStateRecord, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteRuntimeStateRepository {
    pool: SqlitePool,
}

impl SqliteRuntimeStateRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RuntimeStateRepository for SqliteRuntimeStateRepository {
    async fn upsert(&self, record: &RuntimeStateRecord) -> Result<()> {
        sqlx::query(
            "INSERT INTO runtime_state (id, scope, entity_id, state_json, updated_at)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(scope, entity_id) DO UPDATE SET
               state_json = excluded.state_json,
               updated_at = excluded.updated_at",
        )
        .bind(record.id.to_string())
        .bind(&record.scope)
        .bind(&record.entity_id)
        .bind(&record.state_json)
        .bind(record.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn list_by_scope(&self, scope: &str) -> Result<Vec<RuntimeStateRecord>> {
        let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(
            "SELECT id, scope, entity_id, state_json, updated_at FROM runtime_state WHERE scope = ?",
        )
        .bind(scope)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(|(id, scope, entity_id, state_json, updated_at)| {
                Ok(RuntimeStateRecord {
                    id: Uuid::parse_str(&id)
                        .map_err(|e| WireSentinelError::Config(e.to_string()))?,
                    scope,
                    entity_id,
                    state_json,
                    updated_at: DateTime::parse_from_rfc3339(&updated_at)
                        .map_err(|e| WireSentinelError::Config(e.to_string()))?
                        .with_timezone(&Utc),
                })
            })
            .collect()
    }

    async fn delete_scope(&self, scope: &str) -> Result<()> {
        sqlx::query("DELETE FROM runtime_state WHERE scope = ?")
            .bind(scope)
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<RuntimeStateRecord>> {
        let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(
            "SELECT id, scope, entity_id, state_json, updated_at FROM runtime_state",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(|(id, scope, entity_id, state_json, updated_at)| {
                Ok(RuntimeStateRecord {
                    id: Uuid::parse_str(&id)
                        .map_err(|e| WireSentinelError::Config(e.to_string()))?,
                    scope,
                    entity_id,
                    state_json,
                    updated_at: DateTime::parse_from_rfc3339(&updated_at)
                        .map_err(|e| WireSentinelError::Config(e.to_string()))?
                        .with_timezone(&Utc),
                })
            })
            .collect()
    }
}
