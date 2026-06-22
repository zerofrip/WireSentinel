use super::traits::{KatzenpostProfileRepository, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{KatzenpostProfile, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteKatzenpostProfileRepository {
    pool: SqlitePool,
}

impl SqliteKatzenpostProfileRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

type KatzenpostProfileRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    i32,
    i32,
    Option<i64>,
    Option<String>,
    Option<String>,
    String,
    String,
);

const KATZENPOST_PROFILE_SELECT: &str = "SELECT id, name, gateway_id, config_json, enabled, active, latency_ms, last_health_at, last_error, created_at, updated_at FROM katzenpost_profiles";

fn parse_profile_row(
    id: String,
    name: String,
    gateway_id: Option<String>,
    config_json: Option<String>,
    enabled: i32,
    active: i32,
    latency_ms: Option<i64>,
    last_health_at: Option<String>,
    last_error: Option<String>,
    created_at: String,
    updated_at: String,
) -> Result<KatzenpostProfile> {
    let config_json = config_json
        .map(|s| serde_json::from_str(&s))
        .transpose()
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
    Ok(KatzenpostProfile {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        name,
        gateway_id,
        config_json,
        enabled: enabled != 0,
        active: active != 0,
        latency_ms: latency_ms.map(|v| v as u64),
        last_health_at: last_health_at
            .map(|s| DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&Utc)))
            .transpose()
            .map_err(|e| WireSentinelError::Config(e.to_string()))?,
        last_error,
        created_at: DateTime::parse_from_rfc3339(&created_at)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
    })
}

#[async_trait]
impl KatzenpostProfileRepository for SqliteKatzenpostProfileRepository {
    async fn list(&self) -> Result<Vec<KatzenpostProfile>> {
        let rows = sqlx::query_as::<_, KatzenpostProfileRow>(&format!(
            "{KATZENPOST_PROFILE_SELECT} ORDER BY name"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(|r| parse_profile_row(r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8, r.9, r.10))
            .collect()
    }

    async fn get(&self, id: Uuid) -> Result<Option<KatzenpostProfile>> {
        let row = sqlx::query_as::<_, KatzenpostProfileRow>(&format!(
            "{KATZENPOST_PROFILE_SELECT} WHERE id = ?"
        ))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(|r| parse_profile_row(r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8, r.9, r.10))
            .transpose()
    }

    async fn insert(&self, profile: &KatzenpostProfile) -> Result<()> {
        sqlx::query(
            "INSERT INTO katzenpost_profiles (id, name, gateway_id, config_json, enabled, active, latency_ms, last_health_at, last_error, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(profile.id.to_string())
        .bind(&profile.name)
        .bind(&profile.gateway_id)
        .bind(
            profile
                .config_json
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(|e| WireSentinelError::Config(e.to_string()))?,
        )
        .bind(profile.enabled as i32)
        .bind(profile.active as i32)
        .bind(profile.latency_ms.map(|v| v as i64))
        .bind(profile.last_health_at.map(|t| t.to_rfc3339()))
        .bind(&profile.last_error)
        .bind(profile.created_at.to_rfc3339())
        .bind(profile.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn update(&self, profile: &KatzenpostProfile) -> Result<()> {
        sqlx::query(
            "UPDATE katzenpost_profiles SET name = ?, gateway_id = ?, config_json = ?, enabled = ?, active = ?, latency_ms = ?, last_health_at = ?, last_error = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&profile.name)
        .bind(&profile.gateway_id)
        .bind(
            profile
                .config_json
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(|e| WireSentinelError::Config(e.to_string()))?,
        )
        .bind(profile.enabled as i32)
        .bind(profile.active as i32)
        .bind(profile.latency_ms.map(|v| v as i64))
        .bind(profile.last_health_at.map(|t| t.to_rfc3339()))
        .bind(&profile.last_error)
        .bind(profile.updated_at.to_rfc3339())
        .bind(profile.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let r = sqlx::query("DELETE FROM katzenpost_profiles WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(r.rows_affected() > 0)
    }
}
