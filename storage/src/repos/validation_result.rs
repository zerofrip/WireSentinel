use super::traits::{Result, ValidationResultRepository};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{ValidationCheck, ValidationStatus, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteValidationResultRepository {
    pool: SqlitePool,
}

impl SqliteValidationResultRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ValidationResultRepository for SqliteValidationResultRepository {
    async fn upsert(&self, check: &ValidationCheck) -> Result<()> {
        sqlx::query(
            "INSERT INTO validation_results (id, check_name, status, message, checked_at)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
               status = excluded.status,
               message = excluded.message,
               checked_at = excluded.checked_at",
        )
        .bind(check.id.to_string())
        .bind(&check.check_name)
        .bind(status_str(check.status))
        .bind(&check.message)
        .bind(check.checked_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn list_recent(&self, limit: u32) -> Result<Vec<ValidationCheck>> {
        let rows: Vec<(String, String, String, Option<String>, String)> = sqlx::query_as(
            "SELECT id, check_name, status, message, checked_at
             FROM validation_results ORDER BY checked_at DESC LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter().map(parse_row).collect()
    }

    async fn latest_by_name(&self, check_name: &str) -> Result<Option<ValidationCheck>> {
        let row: Option<(String, String, String, Option<String>, String)> = sqlx::query_as(
            "SELECT id, check_name, status, message, checked_at
             FROM validation_results WHERE check_name = ? ORDER BY checked_at DESC LIMIT 1",
        )
        .bind(check_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(parse_row).transpose()
    }
}

fn status_str(status: ValidationStatus) -> &'static str {
    match status {
        ValidationStatus::Pass => "pass",
        ValidationStatus::Fail => "fail",
        ValidationStatus::Warn => "warn",
    }
}

fn parse_status(s: &str) -> ValidationStatus {
    match s {
        "fail" => ValidationStatus::Fail,
        "warn" => ValidationStatus::Warn,
        _ => ValidationStatus::Pass,
    }
}

fn parse_row(row: (String, String, String, Option<String>, String)) -> Result<ValidationCheck> {
    let (id, check_name, status, message, checked_at) = row;
    Ok(ValidationCheck {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        check_name,
        status: parse_status(&status),
        message,
        checked_at: DateTime::parse_from_rfc3339(&checked_at)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
    })
}
