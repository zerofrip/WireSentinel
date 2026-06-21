use super::traits::{Result, TcpTerminationRepository};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{
    TcpTerminationMode, TcpTerminationPolicy, TcpTerminationRule, TcpTerminationSettings,
    TrafficRoute, WireSentinelError,
};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteTcpTerminationRepository {
    pool: SqlitePool,
}

impl SqliteTcpTerminationRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn mode_str(m: TcpTerminationMode) -> &'static str {
    match m {
        TcpTerminationMode::Disabled => "disabled",
        TcpTerminationMode::OnVpnConnect => "on_vpn_connect",
        TcpTerminationMode::OnVpnDisconnect => "on_vpn_disconnect",
        TcpTerminationMode::OnRouteChange => "on_route_change",
        TcpTerminationMode::Always => "always",
    }
}

fn mode_from_str(s: &str) -> Result<TcpTerminationMode> {
    match s {
        "disabled" => Ok(TcpTerminationMode::Disabled),
        "on_vpn_connect" => Ok(TcpTerminationMode::OnVpnConnect),
        "on_vpn_disconnect" => Ok(TcpTerminationMode::OnVpnDisconnect),
        "on_route_change" => Ok(TcpTerminationMode::OnRouteChange),
        "always" => Ok(TcpTerminationMode::Always),
        other => Err(WireSentinelError::Config(format!(
            "unknown tcp termination mode: {other}"
        ))),
    }
}

#[async_trait]
impl TcpTerminationRepository for SqliteTcpTerminationRepository {
    async fn get_settings(&self) -> Result<TcpTerminationSettings> {
        let row: Option<(String, String)> =
            sqlx::query_as("SELECT mode, updated_at FROM tcp_termination_settings WHERE id = 1")
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        match row {
            Some((mode, updated_at)) => Ok(TcpTerminationSettings {
                mode: mode_from_str(&mode)?,
                updated_at: DateTime::parse_from_rfc3339(&updated_at)
                    .map_err(|e| WireSentinelError::Config(e.to_string()))?
                    .with_timezone(&Utc),
            }),
            None => Ok(TcpTerminationSettings::default()),
        }
    }

    async fn set_settings(&self, settings: &TcpTerminationSettings) -> Result<()> {
        sqlx::query(
            "INSERT INTO tcp_termination_settings (id, mode, updated_at) VALUES (1, ?, ?)
             ON CONFLICT(id) DO UPDATE SET mode = excluded.mode, updated_at = excluded.updated_at",
        )
        .bind(mode_str(settings.mode))
        .bind(settings.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn list_rules(&self) -> Result<Vec<TcpTerminationRule>> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                i32,
                String,
                String,
            ),
        >(
            "SELECT id, process_path, process_name, profile_id, route_json, enabled, created_at, updated_at FROM tcp_termination_rules ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(
                |(
                    id,
                    process_path,
                    process_name,
                    profile_id,
                    route_json,
                    enabled,
                    created_at,
                    updated_at,
                )| {
                    let route = route_json
                        .map(|j| serde_json::from_str::<TrafficRoute>(&j))
                        .transpose()
                        .map_err(WireSentinelError::Serde)?;
                    Ok(TcpTerminationRule {
                        id: Uuid::parse_str(&id)
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?,
                        process_path,
                        process_name,
                        profile_id: profile_id
                            .map(|s| Uuid::parse_str(&s))
                            .transpose()
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?,
                        route,
                        enabled: enabled != 0,
                        created_at: DateTime::parse_from_rfc3339(&created_at)
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?
                            .with_timezone(&Utc),
                        updated_at: DateTime::parse_from_rfc3339(&updated_at)
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?
                            .with_timezone(&Utc),
                    })
                },
            )
            .collect()
    }

    async fn get_rule(&self, id: Uuid) -> Result<Option<TcpTerminationRule>> {
        Ok(self.list_rules().await?.into_iter().find(|r| r.id == id))
    }

    async fn insert_rule(&self, rule: &TcpTerminationRule) -> Result<()> {
        let route_json = rule
            .route
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(WireSentinelError::Serde)?;
        sqlx::query(
            "INSERT INTO tcp_termination_rules (id, process_path, process_name, profile_id, route_json, enabled, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(rule.id.to_string())
        .bind(&rule.process_path)
        .bind(&rule.process_name)
        .bind(rule.profile_id.map(|id| id.to_string()))
        .bind(route_json)
        .bind(rule.enabled as i32)
        .bind(rule.created_at.to_rfc3339())
        .bind(rule.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn update_rule(&self, rule: &TcpTerminationRule) -> Result<()> {
        let route_json = rule
            .route
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(WireSentinelError::Serde)?;
        sqlx::query(
            "UPDATE tcp_termination_rules SET process_path = ?, process_name = ?, profile_id = ?, route_json = ?, enabled = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&rule.process_path)
        .bind(&rule.process_name)
        .bind(rule.profile_id.map(|id| id.to_string()))
        .bind(route_json)
        .bind(rule.enabled as i32)
        .bind(rule.updated_at.to_rfc3339())
        .bind(rule.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn delete_rule(&self, id: Uuid) -> Result<bool> {
        let r = sqlx::query("DELETE FROM tcp_termination_rules WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(r.rows_affected() > 0)
    }

    async fn load_policy(&self) -> Result<TcpTerminationPolicy> {
        let settings = self.get_settings().await?;
        let rules = self.list_rules().await?;
        Ok(TcpTerminationPolicy {
            mode: settings.mode,
            rules,
        })
    }
}
