use super::traits::{Result, TorProfileRepository};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{TorProfile, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteTorProfileRepository {
    pool: SqlitePool,
}

impl SqliteTorProfileRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn parse_row(
    id: String,
    name: String,
    control_port: i64,
    socks_port: i64,
    data_dir: String,
    bridge_ids_json: String,
    enabled: i32,
    bootstrap_progress: i64,
    circuit_count: i64,
    created_at: String,
    updated_at: String,
) -> Result<TorProfile> {
    let bridge_ids: Vec<Uuid> = serde_json::from_str(&bridge_ids_json).unwrap_or_default();
    Ok(TorProfile {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        name,
        control_port: control_port as u16,
        socks_port: socks_port as u16,
        data_dir,
        bridge_ids,
        enabled: enabled != 0,
        bootstrap_progress: bootstrap_progress as u8,
        circuit_count: circuit_count as u32,
        created_at: DateTime::parse_from_rfc3339(&created_at)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
    })
}

#[async_trait]
impl TorProfileRepository for SqliteTorProfileRepository {
    async fn list(&self) -> Result<Vec<TorProfile>> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                i64,
                i64,
                String,
                String,
                i32,
                i64,
                i64,
                String,
                String,
            ),
        >(
            "SELECT id, name, control_port, socks_port, data_dir, bridge_ids_json, enabled, bootstrap_progress, circuit_count, created_at, updated_at FROM tor_profiles ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(|r| {
                parse_row(
                    r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8, r.9, r.10,
                )
            })
            .collect()
    }

    async fn get(&self, id: Uuid) -> Result<Option<TorProfile>> {
        let row = sqlx::query_as::<
            _,
            (
                String,
                String,
                i64,
                i64,
                String,
                String,
                i32,
                i64,
                i64,
                String,
                String,
            ),
        >(
            "SELECT id, name, control_port, socks_port, data_dir, bridge_ids_json, enabled, bootstrap_progress, circuit_count, created_at, updated_at FROM tor_profiles WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(|r| parse_row(r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8, r.9, r.10))
            .transpose()
    }

    async fn insert(&self, profile: &TorProfile) -> Result<()> {
        sqlx::query(
            "INSERT INTO tor_profiles (id, name, control_port, socks_port, data_dir, bridge_ids_json, enabled, bootstrap_progress, circuit_count, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(profile.id.to_string())
        .bind(&profile.name)
        .bind(profile.control_port as i64)
        .bind(profile.socks_port as i64)
        .bind(&profile.data_dir)
        .bind(serde_json::to_string(&profile.bridge_ids).unwrap_or_default())
        .bind(profile.enabled as i32)
        .bind(profile.bootstrap_progress as i64)
        .bind(profile.circuit_count as i64)
        .bind(profile.created_at.to_rfc3339())
        .bind(profile.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn update(&self, profile: &TorProfile) -> Result<()> {
        sqlx::query(
            "UPDATE tor_profiles SET name = ?, control_port = ?, socks_port = ?, data_dir = ?, bridge_ids_json = ?, enabled = ?, bootstrap_progress = ?, circuit_count = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&profile.name)
        .bind(profile.control_port as i64)
        .bind(profile.socks_port as i64)
        .bind(&profile.data_dir)
        .bind(serde_json::to_string(&profile.bridge_ids).unwrap_or_default())
        .bind(profile.enabled as i32)
        .bind(profile.bootstrap_progress as i64)
        .bind(profile.circuit_count as i64)
        .bind(profile.updated_at.to_rfc3339())
        .bind(profile.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let r = sqlx::query("DELETE FROM tor_profiles WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(r.rows_affected() > 0)
    }
}
