//! Configuration backup export/import with audit manifest.

use chrono::Utc;
use event_bus::EventBus;
use shared_types::{
    BackupBundle, BackupManifestEntry, EnterprisePolicy, Result, SecurityAuditEntry,
    ServiceEventInner, WireSentinelError,
};
use std::sync::Arc;
use storage::Storage;
use uuid::Uuid;

pub struct BackupService {
    storage: Arc<Storage>,
    events: EventBus,
}

impl BackupService {
    pub fn new(storage: Arc<Storage>, events: EventBus) -> Self {
        Self { storage, events }
    }

    pub async fn export_json(&self) -> Result<(BackupBundle, String)> {
        let bundle = self.build_bundle().await?;
        let json = serde_json::to_string_pretty(&bundle).map_err(WireSentinelError::Serde)?;
        let checksum = format!("{:x}", md5::compute(json.as_bytes()));
        self.record_manifest("export", "json", &checksum, serde_json::json!({}))
            .await?;
        Ok((bundle, json))
    }

    pub async fn export_encrypted(&self) -> Result<Vec<u8>> {
        let (_, json) = self.export_json().await?;
        let encrypted = encrypt_backup(json.as_bytes())?;
        let checksum = format!("{:x}", md5::compute(&encrypted));
        self.record_manifest("export", "encrypted", &checksum, serde_json::json!({}))
            .await?;
        Ok(encrypted)
    }

    pub async fn import_json(&self, json: &str) -> Result<()> {
        let bundle: BackupBundle = serde_json::from_str(json).map_err(WireSentinelError::Serde)?;
        self.apply_bundle(&bundle).await?;
        let checksum = format!("{:x}", md5::compute(json.as_bytes()));
        self.record_manifest("import", "json", &checksum, serde_json::json!({ "version": bundle.version }))
            .await?;
        self.audit("backup_import", Some("json import"));
        Ok(())
    }

    pub async fn import_encrypted(&self, data: &[u8]) -> Result<()> {
        let plain = decrypt_backup(data)?;
        let json = std::str::from_utf8(&plain)
            .map_err(|e| WireSentinelError::Config(format!("backup utf8: {e}")))?;
        self.import_json(json).await?;
        self.audit("backup_import", Some("encrypted import"));
        Ok(())
    }

    async fn build_bundle(&self) -> Result<BackupBundle> {
        let rules = self.storage.rules.list().await?;
        let apps = self.storage.apps.list(storage::AppFilter::default()).await?;
        let dns_settings = self.storage.settings.get_dns_settings().await?;
        let dns_providers = self.storage.dns_providers.list().await?;
        let filter_lists = self.storage.filter_lists.list().await?;
        let transport_profiles = self.storage.transport_profiles.list().await?;
        let chain_profiles = self.storage.chain_profiles.list().await?;
        let obfuscation_profiles = self.storage.obfuscation_profiles.list().await?;
        let proxy_profiles = self.storage.proxy_profiles.list().await?;
        let proxy_chains = self.storage.proxy_chains.list().await?;
        let mixnet_profiles = self.storage.mixnet_profiles.list().await?;
        let anonymous_chains = self.storage.anonymous_chains.list().await?;
        let cover_traffic_settings = self.storage.cover_traffic.list().await?;
        let enterprise_policy = self.storage.enterprise_policy.get_active().await?;
        let plugins = self.storage.plugins.list().await?;
        let tailnet_profiles = self.storage.tailnet_profiles.list().await?;
        let tor_profiles = self.storage.tor_profiles.list().await?;
        let controller_agent_config = self
            .storage
            .settings
            .get("controller_agent")
            .await?
            .and_then(|json| serde_json::from_str(&json).ok());
        let cloud_sync_config = self
            .storage
            .settings
            .get("cloud_sync")
            .await?
            .and_then(|json| serde_json::from_str(&json).ok());

        let vpn_profiles: Vec<serde_json::Value> = self
            .storage
            .vpn_profiles
            .list()
            .await?
            .into_iter()
            .map(|p| serde_json::to_value(p).unwrap_or_default())
            .collect();

        let settings = serde_json::json!({
            "api_port": self.storage.settings.get_api_port().await?,
            "log_level": self.storage.settings.log_level().await?,
            "recovery_enabled": self.storage.settings.recovery_enabled().await?,
            "metrics_interval_secs": self.storage.settings.metrics_interval_secs().await?,
            "update_channel": self.storage.settings.update_channel().await?,
        });

        Ok(BackupBundle {
            version: 1,
            exported_at: Utc::now(),
            settings,
            vpn_profiles,
            rules: rules.into_iter().map(|r| serde_json::to_value(r).unwrap_or_default()).collect(),
            apps: apps.into_iter().map(|a| serde_json::to_value(a).unwrap_or_default()).collect(),
            dns_settings: serde_json::to_value(dns_settings).map_err(WireSentinelError::Serde)?,
            dns_providers: dns_providers.into_iter().map(|p| serde_json::to_value(p).unwrap_or_default()).collect(),
            filter_lists: filter_lists.into_iter().map(|f| serde_json::to_value(f).unwrap_or_default()).collect(),
            transport_profiles: transport_profiles.into_iter().map(|t| serde_json::to_value(t).unwrap_or_default()).collect(),
            chain_profiles: chain_profiles.into_iter().map(|c| serde_json::to_value(c).unwrap_or_default()).collect(),
            obfuscation_profiles: obfuscation_profiles.into_iter().map(|o| serde_json::to_value(o).unwrap_or_default()).collect(),
            proxy_profiles: proxy_profiles.into_iter().map(|p| serde_json::to_value(p).unwrap_or_default()).collect(),
            proxy_chains: proxy_chains.into_iter().map(|c| serde_json::to_value(c).unwrap_or_default()).collect(),
            mixnet_profiles: mixnet_profiles.into_iter().map(|p| serde_json::to_value(p).unwrap_or_default()).collect(),
            anonymous_chains: anonymous_chains.into_iter().map(|c| serde_json::to_value(c).unwrap_or_default()).collect(),
            cover_traffic_settings: cover_traffic_settings.into_iter().map(|s| serde_json::to_value(s).unwrap_or_default()).collect(),
            enterprise_policy,
            bundle_version: 2,
            plugins: plugins.into_iter().map(|p| serde_json::to_value(p).unwrap_or_default()).collect(),
            tailnet_profiles: tailnet_profiles.into_iter().map(|p| serde_json::to_value(p).unwrap_or_default()).collect(),
            tor_profiles: tor_profiles.into_iter().map(|p| serde_json::to_value(p).unwrap_or_default()).collect(),
            controller_agent_config,
            cloud_sync_config,
        })
    }

    async fn apply_bundle(&self, bundle: &BackupBundle) -> Result<()> {
        if let Some(settings) = bundle.settings.as_object() {
            if let Some(port) = settings.get("api_port").and_then(|v| v.as_u64()) {
                self.storage.settings.set_api_port(port as u16).await?;
            }
            if let Some(enabled) = settings.get("recovery_enabled").and_then(|v| v.as_bool()) {
                self.storage.settings.set_recovery_enabled(enabled).await?;
            }
        }

        let dns: shared_types::DnsSettings =
            serde_json::from_value(bundle.dns_settings.clone()).map_err(WireSentinelError::Serde)?;
        self.storage.settings.set_dns_settings(&dns).await?;

        for rule_val in &bundle.rules {
            if let Ok(rule) = serde_json::from_value::<shared_types::Rule>(rule_val.clone()) {
                let _ = self.storage.rules.insert(&rule).await;
            }
        }

        for app_val in &bundle.apps {
            if let Ok(app) = serde_json::from_value::<shared_types::AppRecord>(app_val.clone()) {
                let _ = self.storage.apps.upsert(&app).await;
            }
        }

        for list_val in &bundle.filter_lists {
            if let Ok(list) =
                serde_json::from_value::<shared_types::FilterListRecord>(list_val.clone())
            {
                let _ = self.storage.filter_lists.insert(&list).await;
            }
        }

        for profile_val in &bundle.transport_profiles {
            if let Ok(profile) =
                serde_json::from_value::<shared_types::TransportProfile>(profile_val.clone())
            {
                let _ = self.storage.transport_profiles.insert(&profile).await;
            }
        }

        for chain_val in &bundle.chain_profiles {
            if let Ok(chain) =
                serde_json::from_value::<shared_types::ChainProfile>(chain_val.clone())
            {
                let _ = self.storage.chain_profiles.insert(&chain).await;
            }
        }

        for profile_val in &bundle.obfuscation_profiles {
            if let Ok(profile) =
                serde_json::from_value::<shared_types::ObfuscationProfile>(profile_val.clone())
            {
                let _ = self.storage.obfuscation_profiles.insert(&profile).await;
            }
        }

        for provider_val in &bundle.dns_providers {
            if let Ok(provider) =
                serde_json::from_value::<shared_types::DnsProviderRecord>(provider_val.clone())
            {
                let _ = self.storage.dns_providers.upsert(&provider).await;
            }
        }

        for profile_val in &bundle.proxy_profiles {
            if let Ok(profile) = serde_json::from_value::<shared_types::ProxyProfile>(profile_val.clone()) {
                let _ = self.storage.proxy_profiles.insert(&profile).await;
            }
        }

        for chain_val in &bundle.proxy_chains {
            if let Ok(chain) = serde_json::from_value::<shared_types::ProxyChain>(chain_val.clone()) {
                let _ = self.storage.proxy_chains.insert(&chain).await;
            }
        }

        for profile_val in &bundle.mixnet_profiles {
            if let Ok(profile) = serde_json::from_value::<shared_types::MixnetProfile>(profile_val.clone()) {
                let _ = self.storage.mixnet_profiles.insert(&profile).await;
            }
        }

        for chain_val in &bundle.anonymous_chains {
            if let Ok(chain) = serde_json::from_value::<shared_types::AnonymousChain>(chain_val.clone()) {
                let _ = self.storage.anonymous_chains.insert(&chain).await;
            }
        }

        for settings_val in &bundle.cover_traffic_settings {
            if let Ok(settings) =
                serde_json::from_value::<shared_types::CoverTrafficSettings>(settings_val.clone())
            {
                let _ = self.storage.cover_traffic.insert(&settings).await;
            }
        }

        if let Some(policy) = &bundle.enterprise_policy {
            self.storage.enterprise_policy.upsert(policy).await?;
        }

        for plugin_val in &bundle.plugins {
            if let Ok(plugin) = serde_json::from_value::<shared_types::PluginRecord>(plugin_val.clone())
            {
                let _ = self.storage.plugins.upsert(&plugin).await;
            }
        }

        for profile_val in &bundle.tailnet_profiles {
            if let Ok(profile) =
                serde_json::from_value::<shared_types::TailnetProfile>(profile_val.clone())
            {
                let _ = self.storage.tailnet_profiles.insert(&profile).await;
            }
        }

        for profile_val in &bundle.tor_profiles {
            if let Ok(profile) =
                serde_json::from_value::<shared_types::TorProfile>(profile_val.clone())
            {
                let _ = self.storage.tor_profiles.insert(&profile).await;
            }
        }

        if let Some(cfg) = &bundle.controller_agent_config {
            if let Ok(json) = serde_json::to_string(cfg) {
                let _ = self.storage.settings.set("controller_agent", &json).await;
            }
        }

        if let Some(cfg) = &bundle.cloud_sync_config {
            if let Ok(json) = serde_json::to_string(cfg) {
                let _ = self.storage.settings.set("cloud_sync", &json).await;
            }
        }

        Ok(())
    }

    async fn record_manifest(
        &self,
        operation: &str,
        format: &str,
        checksum: &str,
        detail: serde_json::Value,
    ) -> Result<()> {
        self.storage
            .backup_manifest
            .insert(&BackupManifestEntry {
                id: Uuid::new_v4(),
                operation: operation.into(),
                format: format.into(),
                checksum: checksum.into(),
                created_at: Utc::now(),
                detail_json: detail,
            })
            .await
    }

    fn audit(&self, action: &str, detail: Option<&str>) {
        self.events.publish(
            ServiceEventInner::SecurityAudit {
                entry: SecurityAuditEntry {
                    action: action.into(),
                    actor: None,
                    detail: detail.map(str::to_string),
                    timestamp: Utc::now(),
                },
            }
            .with_timestamp(Utc::now()),
        );
    }
}

fn encrypt_backup(plaintext: &[u8]) -> shared_types::Result<Vec<u8>> {
    #[cfg(windows)]
    {
        use windows::Win32::Security::Cryptography::{
            CryptProtectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_LOCAL_MACHINE,
        };
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: plaintext.len() as u32,
            pbData: plaintext.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        unsafe {
            CryptProtectData(
                &mut input,
                None,
                None,
                None,
                None,
                CRYPTPROTECT_LOCAL_MACHINE,
                &mut output,
            )
            .map_err(|e| WireSentinelError::Config(format!("CryptProtectData: {e}")))?;
            Ok(std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec())
        }
    }
    #[cfg(not(windows))]
    {
        Ok(plaintext.to_vec())
    }
}

fn decrypt_backup(ciphertext: &[u8]) -> shared_types::Result<Vec<u8>> {
    #[cfg(windows)]
    {
        use windows::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: ciphertext.len() as u32,
            pbData: ciphertext.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        unsafe {
            CryptUnprotectData(&mut input, None, None, None, None, 0, &mut output)
                .map_err(|e| WireSentinelError::Config(format!("CryptUnprotectData: {e}")))?;
            Ok(std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec())
        }
    }
    #[cfg(not(windows))]
    {
        Ok(ciphertext.to_vec())
    }
}
