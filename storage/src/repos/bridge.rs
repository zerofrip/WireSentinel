use super::traits::{BridgeProfileRepository, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{BridgeProfile, BridgeType, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteBridgeProfileRepository {
    pool: SqlitePool,
}

impl SqliteBridgeProfileRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn bridge_type_str(t: BridgeType) -> &'static str {
    match t {
        BridgeType::Obfs4 => "obfs4",
        BridgeType::Snowflake => "snowflake",
        BridgeType::Meek => "meek",
        BridgeType::Webtunnel => "webtunnel",
    }
}

fn bridge_type_from_str(s: &str) -> BridgeType {
    match s {
        "snowflake" => BridgeType::Snowflake,
        "meek" => BridgeType::Meek,
        "webtunnel" => BridgeType::Webtunnel,
        _ => BridgeType::Obfs4,
    }
}

fn parse_row(
    id: String,
    name: String,
    bridge_type: String,
    config_json: String,
    enabled: i32,
    created_at: String,
    updated_at: String,
) -> Result<BridgeProfile> {
    let config_json: serde_json::Value =
        serde_json::from_str(&config_json).unwrap_or(serde_json::json!({}));
    Ok(BridgeProfile {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        name,
        bridge_type: bridge_type_from_str(&bridge_type),
        config_json,
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
impl BridgeProfileRepository for SqliteBridgeProfileRepository {
    async fn list(&self) -> Result<Vec<BridgeProfile>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, i32, String, String)>(
            "SELECT id, name, bridge_type, config_json, enabled, created_at, updated_at FROM bridge_profiles ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(|r| parse_row(r.0, r.1, r.2, r.3, r.4, r.5, r.6))
            .collect()
    }

    async fn get(&self, id: Uuid) -> Result<Option<BridgeProfile>> {
        let row = sqlx::query_as::<_, (String, String, String, String, i32, String, String)>(
            "SELECT id, name, bridge_type, config_json, enabled, created_at, updated_at FROM bridge_profiles WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(|r| parse_row(r.0, r.1, r.2, r.3, r.4, r.5, r.6))
            .transpose()
    }

    async fn insert(&self, profile: &BridgeProfile) -> Result<()> {
        sqlx::query(
            "INSERT INTO bridge_profiles (id, name, bridge_type, config_json, enabled, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(profile.id.to_string())
        .bind(&profile.name)
        .bind(bridge_type_str(profile.bridge_type))
        .bind(profile.config_json.to_string())
        .bind(profile.enabled as i32)
        .bind(profile.created_at.to_rfc3339())
        .bind(profile.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn update(&self, profile: &BridgeProfile) -> Result<()> {
        sqlx::query(
            "UPDATE bridge_profiles SET name = ?, bridge_type = ?, config_json = ?, enabled = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&profile.name)
        .bind(bridge_type_str(profile.bridge_type))
        .bind(profile.config_json.to_string())
        .bind(profile.enabled as i32)
        .bind(profile.updated_at.to_rfc3339())
        .bind(profile.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let r = sqlx::query("DELETE FROM bridge_profiles WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(r.rows_affected() > 0)
    }
}
