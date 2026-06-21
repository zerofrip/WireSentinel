use super::traits::{Result, SplitTemplateRepository};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_types::{
    AppRule, DomainRule, SplitTemplateModeSettings, SplitTunnelTemplate, TemplateMode,
    TrafficRoute, WireSentinelError,
};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteSplitTemplateRepository {
    pool: SqlitePool,
}

impl SqliteSplitTemplateRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

const MODE_KEY: &str = "split_tunnel_template_mode";

fn mode_str(m: TemplateMode) -> &'static str {
    match m {
        TemplateMode::Disabled => "disabled",
        TemplateMode::Merge => "merge",
        TemplateMode::Override => "override",
    }
}

fn mode_from_str(s: &str) -> Result<TemplateMode> {
    match s {
        "disabled" => Ok(TemplateMode::Disabled),
        "merge" => Ok(TemplateMode::Merge),
        "override" => Ok(TemplateMode::Override),
        other => Err(WireSentinelError::Config(format!("unknown template mode: {other}"))),
    }
}

#[async_trait]
impl SplitTemplateRepository for SqliteSplitTemplateRepository {
    async fn list(&self) -> Result<Vec<SplitTunnelTemplate>> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                String,
                String,
                i32,
                String,
                String,
            ),
        >(
            "SELECT id, name, description, default_route_json, app_rules_json, domain_rules_json, enabled, created_at, updated_at FROM split_tunnel_templates ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(
                |(
                    id,
                    name,
                    description,
                    default_route_json,
                    app_rules_json,
                    domain_rules_json,
                    enabled,
                    created_at,
                    updated_at,
                )| {
                    Ok(SplitTunnelTemplate {
                        id: Uuid::parse_str(&id)
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?,
                        name,
                        description,
                        default_route: serde_json::from_str(&default_route_json)
                            .map_err(WireSentinelError::Serde)?,
                        app_rules: serde_json::from_str(&app_rules_json)
                            .unwrap_or_default(),
                        domain_rules: serde_json::from_str(&domain_rules_json)
                            .unwrap_or_default(),
                        enabled: enabled != 0,
                        created_at: DateTime::parse_from_rfc3339(&created_at)
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?
                            .with_timezone(&Utc),
                        updated_at: DateTime::parse_from_rfc3339(&updated_at)
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?
                            .with_timezone(&Utc),
                    })
                },
            )
            .collect()
    }

    async fn get(&self, id: Uuid) -> Result<Option<SplitTunnelTemplate>> {
        Ok(self.list().await?.into_iter().find(|t| t.id == id))
    }

    async fn insert(&self, template: &SplitTunnelTemplate) -> Result<()> {
        sqlx::query(
            "INSERT INTO split_tunnel_templates (id, name, description, default_route_json, app_rules_json, domain_rules_json, enabled, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(template.id.to_string())
        .bind(&template.name)
        .bind(&template.description)
        .bind(serde_json::to_string(&template.default_route).map_err(WireSentinelError::Serde)?)
        .bind(serde_json::to_string(&template.app_rules).map_err(WireSentinelError::Serde)?)
        .bind(serde_json::to_string(&template.domain_rules).map_err(WireSentinelError::Serde)?)
        .bind(template.enabled as i32)
        .bind(template.created_at.to_rfc3339())
        .bind(template.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn update(&self, template: &SplitTunnelTemplate) -> Result<()> {
        sqlx::query(
            "UPDATE split_tunnel_templates SET name = ?, description = ?, default_route_json = ?, app_rules_json = ?, domain_rules_json = ?, enabled = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&template.name)
        .bind(&template.description)
        .bind(serde_json::to_string(&template.default_route).map_err(WireSentinelError::Serde)?)
        .bind(serde_json::to_string(&template.app_rules).map_err(WireSentinelError::Serde)?)
        .bind(serde_json::to_string(&template.domain_rules).map_err(WireSentinelError::Serde)?)
        .bind(template.enabled as i32)
        .bind(template.updated_at.to_rfc3339())
        .bind(template.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let r = sqlx::query("DELETE FROM split_tunnel_templates WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(r.rows_affected() > 0)
    }

    async fn get_mode(&self) -> Result<SplitTemplateModeSettings> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT value_json FROM settings WHERE key = ?")
                .bind(MODE_KEY)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        match row {
            Some((json,)) => serde_json::from_str(&json).map_err(WireSentinelError::Serde),
            None => Ok(SplitTemplateModeSettings::default()),
        }
    }

    async fn set_mode(&self, settings: &SplitTemplateModeSettings) -> Result<()> {
        let json = serde_json::to_string(settings).map_err(WireSentinelError::Serde)?;
        sqlx::query(
            "INSERT INTO settings (key, value_json) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json",
        )
        .bind(MODE_KEY)
        .bind(json)
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }
}
