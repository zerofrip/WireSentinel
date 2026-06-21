//! Enterprise policy provider framework.

use async_trait::async_trait;
use chrono::Utc;
use shared_types::{EnterprisePolicy, Result, WireSentinelError};
use std::path::PathBuf;
use std::sync::Arc;
use storage::{data_dir, Storage};
use uuid::Uuid;

#[async_trait]
pub trait PolicyProvider: Send + Sync {
    async fn load(&self) -> Result<EnterprisePolicy>;
    async fn apply(&self, policy: &EnterprisePolicy) -> Result<()>;
    fn is_locked(&self, key: &str) -> bool;
}

pub struct LocalPolicyProvider {
    storage: Arc<Storage>,
    locked_keys: parking_lot::RwLock<Vec<String>>,
}

impl LocalPolicyProvider {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self {
            storage,
            locked_keys: parking_lot::RwLock::new(Vec::new()),
        }
    }

    fn policy_file() -> PathBuf {
        data_dir().join("policy.json")
    }
}

#[async_trait]
impl PolicyProvider for LocalPolicyProvider {
    async fn load(&self) -> Result<EnterprisePolicy> {
        if Self::policy_file().exists() {
            let text = std::fs::read_to_string(Self::policy_file()).map_err(WireSentinelError::Io)?;
            let policy: EnterprisePolicy = serde_json::from_str(&text).map_err(WireSentinelError::Serde)?;
            *self.locked_keys.write() = policy.locked_keys.clone();
            return Ok(policy);
        }

        match self.storage.enterprise_policy.get_active().await? {
            Some(policy) => {
                *self.locked_keys.write() = policy.locked_keys.clone();
                Ok(policy)
            }
            None => Ok(EnterprisePolicy {
                id: Uuid::new_v4(),
                version: 1,
                policy_json: serde_json::json!({}),
                locked_keys: Vec::new(),
                updated_at: Utc::now(),
            }),
        }
    }

    async fn apply(&self, policy: &EnterprisePolicy) -> Result<()> {
        self.storage.enterprise_policy.upsert(policy).await?;
        self.storage
            .settings
            .set_enterprise_policy_id(Some(policy.id))
            .await?;
        *self.locked_keys.write() = policy.locked_keys.clone();
        Ok(())
    }

    fn is_locked(&self, key: &str) -> bool {
        self.locked_keys.read().iter().any(|k| k == key)
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RemotePolicyBundle {
    pub device_id: String,
    pub policy_json: serde_json::Value,
    pub locked_keys: Vec<String>,
    pub version: u32,
}

pub struct RemotePolicyProvider {
    base_url: String,
    device_id: String,
    http: reqwest::Client,
    local: Arc<LocalPolicyProvider>,
}

impl RemotePolicyProvider {
    pub fn new(
        base_url: String,
        device_id: String,
        http: reqwest::Client,
        local: Arc<LocalPolicyProvider>,
    ) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            device_id,
            http,
            local,
        }
    }

    pub async fn pull_and_apply(&self) -> Result<EnterprisePolicy> {
        let policy = self.fetch().await?;
        self.local.apply(&policy).await?;
        Ok(policy)
    }

    async fn fetch(&self) -> Result<EnterprisePolicy> {
        let url = format!(
            "{}/api/v1/policies/devices/{}",
            self.base_url, self.device_id
        );
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| WireSentinelError::Config(format!("remote policy fetch: {e}")))?;
        if !resp.status().is_success() {
            return Err(WireSentinelError::Config(format!(
                "remote policy status {}",
                resp.status()
            )));
        }
        let bundle: RemotePolicyBundle = resp
            .json()
            .await
            .map_err(|e| WireSentinelError::Config(format!("remote policy decode: {e}")))?;

        Ok(EnterprisePolicy {
            id: Uuid::new_v4(),
            version: bundle.version.max(1),
            policy_json: bundle.policy_json,
            locked_keys: bundle.locked_keys,
            updated_at: Utc::now(),
        })
    }
}

#[async_trait]
impl PolicyProvider for RemotePolicyProvider {
    async fn load(&self) -> Result<EnterprisePolicy> {
        self.fetch().await
    }

    async fn apply(&self, policy: &EnterprisePolicy) -> Result<()> {
        self.local.apply(policy).await
    }

    fn is_locked(&self, key: &str) -> bool {
        self.local.is_locked(key)
    }
}
