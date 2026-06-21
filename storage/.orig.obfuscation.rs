use super::traits::{ObfuscationProfileRepository, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{HandshakeProxySettings, ObfuscationPreset, ObfuscationProfile, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteObfuscationProfileRepository {
    pool: SqlitePool,
}

impl SqliteObfuscationProfileRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn preset_str(p: ObfuscationPreset) -> &'static str {
    match p {
        ObfuscationPreset::Disabled => "disabled",
        ObfuscationPreset::Basic => "basic",
        ObfuscationPreset::Balanced => "balanced",
        ObfuscationPreset::Aggressive => "aggressive",
        ObfuscationPreset::Lwo => "lwo",
        ObfuscationPreset::Socks5Handshake => "socks5_handshake",
    }
}

fn preset_from_str(s: &str) -> Result<ObfuscationPreset> {
    match s {
        "disabled" => Ok(ObfuscationPreset::Disabled),
        "basic" => Ok(ObfuscationPreset::Basic),
        "balanced" => Ok(ObfuscationPreset::Balanced),
        "aggressive" => Ok(ObfuscationPreset::Aggressive),
        "lwo" => Ok(ObfuscationPreset::Lwo),
        "socks5_handshake" => Ok(ObfuscationPreset::Socks5Handshake),
        other => Err(WireSentinelError::Config(format!("unknown preset: {other}"))),
    }
}

#[async_trait]
impl ObfuscationProfileRepository for SqliteObfuscationProfileRepository {
    async fn list(&self) -> Result<Vec<ObfuscationProfile>> {
        let rows = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>, String, String)>(
            "SELECT id, name, preset, modules_json, handshake_proxy_json, created_at, updated_at FROM obfuscation_profiles ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(|(id, name, preset, modules_json, handshake_proxy_json, created_at, updated_at)| {
                let handshake_proxy = handshake_proxy_json
                    .map(|j| serde_json::from_str::<HandshakeProxySettings>(&j))
                    .transpose()
                    .map_err(WireSentinelError::Serde)?;
                Ok(ObfuscationProfile {
                    id: Uuid::parse_str(&id)
                        .map_err(|e| WireSentinelError::Config(e.to_string()))?,
                    name,
                    preset: preset_from_str(&preset)?,
                    modules_json,
                    handshake_proxy,
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .map_err(|e| WireSentinelError::Config(e.to_string()))?
                        .with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&updated_at)
                        .map_err(|e| WireSentinelError::Config(e.to_string()))?
                        .with_timezone(&Utc),
                })
            })
            .collect()
    }

    async fn get(&self, id: Uuid) -> Result<Option<ObfuscationProfile>> {
        Ok(self.list().await?.into_iter().find(|p| p.id == id))
    }

    async fn insert(&self, profile: &ObfuscationProfile) -> Result<()> {
        sqlx::query(
            "INSERT INTO obfuscation_profiles (id, name, preset, modules_json, handshake_proxy_json, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(profile.id.to_string())
        .bind(&profile.name)
        .bind(preset_str(profile.preset))
        .bind(&profile.modules_json)
        .bind(
            profile
                .handshake_proxy
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(WireSentinelError::Serde)?,
        )
        .bind(profile.created_at.to_rfc3339())
        .bind(profile.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn update(&self, profile: &ObfuscationProfile) -> Result<()> {
        sqlx::query(
            "UPDATE obfuscation_profiles SET name = ?, preset = ?, modules_json = ?, handshake_proxy_json = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&profile.name)
        .bind(preset_str(profile.preset))
        .bind(&profile.modules_json)
        .bind(
            profile
                .handshake_proxy
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(WireSentinelError::Serde)?,
        )
        .bind(profile.updated_at.to_rfc3339())
        .bind(profile.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let r = sqlx::query("DELETE FROM obfuscation_profiles WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(r.rows_affected() > 0)
    }
}
