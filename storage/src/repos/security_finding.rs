use super::traits::{Result, SecurityFindingRepository};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{SecurityFinding, SecuritySeverity, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteSecurityFindingRepository {
    pool: SqlitePool,
}

impl SqliteSecurityFindingRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SecurityFindingRepository for SqliteSecurityFindingRepository {
    async fn insert(&self, finding: &SecurityFinding) -> Result<()> {
        let detail_json =
            serde_json::to_string(&finding.detail_json).map_err(WireSentinelError::Serde)?;

        sqlx::query(
            "INSERT INTO security_findings (id, severity, category, title, detail_json, resolved, created_at, resolved_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(finding.id.to_string())
        .bind(severity_str(finding.severity))
        .bind(&finding.category)
        .bind(&finding.title)
        .bind(detail_json)
        .bind(finding.resolved as i32)
        .bind(finding.created_at.to_rfc3339())
        .bind(finding.resolved_at.map(|t| t.to_rfc3339()))
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn list(&self, include_resolved: bool, limit: u32) -> Result<Vec<SecurityFinding>> {
        let rows: Vec<(String, String, String, String, String, i32, String, Option<String>)> =
            if include_resolved {
                sqlx::query_as(
                    "SELECT id, severity, category, title, detail_json, resolved, created_at, resolved_at
                     FROM security_findings ORDER BY created_at DESC LIMIT ?",
                )
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await
            } else {
                sqlx::query_as(
                    "SELECT id, severity, category, title, detail_json, resolved, created_at, resolved_at
                     FROM security_findings WHERE resolved = 0 ORDER BY created_at DESC LIMIT ?",
                )
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await
            }
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter().map(parse_row).collect()
    }

    async fn resolve(&self, id: Uuid) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE security_findings SET resolved = 1, resolved_at = ? WHERE id = ? AND resolved = 0",
        )
        .bind(now)
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(result.rows_affected() > 0)
    }
}

fn severity_str(severity: SecuritySeverity) -> &'static str {
    match severity {
        SecuritySeverity::Info => "info",
        SecuritySeverity::Low => "low",
        SecuritySeverity::Medium => "medium",
        SecuritySeverity::High => "high",
        SecuritySeverity::Critical => "critical",
    }
}

fn parse_severity(s: &str) -> SecuritySeverity {
    match s {
        "low" => SecuritySeverity::Low,
        "medium" => SecuritySeverity::Medium,
        "high" => SecuritySeverity::High,
        "critical" => SecuritySeverity::Critical,
        _ => SecuritySeverity::Info,
    }
}

fn parse_row(
    row: (String, String, String, String, String, i32, String, Option<String>),
) -> Result<SecurityFinding> {
    let (id, severity, category, title, detail_json, resolved, created_at, resolved_at) = row;
    Ok(SecurityFinding {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        severity: parse_severity(&severity),
        category,
        title,
        detail_json: serde_json::from_str(&detail_json).map_err(WireSentinelError::Serde)?,
        resolved: resolved != 0,
        created_at: DateTime::parse_from_rfc3339(&created_at)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
        resolved_at: resolved_at
            .map(|t| DateTime::parse_from_rfc3339(&t))
            .transpose()
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .map(|t| t.with_timezone(&Utc)),
    })
}
