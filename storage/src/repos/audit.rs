use super::traits::{AuditLogRepository, Result};
use async_trait::async_trait;
use chrono::Utc;
use shared_types::{AuditLogEntry, AuditLogQuery, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteAuditLogRepository {
    pool: SqlitePool,
}

impl SqliteAuditLogRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuditLogRepository for SqliteAuditLogRepository {
    async fn insert(&self, entry: &AuditLogEntry) -> Result<()> {
        sqlx::query(
            "INSERT INTO audit_log (id, event_type, actor, target_type, target_id, detail_json, timestamp) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(entry.id.to_string())
        .bind(&entry.event_type)
        .bind(&entry.actor)
        .bind(&entry.target_type)
        .bind(&entry.target_id)
        .bind(&entry.detail_json)
        .bind(entry.timestamp.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn list(&self, query: AuditLogQuery) -> Result<Vec<AuditLogEntry>> {
        let mut sql = String::from(
            "SELECT id, event_type, actor, target_type, target_id, detail_json, timestamp FROM audit_log WHERE 1=1",
        );
        if query.event_type.is_some() {
            sql.push_str(" AND event_type = ?");
        }
        sql.push_str(" ORDER BY timestamp DESC LIMIT ? OFFSET ?");

        let mut q = sqlx::query_as::<
            _,
            (
                String,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                String,
            ),
        >(&sql);
        if let Some(ref event_type) = query.event_type {
            q = q.bind(event_type);
        }
        q = q.bind(query.limit.max(1) as i64).bind(query.offset as i64);

        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(|(id, event_type, actor, target_type, target_id, detail_json, timestamp)| {
                Ok(AuditLogEntry {
                    id: Uuid::parse_str(&id)
                        .map_err(|e| WireSentinelError::Config(e.to_string()))?,
                    event_type,
                    actor,
                    target_type,
                    target_id,
                    detail_json,
                    timestamp: chrono::DateTime::parse_from_rfc3339(&timestamp)
                        .map_err(|e| WireSentinelError::Config(e.to_string()))?
                        .with_timezone(&Utc),
                })
            })
            .collect()
    }

    async fn count_since(
        &self,
        event_type: Option<&str>,
        since: chrono::DateTime<Utc>,
    ) -> Result<u64> {
        let row: (i64,) = if let Some(et) = event_type {
            sqlx::query_as(
                "SELECT COUNT(*) FROM audit_log WHERE event_type = ? AND timestamp >= ?",
            )
            .bind(et)
            .bind(since.to_rfc3339())
            .fetch_one(&self.pool)
            .await
        } else {
            sqlx::query_as("SELECT COUNT(*) FROM audit_log WHERE timestamp >= ?")
                .bind(since.to_rfc3339())
                .fetch_one(&self.pool)
                .await
        }
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(row.0 as u64)
    }
}
