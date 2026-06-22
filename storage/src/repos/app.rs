use super::traits::{AppFilter, AppRepository, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{AppRecord, WireSentinelError};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct SqliteAppRepository {
    pool: SqlitePool,
}

impl SqliteAppRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn row_to_app(
    app_id: String,
    display_name: String,
    exe_path: String,
    publisher: Option<String>,
    sha256: Option<String>,
    icon_path: Option<String>,
    first_seen: String,
    last_seen: String,
    default_route_json: Option<String>,
) -> Result<AppRecord> {
    let default_route = default_route_json
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(serde_json::from_str)
        .transpose()
        .map_err(WireSentinelError::Serde)?;

    Ok(AppRecord {
        app_id: Uuid::parse_str(&app_id)
            .map_err(|e| WireSentinelError::Config(format!("invalid app_id: {e}")))?,
        display_name,
        exe_path: PathBuf::from(exe_path),
        publisher,
        sha256,
        icon_path: icon_path.map(PathBuf::from),
        first_seen: DateTime::parse_from_rfc3339(&first_seen)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
        last_seen: DateTime::parse_from_rfc3339(&last_seen)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
        default_route,
    })
}

#[async_trait]
impl AppRepository for SqliteAppRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<AppRecord>> {
        let row = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>, Option<String>, String, String, Option<String>)>(
            "SELECT app_id, display_name, exe_path, publisher, sha256, icon_path, first_seen, last_seen, default_route_json FROM apps WHERE app_id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(|r| row_to_app(r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8))
            .transpose()
    }

    async fn find_by_exe_path(&self, path: &Path) -> Result<Option<AppRecord>> {
        let path_str = path.to_string_lossy().to_string();
        let row = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>, Option<String>, String, String, Option<String>)>(
            "SELECT app_id, display_name, exe_path, publisher, sha256, icon_path, first_seen, last_seen, default_route_json FROM apps WHERE exe_path = ?",
        )
        .bind(path_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(|r| row_to_app(r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8))
            .transpose()
    }

    async fn find_by_sha256(&self, sha256: &str) -> Result<Option<AppRecord>> {
        let row = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>, Option<String>, String, String, Option<String>)>(
            "SELECT app_id, display_name, exe_path, publisher, sha256, icon_path, first_seen, last_seen, default_route_json FROM apps WHERE sha256 = ?",
        )
        .bind(sha256)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(|r| row_to_app(r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8))
            .transpose()
    }

    async fn upsert(&self, app: &AppRecord) -> Result<()> {
        let route_json = app
            .default_route
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(WireSentinelError::Serde)?;

        sqlx::query(
            r#"
            INSERT INTO apps (app_id, display_name, exe_path, publisher, sha256, icon_path, first_seen, last_seen, default_route_json)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(exe_path) DO UPDATE SET
                display_name = excluded.display_name,
                publisher = excluded.publisher,
                sha256 = excluded.sha256,
                icon_path = excluded.icon_path,
                last_seen = excluded.last_seen,
                default_route_json = excluded.default_route_json
            "#,
        )
        .bind(app.app_id.to_string())
        .bind(&app.display_name)
        .bind(app.exe_path.to_string_lossy().as_ref())
        .bind(&app.publisher)
        .bind(&app.sha256)
        .bind(app.icon_path.as_ref().map(|p| p.to_string_lossy().to_string()))
        .bind(app.first_seen.to_rfc3339())
        .bind(app.last_seen.to_rfc3339())
        .bind(route_json)
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn list(&self, filter: AppFilter) -> Result<Vec<AppRecord>> {
        let limit = filter.limit.unwrap_or(500) as i64;
        let rows = if let Some(search) = filter.search {
            let pattern = format!("%{search}%");
            sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>, Option<String>, String, String, Option<String>)>(
                "SELECT app_id, display_name, exe_path, publisher, sha256, icon_path, first_seen, last_seen, default_route_json FROM apps WHERE display_name LIKE ? OR exe_path LIKE ? ORDER BY last_seen DESC LIMIT ?",
            )
            .bind(&pattern)
            .bind(&pattern)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>, Option<String>, String, String, Option<String>)>(
                "SELECT app_id, display_name, exe_path, publisher, sha256, icon_path, first_seen, last_seen, default_route_json FROM apps ORDER BY last_seen DESC LIMIT ?",
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(|r| row_to_app(r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8))
            .collect()
    }

    async fn count(&self) -> Result<u32> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM apps")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(row.0 as u32)
    }
}
