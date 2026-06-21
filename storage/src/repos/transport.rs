use super::traits::{Result, TransportProfileRepository};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{TransportProfile, TransportProfileKind, WireSentinelError};
use sqlx::SqlitePool;
use std::path::PathBuf;
use uuid::Uuid;

pub struct SqliteTransportProfileRepository {
    pool: SqlitePool,
}

impl SqliteTransportProfileRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn kind_str(k: TransportProfileKind) -> &'static str {
    match k {
        TransportProfileKind::SingBox => "sing_box",
        TransportProfileKind::Xray => "xray",
    }
}

fn kind_from_str(s: &str) -> Result<TransportProfileKind> {
    match s {
        "sing_box" => Ok(TransportProfileKind::SingBox),
        "xray" => Ok(TransportProfileKind::Xray),
        other => Err(WireSentinelError::Config(format!("unknown transport kind: {other}"))),
    }
}

fn parse_row(
    id: String,
    name: String,
    transport_kind: String,
    config_json: Option<String>,
    config_path: Option<String>,
    binary_path: Option<String>,
    enabled: i32,
    created_at: String,
    updated_at: String,
) -> Result<TransportProfile> {
    Ok(TransportProfile {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        name,
        transport_kind: kind_from_str(&transport_kind)?,
        config_json,
        config_path: config_path.map(PathBuf::from),
        binary_path: binary_path.map(PathBuf::from),
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
impl TransportProfileRepository for SqliteTransportProfileRepository {
    async fn list(&self) -> Result<Vec<TransportProfile>> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                i32,
                String,
                String,
            ),
        >(
            "SELECT id, name, transport_kind, config_json, config_path, binary_path, enabled, created_at, updated_at FROM transport_profiles ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(|r| parse_row(r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8))
            .collect()
    }

    async fn get(&self, id: Uuid) -> Result<Option<TransportProfile>> {
        Ok(self.list().await?.into_iter().find(|p| p.id == id))
    }

    async fn insert(&self, profile: &TransportProfile) -> Result<()> {
        sqlx::query(
            "INSERT INTO transport_profiles (id, name, transport_kind, config_json, config_path, binary_path, enabled, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(profile.id.to_string())
        .bind(&profile.name)
        .bind(kind_str(profile.transport_kind))
        .bind(&profile.config_json)
        .bind(profile.config_path.as_ref().map(|p| p.to_string_lossy().to_string()))
        .bind(profile.binary_path.as_ref().map(|p| p.to_string_lossy().to_string()))
        .bind(profile.enabled as i32)
        .bind(profile.created_at.to_rfc3339())
        .bind(profile.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn update(&self, profile: &TransportProfile) -> Result<()> {
        sqlx::query(
            "UPDATE transport_profiles SET name = ?, transport_kind = ?, config_json = ?, config_path = ?, binary_path = ?, enabled = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&profile.name)
        .bind(kind_str(profile.transport_kind))
        .bind(&profile.config_json)
        .bind(profile.config_path.as_ref().map(|p| p.to_string_lossy().to_string()))
        .bind(profile.binary_path.as_ref().map(|p| p.to_string_lossy().to_string()))
        .bind(profile.enabled as i32)
        .bind(profile.updated_at.to_rfc3339())
        .bind(profile.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let r = sqlx::query("DELETE FROM transport_profiles WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(r.rows_affected() > 0)
    }
}
