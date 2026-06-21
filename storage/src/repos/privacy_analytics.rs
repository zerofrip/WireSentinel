use super::traits::{PrivacyAnalyticsRepository, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{PrivacyAnalyticsSnapshot, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqlitePrivacyAnalyticsRepository {
    pool: SqlitePool,
}

impl SqlitePrivacyAnalyticsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PrivacyAnalyticsRepository for SqlitePrivacyAnalyticsRepository {
    async fn insert(&self, snapshot: &PrivacyAnalyticsSnapshot) -> Result<()> {
        sqlx::query(
            "INSERT INTO privacy_analytics (id, anonymity_score, route_entropy, path_diversity, cover_traffic_effectiveness, anonymity_set_estimate, cover_traffic_efficiency, mixnet_diversity, federation_diversity, timestamp) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(snapshot.id.to_string())
        .bind(snapshot.anonymity_score as i32)
        .bind(snapshot.route_entropy)
        .bind(snapshot.path_diversity)
        .bind(snapshot.cover_traffic_effectiveness)
        .bind(snapshot.anonymity_set_estimate)
        .bind(snapshot.cover_traffic_efficiency)
        .bind(snapshot.mixnet_diversity)
        .bind(snapshot.federation_diversity)
        .bind(snapshot.timestamp.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn latest(&self) -> Result<Option<PrivacyAnalyticsSnapshot>> {
        let row = sqlx::query_as::<_, (String, i32, f64, f64, f64, Option<f64>, Option<f64>, Option<f64>, Option<f64>, String)>(
            "SELECT id, anonymity_score, route_entropy, path_diversity, cover_traffic_effectiveness, anonymity_set_estimate, cover_traffic_efficiency, mixnet_diversity, federation_diversity, timestamp FROM privacy_analytics ORDER BY timestamp DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(
            |(
                id,
                anonymity_score,
                route_entropy,
                path_diversity,
                cover_traffic_effectiveness,
                anonymity_set_estimate,
                cover_traffic_efficiency,
                mixnet_diversity,
                federation_diversity,
                timestamp,
            )| {
                Ok(PrivacyAnalyticsSnapshot {
                    id: Uuid::parse_str(&id)
                        .map_err(|e| WireSentinelError::Config(e.to_string()))?,
                    anonymity_score: anonymity_score.clamp(0, 100) as u8,
                    route_entropy,
                    path_diversity,
                    cover_traffic_effectiveness,
                    anonymity_set_estimate,
                    cover_traffic_efficiency,
                    mixnet_diversity,
                    federation_diversity,
                    timestamp: DateTime::parse_from_rfc3339(&timestamp)
                        .map_err(|e| WireSentinelError::Config(e.to_string()))?
                        .with_timezone(&Utc),
                })
            },
        )
        .transpose()
    }

    async fn list(&self, limit: u32) -> Result<Vec<PrivacyAnalyticsSnapshot>> {
        let rows = sqlx::query_as::<_, (String, i32, f64, f64, f64, Option<f64>, Option<f64>, Option<f64>, Option<f64>, String)>(
            "SELECT id, anonymity_score, route_entropy, path_diversity, cover_traffic_effectiveness, anonymity_set_estimate, cover_traffic_efficiency, mixnet_diversity, federation_diversity, timestamp FROM privacy_analytics ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(
                |(
                    id,
                    anonymity_score,
                    route_entropy,
                    path_diversity,
                    cover_traffic_effectiveness,
                    anonymity_set_estimate,
                    cover_traffic_efficiency,
                    mixnet_diversity,
                    federation_diversity,
                    timestamp,
                )| {
                    Ok(PrivacyAnalyticsSnapshot {
                        id: Uuid::parse_str(&id)
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?,
                        anonymity_score: anonymity_score.clamp(0, 100) as u8,
                        route_entropy,
                        path_diversity,
                        cover_traffic_effectiveness,
                        anonymity_set_estimate,
                        cover_traffic_efficiency,
                        mixnet_diversity,
                        federation_diversity,
                        timestamp: DateTime::parse_from_rfc3339(&timestamp)
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?
                            .with_timezone(&Utc),
                    })
                },
            )
            .collect()
    }
}
