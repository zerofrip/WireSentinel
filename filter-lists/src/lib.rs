//! Filter list subscription engine.

mod engine;
mod plugin;
mod providers;
mod scheduler;

pub use engine::{filters_cache_dir, FilterListEngine};
pub use plugin::{
    default_plugins, EasyListFilterPlugin, FilterPlugin, HostsFilterPlugin, WasmFilterPlugin,
};
pub use scheduler::FilterUpdateScheduler;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use shared_types::FilterListType;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterSubscription {
    pub id: Uuid,
    pub name: String,
    pub url: String,
    pub list_type: FilterListType,
    pub enabled: bool,
    pub update_interval_secs: Option<u32>,
    pub last_updated: Option<chrono::DateTime<chrono::Utc>>,
    pub cache_path: PathBuf,
}

#[async_trait]
pub trait FilterListProvider: Send + Sync {
    async fn update_all(&self) -> Result<(), String>;

    async fn update_one(&self, id: Uuid) -> Result<(), String> {
        let _ = id;
        self.update_all().await
    }

    fn is_blocked(&self, domain: &str) -> bool;

    fn list_subscriptions(&self) -> Vec<FilterSubscription> {
        Vec::new()
    }
}

#[async_trait]
impl FilterListProvider for FilterListEngine {
    async fn update_all(&self) -> Result<(), String> {
        FilterListEngine::update_all(self).await
    }

    async fn update_one(&self, id: Uuid) -> Result<(), String> {
        FilterListEngine::update_one(self, id).await
    }

    fn is_blocked(&self, domain: &str) -> bool {
        FilterListEngine::is_blocked(self, domain)
    }

    fn list_subscriptions(&self) -> Vec<FilterSubscription> {
        FilterListEngine::subscriptions(self)
    }
}
