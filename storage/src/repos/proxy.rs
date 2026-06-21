use super::traits::{ProxyProfileRepository, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{ProxyKind, ProxyProfile, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteProxyProfileRepository {
    pool: SqlitePool,
}

impl SqliteProxyProfileRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn kind_str(k: ProxyKind) -> &'static str {
    match k {
        ProxyKind::Socks5 => "socks5",
        ProxyKind::Http => "http",
        ProxyKind::Https => "https",
    }
}

fn kind_from_str(s: &str) -> ProxyKind {
    match s {
        "http" => ProxyKind::Http,
        "https" => ProxyKind::Https,
        _ => ProxyKind::Socks5,
    }
}

fn parse_row(
    id: String,
    name: String,
    kind: String,
    host: String,
    port: i64,
    username: Option<String>,
    password_encrypted: Option<String>,
    enabled: i32,
    active: i32,
    latency_ms: Option<i64>,
    last_health_at: Option<String>,
    last_error: Option<String>,
    created_at: String,
    updated_at: String,
) -> Result<ProxyProfile> {
    Ok(ProxyProfile {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        name,
        kind: kind_from_str(&kind),
        host,
        port: port as u16,
        username,
        password_encrypted,
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

type ProxyRow = (
    String,
    String,
    String,
    String,
    i64,
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

const PROXY_SELECT: &str = "SELECT id, name, kind, host, port, username, password_encrypted, enabled, active, latency_ms, last_health_at, last_error, created_at, updated_at FROM proxy_profiles";

#[async_trait]
impl ProxyProfileRepository for SqliteProxyProfileRepository {
    async fn list(&self) -> Result<Vec<ProxyProfile>> {
        let rows = sqlx::query_as::<_, ProxyRow>(&format!("{PROXY_SELECT} ORDER BY name"))
            .fetch_all(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(|r| {
                parse_row(
                    r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8, r.9, r.10, r.11, r.12, r.13,
                )
            })
            .collect()
    }

    async fn get(&self, id: Uuid) -> Result<Option<ProxyProfile>> {
        let row = sqlx::query_as::<_, ProxyRow>(&format!("{PROXY_SELECT} WHERE id = ?"))
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(|r| {
            parse_row(
                r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8, r.9, r.10, r.11, r.12, r.13,
            )
        })
        .transpose()
    }

    async fn insert(&self, profile: &ProxyProfile) -> Result<()> {
        sqlx::query(
            "INSERT INTO proxy_profiles (id, name, kind, host, port, username, password_encrypted, enabled, active, latency_ms, last_health_at, last_error, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(profile.id.to_string())
        .bind(&profile.name)
        .bind(kind_str(profile.kind))
        .bind(&profile.host)
        .bind(profile.port as i64)
        .bind(&profile.username)
        .bind(&profile.password_encrypted)
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

    async fn update(&self, profile: &ProxyProfile) -> Result<()> {
        sqlx::query(
            "UPDATE proxy_profiles SET name = ?, kind = ?, host = ?, port = ?, username = ?, password_encrypted = ?, enabled = ?, active = ?, latency_ms = ?, last_health_at = ?, last_error = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&profile.name)
        .bind(kind_str(profile.kind))
        .bind(&profile.host)
        .bind(profile.port as i64)
        .bind(&profile.username)
        .bind(&profile.password_encrypted)
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
        let r = sqlx::query("DELETE FROM proxy_profiles WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(r.rows_affected() > 0)
    }
}
