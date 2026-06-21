use super::traits::{PrivacySnapshotRepository, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{PrivacyScoreComponents, PrivacyScoreSnapshot, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqlitePrivacySnapshotRepository {
    pool: SqlitePool,
}

impl SqlitePrivacySnapshotRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PrivacySnapshotRepository for SqlitePrivacySnapshotRepository {
    async fn insert(&self, snapshot: &PrivacyScoreSnapshot) -> Result<()> {
        let components_json = serde_json::to_string(&snapshot.components)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        sqlx::query(
            "INSERT INTO privacy_snapshots (id, score, components_json, timestamp) VALUES (?, ?, ?, ?)",
        )
        .bind(snapshot.id.to_string())
        .bind(snapshot.score as i32)
        .bind(components_json)
        .bind(snapshot.timestamp.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn latest(&self) -> Result<Option<PrivacyScoreSnapshot>> {
        let row = sqlx::query_as::<_, (String, i32, String, String)>(
            "SELECT id, score, components_json, timestamp FROM privacy_snapshots ORDER BY timestamp DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(|(id, score, components_json, timestamp)| {
            let components: PrivacyScoreComponents = serde_json::from_str(&components_json)
                .map_err(|e| WireSentinelError::Config(e.to_string()))?;
            Ok(PrivacyScoreSnapshot {
                id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
                score: score.clamp(0, 100) as u8,
                components,
                timestamp: DateTime::parse_from_rfc3339(&timestamp)
                    .map_err(|e| WireSentinelError::Config(e.to_string()))?
                    .with_timezone(&Utc),
            })
        })
        .transpose()
    }

    async fn list(&self, limit: u32) -> Result<Vec<PrivacyScoreSnapshot>> {
        let rows = sqlx::query_as::<_, (String, i32, String, String)>(
            "SELECT id, score, components_json, timestamp FROM privacy_snapshots ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(|(id, score, components_json, timestamp)| {
                let components: PrivacyScoreComponents = serde_json::from_str(&components_json)
                    .map_err(|e| WireSentinelError::Config(e.to_string()))?;
                Ok(PrivacyScoreSnapshot {
                    id: Uuid::parse_str(&id)
                        .map_err(|e| WireSentinelError::Config(e.to_string()))?,
                    score: score.clamp(0, 100) as u8,
                    components,
                    timestamp: DateTime::parse_from_rfc3339(&timestamp)
                        .map_err(|e| WireSentinelError::Config(e.to_string()))?
                        .with_timezone(&Utc),
                })
            })
            .collect()
    }
}
