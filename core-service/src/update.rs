//! Auto-update framework (check/notify only in Phase 5).

use shared_types::{Result, UpdateInfo, WireSentinelError};
use std::sync::Arc;
use storage::Storage;

pub struct UpdateManager {
    storage: Arc<Storage>,
    current_version: String,
}

impl UpdateManager {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self {
            storage,
            current_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    pub async fn info(&self) -> Result<UpdateInfo> {
        let channel = self.storage.settings.update_channel().await?;
        Ok(UpdateInfo {
            current_version: self.current_version.clone(),
            latest_version: None,
            channel,
            staged_percent: 100,
            download_url: None,
            update_available: false,
        })
    }

    pub async fn check(&self) -> Result<UpdateInfo> {
        let channel = self.storage.settings.update_channel().await?;
        let feed_url = std::env::var("WIRESENTINEL_UPDATE_FEED").ok();

        if let Some(url) = feed_url {
            if let Ok(latest) = self.fetch_latest(&url, &channel).await {
                let update_available = semver_compare(&latest, &self.current_version) > 0;
                return Ok(UpdateInfo {
                    current_version: self.current_version.clone(),
                    latest_version: Some(latest.clone()),
                    channel,
                    staged_percent: staged_rollout_percent(&latest),
                    download_url: Some(format!("{url}/download/{latest}")),
                    update_available,
                });
            }
        }

        Ok(UpdateInfo {
            current_version: self.current_version.clone(),
            latest_version: Some(self.current_version.clone()),
            channel,
            staged_percent: 100,
            download_url: None,
            update_available: false,
        })
    }

    async fn fetch_latest(&self, base_url: &str, channel: &str) -> Result<String> {
        let url = format!("{base_url}/releases/latest?channel={channel}");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        body.get("version")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| WireSentinelError::Config("missing version in update feed".into()))
    }
}

fn semver_compare(a: &str, b: &str) -> i32 {
    let parse = |s: &str| -> Vec<u32> {
        s.trim_start_matches('v')
            .split('.')
            .filter_map(|p| p.parse().ok())
            .collect()
    };
    let va = parse(a);
    let vb = parse(b);
    for i in 0..va.len().max(vb.len()) {
        let da = *va.get(i).unwrap_or(&0);
        let db = *vb.get(i).unwrap_or(&0);
        if da != db {
            return da.cmp(&db) as i32;
        }
    }
    0
}

fn staged_rollout_percent(version: &str) -> u8 {
    let hash: u32 = version.bytes().map(|b| b as u32).sum();
    ((hash % 100) + 1) as u8
}
