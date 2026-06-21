use super::traits::{CorrelationQuery, CorrelationRepository, Result};
use async_trait::async_trait;
use chrono::Utc;
use shared_types::{DomainCorrelation, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteCorrelationRepository {
    pool: SqlitePool,
}

impl SqliteCorrelationRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CorrelationRepository for SqliteCorrelationRepository {
    async fn upsert(&self, corr: &DomainCorrelation) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO domain_correlations (id, app_id, domain, ip_address, first_seen, last_seen, query_count, traffic_count)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(id) DO UPDATE SET
                 last_seen=excluded.last_seen,
                 query_count=excluded.query_count,
                 traffic_count=excluded.traffic_count"#,
        )
        .bind(corr.id.to_string())
        .bind(corr.app_id.map(|id| id.to_string()))
        .bind(&corr.domain)
        .bind(&corr.ip_address)
        .bind(corr.first_seen.to_rfc3339())
        .bind(corr.last_seen.to_rfc3339())
        .bind(corr.query_count as i64)
        .bind(corr.traffic_count as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn list(&self, query: CorrelationQuery) -> Result<Vec<DomainCorrelation>> {
        let mut sql = String::from(
            "SELECT id, app_id, domain, ip_address, first_seen, last_seen, query_count, traffic_count FROM domain_correlations WHERE 1=1",
        );
        if query.app_id.is_some() {
            sql.push_str(" AND app_id = ?");
        }
        if query.domain.is_some() {
            sql.push_str(" AND domain LIKE ?");
        }
        sql.push_str(" ORDER BY last_seen DESC LIMIT ?");

        let mut q = sqlx::query_as::<_, (String, Option<String>, String, Option<String>, String, String, i64, i64)>(&sql);
        if let Some(app_id) = query.app_id {
            q = q.bind(app_id.to_string());
        }
        if let Some(ref domain) = query.domain {
            q = q.bind(format!("%{domain}%"));
        }
        q = q.bind(query.limit as i64);

        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter().map(row_to_corr).collect()
    }

    async fn record_dns(&self, app_id: Option<Uuid>, domain: &str, ip: &str) -> Result<()> {
        let now = Utc::now();
        let existing = sqlx::query_as::<_, (String, i64, i64)>(
            "SELECT id, query_count, traffic_count FROM domain_correlations WHERE app_id IS ? AND domain = ? AND ip_address = ? LIMIT 1",
        )
        .bind(app_id.map(|id| id.to_string()))
        .bind(domain)
        .bind(ip)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        if let Some((id, qc, tc)) = existing {
            sqlx::query(
                "UPDATE domain_correlations SET query_count = ?, last_seen = ? WHERE id = ?",
            )
            .bind(qc + 1)
            .bind(now.to_rfc3339())
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
            let _ = tc;
        } else {
            let corr = DomainCorrelation {
                id: Uuid::new_v4(),
                app_id,
                domain: domain.to_string(),
                ip_address: Some(ip.to_string()),
                first_seen: now,
                last_seen: now,
                query_count: 1,
                traffic_count: 0,
            };
            self.upsert(&corr).await?;
        }
        Ok(())
    }

    async fn record_traffic(&self, app_id: Option<Uuid>, ip: &str) -> Result<Option<String>> {
        let row = sqlx::query_as::<_, (String, String, i64)>(
            "SELECT id, domain, traffic_count FROM domain_correlations WHERE app_id IS ? AND ip_address = ? ORDER BY last_seen DESC LIMIT 1",
        )
        .bind(app_id.map(|id| id.to_string()))
        .bind(ip)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        if let Some((id, domain, tc)) = row {
            sqlx::query(
                "UPDATE domain_correlations SET traffic_count = ?, last_seen = ? WHERE id = ?",
            )
            .bind(tc + 1)
            .bind(Utc::now().to_rfc3339())
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
            return Ok(Some(domain));
        }
        Ok(None)
    }
}

fn row_to_corr(
    (id, app_id, domain, ip, first_seen, last_seen, qc, tc): (
        String,
        Option<String>,
        String,
        Option<String>,
        String,
        String,
        i64,
        i64,
    ),
) -> Result<DomainCorrelation> {
    Ok(DomainCorrelation {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        app_id: app_id.and_then(|s| Uuid::parse_str(&s).ok()),
        domain,
        ip_address: ip,
        first_seen: chrono::DateTime::parse_from_rfc3339(&first_seen)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
        last_seen: chrono::DateTime::parse_from_rfc3339(&last_seen)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
        query_count: qc as u32,
        traffic_count: tc as u32,
    })
}
