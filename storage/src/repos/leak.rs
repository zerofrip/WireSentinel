use super::traits::{LeakIncidentRepository, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{LeakIncident, LeakType, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteLeakIncidentRepository {
    pool: SqlitePool,
}

impl SqliteLeakIncidentRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn leak_type_str(t: LeakType) -> &'static str {
    match t {
        LeakType::Dns => "dns",
        LeakType::Route => "route",
        LeakType::VpnDisconnect => "vpn_disconnect",
    }
}

fn leak_type_from_str(s: &str) -> Result<LeakType> {
    match s {
        "dns" => Ok(LeakType::Dns),
        "route" => Ok(LeakType::Route),
        "vpn_disconnect" => Ok(LeakType::VpnDisconnect),
        other => Err(WireSentinelError::Config(format!("unknown leak type: {other}"))),
    }
}

#[async_trait]
impl LeakIncidentRepository for SqliteLeakIncidentRepository {
    async fn insert(&self, incident: &LeakIncident) -> Result<()> {
        sqlx::query(
            "INSERT INTO leak_incidents (id, leak_type, app_id, detail_json, severity, detected_at, resolved_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(incident.id.to_string())
        .bind(leak_type_str(incident.leak_type))
        .bind(incident.app_id.map(|id| id.to_string()))
        .bind(&incident.detail_json)
        .bind(&incident.severity)
        .bind(incident.detected_at.to_rfc3339())
        .bind(incident.resolved_at.map(|t| t.to_rfc3339()))
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn list_recent(&self, limit: u32) -> Result<Vec<LeakIncident>> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                Option<String>,
                Option<String>,
                String,
                String,
                Option<String>,
            ),
        >(
            "SELECT id, leak_type, app_id, detail_json, severity, detected_at, resolved_at FROM leak_incidents ORDER BY detected_at DESC LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(
                |(id, leak_type, app_id, detail_json, severity, detected_at, resolved_at)| {
                    Ok(LeakIncident {
                        id: Uuid::parse_str(&id)
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?,
                        leak_type: leak_type_from_str(&leak_type)?,
                        app_id: app_id
                            .map(|s| Uuid::parse_str(&s))
                            .transpose()
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?,
                        detail_json,
                        severity,
                        detected_at: DateTime::parse_from_rfc3339(&detected_at)
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?
                            .with_timezone(&Utc),
                        resolved_at: resolved_at
                            .map(|s| DateTime::parse_from_rfc3339(&s))
                            .transpose()
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?
                            .map(|d| d.with_timezone(&Utc)),
                    })
                },
            )
            .collect()
    }

    async fn resolve(&self, id: Uuid) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let r = sqlx::query("UPDATE leak_incidents SET resolved_at = ? WHERE id = ? AND resolved_at IS NULL")
            .bind(now)
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(r.rows_affected() > 0)
    }
}
