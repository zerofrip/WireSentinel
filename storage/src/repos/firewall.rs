use super::traits::{FirewallDecisionRepository, Result};
use async_trait::async_trait;
use chrono::Utc;
use shared_types::{FirewallDecisionRecord, TrafficRoute, Verdict, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteFirewallDecisionRepository {
    pool: SqlitePool,
}

impl SqliteFirewallDecisionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FirewallDecisionRepository for SqliteFirewallDecisionRepository {
    async fn insert(&self, record: &FirewallDecisionRecord) -> Result<()> {
        let route_json =
            serde_json::to_string(&record.route).map_err(WireSentinelError::Serde)?;
        let verdict_json =
            serde_json::to_string(&record.verdict).map_err(WireSentinelError::Serde)?;
        sqlx::query(
            "INSERT INTO firewall_decisions (id, app_id, domain, dest_ip, route_json, verdict_json, timestamp) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(record.id.to_string())
        .bind(record.app_id.map(|id| id.to_string()))
        .bind(&record.domain)
        .bind(&record.dest_ip)
        .bind(route_json)
        .bind(verdict_json)
        .bind(record.timestamp.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn list_recent(&self, limit: u32) -> Result<Vec<FirewallDecisionRecord>> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                String,
                Option<String>,
                String,
            ),
        >(
            "SELECT id, app_id, domain, dest_ip, route_json, verdict_json, timestamp FROM firewall_decisions ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter().map(parse_row).collect()
    }

    async fn count(&self) -> Result<u64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM firewall_decisions")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(row.0 as u64)
    }
}

fn parse_row(
    (id, app_id, domain, dest_ip, route_json, verdict_json, timestamp): (
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
        Option<String>,
        String,
    ),
) -> Result<FirewallDecisionRecord> {
    Ok(FirewallDecisionRecord {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        app_id: app_id.and_then(|s| Uuid::parse_str(&s).ok()),
        domain,
        dest_ip,
        route: serde_json::from_str(&route_json).unwrap_or(TrafficRoute::Direct),
        verdict: verdict_json
            .and_then(|v| serde_json::from_str(&v).ok())
            .unwrap_or(Verdict::allow("unknown")),
        timestamp: chrono::DateTime::parse_from_rfc3339(&timestamp)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
    })
}
