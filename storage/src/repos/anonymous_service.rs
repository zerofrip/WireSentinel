use super::traits::{AnonymousServiceRepository, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{
    AnonymousService, AnonymousServiceEndpoint, AnonymityProvider, WireSentinelError,
};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteAnonymousServiceRepository {
    pool: SqlitePool,
}

impl SqliteAnonymousServiceRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn provider_to_parts(provider: &AnonymityProvider) -> (&'static str, Option<String>) {
    match provider {
        AnonymityProvider::Katzenpost => ("katzenpost", None),
        AnonymityProvider::Loopix => ("loopix", None),
        AnonymityProvider::FederatedMixnet => ("federated_mixnet", None),
        AnonymityProvider::Plugin(id) => ("plugin", Some(id.to_string())),
    }
}

fn provider_from_parts(provider: &str, plugin_id: Option<String>) -> Result<AnonymityProvider> {
    match provider {
        "loopix" => Ok(AnonymityProvider::Loopix),
        "federated_mixnet" => Ok(AnonymityProvider::FederatedMixnet),
        "plugin" => {
            let id = plugin_id.ok_or_else(|| {
                WireSentinelError::Config("anonymous service missing plugin_id".into())
            })?;
            Ok(AnonymityProvider::Plugin(
                Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
            ))
        }
        _ => Ok(AnonymityProvider::Katzenpost),
    }
}

type AnonymousServiceRow = (
    String,
    String,
    Option<String>,
    String,
    Option<String>,
    String,
    i32,
    String,
    String,
);

const SERVICE_SELECT: &str = "SELECT id, name, description, provider, plugin_id, profile_id, enabled, created_at, updated_at FROM anonymous_services";

fn parse_service_row(
    id: String,
    name: String,
    description: Option<String>,
    provider: String,
    plugin_id: Option<String>,
    profile_id: String,
    enabled: i32,
    created_at: String,
    updated_at: String,
) -> Result<AnonymousService> {
    Ok(AnonymousService {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        name,
        description,
        provider: provider_from_parts(&provider, plugin_id)?,
        profile_id: Uuid::parse_str(&profile_id)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?,
        enabled: enabled != 0,
        created_at: DateTime::parse_from_rfc3339(&created_at)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&Utc),
    })
}

#[async_trait]
impl AnonymousServiceRepository for SqliteAnonymousServiceRepository {
    async fn list(&self) -> Result<Vec<AnonymousService>> {
        let rows = sqlx::query_as::<_, AnonymousServiceRow>(&format!(
            "{SERVICE_SELECT} ORDER BY name"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(|r| {
                parse_service_row(
                    r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8,
                )
            })
            .collect()
    }

    async fn get(&self, id: Uuid) -> Result<Option<AnonymousService>> {
        let row = sqlx::query_as::<_, AnonymousServiceRow>(&format!(
            "{SERVICE_SELECT} WHERE id = ?"
        ))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        row.map(|r| parse_service_row(r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8))
            .transpose()
    }

    async fn insert(&self, service: &AnonymousService) -> Result<()> {
        let (provider, plugin_id) = provider_to_parts(&service.provider);
        sqlx::query(
            "INSERT INTO anonymous_services (id, name, description, provider, plugin_id, profile_id, enabled, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(service.id.to_string())
        .bind(&service.name)
        .bind(&service.description)
        .bind(provider)
        .bind(plugin_id)
        .bind(service.profile_id.to_string())
        .bind(service.enabled as i32)
        .bind(service.created_at.to_rfc3339())
        .bind(service.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn update(&self, service: &AnonymousService) -> Result<()> {
        let (provider, plugin_id) = provider_to_parts(&service.provider);
        sqlx::query(
            "UPDATE anonymous_services SET name = ?, description = ?, provider = ?, plugin_id = ?, profile_id = ?, enabled = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&service.name)
        .bind(&service.description)
        .bind(provider)
        .bind(plugin_id)
        .bind(service.profile_id.to_string())
        .bind(service.enabled as i32)
        .bind(service.updated_at.to_rfc3339())
        .bind(service.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let r = sqlx::query("DELETE FROM anonymous_services WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(r.rows_affected() > 0)
    }

    async fn list_endpoints(&self, service_id: Uuid) -> Result<Vec<AnonymousServiceEndpoint>> {
        let rows = sqlx::query_as::<_, (String, String, String, i64, String, Option<String>, i32)>(
            "SELECT id, service_id, host, port, protocol, path, enabled FROM anonymous_service_endpoints WHERE service_id = ?",
        )
        .bind(service_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(
                |(id, service_id, host, port, protocol, path, enabled)| {
                    Ok(AnonymousServiceEndpoint {
                        id: Uuid::parse_str(&id)
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?,
                        service_id: Uuid::parse_str(&service_id)
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?,
                        host,
                        port: port as u16,
                        protocol,
                        path,
                        enabled: enabled != 0,
                    })
                },
            )
            .collect()
    }

    async fn upsert_endpoint(&self, endpoint: &AnonymousServiceEndpoint) -> Result<()> {
        sqlx::query(
            "INSERT INTO anonymous_service_endpoints (id, service_id, host, port, protocol, path, enabled) VALUES (?, ?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET host = excluded.host, port = excluded.port, protocol = excluded.protocol, path = excluded.path, enabled = excluded.enabled",
        )
        .bind(endpoint.id.to_string())
        .bind(endpoint.service_id.to_string())
        .bind(&endpoint.host)
        .bind(endpoint.port as i64)
        .bind(&endpoint.protocol)
        .bind(&endpoint.path)
        .bind(endpoint.enabled as i32)
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }
}
