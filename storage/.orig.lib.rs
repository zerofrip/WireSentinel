//! SQLite persistence layer with repository traits.

mod migrate_legacy;
mod pool;
pub mod repos;

pub use migrate_legacy::migrate_legacy_if_needed;
pub use pool::{data_dir, db_path, init_pool, init_pool_in_memory};
pub use repos::*;

use sqlx::SqlitePool;
use std::sync::Arc;

/// Aggregated database access for dependency injection.
pub struct Storage {
    pub pool: SqlitePool,
    pub apps: Arc<dyn AppRepository>,
    pub rules: Arc<dyn RuleRepository>,
    pub vpn_profiles: Arc<dyn VpnProfileRepository>,
    pub traffic_logs: Arc<dyn TrafficLogRepository>,
    pub dns_logs: Arc<dyn DnsLogRepository>,
    pub bandwidth: Arc<dyn BandwidthRepository>,
    pub settings: Arc<dyn SettingsRepository>,
    pub filter_lists: Arc<dyn FilterListRepository>,
    pub correlations: Arc<dyn CorrelationRepository>,
    pub route_statistics: Arc<dyn RouteStatisticsRepository>,
    pub audit_log: Arc<dyn AuditLogRepository>,
    pub domain_cache: Arc<dyn DomainCacheRepository>,
    pub wfp_filter_state: Arc<dyn WfpFilterStateRepository>,
    pub firewall_decisions: Arc<dyn FirewallDecisionRepository>,
    pub vpn_config_files: Arc<dyn VpnConfigFileRepository>,
    pub transport_profiles: Arc<dyn TransportProfileRepository>,
    pub chain_profiles: Arc<dyn ChainProfileRepository>,
    pub obfuscation_profiles: Arc<dyn ObfuscationProfileRepository>,
    pub dns_providers: Arc<dyn DnsProviderRepository>,
    pub leak_incidents: Arc<dyn LeakIncidentRepository>,
    pub privacy_snapshots: Arc<dyn PrivacySnapshotRepository>,
    pub runtime_state: Arc<dyn RuntimeStateRepository>,
    pub performance: Arc<dyn PerformanceRepository>,
    pub enterprise_policy: Arc<dyn EnterprisePolicyRepository>,
    pub backup_manifest: Arc<dyn BackupManifestRepository>,
    pub validation_results: Arc<dyn ValidationResultRepository>,
    pub benchmarks: Arc<dyn BenchmarkRepository>,
    pub security_findings: Arc<dyn SecurityFindingRepository>,
    pub plugins: Arc<dyn PluginRepository>,
    pub tailnet_profiles: Arc<dyn TailnetProfileRepository>,
    pub tor_profiles: Arc<dyn TorProfileRepository>,
    pub bridge_profiles: Arc<dyn BridgeProfileRepository>,
    pub proxy_profiles: Arc<dyn ProxyProfileRepository>,
    pub proxy_chains: Arc<dyn ProxyChainRepository>,
    pub mixnet_profiles: Arc<dyn MixnetProfileRepository>,
    pub mixnet_sessions: Arc<dyn MixnetSessionRepository>,
    pub anonymous_chains: Arc<dyn AnonymousChainRepository>,
    pub cover_traffic: Arc<dyn CoverTrafficRepository>,
    pub privacy_analytics: Arc<dyn PrivacyAnalyticsRepository>,
    pub katzenpost_profiles: Arc<dyn KatzenpostProfileRepository>,
    pub loopix_profiles: Arc<dyn LoopixProfileRepository>,
    pub anonymous_services: Arc<dyn AnonymousServiceRepository>,
    pub tcp_termination: Arc<dyn TcpTerminationRepository>,
    pub split_templates: Arc<dyn SplitTemplateRepository>,
}

impl Storage {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            apps: Arc::new(SqliteAppRepository::new(pool.clone())),
            rules: Arc::new(SqliteRuleRepository::new(pool.clone())),
            vpn_profiles: Arc::new(SqliteVpnProfileRepository::new(pool.clone())),
            traffic_logs: Arc::new(SqliteTrafficLogRepository::new(pool.clone())),
            dns_logs: Arc::new(SqliteDnsLogRepository::new(pool.clone())),
            bandwidth: Arc::new(SqliteBandwidthRepository::new(pool.clone())),
            settings: Arc::new(SqliteSettingsRepository::new(pool.clone())),
            filter_lists: Arc::new(SqliteFilterListRepository::new(pool.clone())),
            correlations: Arc::new(SqliteCorrelationRepository::new(pool.clone())),
            route_statistics: Arc::new(SqliteRouteStatisticsRepository::new(pool.clone())),
            audit_log: Arc::new(SqliteAuditLogRepository::new(pool.clone())),
            domain_cache: Arc::new(SqliteDomainCacheRepository::new(pool.clone())),
            wfp_filter_state: Arc::new(SqliteWfpFilterStateRepository::new(pool.clone())),
            firewall_decisions: Arc::new(SqliteFirewallDecisionRepository::new(pool.clone())),
            vpn_config_files: Arc::new(SqliteVpnConfigFileRepository::new(pool.clone())),
            transport_profiles: Arc::new(SqliteTransportProfileRepository::new(pool.clone())),
            chain_profiles: Arc::new(SqliteChainProfileRepository::new(pool.clone())),
            obfuscation_profiles: Arc::new(SqliteObfuscationProfileRepository::new(pool.clone())),
            dns_providers: Arc::new(SqliteDnsProviderRepository::new(pool.clone())),
            leak_incidents: Arc::new(SqliteLeakIncidentRepository::new(pool.clone())),
            privacy_snapshots: Arc::new(SqlitePrivacySnapshotRepository::new(pool.clone())),
            runtime_state: Arc::new(SqliteRuntimeStateRepository::new(pool.clone())),
            performance: Arc::new(SqlitePerformanceRepository::new(pool.clone())),
            enterprise_policy: Arc::new(SqliteEnterprisePolicyRepository::new(pool.clone())),
            backup_manifest: Arc::new(SqliteBackupManifestRepository::new(pool.clone())),
            validation_results: Arc::new(SqliteValidationResultRepository::new(pool.clone())),
            benchmarks: Arc::new(SqliteBenchmarkRepository::new(pool.clone())),
            security_findings: Arc::new(SqliteSecurityFindingRepository::new(pool.clone())),
            plugins: Arc::new(SqlitePluginRepository::new(pool.clone())),
            tailnet_profiles: Arc::new(SqliteTailnetProfileRepository::new(pool.clone())),
            tor_profiles: Arc::new(SqliteTorProfileRepository::new(pool.clone())),
            bridge_profiles: Arc::new(SqliteBridgeProfileRepository::new(pool.clone())),
            proxy_profiles: Arc::new(SqliteProxyProfileRepository::new(pool.clone())),
            proxy_chains: Arc::new(SqliteProxyChainRepository::new(pool.clone())),
            mixnet_profiles: Arc::new(SqliteMixnetProfileRepository::new(pool.clone())),
            mixnet_sessions: Arc::new(SqliteMixnetSessionRepository::new(pool.clone())),
            anonymous_chains: Arc::new(SqliteAnonymousChainRepository::new(pool.clone())),
            cover_traffic: Arc::new(SqliteCoverTrafficRepository::new(pool.clone())),
            privacy_analytics: Arc::new(SqlitePrivacyAnalyticsRepository::new(pool.clone())),
            katzenpost_profiles: Arc::new(SqliteKatzenpostProfileRepository::new(pool.clone())),
            loopix_profiles: Arc::new(SqliteLoopixProfileRepository::new(pool.clone())),
            anonymous_services: Arc::new(SqliteAnonymousServiceRepository::new(pool.clone())),
            tcp_termination: Arc::new(SqliteTcpTerminationRepository::new(pool.clone())),
            split_templates: Arc::new(SqliteSplitTemplateRepository::new(pool.clone())),
            pool,
        }
    }

    pub async fn open() -> shared_types::Result<Self> {
        let pool = init_pool(None).await?;
        migrate_legacy_if_needed(&pool).await?;
        Ok(Self::new(pool))
    }
}
