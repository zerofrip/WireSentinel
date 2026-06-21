use crate::plugin::{default_plugins, FilterPlugin};
use crate::FilterSubscription;
use parking_lot::RwLock;
use shared_types::FilterListType;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

/// In-memory filter engine backed by on-disk cache files and pluggable parsers.
pub struct FilterListEngine {
    blocked: RwLock<HashSet<String>>,
    subscriptions: RwLock<Vec<FilterSubscription>>,
    plugins: RwLock<Vec<Arc<dyn FilterPlugin>>>,
    http: reqwest::Client,
}

impl Default for FilterListEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl FilterListEngine {
    pub fn new() -> Self {
        Self {
            blocked: RwLock::new(HashSet::new()),
            subscriptions: RwLock::new(Vec::new()),
            plugins: RwLock::new(default_plugins()),
            http: reqwest::Client::builder()
                .user_agent("WireSentinel/0.1")
                .build()
                .expect("reqwest client"),
        }
    }

    pub fn register_plugin(&self, plugin: Arc<dyn FilterPlugin>) {
        let mut plugins = self.plugins.write();
        plugins.retain(|p| p.list_type() != plugin.list_type());
        plugins.push(plugin);
    }

    pub fn plugins(&self) -> Vec<Arc<dyn FilterPlugin>> {
        self.plugins.read().clone()
    }

    pub fn add_subscription(&self, mut sub: FilterSubscription) {
        if sub.cache_path.as_os_str().is_empty() {
            sub.cache_path = default_cache_path(sub.id);
        }
        self.subscriptions.write().push(sub);
    }

    pub fn subscriptions(&self) -> Vec<FilterSubscription> {
        self.subscriptions.read().clone()
    }

    pub fn replace_subscriptions(&self, subs: Vec<FilterSubscription>) {
        *self.subscriptions.write() = subs;
        let _ = self.rebuild_blocked();
    }

    pub fn is_blocked(&self, domain: &str) -> bool {
        let blocked = self.blocked.read();
        domain_chain(domain).iter().any(|d| blocked.contains(d))
    }

    pub async fn update_all(&self) -> Result<(), String> {
        let subs: Vec<_> = self
            .subscriptions
            .read()
            .iter()
            .filter(|s| s.enabled)
            .cloned()
            .collect();

        for sub in subs {
            self.fetch_and_cache(&sub).await?;
            self.mark_updated(sub.id);
        }

        self.rebuild_blocked()
    }

    pub async fn update_one(&self, id: Uuid) -> Result<(), String> {
        let sub = self
            .subscriptions
            .read()
            .iter()
            .find(|s| s.id == id)
            .cloned()
            .ok_or_else(|| format!("subscription {id} not found"))?;

        if !sub.enabled {
            return Err(format!("subscription {id} is disabled"));
        }

        self.fetch_and_cache(&sub).await?;
        self.mark_updated(id);
        self.rebuild_blocked()
    }

    pub fn reload_from_cache(&self) -> Result<(), String> {
        self.rebuild_blocked()
    }

    fn rebuild_blocked(&self) -> Result<(), String> {
        let mut merged = HashSet::new();
        let subs = self.subscriptions.read().clone();
        let plugins = self.plugins.read().clone();

        for sub in subs.iter().filter(|s| s.enabled) {
            let content = std::fs::read_to_string(&sub.cache_path).map_err(|e| {
                format!(
                    "read cache {}: {e}",
                    sub.cache_path.display()
                )
            })?;
            merged.extend(parse_with_plugins(sub.list_type, &content, &plugins));
        }

        *self.blocked.write() = merged;
        Ok(())
    }

    async fn fetch_and_cache(&self, sub: &FilterSubscription) -> Result<(), String> {
        let response = self
            .http
            .get(&sub.url)
            .send()
            .await
            .map_err(|e| format!("fetch {}: {e}", sub.url))?;

        if !response.status().is_success() {
            return Err(format!(
                "fetch {}: HTTP {}",
                sub.url,
                response.status()
            ));
        }

        let body = response
            .text()
            .await
            .map_err(|e| format!("read body {}: {e}", sub.url))?;

        if let Some(parent) = sub.cache_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("create cache dir {}: {e}", parent.display()))?;
        }

        std::fs::write(&sub.cache_path, &body).map_err(|e| {
            format!(
                "write cache {}: {e}",
                sub.cache_path.display()
            )
        })?;

        tracing::info!(
            id = %sub.id,
            name = %sub.name,
            path = %sub.cache_path.display(),
            "filter list cached"
        );
        Ok(())
    }

    fn mark_updated(&self, id: Uuid) {
        let now = chrono::Utc::now();
        let mut subs = self.subscriptions.write();
        if let Some(sub) = subs.iter_mut().find(|s| s.id == id) {
            sub.last_updated = Some(now);
        }
    }
}

fn parse_with_plugins(
    list_type: FilterListType,
    content: &str,
    plugins: &[Arc<dyn FilterPlugin>],
) -> HashSet<String> {
    plugins
        .iter()
        .find(|p| p.list_type() == list_type)
        .map(|p| p.parse(content))
        .unwrap_or_default()
}

fn domain_chain(domain: &str) -> Vec<String> {
    let normalized = domain.trim_end_matches('.').to_ascii_lowercase();
    let labels: Vec<&str> = normalized.split('.').collect();
    (0..labels.len())
        .map(|i| labels[i..].join("."))
        .collect()
}

/// Cache directory matching `storage::data_dir()/filters/`.
pub fn filters_cache_dir() -> PathBuf {
    filters_data_dir().join("filters")
}

fn filters_data_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var("PROGRAMDATA")
            .map(|p| PathBuf::from(p).join("WireSentinel"))
            .unwrap_or_else(|_| PathBuf::from(r"C:\ProgramData\WireSentinel"))
    } else {
        PathBuf::from("/tmp/WireSentinel")
    }
}

fn default_cache_path(id: Uuid) -> PathBuf {
    filters_cache_dir().join(format!("{id}.cache"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::FilterListType;

    #[test]
    fn blocks_domain_and_parents() {
        let engine = FilterListEngine::new();
        engine.blocked.write().insert("example.com".into());

        assert!(engine.is_blocked("ads.example.com"));
        assert!(engine.is_blocked("example.com"));
        assert!(!engine.is_blocked("other.test"));
    }

    #[test]
    fn reloads_from_cache_file() {
        let dir = std::env::temp_dir().join(format!("ws-filter-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let cache_path = dir.join("test.cache");
        std::fs::write(&cache_path, "0.0.0.0 blocked.test\n").unwrap();

        let engine = FilterListEngine::new();
        engine.add_subscription(FilterSubscription {
            id: Uuid::new_v4(),
            name: "test".into(),
            url: "https://example.com/list.txt".into(),
            list_type: FilterListType::Hosts,
            enabled: true,
            update_interval_secs: None,
            last_updated: None,
            cache_path: cache_path.clone(),
        });

        engine.reload_from_cache().unwrap();
        assert!(engine.is_blocked("blocked.test"));

        let _ = std::fs::remove_dir_all(dir);
    }
}
