use super::traits::{Result, VpnConfigFileRepository};
use async_trait::async_trait;
use chrono::Utc;
use shared_types::{VpnConfigFileRecord, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteVpnConfigFileRepository {
    pool: SqlitePool,
}

impl SqliteVpnConfigFileRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl VpnConfigFileRepository for SqliteVpnConfigFileRepository {
    async fn upsert(&self, record: &VpnConfigFileRecord) -> Result<()> {
        sqlx::query(
            "INSERT INTO vpn_config_files (profile_id, disk_path, materialized_at) VALUES (?, ?, ?)
             ON CONFLICT(profile_id) DO UPDATE SET disk_path = excluded.disk_path, materialized_at = excluded.materialized_at",
        )
        .bind(record.profile_id.to_string())
        .bind(&record.disk_path)
        .bind(record.materialized_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn get(&self, profile_id: Uuid) -> Result<Option<VpnConfigFileRecord>> {
        let row = sqlx::query_as::<_, (String, String, String)>(
            "SELECT profile_id, disk_path, materialized_at FROM vpn_config_files WHERE profile_id = ?",
        )
        .bind(profile_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(|(profile_id, disk_path, materialized_at)| {
            Ok(VpnConfigFileRecord {
                profile_id: Uuid::parse_str(&profile_id)
                    .map_err(|e| WireSentinelError::Config(e.to_string()))?,
                disk_path,
                materialized_at: chrono::DateTime::parse_from_rfc3339(&materialized_at)
                    .map_err(|e| WireSentinelError::Config(e.to_string()))?
                    .with_timezone(&Utc),
            })
        })
        .transpose()
    }
}
