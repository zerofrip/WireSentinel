use super::traits::{FilterListRepository, Result};
use async_trait::async_trait;
use shared_types::{FilterListRecord, FilterListType, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteFilterListRepository {
    pool: SqlitePool,
}

impl SqliteFilterListRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn parse_list_type(s: &str) -> FilterListType {
    match s {
        "easylist" => FilterListType::Easylist,
        _ => FilterListType::Hosts,
    }
}

fn list_type_str(t: FilterListType) -> &'static str {
    match t {
        FilterListType::Hosts => "hosts",
        FilterListType::Easylist => "easylist",
    }
}

#[async_trait]
impl FilterListRepository for SqliteFilterListRepository {
    async fn list(&self) -> Result<Vec<FilterListRecord>> {
        let rows = sqlx::query_as::<_, (String, String, Option<String>, String, i32, Option<i64>, Option<String>, Option<String>)>(
            "SELECT id, name, url, type, enabled, update_interval_secs, last_updated, cache_path FROM filter_lists ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter().map(row_to_record).collect()
    }

    async fn get(&self, id: Uuid) -> Result<Option<FilterListRecord>> {
        let row = sqlx::query_as::<_, (String, String, Option<String>, String, i32, Option<i64>, Option<String>, Option<String>)>(
            "SELECT id, name, url, type, enabled, update_interval_secs, last_updated, cache_path FROM filter_lists WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(row_to_record).transpose()
    }

    async fn insert(&self, record: &FilterListRecord) -> Result<()> {
        sqlx::query(
            "INSERT INTO filter_lists (id, name, url, type, enabled, update_interval_secs, last_updated, cache_path) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(record.id.to_string())
        .bind(&record.name)
        .bind(&record.url)
        .bind(list_type_str(record.list_type))
        .bind(record.enabled as i32)
        .bind(record.update_interval_secs.map(|v| v as i64))
        .bind(record.last_updated.map(|t| t.to_rfc3339()))
        .bind(record.cache_path.as_ref().map(|p| p.to_string_lossy().to_string()))
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn update(&self, record: &FilterListRecord) -> Result<()> {
        sqlx::query(
            "UPDATE filter_lists SET name=?, url=?, type=?, enabled=?, update_interval_secs=?, last_updated=?, cache_path=? WHERE id=?",
        )
        .bind(&record.name)
        .bind(&record.url)
        .bind(list_type_str(record.list_type))
        .bind(record.enabled as i32)
        .bind(record.update_interval_secs.map(|v| v as i64))
        .bind(record.last_updated.map(|t| t.to_rfc3339()))
        .bind(record.cache_path.as_ref().map(|p| p.to_string_lossy().to_string()))
        .bind(record.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let r = sqlx::query("DELETE FROM filter_lists WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(r.rows_affected() > 0)
    }
}

fn row_to_record(
    (id, name, url, list_type, enabled, interval, last_updated, cache_path): (
        String,
        String,
        Option<String>,
        String,
        i32,
        Option<i64>,
        Option<String>,
        Option<String>,
    ),
) -> Result<FilterListRecord> {
    Ok(FilterListRecord {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        name,
        url,
        list_type: parse_list_type(&list_type),
        enabled: enabled != 0,
        update_interval_secs: interval.map(|v| v as u32),
        last_updated: last_updated
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|d| d.with_timezone(&chrono::Utc)),
        cache_path: cache_path.map(std::path::PathBuf::from),
    })
}
