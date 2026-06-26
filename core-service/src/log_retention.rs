//! Periodic deletion of aged log/snapshot rows.

use chrono::{Duration, Utc};
use shared_types::Result;
use std::sync::Arc;
use storage::Storage;
use tokio::sync::watch;
use tracing::{debug, warn};

pub async fn purge_older_than(storage: &Storage, days: u64) -> Result<u64> {
    let cutoff = (Utc::now() - Duration::days(days as i64)).to_rfc3339();
    let pool = &storage.pool;
    let mut total = 0u64;

    for (table, col) in [
        ("dns_logs", "timestamp"),
        ("traffic_logs", "timestamp"),
        ("firewall_decisions", "timestamp"),
        ("performance_snapshots", "timestamp"),
        ("privacy_snapshots", "timestamp"),
        ("privacy_analytics", "timestamp"),
        ("audit_log", "timestamp"),
    ] {
        let sql = format!("DELETE FROM {table} WHERE {col} < ?");
        match sqlx::query(&sql).bind(&cutoff).execute(pool).await {
            Ok(r) => total += r.rows_affected(),
            Err(e) => warn!(table, error = %e, "retention purge failed"),
        }
    }

    Ok(total)
}

pub fn start_retention_task(storage: Arc<Storage>, mut shutdown: watch::Receiver<bool>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(24 * 3600));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let days = storage.settings.log_retention_days().await.unwrap_or(7);
                    match purge_older_than(&storage, days).await {
                        Ok(n) => debug!(rows = n, days, "log retention purge completed"),
                        Err(e) => warn!(error = %e, "log retention purge failed"),
                    }
                }
                changed = shutdown.changed() => {
                    if changed.is_ok() && *shutdown.borrow() {
                        break;
                    }
                }
            }
        }
    });
}
