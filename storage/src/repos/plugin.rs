use super::traits::{PluginRepository, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{PluginFormat, PluginManifest, PluginRecord, PluginState, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqlitePluginRepository {
    pool: SqlitePool,
}

impl SqlitePluginRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PluginRepository for SqlitePluginRepository {
    async fn upsert(&self, record: &PluginRecord) -> Result<()> {
        sqlx::query(
            "INSERT INTO plugins (id, name, version, format, manifest_json, state, permissions_json, wasm_path, sha256, error_message, installed_at, loaded_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
               name = excluded.name,
               version = excluded.version,
               format = excluded.format,
               manifest_json = excluded.manifest_json,
               state = excluded.state,
               permissions_json = excluded.permissions_json,
               wasm_path = excluded.wasm_path,
               sha256 = excluded.sha256,
               error_message = excluded.error_message,
               loaded_at = excluded.loaded_at",
        )
        .bind(record.id.to_string())
        .bind(&record.manifest.name)
        .bind(&record.manifest.version)
        .bind(format_str(record.manifest.format))
        .bind(serde_json::to_string(&record.manifest).unwrap_or_default())
        .bind(state_str(record.state))
        .bind(serde_json::to_string(&record.manifest.permissions).unwrap_or_default())
        .bind(&record.manifest.path)
        .bind(&record.manifest.sha256)
        .bind(&record.error_message)
        .bind(record.installed_at.to_rfc3339())
        .bind(record.loaded_at.map(|t| t.to_rfc3339()))
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<PluginRecord>> {
        let rows: Vec<(String, String, String, String, String, String, Option<String>, Option<String>, Option<String>, Option<String>, String, Option<String>)> =
            sqlx::query_as(
                "SELECT id, name, version, format, manifest_json, state, permissions_json, wasm_path, sha256, error_message, installed_at, loaded_at FROM plugins ORDER BY installed_at DESC",
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        rows.into_iter().map(parse_row).collect()
    }

    async fn get(&self, id: Uuid) -> Result<Option<PluginRecord>> {
        let row: Option<(String, String, String, String, String, String, Option<String>, Option<String>, Option<String>, Option<String>, String, Option<String>)> =
            sqlx::query_as(
                "SELECT id, name, version, format, manifest_json, state, permissions_json, wasm_path, sha256, error_message, installed_at, loaded_at FROM plugins WHERE id = ?",
            )
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        row.map(parse_row).transpose()
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM plugins WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }
}

fn format_str(f: PluginFormat) -> &'static str {
    match f {
        PluginFormat::Wasm => "wasm",
        PluginFormat::Native => "native",
    }
}

fn state_str(s: PluginState) -> &'static str {
    match s {
        PluginState::Installed => "installed",
        PluginState::Loaded => "loaded",
        PluginState::Failed => "failed",
        PluginState::Unloaded => "unloaded",
    }
}

fn parse_row(
    row: (
        String,
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
        Option<String>,
    ),
) -> Result<PluginRecord> {
    let (
        id,
        _name,
        _version,
        _format,
        manifest_json,
        state,
        _perms,
        _path,
        _sha,
        error,
        installed_at,
        loaded_at,
    ) = row;
    let manifest: PluginManifest = serde_json::from_str(&manifest_json)
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
    let state = match state.as_str() {
        "loaded" => PluginState::Loaded,
        "failed" => PluginState::Failed,
        "unloaded" => PluginState::Unloaded,
        _ => PluginState::Installed,
    };
    Ok(PluginRecord {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        manifest,
        state,
        error_message: error,
        installed_at: DateTime::parse_from_rfc3339(&installed_at)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
        loaded_at: loaded_at
            .map(|s| DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&Utc)))
            .transpose()
            .map_err(|e| WireSentinelError::Config(e.to_string()))?,
    })
}
