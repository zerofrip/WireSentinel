use super::traits::{Result, SettingsRepository};
use async_trait::async_trait;
use shared_types::{DnsSettings, LogLevel, WireSentinelError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteSettingsRepository {
    pool: SqlitePool,
}

impl SqliteSettingsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, key: &str, default: T) -> Result<T> {
        match self.get(key).await? {
            Some(json) => serde_json::from_str(&json).map_err(WireSentinelError::Serde),
            None => Ok(default),
        }
    }
}

#[async_trait]
impl SettingsRepository for SqliteSettingsRepository {
    async fn get(&self, key: &str) -> Result<Option<String>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT value_json FROM settings WHERE key = ?")
                .bind(key)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(row.map(|r| r.0))
    }

    async fn set(&self, key: &str, value_json: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO settings (key, value_json) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json",
        )
        .bind(key)
        .bind(value_json)
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn get_dns_settings(&self) -> Result<DnsSettings> {
        self.get_json("dns", DnsSettings::default()).await
    }

    async fn set_dns_settings(&self, settings: &DnsSettings) -> Result<()> {
        let json = serde_json::to_string(settings).map_err(WireSentinelError::Serde)?;
        self.set("dns", &json).await
    }

    async fn get_api_port(&self) -> Result<u16> {
        self.get_json("api_port", 8170u16).await
    }

    async fn set_api_port(&self, port: u16) -> Result<()> {
        let json = serde_json::to_string(&port).map_err(WireSentinelError::Serde)?;
        self.set("api_port", &json).await
    }

    async fn store_traffic_logs(&self) -> Result<bool> {
        self.get_json("store_traffic_logs", true).await
    }

    async fn store_dns_logs(&self) -> Result<bool> {
        self.get_json("store_dns_logs", true).await
    }

    async fn vpn_wireguard_impl(&self) -> Result<String> {
        self.get_json("vpn_wireguard_impl", "scm".to_string()).await
    }

    async fn traffic_monitor_backend(&self) -> Result<String> {
        self.get_json("traffic_monitor_backend", "iphlpapi".to_string())
            .await
    }

    async fn set_traffic_monitor_backend(&self, backend: &str) -> Result<()> {
        let json = serde_json::to_string(backend).map_err(WireSentinelError::Serde)?;
        self.set("traffic_monitor_backend", &json).await
    }

    async fn wfp_engine_impl(&self) -> Result<String> {
        self.get_json("wfp_engine_impl", "userspace".to_string())
            .await
    }

    async fn dns_block_mode(&self) -> Result<String> {
        self.get_json("dns_block_mode", "nxdomain".to_string())
            .await
    }

    async fn store_firewall_decisions(&self) -> Result<bool> {
        self.get_json("store_firewall_decisions", true).await
    }

    async fn vpn_amnezia_impl(&self) -> Result<String> {
        self.get_json("vpn_amnezia_impl", "scm".to_string()).await
    }

    async fn global_obfuscation_profile(&self) -> Result<Option<Uuid>> {
        self.get_json("global_obfuscation_profile", None::<Uuid>)
            .await
    }

    async fn set_global_obfuscation_profile(&self, id: Option<Uuid>) -> Result<()> {
        let json = serde_json::to_string(&id).map_err(WireSentinelError::Serde)?;
        self.set("global_obfuscation_profile", &json).await
    }

    async fn dns_provider_failover(&self) -> Result<bool> {
        self.get_json("dns_provider_failover", true).await
    }

    async fn leak_detection_enabled(&self) -> Result<bool> {
        self.get_json("leak_detection_enabled", true).await
    }

    async fn privacy_score_interval_secs(&self) -> Result<u64> {
        self.get_json("privacy_score_interval_secs", 60u64).await
    }

    async fn log_level(&self) -> Result<LogLevel> {
        self.get_json("log_level", LogLevel::Info).await
    }

    async fn set_log_level(&self, level: LogLevel) -> Result<()> {
        let json = serde_json::to_string(&level).map_err(WireSentinelError::Serde)?;
        self.set("log_level", &json).await
    }

    async fn log_json_enabled(&self) -> Result<bool> {
        self.get_json("log_json_enabled", false).await
    }

    async fn set_log_json_enabled(&self, enabled: bool) -> Result<()> {
        let json = serde_json::to_string(&enabled).map_err(WireSentinelError::Serde)?;
        self.set("log_json_enabled", &json).await
    }

    async fn log_max_files(&self) -> Result<u32> {
        self.get_json("log_max_files", 5u32).await
    }

    async fn set_log_max_files(&self, count: u32) -> Result<()> {
        let json = serde_json::to_string(&count).map_err(WireSentinelError::Serde)?;
        self.set("log_max_files", &json).await
    }

    async fn recovery_enabled(&self) -> Result<bool> {
        self.get_json("recovery_enabled", true).await
    }

    async fn set_recovery_enabled(&self, enabled: bool) -> Result<()> {
        let json = serde_json::to_string(&enabled).map_err(WireSentinelError::Serde)?;
        self.set("recovery_enabled", &json).await
    }

    async fn metrics_interval_secs(&self) -> Result<u64> {
        self.get_json("metrics_interval_secs", 30u64).await
    }

    async fn set_metrics_interval_secs(&self, secs: u64) -> Result<()> {
        let json = serde_json::to_string(&secs).map_err(WireSentinelError::Serde)?;
        self.set("metrics_interval_secs", &json).await
    }

    async fn update_channel(&self) -> Result<String> {
        self.get_json("update_channel", "stable".to_string()).await
    }

    async fn set_update_channel(&self, channel: &str) -> Result<()> {
        let json = serde_json::to_string(channel).map_err(WireSentinelError::Serde)?;
        self.set("update_channel", &json).await
    }

    async fn enterprise_policy_id(&self) -> Result<Option<Uuid>> {
        self.get_json("enterprise_policy_id", None::<Uuid>).await
    }

    async fn set_enterprise_policy_id(&self, id: Option<Uuid>) -> Result<()> {
        let json = serde_json::to_string(&id).map_err(WireSentinelError::Serde)?;
        self.set("enterprise_policy_id", &json).await
    }

    async fn benchmark_interval_secs(&self) -> Result<u64> {
        self.get_json("benchmark_interval_secs", 60u64).await
    }

    async fn set_benchmark_interval_secs(&self, secs: u64) -> Result<()> {
        let json = serde_json::to_string(&secs).map_err(WireSentinelError::Serde)?;
        self.set("benchmark_interval_secs", &json).await
    }

    async fn guardian_mode(&self) -> Result<String> {
        self.get_json("guardian_mode", "wfp".to_string()).await
    }

    async fn enforcement_backend(&self) -> Result<String> {
        self.get_json("enforcement_backend", "signed".to_string())
            .await
    }

    async fn set_enforcement_backend(&self, backend: &str) -> Result<()> {
        let json = serde_json::to_string(backend).map_err(WireSentinelError::Serde)?;
        self.set("enforcement_backend", &json).await
    }

    async fn set_guardian_mode(&self, mode: &str) -> Result<()> {
        let json = serde_json::to_string(mode).map_err(WireSentinelError::Serde)?;
        self.set("guardian_mode", &json).await
    }

    async fn set_wfp_engine_impl(&self, impl_name: &str) -> Result<()> {
        let json = serde_json::to_string(impl_name).map_err(WireSentinelError::Serde)?;
        self.set("wfp_engine_impl", &json).await
    }
}
