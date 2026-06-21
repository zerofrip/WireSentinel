//! Migrate legacy JSON config to SQLite on first startup.

use crate::pool::data_dir;
use crate::repos::{
    RuleRepository, SettingsRepository, SqliteRuleRepository, SqliteSettingsRepository,
    SqliteVpnProfileRepository, VpnProfileRepository,
};
use shared_types::{AppConfig, WireSentinelError};
use sqlx::SqlitePool;
use std::path::PathBuf;
use tracing::{info, warn};

const MIGRATED_FLAG: &str = "legacy_json_migrated";

pub async fn migrate_legacy_if_needed(pool: &SqlitePool) -> Result<(), WireSentinelError> {
    let settings = SqliteSettingsRepository::new(pool.clone());
    if settings.get(MIGRATED_FLAG).await?.is_some() {
        return Ok(());
    }

    let config_path = data_dir().join("config.json");
    if !config_path.exists() {
        settings.set(MIGRATED_FLAG, "true").await?;
        return Ok(());
    }

    info!(path = %config_path.display(), "migrating legacy config.json to SQLite");

    let data = std::fs::read_to_string(&config_path).map_err(WireSentinelError::Io)?;
    let config: AppConfig = serde_json::from_str(&data).map_err(WireSentinelError::Serde)?;

    settings
        .set("policy_mode", &serde_json::to_string(&config.policy_mode).map_err(WireSentinelError::Serde)?)
        .await?;
    settings
        .set("dns", &serde_json::to_string(&config.dns).map_err(WireSentinelError::Serde)?)
        .await?;
    settings
        .set("api_port", &serde_json::to_string(&config.api_port).map_err(WireSentinelError::Serde)?)
        .await?;
    settings
        .set(
            "store_traffic_logs",
            &serde_json::to_string(&config.store_traffic_logs).map_err(WireSentinelError::Serde)?,
        )
        .await?;
    settings
        .set(
            "store_dns_logs",
            &serde_json::to_string(&config.store_dns_logs).map_err(WireSentinelError::Serde)?,
        )
        .await?;

    let rules_repo = SqliteRuleRepository::new(pool.clone());
    for rule in &config.rules {
        if rules_repo.get(rule.id).await?.is_none() {
            rules_repo.insert(rule).await?;
        }
    }

    let vpn_repo = SqliteVpnProfileRepository::new(pool.clone());
    let tunnels_dir = data_dir().join("tunnels");
    for profile in &config.vpn_profiles {
        if vpn_repo.get(profile.id).await?.is_some() {
            continue;
        }
        let blob = if profile.config_path.exists() {
            std::fs::read(&profile.config_path).map_err(WireSentinelError::Io)?
        } else {
            warn!(path = %profile.config_path.display(), "tunnel config missing during migration");
            Vec::new()
        };
        vpn_repo.insert(profile, &blob).await?;
    }

    // Also migrate loose .conf.dpapi files in tunnels dir
    if tunnels_dir.exists() {
        for entry in std::fs::read_dir(&tunnels_dir).map_err(WireSentinelError::Io)? {
            let entry = entry.map_err(WireSentinelError::Io)?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("dpapi") {
                continue;
            }
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("imported")
                .trim_end_matches(".conf");
            let blob = std::fs::read(&path).map_err(WireSentinelError::Io)?;
            let profile = shared_types::VPNProfile::new(
                name.to_string(),
                shared_types::VpnBackendKind::WireGuardNt,
                PathBuf::from(format!("db://migrated/{name}")),
            );
            if vpn_repo.get(profile.id).await?.is_none() {
                vpn_repo.insert(&profile, &blob).await?;
            }
        }
    }

    let migrated_path = data_dir().join("config.json.migrated");
    std::fs::rename(&config_path, &migrated_path).map_err(WireSentinelError::Io)?;

    settings.set(MIGRATED_FLAG, "true").await?;
    info!("legacy JSON migration complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_pool_in_memory;

    #[tokio::test]
    async fn migration_flag_set_without_json() {
        let pool = init_pool_in_memory().await.unwrap();
        migrate_legacy_if_needed(&pool).await.unwrap();
        let settings = SqliteSettingsRepository::new(pool);
        assert!(settings.get(MIGRATED_FLAG).await.unwrap().is_some());
    }
}
