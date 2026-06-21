use super::traits::{BackupManifestRepository, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{BackupManifestEntry, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteBackupManifestRepository {
    pool: SqlitePool,
}

impl SqliteBackupManifestRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BackupManifestRepository for SqliteBackupManifestRepository {
    async fn insert(&self, entry: &BackupManifestEntry) -> Result<()> {
        let detail_json =
            serde_json::to_string(&entry.detail_json).map_err(WireSentinelError::Serde)?;

        sqlx::query(
            "INSERT INTO backup_manifest (id, operation, format, checksum, created_at, detail_json)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(entry.id.to_string())
        .bind(&entry.operation)
        .bind(&entry.format)
        .bind(&entry.checksum)
        .bind(entry.created_at.to_rfc3339())
        .bind(detail_json)
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn list_recent(&self, limit: u32) -> Result<Vec<BackupManifestEntry>> {
        let rows: Vec<(String, String, String, String, String, String)> = sqlx::query_as(
            "SELECT id, operation, format, checksum, created_at, detail_json
             FROM backup_manifest ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter().map(parse_row).collect()
    }
}

fn parse_row(row: (String, String, String, String, String, String)) -> Result<BackupManifestEntry> {
    let (id, operation, format, checksum, created_at, detail_json) = row;
    Ok(BackupManifestEntry {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        operation,
        format,
        checksum,
        created_at: DateTime::parse_from_rfc3339(&created_at)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
        detail_json: serde_json::from_str(&detail_json).map_err(WireSentinelError::Serde)?,
    })
}
