use super::traits::{Result, RouteStatisticsRepository};
use async_trait::async_trait;
use chrono::Utc;
use shared_types::{RouteStatisticsQuery, RouteStatisticsRecord, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteRouteStatisticsRepository {
    pool: SqlitePool,
}

impl SqliteRouteStatisticsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RouteStatisticsRepository for SqliteRouteStatisticsRepository {
    async fn upsert(&self, record: &RouteStatisticsRecord) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO route_statistics (id, app_id, profile_id, domain, route_type, bytes_in, bytes_out, connection_count, window_start, window_end, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(id) DO UPDATE SET
                 bytes_in = bytes_in + excluded.bytes_in,
                 bytes_out = bytes_out + excluded.bytes_out,
                 connection_count = connection_count + excluded.connection_count,
                 updated_at = excluded.updated_at"#,
        )
        .bind(record.id.to_string())
        .bind(record.app_id.map(|id| id.to_string()))
        .bind(record.profile_id.map(|id| id.to_string()))
        .bind(&record.domain)
        .bind(&record.route_type)
        .bind(record.bytes_in as i64)
        .bind(record.bytes_out as i64)
        .bind(record.connection_count as i64)
        .bind(record.window_start.to_rfc3339())
        .bind(record.window_end.to_rfc3339())
        .bind(record.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn list(&self, query: RouteStatisticsQuery) -> Result<Vec<RouteStatisticsRecord>> {
        let mut sql = String::from(
            "SELECT id, app_id, profile_id, domain, route_type, bytes_in, bytes_out, connection_count, window_start, window_end, updated_at FROM route_statistics WHERE 1=1",
        );
        if query.app_id.is_some() {
            sql.push_str(" AND app_id = ?");
        }
        if query.domain.is_some() {
            sql.push_str(" AND domain = ?");
        }
        if query.route_type.is_some() {
            sql.push_str(" AND route_type = ?");
        }
        sql.push_str(" ORDER BY updated_at DESC LIMIT ?");

        let mut q = sqlx::query_as::<
            _,
            (
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                String,
                i64,
                i64,
                i64,
                String,
                String,
                String,
            ),
        >(&sql);
        if let Some(app_id) = query.app_id {
            q = q.bind(app_id.to_string());
        }
        if let Some(ref domain) = query.domain {
            q = q.bind(domain);
        }
        if let Some(ref route_type) = query.route_type {
            q = q.bind(route_type);
        }
        q = q.bind(query.limit.max(1) as i64);

        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter().map(parse_row).collect()
    }

    async fn blocked_summary(&self, limit: u32) -> Result<Vec<RouteStatisticsRecord>> {
        let query = RouteStatisticsQuery {
            route_type: Some("blocked".into()),
            limit,
            ..Default::default()
        };
        self.list(query).await
    }
}

fn parse_row(
    (
        id,
        app_id,
        profile_id,
        domain,
        route_type,
        bytes_in,
        bytes_out,
        connection_count,
        window_start,
        window_end,
        updated_at,
    ): (
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
        i64,
        i64,
        i64,
        String,
        String,
        String,
    ),
) -> Result<RouteStatisticsRecord> {
    Ok(RouteStatisticsRecord {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        app_id: app_id.and_then(|s| Uuid::parse_str(&s).ok()),
        profile_id: profile_id.and_then(|s| Uuid::parse_str(&s).ok()),
        domain,
        route_type,
        bytes_in: bytes_in as u64,
        bytes_out: bytes_out as u64,
        connection_count: connection_count as u32,
        window_start: chrono::DateTime::parse_from_rfc3339(&window_start)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
        window_end: chrono::DateTime::parse_from_rfc3339(&window_end)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
    })
}
