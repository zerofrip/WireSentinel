use super::traits::{Result, WfpFilterStateRepository};
use async_trait::async_trait;
use chrono::Utc;
use shared_types::{TrafficRoute, WfpFilterStateRecord, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteWfpFilterStateRepository {
    pool: SqlitePool,
}

impl SqliteWfpFilterStateRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WfpFilterStateRepository for SqliteWfpFilterStateRepository {
    async fn upsert(&self, record: &WfpFilterStateRecord) -> Result<()> {
        let route_json =
            serde_json::to_string(&record.route).map_err(WireSentinelError::Serde)?;
        sqlx::query(
            r#"INSERT INTO wfp_filter_state (id, scope_type, scope_value, filter_id, route_json, rule_id, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(id) DO UPDATE SET filter_id = excluded.filter_id, route_json = excluded.route_json, updated_at = excluded.updated_at"#,
        )
        .bind(record.id.to_string())
        .bind(&record.scope_type)
        .bind(&record.scope_value)
        .bind(record.filter_id as i64)
        .bind(route_json)
        .bind(record.rule_id.map(|id| id.to_string()))
        .bind(record.created_at.to_rfc3339())
        .bind(record.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<WfpFilterStateRecord>> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                Option<String>,
                i64,
                String,
                Option<String>,
                String,
                String,
            ),
        >(
            "SELECT id, scope_type, scope_value, filter_id, route_json, rule_id, created_at, updated_at FROM wfp_filter_state",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter().map(parse_row).collect()
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let r = sqlx::query("DELETE FROM wfp_filter_state WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(r.rows_affected() > 0)
    }
}

fn parse_row(
    (id, scope_type, scope_value, filter_id, route_json, rule_id, created_at, updated_at): (
        String,
        String,
        Option<String>,
        i64,
        String,
        Option<String>,
        String,
        String,
    ),
) -> Result<WfpFilterStateRecord> {
    Ok(WfpFilterStateRecord {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        scope_type,
        scope_value,
        filter_id: filter_id as u64,
        route: serde_json::from_str(&route_json).unwrap_or(TrafficRoute::Direct),
        rule_id: rule_id.and_then(|s| Uuid::parse_str(&s).ok()),
        created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
    })
}
