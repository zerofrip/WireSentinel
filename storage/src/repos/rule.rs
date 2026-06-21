use super::traits::{Result, RuleRepository, SettingsRepository};
use async_trait::async_trait;
use shared_types::{PolicyMode, Rule, RuleAction, RuleScope, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteRuleRepository {
    pool: SqlitePool,
}

impl SqliteRuleRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn scope_to_db(scope: &RuleScope) -> (String, Option<String>) {
    match scope {
        RuleScope::Global => ("global".into(), None),
        RuleScope::App(id) => ("app".into(), Some(id.to_string())),
        RuleScope::Domain(d) => ("domain".into(), Some(d.clone())),
    }
}

fn scope_from_db(scope_type: &str, scope_value: Option<String>) -> Result<RuleScope> {
    match scope_type {
        "global" => Ok(RuleScope::Global),
        "app" => {
            let id = scope_value.ok_or_else(|| WireSentinelError::Config("missing app id".into()))?;
            Ok(RuleScope::App(Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?))
        }
        "domain" => {
            let d = scope_value.ok_or_else(|| WireSentinelError::Config("missing domain".into()))?;
            Ok(RuleScope::Domain(d))
        }
        other => Err(WireSentinelError::Config(format!("unknown scope: {other}"))),
    }
}

#[async_trait]
impl RuleRepository for SqliteRuleRepository {
    async fn list(&self) -> Result<Vec<Rule>> {
        let rows = sqlx::query_as::<_, (String, i32, String, Option<String>, String, i32, Option<String>)>(
            "SELECT id, priority, scope_type, scope_value, action_json, enabled, description FROM rules ORDER BY priority DESC, id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(|(id, priority, scope_type, scope_value, action_json, enabled, description)| {
                Ok(Rule {
                    id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
                    priority,
                    scope: scope_from_db(&scope_type, scope_value)?,
                    action: serde_json::from_str(&action_json).map_err(WireSentinelError::Serde)?,
                    enabled: enabled != 0,
                    description,
                })
            })
            .collect()
    }

    async fn get(&self, id: Uuid) -> Result<Option<Rule>> {
        Ok(self.list().await?.into_iter().find(|r| r.id == id))
    }

    async fn insert(&self, rule: &Rule) -> Result<()> {
        let (scope_type, scope_value) = scope_to_db(&rule.scope);
        let action_json = serde_json::to_string(&rule.action).map_err(WireSentinelError::Serde)?;
        sqlx::query(
            "INSERT INTO rules (id, priority, scope_type, scope_value, action_json, enabled, description) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(rule.id.to_string())
        .bind(rule.priority)
        .bind(scope_type)
        .bind(scope_value)
        .bind(action_json)
        .bind(rule.enabled as i32)
        .bind(&rule.description)
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn update(&self, rule: &Rule) -> Result<()> {
        self.delete(rule.id).await?;
        self.insert(rule).await
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let r = sqlx::query("DELETE FROM rules WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(r.rows_affected() > 0)
    }

    async fn get_policy_mode(&self) -> Result<PolicyMode> {
        let settings = super::settings::SqliteSettingsRepository::new(self.pool.clone());
        let json = settings
            .get("policy_mode")
            .await?
            .unwrap_or_else(|| "\"blacklist\"".into());
        serde_json::from_str(&json).map_err(WireSentinelError::Serde)
    }

    async fn set_policy_mode(&self, mode: PolicyMode) -> Result<()> {
        let settings = super::settings::SqliteSettingsRepository::new(self.pool.clone());
        let json = serde_json::to_string(&mode).map_err(WireSentinelError::Serde)?;
        settings.set("policy_mode", &json).await
    }
}
