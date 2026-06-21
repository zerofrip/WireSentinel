use super::traits::{CoverTrafficRepository, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{CoverTrafficProfile, CoverTrafficSettings, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteCoverTrafficRepository {
    pool: SqlitePool,
}

impl SqliteCoverTrafficRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn profile_str(profile: CoverTrafficProfile) -> &'static str {
    match profile {
        CoverTrafficProfile::Disabled => "disabled",
        CoverTrafficProfile::Low => "low",
        CoverTrafficProfile::Medium => "medium",
        CoverTrafficProfile::High => "high",
        CoverTrafficProfile::Maximum => "maximum",
    }
}

fn profile_from_str(s: &str) -> CoverTrafficProfile {
    match s {
        "low" => CoverTrafficProfile::Low,
        "medium" => CoverTrafficProfile::Medium,
        "high" => CoverTrafficProfile::High,
        "maximum" => CoverTrafficProfile::Maximum,
        _ => CoverTrafficProfile::Disabled,
    }
}

type CoverTrafficRow = (
    String,
    Option<String>,
    String,
    i32,
    Option<i64>,
    String,
    String,
);

const COVER_TRAFFIC_SELECT: &str = "SELECT id, mixnet_profile_id, cover_profile, enabled, rate_bps, created_at, updated_at FROM cover_traffic_settings";

fn parse_row(
    id: String,
    mixnet_profile_id: Option<String>,
    cover_profile: String,
    enabled: i32,
    rate_bps: Option<i64>,
    created_at: String,
    updated_at: String,
) -> Result<CoverTrafficSettings> {
    Ok(CoverTrafficSettings {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        mixnet_profile_id: mixnet_profile_id
            .map(|s| Uuid::parse_str(&s))
            .transpose()
            .map_err(|e| WireSentinelError::Config(e.to_string()))?,
        profile: profile_from_str(&cover_profile),
        enabled: enabled != 0,
        rate_bps: rate_bps.map(|v| v as u64),
        created_at: DateTime::parse_from_rfc3339(&created_at)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
    })
}

#[async_trait]
impl CoverTrafficRepository for SqliteCoverTrafficRepository {
    async fn list(&self) -> Result<Vec<CoverTrafficSettings>> {
        let rows = sqlx::query_as::<_, CoverTrafficRow>(&format!(
            "{COVER_TRAFFIC_SELECT} ORDER BY created_at"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(|r| parse_row(r.0, r.1, r.2, r.3, r.4, r.5, r.6))
            .collect()
    }

    async fn get(&self, id: Uuid) -> Result<Option<CoverTrafficSettings>> {
        let row =
            sqlx::query_as::<_, CoverTrafficRow>(&format!("{COVER_TRAFFIC_SELECT} WHERE id = ?"))
                .bind(id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(|r| parse_row(r.0, r.1, r.2, r.3, r.4, r.5, r.6))
            .transpose()
    }

    async fn get_by_mixnet_profile(
        &self,
        profile_id: Uuid,
    ) -> Result<Option<CoverTrafficSettings>> {
        let row = sqlx::query_as::<_, CoverTrafficRow>(&format!(
            "{COVER_TRAFFIC_SELECT} WHERE mixnet_profile_id = ?"
        ))
        .bind(profile_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(|r| parse_row(r.0, r.1, r.2, r.3, r.4, r.5, r.6))
            .transpose()
    }

    async fn insert(&self, settings: &CoverTrafficSettings) -> Result<()> {
        sqlx::query(
            "INSERT INTO cover_traffic_settings (id, mixnet_profile_id, cover_profile, enabled, rate_bps, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(settings.id.to_string())
        .bind(settings.mixnet_profile_id.map(|id| id.to_string()))
        .bind(profile_str(settings.profile))
        .bind(settings.enabled as i32)
        .bind(settings.rate_bps.map(|v| v as i64))
        .bind(settings.created_at.to_rfc3339())
        .bind(settings.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn update(&self, settings: &CoverTrafficSettings) -> Result<()> {
        sqlx::query(
            "UPDATE cover_traffic_settings SET mixnet_profile_id = ?, cover_profile = ?, enabled = ?, rate_bps = ?, updated_at = ? WHERE id = ?",
        )
        .bind(settings.mixnet_profile_id.map(|id| id.to_string()))
        .bind(profile_str(settings.profile))
        .bind(settings.enabled as i32)
        .bind(settings.rate_bps.map(|v| v as i64))
        .bind(settings.updated_at.to_rfc3339())
        .bind(settings.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let r = sqlx::query("DELETE FROM cover_traffic_settings WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(r.rows_affected() > 0)
    }
}
