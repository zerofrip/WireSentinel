use super::traits::{Result, TailnetProfileRepository};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{TailnetProfile, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteTailnetProfileRepository {
    pool: SqlitePool,
}

impl SqliteTailnetProfileRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn parse_row(
    id: String,
    name: String,
    auth_key: Option<String>,
    exit_node: Option<String>,
    subnet_router: i32,
    magic_dns: i32,
    hostname: Option<String>,
    tailnet_ip: Option<String>,
    connected: i32,
    created_at: String,
    updated_at: String,
) -> Result<TailnetProfile> {
    Ok(TailnetProfile {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        name,
        auth_key,
        exit_node,
        subnet_router: subnet_router != 0,
        magic_dns: magic_dns != 0,
        hostname,
        tailnet_ip,
        connected: connected != 0,
        created_at: DateTime::parse_from_rfc3339(&created_at)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
    })
}

#[async_trait]
impl TailnetProfileRepository for SqliteTailnetProfileRepository {
    async fn list(&self) -> Result<Vec<TailnetProfile>> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                Option<String>,
                Option<String>,
                i32,
                i32,
                Option<String>,
                Option<String>,
                i32,
                String,
                String,
            ),
        >(
            "SELECT id, name, auth_key, exit_node, subnet_router, magic_dns, hostname, tailnet_ip, connected, created_at, updated_at FROM tailnet_profiles ORDER BY name",
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

    async fn get(&self, id: Uuid) -> Result<Option<TailnetProfile>> {
        let row = sqlx::query_as::<
            _,
            (
                String,
                String,
                Option<String>,
                Option<String>,
                i32,
                i32,
                Option<String>,
                Option<String>,
                i32,
                String,
                String,
            ),
        >(
            "SELECT id, name, auth_key, exit_node, subnet_router, magic_dns, hostname, tailnet_ip, connected, created_at, updated_at FROM tailnet_profiles WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(|r| parse_row(r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8, r.9, r.10))
            .transpose()
    }

    async fn insert(&self, profile: &TailnetProfile) -> Result<()> {
        sqlx::query(
            "INSERT INTO tailnet_profiles (id, name, auth_key, exit_node, subnet_router, magic_dns, hostname, tailnet_ip, connected, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(profile.id.to_string())
        .bind(&profile.name)
        .bind(&profile.auth_key)
        .bind(&profile.exit_node)
        .bind(profile.subnet_router as i32)
        .bind(profile.magic_dns as i32)
        .bind(&profile.hostname)
        .bind(&profile.tailnet_ip)
        .bind(profile.connected as i32)
        .bind(profile.created_at.to_rfc3339())
        .bind(profile.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn update(&self, profile: &TailnetProfile) -> Result<()> {
        sqlx::query(
            "UPDATE tailnet_profiles SET name = ?, auth_key = ?, exit_node = ?, subnet_router = ?, magic_dns = ?, hostname = ?, tailnet_ip = ?, connected = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&profile.name)
        .bind(&profile.auth_key)
        .bind(&profile.exit_node)
        .bind(profile.subnet_router as i32)
        .bind(profile.magic_dns as i32)
        .bind(&profile.hostname)
        .bind(&profile.tailnet_ip)
        .bind(profile.connected as i32)
        .bind(profile.updated_at.to_rfc3339())
        .bind(profile.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let r = sqlx::query("DELETE FROM tailnet_profiles WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(r.rows_affected() > 0)
    }
}
