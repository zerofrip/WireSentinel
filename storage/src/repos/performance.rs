use super::traits::{PerformanceRepository, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{PerformanceSnapshot, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqlitePerformanceRepository {
    pool: SqlitePool,
}

impl SqlitePerformanceRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PerformanceRepository for SqlitePerformanceRepository {
    async fn insert(&self, snapshot: &PerformanceSnapshot) -> Result<()> {
        sqlx::query(
            "INSERT INTO performance_snapshots
             (id, cpu_percent, memory_bytes, api_latency_ms, wfp_latency_ms, event_throughput, timestamp)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(snapshot.id.to_string())
        .bind(snapshot.cpu_percent)
        .bind(snapshot.memory_bytes as i64)
        .bind(snapshot.api_latency_ms)
        .bind(snapshot.wfp_latency_ms)
        .bind(snapshot.event_throughput)
        .bind(snapshot.timestamp.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn latest(&self) -> Result<Option<PerformanceSnapshot>> {
        let row: Option<(String, f64, i64, f64, f64, f64, String)> = sqlx::query_as(
            "SELECT id, cpu_percent, memory_bytes, api_latency_ms, wfp_latency_ms, event_throughput, timestamp
             FROM performance_snapshots ORDER BY timestamp DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(parse_row).transpose()
    }

    async fn list_recent(&self, limit: u32) -> Result<Vec<PerformanceSnapshot>> {
        let rows: Vec<(String, f64, i64, f64, f64, f64, String)> = sqlx::query_as(
            "SELECT id, cpu_percent, memory_bytes, api_latency_ms, wfp_latency_ms, event_throughput, timestamp
             FROM performance_snapshots ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter().map(parse_row).collect()
    }
}

fn parse_row(
    row: (String, f64, i64, f64, f64, f64, String),
) -> Result<PerformanceSnapshot> {
    let (id, cpu_percent, memory_bytes, api_latency_ms, wfp_latency_ms, event_throughput, timestamp) =
        row;
    Ok(PerformanceSnapshot {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        cpu_percent,
        memory_bytes: memory_bytes as u64,
        api_latency_ms,
        wfp_latency_ms,
        event_throughput,
        timestamp: DateTime::parse_from_rfc3339(&timestamp)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
    })
}
