use async_trait::async_trait;
use shared_types::{
    AppRecord, AuditLogEntry, AuditLogQuery, BandwidthStats, BridgeProfile, ChainProfile,
    CoverTrafficSettings, DNSQueryLog, BackupManifestEntry, BenchmarkSnapshot, DnsProviderRecord,
    DnsSettings, DomainCacheEntry, DomainCorrelation, EnterprisePolicy, FilterListRecord,
    FirewallDecisionRecord, LeakIncident, LogLevel, MixnetProfile, MixnetSession,
    ObfuscationProfile, PerformanceSnapshot, PluginRecord, PolicyMode, PrivacyAnalyticsSnapshot,
    PrivacyScoreSnapshot, ProxyChain, ProxyProfile, RouteStatisticsQuery, AnonymousChain,
    KatzenpostProfile, LoopixProfile, AnonymousService, AnonymousServiceEndpoint,
    RouteStatisticsRecord, Rule, RuntimeStateRecord, SecurityFinding, TailnetProfile, TorProfile,
    TopDomainEntry, TrafficEvent, TransportProfile, ValidationCheck, VpnConfigFileRecord,
    VPNProfile, WfpFilterStateRecord, WireSentinelError,
    TcpTerminationPolicy, TcpTerminationRule, TcpTerminationSettings,
    SplitTunnelTemplate, SplitTemplateModeSettings,
};
use std::path::Path;
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, WireSentinelError>;

#[derive(Debug, Clone, Default)]
pub struct AppFilter {
    pub search: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct TrafficLogQuery {
    pub limit: u32,
    pub offset: u32,
    pub app_id: Option<Uuid>,
    pub sort: TrafficSortField,
    pub order: SortOrder,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum TrafficSortField {
    #[default]
    Timestamp,
    Bytes,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum SortOrder {
    #[default]
    Desc,
    Asc,
}

#[derive(Debug, Clone, Default)]
pub struct DnsLogQuery {
    pub limit: u32,
    pub offset: u32,
    pub qname: Option<String>,
    pub blocked: Option<bool>,
    pub sort: DnsSortField,
    pub order: SortOrder,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum DnsSortField {
    #[default]
    Timestamp,
}

#[derive(Debug, Clone, Default)]
pub struct CorrelationQuery {
    pub limit: u32,
    pub app_id: Option<Uuid>,
    pub domain: Option<String>,
}

#[async_trait]
pub trait AppRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<AppRecord>>;
    async fn find_by_exe_path(&self, path: &Path) -> Result<Option<AppRecord>>;
    async fn find_by_sha256(&self, sha256: &str) -> Result<Option<AppRecord>>;
    async fn upsert(&self, app: &AppRecord) -> Result<()>;
    async fn list(&self, filter: AppFilter) -> Result<Vec<AppRecord>>;
    async fn count(&self) -> Result<u32>;
}

#[async_trait]
pub trait RuleRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<Rule>>;
    async fn get(&self, id: Uuid) -> Result<Option<Rule>>;
    async fn insert(&self, rule: &Rule) -> Result<()>;
    async fn update(&self, rule: &Rule) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
    async fn get_policy_mode(&self) -> Result<PolicyMode>;
    async fn set_policy_mode(&self, mode: PolicyMode) -> Result<()>;
}

#[async_trait]
pub trait VpnProfileRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<VPNProfile>>;
    async fn get(&self, id: Uuid) -> Result<Option<VPNProfile>>;
    async fn get_config_blob(&self, id: Uuid) -> Result<Option<Vec<u8>>>;
    async fn insert(&self, profile: &VPNProfile, config_blob: &[u8]) -> Result<()>;
    async fn update(&self, profile: &VPNProfile, config_blob: Option<&[u8]>) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
}

#[async_trait]
pub trait TrafficLogRepository: Send + Sync {
    async fn insert(&self, event: &TrafficEvent) -> Result<()>;
    async fn list(&self, query: TrafficLogQuery) -> Result<Vec<TrafficEvent>>;
}

#[async_trait]
pub trait DnsLogRepository: Send + Sync {
    async fn insert(&self, log: &DNSQueryLog) -> Result<()>;
    async fn list(&self, query: DnsLogQuery) -> Result<Vec<DNSQueryLog>>;
    async fn top_domains(&self, limit: u32) -> Result<Vec<TopDomainEntry>>;
    async fn count(&self) -> Result<u64>;
}

#[async_trait]
pub trait BandwidthRepository: Send + Sync {
    async fn insert(&self, stats: &BandwidthStats) -> Result<()>;
    async fn latest_per_app(&self, limit: u32) -> Result<Vec<BandwidthStats>>;
}

#[async_trait]
pub trait FilterListRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<FilterListRecord>>;
    async fn get(&self, id: Uuid) -> Result<Option<FilterListRecord>>;
    async fn insert(&self, record: &FilterListRecord) -> Result<()>;
    async fn update(&self, record: &FilterListRecord) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
}

#[async_trait]
pub trait CorrelationRepository: Send + Sync {
    async fn upsert(&self, corr: &DomainCorrelation) -> Result<()>;
    async fn list(&self, query: CorrelationQuery) -> Result<Vec<DomainCorrelation>>;
    async fn record_dns(&self, app_id: Option<Uuid>, domain: &str, ip: &str) -> Result<()>;
    async fn record_traffic(&self, app_id: Option<Uuid>, ip: &str) -> Result<Option<String>>;
}

#[async_trait]
pub trait SettingsRepository: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<String>>;
    async fn set(&self, key: &str, value_json: &str) -> Result<()>;
    async fn get_dns_settings(&self) -> Result<DnsSettings>;
    async fn set_dns_settings(&self, settings: &DnsSettings) -> Result<()>;
    async fn get_api_port(&self) -> Result<u16>;
    async fn set_api_port(&self, port: u16) -> Result<()>;
    async fn store_traffic_logs(&self) -> Result<bool>;
    async fn store_dns_logs(&self) -> Result<bool>;
    async fn vpn_wireguard_impl(&self) -> Result<String>;
    async fn traffic_monitor_backend(&self) -> Result<String>;
    async fn set_traffic_monitor_backend(&self, backend: &str) -> Result<()>;
    async fn wfp_engine_impl(&self) -> Result<String>;
    async fn dns_block_mode(&self) -> Result<String>;
    async fn store_firewall_decisions(&self) -> Result<bool>;
    async fn vpn_amnezia_impl(&self) -> Result<String>;
    async fn global_obfuscation_profile(&self) -> Result<Option<Uuid>>;
    async fn set_global_obfuscation_profile(&self, id: Option<Uuid>) -> Result<()>;
    async fn dns_provider_failover(&self) -> Result<bool>;
    async fn leak_detection_enabled(&self) -> Result<bool>;
    async fn privacy_score_interval_secs(&self) -> Result<u64>;
    async fn log_level(&self) -> Result<LogLevel>;
    async fn set_log_level(&self, level: LogLevel) -> Result<()>;
    async fn log_json_enabled(&self) -> Result<bool>;
    async fn set_log_json_enabled(&self, enabled: bool) -> Result<()>;
    async fn log_max_files(&self) -> Result<u32>;
    async fn set_log_max_files(&self, count: u32) -> Result<()>;
    async fn recovery_enabled(&self) -> Result<bool>;
    async fn set_recovery_enabled(&self, enabled: bool) -> Result<()>;
    async fn metrics_interval_secs(&self) -> Result<u64>;
    async fn set_metrics_interval_secs(&self, secs: u64) -> Result<()>;
    async fn update_channel(&self) -> Result<String>;
    async fn set_update_channel(&self, channel: &str) -> Result<()>;
    async fn enterprise_policy_id(&self) -> Result<Option<Uuid>>;
    async fn set_enterprise_policy_id(&self, id: Option<Uuid>) -> Result<()>;
    async fn benchmark_interval_secs(&self) -> Result<u64>;
    async fn set_benchmark_interval_secs(&self, secs: u64) -> Result<()>;
    async fn guardian_mode(&self) -> Result<String>;
}

#[async_trait]
pub trait ValidationResultRepository: Send + Sync {
    async fn upsert(&self, check: &ValidationCheck) -> Result<()>;
    async fn list_recent(&self, limit: u32) -> Result<Vec<ValidationCheck>>;
    async fn latest_by_name(&self, check_name: &str) -> Result<Option<ValidationCheck>>;
}

#[async_trait]
pub trait BenchmarkRepository: Send + Sync {
    async fn insert(&self, snapshot: &BenchmarkSnapshot) -> Result<()>;
    async fn latest(&self) -> Result<Option<BenchmarkSnapshot>>;
    async fn list_recent(&self, limit: u32) -> Result<Vec<BenchmarkSnapshot>>;
}

#[async_trait]
pub trait SecurityFindingRepository: Send + Sync {
    async fn insert(&self, finding: &SecurityFinding) -> Result<()>;
    async fn list(&self, include_resolved: bool, limit: u32) -> Result<Vec<SecurityFinding>>;
    async fn resolve(&self, id: Uuid) -> Result<bool>;
}

#[async_trait]
pub trait RuntimeStateRepository: Send + Sync {
    async fn upsert(&self, record: &RuntimeStateRecord) -> Result<()>;
    async fn list_by_scope(&self, scope: &str) -> Result<Vec<RuntimeStateRecord>>;
    async fn delete_scope(&self, scope: &str) -> Result<()>;
    async fn list_all(&self) -> Result<Vec<RuntimeStateRecord>>;
}

#[async_trait]
pub trait PerformanceRepository: Send + Sync {
    async fn insert(&self, snapshot: &PerformanceSnapshot) -> Result<()>;
    async fn latest(&self) -> Result<Option<PerformanceSnapshot>>;
    async fn list_recent(&self, limit: u32) -> Result<Vec<PerformanceSnapshot>>;
}

#[async_trait]
pub trait EnterprisePolicyRepository: Send + Sync {
    async fn get_active(&self) -> Result<Option<EnterprisePolicy>>;
    async fn upsert(&self, policy: &EnterprisePolicy) -> Result<()>;
}

#[async_trait]
pub trait BackupManifestRepository: Send + Sync {
    async fn insert(&self, entry: &BackupManifestEntry) -> Result<()>;
    async fn list_recent(&self, limit: u32) -> Result<Vec<BackupManifestEntry>>;
}

#[async_trait]
pub trait RouteStatisticsRepository: Send + Sync {
    async fn upsert(&self, record: &RouteStatisticsRecord) -> Result<()>;
    async fn list(&self, query: RouteStatisticsQuery) -> Result<Vec<RouteStatisticsRecord>>;
    async fn blocked_summary(&self, limit: u32) -> Result<Vec<RouteStatisticsRecord>>;
}

#[async_trait]
pub trait AuditLogRepository: Send + Sync {
    async fn insert(&self, entry: &AuditLogEntry) -> Result<()>;
    async fn list(&self, query: AuditLogQuery) -> Result<Vec<AuditLogEntry>>;
    async fn count_since(&self, event_type: Option<&str>, since: chrono::DateTime<chrono::Utc>) -> Result<u64>;
}

#[async_trait]
pub trait DomainCacheRepository: Send + Sync {
    async fn upsert(&self, entry: &DomainCacheEntry) -> Result<()>;
    async fn lookup_by_ip(&self, app_id: Option<Uuid>, ip: &str) -> Result<Option<DomainCacheEntry>>;
    async fn purge_expired(&self) -> Result<u64>;
}

#[async_trait]
pub trait WfpFilterStateRepository: Send + Sync {
    async fn upsert(&self, record: &WfpFilterStateRecord) -> Result<()>;
    async fn list_all(&self) -> Result<Vec<WfpFilterStateRecord>>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
}

#[async_trait]
pub trait FirewallDecisionRepository: Send + Sync {
    async fn insert(&self, record: &FirewallDecisionRecord) -> Result<()>;
    async fn list_recent(&self, limit: u32) -> Result<Vec<FirewallDecisionRecord>>;
    async fn count(&self) -> Result<u64>;
}

#[async_trait]
pub trait VpnConfigFileRepository: Send + Sync {
    async fn upsert(&self, record: &VpnConfigFileRecord) -> Result<()>;
    async fn get(&self, profile_id: Uuid) -> Result<Option<VpnConfigFileRecord>>;
}

#[async_trait]
pub trait TransportProfileRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<TransportProfile>>;
    async fn get(&self, id: Uuid) -> Result<Option<TransportProfile>>;
    async fn insert(&self, profile: &TransportProfile) -> Result<()>;
    async fn update(&self, profile: &TransportProfile) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
}

#[async_trait]
pub trait ChainProfileRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<ChainProfile>>;
    async fn get(&self, id: Uuid) -> Result<Option<ChainProfile>>;
    async fn insert(&self, profile: &ChainProfile) -> Result<()>;
    async fn update(&self, profile: &ChainProfile) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
}

#[async_trait]
pub trait ObfuscationProfileRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<ObfuscationProfile>>;
    async fn get(&self, id: Uuid) -> Result<Option<ObfuscationProfile>>;
    async fn insert(&self, profile: &ObfuscationProfile) -> Result<()>;
    async fn update(&self, profile: &ObfuscationProfile) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
}

#[async_trait]
pub trait DnsProviderRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<DnsProviderRecord>>;
    async fn get(&self, id: Uuid) -> Result<Option<DnsProviderRecord>>;
    async fn upsert(&self, provider: &DnsProviderRecord) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
}

#[async_trait]
pub trait LeakIncidentRepository: Send + Sync {
    async fn insert(&self, incident: &LeakIncident) -> Result<()>;
    async fn list_recent(&self, limit: u32) -> Result<Vec<LeakIncident>>;
    async fn resolve(&self, id: Uuid) -> Result<bool>;
}

#[async_trait]
pub trait PrivacySnapshotRepository: Send + Sync {
    async fn insert(&self, snapshot: &PrivacyScoreSnapshot) -> Result<()>;
    async fn latest(&self) -> Result<Option<PrivacyScoreSnapshot>>;
    async fn list(&self, limit: u32) -> Result<Vec<PrivacyScoreSnapshot>>;
}

#[async_trait]
pub trait PluginRepository: Send + Sync {
    async fn upsert(&self, record: &PluginRecord) -> Result<()>;
    async fn list(&self) -> Result<Vec<PluginRecord>>;
    async fn get(&self, id: Uuid) -> Result<Option<PluginRecord>>;
    async fn delete(&self, id: Uuid) -> Result<()>;
}

#[async_trait]
pub trait TailnetProfileRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<TailnetProfile>>;
    async fn get(&self, id: Uuid) -> Result<Option<TailnetProfile>>;
    async fn insert(&self, profile: &TailnetProfile) -> Result<()>;
    async fn update(&self, profile: &TailnetProfile) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
}

#[async_trait]
pub trait TorProfileRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<TorProfile>>;
    async fn get(&self, id: Uuid) -> Result<Option<TorProfile>>;
    async fn insert(&self, profile: &TorProfile) -> Result<()>;
    async fn update(&self, profile: &TorProfile) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
}

#[async_trait]
pub trait BridgeProfileRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<BridgeProfile>>;
    async fn get(&self, id: Uuid) -> Result<Option<BridgeProfile>>;
    async fn insert(&self, profile: &BridgeProfile) -> Result<()>;
    async fn update(&self, profile: &BridgeProfile) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
}

#[async_trait]
pub trait ProxyProfileRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<ProxyProfile>>;
    async fn get(&self, id: Uuid) -> Result<Option<ProxyProfile>>;
    async fn insert(&self, profile: &ProxyProfile) -> Result<()>;
    async fn update(&self, profile: &ProxyProfile) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
}

#[async_trait]
pub trait ProxyChainRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<ProxyChain>>;
    async fn get(&self, id: Uuid) -> Result<Option<ProxyChain>>;
    async fn insert(&self, chain: &ProxyChain) -> Result<()>;
    async fn update(&self, chain: &ProxyChain) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
}

#[async_trait]
pub trait MixnetProfileRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<MixnetProfile>>;
    async fn get(&self, id: Uuid) -> Result<Option<MixnetProfile>>;
    async fn insert(&self, profile: &MixnetProfile) -> Result<()>;
    async fn update(&self, profile: &MixnetProfile) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
}

#[async_trait]
pub trait MixnetSessionRepository: Send + Sync {
    async fn insert(&self, session: &MixnetSession) -> Result<()>;
    async fn update(&self, session: &MixnetSession) -> Result<()>;
    async fn get(&self, id: Uuid) -> Result<Option<MixnetSession>>;
    async fn list_recent(&self, limit: u32) -> Result<Vec<MixnetSession>>;
    async fn list_by_profile(&self, profile_id: Uuid, limit: u32) -> Result<Vec<MixnetSession>>;
}

#[async_trait]
pub trait AnonymousChainRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<AnonymousChain>>;
    async fn get(&self, id: Uuid) -> Result<Option<AnonymousChain>>;
    async fn insert(&self, chain: &AnonymousChain) -> Result<()>;
    async fn update(&self, chain: &AnonymousChain) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
}

#[async_trait]
pub trait CoverTrafficRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<CoverTrafficSettings>>;
    async fn get(&self, id: Uuid) -> Result<Option<CoverTrafficSettings>>;
    async fn get_by_mixnet_profile(&self, profile_id: Uuid) -> Result<Option<CoverTrafficSettings>>;
    async fn insert(&self, settings: &CoverTrafficSettings) -> Result<()>;
    async fn update(&self, settings: &CoverTrafficSettings) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
}

#[async_trait]
pub trait PrivacyAnalyticsRepository: Send + Sync {
    async fn insert(&self, snapshot: &PrivacyAnalyticsSnapshot) -> Result<()>;
    async fn latest(&self) -> Result<Option<PrivacyAnalyticsSnapshot>>;
    async fn list(&self, limit: u32) -> Result<Vec<PrivacyAnalyticsSnapshot>>;
}

#[async_trait]
pub trait KatzenpostProfileRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<KatzenpostProfile>>;
    async fn get(&self, id: Uuid) -> Result<Option<KatzenpostProfile>>;
    async fn insert(&self, profile: &KatzenpostProfile) -> Result<()>;
    async fn update(&self, profile: &KatzenpostProfile) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
}

#[async_trait]
pub trait LoopixProfileRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<LoopixProfile>>;
    async fn get(&self, id: Uuid) -> Result<Option<LoopixProfile>>;
    async fn insert(&self, profile: &LoopixProfile) -> Result<()>;
    async fn update(&self, profile: &LoopixProfile) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
}

#[async_trait]
pub trait AnonymousServiceRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<AnonymousService>>;
    async fn get(&self, id: Uuid) -> Result<Option<AnonymousService>>;
    async fn insert(&self, service: &AnonymousService) -> Result<()>;
    async fn update(&self, service: &AnonymousService) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
    async fn list_endpoints(&self, service_id: Uuid) -> Result<Vec<AnonymousServiceEndpoint>>;
    async fn upsert_endpoint(&self, endpoint: &AnonymousServiceEndpoint) -> Result<()>;
}

#[async_trait]
pub trait TcpTerminationRepository: Send + Sync {
    async fn get_settings(&self) -> Result<TcpTerminationSettings>;
    async fn set_settings(&self, settings: &TcpTerminationSettings) -> Result<()>;
    async fn list_rules(&self) -> Result<Vec<TcpTerminationRule>>;
    async fn get_rule(&self, id: Uuid) -> Result<Option<TcpTerminationRule>>;
    async fn insert_rule(&self, rule: &TcpTerminationRule) -> Result<()>;
    async fn update_rule(&self, rule: &TcpTerminationRule) -> Result<()>;
    async fn delete_rule(&self, id: Uuid) -> Result<bool>;
    async fn load_policy(&self) -> Result<TcpTerminationPolicy>;
}

#[async_trait]
pub trait SplitTemplateRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<SplitTunnelTemplate>>;
    async fn get(&self, id: Uuid) -> Result<Option<SplitTunnelTemplate>>;
    async fn insert(&self, template: &SplitTunnelTemplate) -> Result<()>;
    async fn update(&self, template: &SplitTunnelTemplate) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
    async fn get_mode(&self) -> Result<SplitTemplateModeSettings>;
    async fn set_mode(&self, settings: &SplitTemplateModeSettings) -> Result<()>;
}
