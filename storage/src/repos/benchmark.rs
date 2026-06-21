use super::traits::{BenchmarkRepository, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{BenchmarkSnapshot, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteBenchmarkRepository {
    pool: SqlitePool,
}

impl SqliteBenchmarkRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BenchmarkRepository for SqliteBenchmarkRepository {
    async fn insert(&self, snapshot: &BenchmarkSnapshot) -> Result<()> {
        sqlx::query(
            "INSERT INTO benchmark_snapshots
             (id, wfp_latency_ms, route_latency_ms, dns_latency_ms, transport_startup_ms, ui_event_throughput, timestamp)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(snapshot.id.to_string())
        .bind(snapshot.wfp_latency_ms)
        .bind(snapshot.route_latency_ms)
        .bind(snapshot.dns_latency_ms)
        .bind(snapshot.transport_startup_ms)
        .bind(snapshot.ui_event_throughput)
        .bind(snapshot.timestamp.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn latest(&self) -> Result<Option<BenchmarkSnapshot>> {
        let row: Option<(String, f64, f64, f64, f64, f64, String)> = sqlx::query_as(
            "SELECT id, wfp_latency_ms, route_latency_ms, dns_latency_ms, transport_startup_ms, ui_event_throughput, timestamp
             FROM benchmark_snapshots ORDER BY timestamp DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(parse_row).transpose()
    }

    async fn list_recent(&self, limit: u32) -> Result<Vec<BenchmarkSnapshot>> {
        let rows: Vec<(String, f64, f64, f64, f64, f64, String)> = sqlx::query_as(
            "SELECT id, wfp_latency_ms, route_latency_ms, dns_latency_ms, transport_startup_ms, ui_event_throughput, timestamp
             FROM benchmark_snapshots ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter().map(parse_row).collect()
    }
}

fn parse_row(row: (String, f64, f64, f64, f64, f64, String)) -> Result<BenchmarkSnapshot> {
    let (
        id,
        wfp_latency_ms,
        route_latency_ms,
        dns_latency_ms,
        transport_startup_ms,
        ui_event_throughput,
        timestamp,
    ) = row;
    Ok(BenchmarkSnapshot {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        wfp_latency_ms,
        route_latency_ms,
        dns_latency_ms,
        transport_startup_ms,
        ui_event_throughput,
        timestamp: DateTime::parse_from_rfc3339(&timestamp)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
    })
}
