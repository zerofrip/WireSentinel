use super::traits::{MixnetProfileRepository, MixnetSessionRepository, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{
    MixnetProfile, MixnetProvider, MixnetRoute, MixnetSession, TransportState, WireSentinelError,
};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteMixnetProfileRepository {
    pool: SqlitePool,
}

impl SqliteMixnetProfileRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn provider_to_parts(provider: &MixnetProvider) -> (&'static str, Option<String>) {
    match provider {
        MixnetProvider::Nym => ("nym", None),
        MixnetProvider::Plugin(id) => ("plugin", Some(id.to_string())),
    }
}

fn provider_from_parts(provider: &str, plugin_id: Option<String>) -> Result<MixnetProvider> {
    match provider {
        "plugin" => {
            let id = plugin_id.ok_or_else(|| {
                WireSentinelError::Config("mixnet profile missing plugin_id".into())
            })?;
            Ok(MixnetProvider::Plugin(
                Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
            ))
        }
        _ => Ok(MixnetProvider::Nym),
    }
}

type MixnetProfileRow = (
    String,
    String,
    String,
    Option<String>,
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

const MIXNET_PROFILE_SELECT: &str = "SELECT id, name, provider, plugin_id, gateway_id, config_json, enabled, active, latency_ms, last_health_at, last_error, created_at, updated_at FROM mixnet_profiles";

fn parse_profile_row(
    id: String,
    name: String,
    provider: String,
    plugin_id: Option<String>,
    gateway_id: Option<String>,
    config_json: Option<String>,
    enabled: i32,
    active: i32,
    latency_ms: Option<i64>,
    last_health_at: Option<String>,
    last_error: Option<String>,
    created_at: String,
    updated_at: String,
) -> Result<MixnetProfile> {
    let config_json = config_json
        .map(|s| serde_json::from_str(&s))
        .transpose()
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
    Ok(MixnetProfile {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        name,
        provider: provider_from_parts(&provider, plugin_id)?,
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
impl MixnetProfileRepository for SqliteMixnetProfileRepository {
    async fn list(&self) -> Result<Vec<MixnetProfile>> {
        let rows = sqlx::query_as::<_, MixnetProfileRow>(&format!(
            "{MIXNET_PROFILE_SELECT} ORDER BY name"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(|r| {
                parse_profile_row(
                    r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8, r.9, r.10, r.11, r.12,
                )
            })
            .collect()
    }

    async fn get(&self, id: Uuid) -> Result<Option<MixnetProfile>> {
        let row = sqlx::query_as::<_, MixnetProfileRow>(&format!(
            "{MIXNET_PROFILE_SELECT} WHERE id = ?"
        ))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(|r| {
            parse_profile_row(
                r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8, r.9, r.10, r.11, r.12,
            )
        })
        .transpose()
    }

    async fn insert(&self, profile: &MixnetProfile) -> Result<()> {
        let (provider, plugin_id) = provider_to_parts(&profile.provider);
        sqlx::query(
            "INSERT INTO mixnet_profiles (id, name, provider, plugin_id, gateway_id, config_json, enabled, active, latency_ms, last_health_at, last_error, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(profile.id.to_string())
        .bind(&profile.name)
        .bind(provider)
        .bind(plugin_id)
        .bind(&profile.gateway_id)
        .bind(
            profile
                .config_json
                .as_ref()
                .map(|v| serde_json::to_string(v))
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

    async fn update(&self, profile: &MixnetProfile) -> Result<()> {
        let (provider, plugin_id) = provider_to_parts(&profile.provider);
        sqlx::query(
            "UPDATE mixnet_profiles SET name = ?, provider = ?, plugin_id = ?, gateway_id = ?, config_json = ?, enabled = ?, active = ?, latency_ms = ?, last_health_at = ?, last_error = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&profile.name)
        .bind(provider)
        .bind(plugin_id)
        .bind(&profile.gateway_id)
        .bind(
            profile
                .config_json
                .as_ref()
                .map(|v| serde_json::to_string(v))
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
        let r = sqlx::query("DELETE FROM mixnet_profiles WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(r.rows_affected() > 0)
    }
}

pub struct SqliteMixnetSessionRepository {
    pool: SqlitePool,
}

impl SqliteMixnetSessionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn state_str(state: TransportState) -> &'static str {
    match state {
        TransportState::Stopped => "stopped",
        TransportState::Starting => "starting",
        TransportState::Running => "running",
        TransportState::Stopping => "stopping",
        TransportState::Error => "error",
    }
}

fn state_from_str(s: &str) -> TransportState {
    match s {
        "starting" => TransportState::Starting,
        "running" => TransportState::Running,
        "stopping" => TransportState::Stopping,
        "error" => TransportState::Error,
        _ => TransportState::Stopped,
    }
}

type MixnetSessionRow = (
    String,
    String,
    Option<String>,
    String,
    String,
    Option<String>,
    i64,
    i64,
);

const MIXNET_SESSION_SELECT: &str =
    "SELECT id, profile_id, route_json, state, started_at, ended_at, rx_bytes, tx_bytes FROM mixnet_sessions";

fn parse_session_row(
    id: String,
    profile_id: String,
    route_json: Option<String>,
    state: String,
    started_at: String,
    ended_at: Option<String>,
    rx_bytes: i64,
    tx_bytes: i64,
) -> Result<MixnetSession> {
    let route = route_json
        .map(|s| serde_json::from_str::<MixnetRoute>(&s))
        .transpose()
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
    Ok(MixnetSession {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        profile_id: Uuid::parse_str(&profile_id)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?,
        route,
        state: state_from_str(&state),
        started_at: DateTime::parse_from_rfc3339(&started_at)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
        ended_at: ended_at
            .map(|s| DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&Utc)))
            .transpose()
            .map_err(|e| WireSentinelError::Config(e.to_string()))?,
        rx_bytes: rx_bytes as u64,
        tx_bytes: tx_bytes as u64,
    })
}

#[async_trait]
impl MixnetSessionRepository for SqliteMixnetSessionRepository {
    async fn insert(&self, session: &MixnetSession) -> Result<()> {
        let route_json = session
            .route
            .as_ref()
            .map(|r| serde_json::to_string(r))
            .transpose()
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        sqlx::query(
            "INSERT INTO mixnet_sessions (id, profile_id, route_json, state, started_at, ended_at, rx_bytes, tx_bytes) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(session.id.to_string())
        .bind(session.profile_id.to_string())
        .bind(route_json)
        .bind(state_str(session.state))
        .bind(session.started_at.to_rfc3339())
        .bind(session.ended_at.map(|t| t.to_rfc3339()))
        .bind(session.rx_bytes as i64)
        .bind(session.tx_bytes as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn update(&self, session: &MixnetSession) -> Result<()> {
        let route_json = session
            .route
            .as_ref()
            .map(|r| serde_json::to_string(r))
            .transpose()
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        sqlx::query(
            "UPDATE mixnet_sessions SET profile_id = ?, route_json = ?, state = ?, started_at = ?, ended_at = ?, rx_bytes = ?, tx_bytes = ? WHERE id = ?",
        )
        .bind(session.profile_id.to_string())
        .bind(route_json)
        .bind(state_str(session.state))
        .bind(session.started_at.to_rfc3339())
        .bind(session.ended_at.map(|t| t.to_rfc3339()))
        .bind(session.rx_bytes as i64)
        .bind(session.tx_bytes as i64)
        .bind(session.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn get(&self, id: Uuid) -> Result<Option<MixnetSession>> {
        let row = sqlx::query_as::<_, MixnetSessionRow>(&format!(
            "{MIXNET_SESSION_SELECT} WHERE id = ?"
        ))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(|r| parse_session_row(r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7))
            .transpose()
    }

    async fn list_recent(&self, limit: u32) -> Result<Vec<MixnetSession>> {
        let rows = sqlx::query_as::<_, MixnetSessionRow>(&format!(
            "{MIXNET_SESSION_SELECT} ORDER BY started_at DESC LIMIT ?"
        ))
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(|r| parse_session_row(r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7))
            .collect()
    }

    async fn list_by_profile(&self, profile_id: Uuid, limit: u32) -> Result<Vec<MixnetSession>> {
        let rows = sqlx::query_as::<_, MixnetSessionRow>(&format!(
            "{MIXNET_SESSION_SELECT} WHERE profile_id = ? ORDER BY started_at DESC LIMIT ?"
        ))
        .bind(profile_id.to_string())
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(|r| parse_session_row(r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7))
            .collect()
    }
}
