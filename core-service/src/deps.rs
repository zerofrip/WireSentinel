//! Service dependency injection container.

use anonymity_federation::MixnetFederationManager;
use app_registry::AppRegistryService;
use chrono::Utc;
use dns::DnsLayer;
use event_bus::EventBus;
use filter_lists::{
    filters_cache_dir, FilterListEngine, FilterListProvider, FilterSubscription,
    FilterUpdateScheduler,
};
use parking_lot::RwLock;
use policy_engine::{PolicyEngine, ProfileLookup};
use proxy_engine::{ProxyListenPort, ProxyManager};
use shared_types::{
    DnsProviderRecord, DnsTransport, FilterListRecord, Ruleset, ServiceEventInner, VPNProfile,
    VpnBackendKind,
};
use split_tunnel::SplitTunnelTemplateManager;
use std::collections::HashMap;
use std::sync::Arc;
use storage::Storage;
use tcp_termination::TcpTerminationEngine;
use traffic_monitor::TrafficMonitor;
use transport_engine::{BridgeManager, ProcessManager, TorSingBoxRunner, TransportConfigStore};
use uuid::Uuid;
use vpn_engine::{default_dll_path, default_factory, VpnBackendFactory, VpnManager};
use wfp::WfpEngine;

use crate::anonymity::AnonymityService;
use crate::anonymity_decoy::AnonymityDecoyService;
use crate::anonymity_discovery::AnonymityDiscoveryService;
use crate::anonymity_entropy::RouteEntropyBridge;
use crate::anonymity_security::AnonymitySecurityPolicy;
use crate::anonymous_routing::AnonymousRoutingService;
use crate::audit::AuditRecorder;
use crate::backup::BackupService;
use crate::benchmark::BenchmarkService;
use crate::binary_paths::{resolve_singbox_exe, resolve_tor_exe};
use crate::correlation::TrafficCorrelator;
use crate::cover_traffic::CoverTrafficService;
use crate::diagnostics::DiagnosticsService;
use crate::domain_cache::DomainResolverCache;
use crate::enterprise::LocalPolicyProvider;
use crate::exit_failover::ExitFailoverService;
use crate::fault_injection::FaultInjectionService;
use crate::guardian_hybrid::GuardianHybridService;
use crate::kernel_route_bridge::KernelRouteBridge;
use crate::kernel_telemetry::KernelTelemetryService;
use crate::leak_detector::LeakDetector;
use crate::metrics::MetricsService;
use crate::mixnet::MixnetService;
use crate::mixnet_redirect::MixnetRedirectEngine;
use crate::mixnet_security::MixnetSecurityPolicy;
use crate::performance::PerformanceMonitor;
use crate::plugins::PluginService;
use crate::privacy::PrivacyScoreService;
use crate::privacy_analytics::PrivacyAnalyticsService;
use crate::proxy::ProxyService;
use crate::proxy_redirect::ProxyRedirectEngine;
use crate::recovery::RecoveryService;
use crate::security_audit::SecurityAuditService;
use crate::split_templates::SplitTemplateService;
use crate::split_tunnel::SplitTunnelEngine;
use crate::sse_agent::SseAgent;
use crate::tailscale::TailscaleService;
use crate::tcp_termination::TcpTerminationService;
use crate::tor::TorService;
use crate::transport::TransportManager;
use crate::update::UpdateManager;
use crate::validation::ValidationService;
use crate::xdr_agent::XdrAgent;
use crate::ztna_agent::ZtnaAgent;

pub struct ProfileCache {
    map: RwLock<HashMap<Uuid, VpnBackendKind>>,
}

impl Default for ProfileCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ProfileCache {
    pub fn new() -> Self {
        Self {
            map: RwLock::new(HashMap::new()),
        }
    }

    pub fn load(&self, profiles: &[VPNProfile]) {
        let mut map = self.map.write();
        map.clear();
        for p in profiles {
            map.insert(p.id, p.backend);
        }
    }
}

impl ProfileLookup for ProfileCache {
    fn backend_for(&self, profile_id: Uuid) -> Option<VpnBackendKind> {
        self.map.read().get(&profile_id).copied()
    }
}

pub struct ServiceDeps {
    pub storage: Arc<Storage>,
    pub events: EventBus,
    pub policy: Arc<RwLock<PolicyEngine>>,
    pub profile_cache: Arc<ProfileCache>,
    pub wfp: Arc<dyn WfpEngine>,
    pub vpn_factory: Arc<VpnBackendFactory>,
    pub vpn: Arc<VpnManager>,
    pub traffic: Arc<TrafficMonitor>,
    pub dns: Arc<DnsLayer>,
    pub app_registry: Arc<AppRegistryService>,
    pub filter_lists: Arc<FilterListEngine>,
    pub filter_scheduler: parking_lot::Mutex<Option<FilterUpdateScheduler>>,
    pub correlator: Arc<TrafficCorrelator>,
    pub domain_cache: Arc<DomainResolverCache>,
    pub split_tunnel: Arc<SplitTunnelEngine>,
    pub audit: Arc<AuditRecorder>,
    pub privacy: Arc<PrivacyScoreService>,
    pub leak_detector: Arc<LeakDetector>,
    pub transport: Arc<TransportManager>,
    pub recovery: Arc<RecoveryService>,
    pub performance: Arc<PerformanceMonitor>,
    pub diagnostics: Arc<DiagnosticsService>,
    pub backup: Arc<BackupService>,
    pub enterprise: Arc<LocalPolicyProvider>,
    pub update: Arc<UpdateManager>,
    pub metrics: Arc<MetricsService>,
    pub validation: Arc<ValidationService>,
    pub benchmark: Arc<BenchmarkService>,
    pub security_audit: Arc<SecurityAuditService>,
    pub plugins: Arc<PluginService>,
    pub fault_injection: Arc<FaultInjectionService>,
    pub tailscale: Arc<TailscaleService>,
    pub tor: Arc<TorService>,
    pub proxy: Arc<ProxyService>,
    pub mixnet_security: Arc<MixnetSecurityPolicy>,
    pub mixnet: Arc<MixnetService>,
    pub cover_traffic: Arc<CoverTrafficService>,
    pub anonymous_routing: Arc<AnonymousRoutingService>,
    pub privacy_analytics: Arc<PrivacyAnalyticsService>,
    pub guardian_hybrid: Arc<GuardianHybridService>,
    pub kernel_telemetry: Arc<KernelTelemetryService>,
    pub kernel_route_bridge: Arc<KernelRouteBridge>,
    pub proxy_redirect: Arc<ProxyRedirectEngine>,
    pub mixnet_redirect: Arc<MixnetRedirectEngine>,
    pub anonymity_security: Arc<AnonymitySecurityPolicy>,
    pub anonymity: Arc<AnonymityService>,
    pub anonymity_entropy: Arc<RouteEntropyBridge>,
    pub anonymity_discovery: Arc<AnonymityDiscoveryService>,
    pub anonymity_decoy: Arc<AnonymityDecoyService>,
    pub ztna: Arc<ZtnaAgent>,
    pub sse: Arc<SseAgent>,
    pub xdr: Arc<XdrAgent>,
    pub tcp_termination: Arc<TcpTerminationService>,
    pub split_templates: Arc<SplitTemplateService>,
    pub exit_failover: Arc<crate::exit_failover::ExitFailoverService>,
    pub api_token: Arc<RwLock<String>>,
}

fn record_to_subscription(record: &FilterListRecord) -> FilterSubscription {
    FilterSubscription {
        id: record.id,
        name: record.name.clone(),
        url: record.url.clone().unwrap_or_default(),
        list_type: record.list_type,
        enabled: record.enabled,
        update_interval_secs: record.update_interval_secs,
        last_updated: record.last_updated,
        cache_path: record
            .cache_path
            .clone()
            .unwrap_or_else(|| filters_cache_dir().join(format!("{}.cache", record.id))),
    }
}

async fn create_wfp_engine(
    storage: &Storage,
    listen_ports: Arc<ProxyListenPort>,
) -> Arc<dyn WfpEngine> {
    let mapping = crate::enforcement::resolve_mapping(storage)
        .await
        .unwrap_or_else(|_| {
            shared_types::EnforcementMapping::from_backend(shared_types::EnforcementBackend::Signed)
        });
    wfp::create_wfp_engine(mapping.backend.as_str(), listen_ports)
}

async fn seed_dns_providers(storage: &Storage) -> shared_types::Result<()> {
    let existing = storage.dns_providers.list().await?;
    if !existing.is_empty() {
        return Ok(());
    }

    let now = Utc::now();
    let defaults = [
        DnsProviderRecord {
            id: Uuid::new_v4(),
            name: "cloudflare".into(),
            transport: DnsTransport::Doh,
            endpoint: "https://cloudflare-dns.com/dns-query".into(),
            priority: 10,
            enabled: true,
            latency_ms: None,
            last_check: None,
            failure_count: 0,
            created_at: now,
            updated_at: now,
        },
        DnsProviderRecord {
            id: Uuid::new_v4(),
            name: "quad9".into(),
            transport: DnsTransport::Doh,
            endpoint: "https://dns.quad9.net/dns-query".into(),
            priority: 20,
            enabled: true,
            latency_ms: None,
            last_check: None,
            failure_count: 0,
            created_at: now,
            updated_at: now,
        },
    ];

    for provider in &defaults {
        storage.dns_providers.upsert(provider).await?;
    }
    Ok(())
}

fn wire_dns_log_handler(dns: &DnsLayer, storage: Arc<Storage>, events: EventBus) {
    dns.set_log_handler(Some(Arc::new(move |log| {
        let storage = Arc::clone(&storage);
        let events = events.clone();
        tokio::spawn(async move {
            if storage.settings.store_dns_logs().await.unwrap_or(true) {
                let _ = storage.dns_logs.insert(&log).await;
            }
            let ts = Utc::now();
            if log.blocked {
                events.publish(
                    ServiceEventInner::DnsQueryBlocked { log: log.clone() }.with_timestamp(ts),
                );
            } else {
                events.publish(
                    ServiceEventInner::DnsQueryObserved { log: log.clone() }.with_timestamp(ts),
                );
            }
        });
    })));
}

impl ServiceDeps {
    pub async fn sync_filter_engine(&self) -> shared_types::Result<()> {
        let records = self.storage.filter_lists.list().await?;
        let subs: Vec<_> = records.iter().map(record_to_subscription).collect();
        self.filter_lists.replace_subscriptions(subs);
        let _ = self.filter_lists.reload_from_cache();
        Ok(())
    }

    pub async fn build(
        storage: Arc<Storage>,
        events: EventBus,
        api_token: Arc<RwLock<String>>,
    ) -> shared_types::Result<Self> {
        let listen_ports = Arc::new(ProxyListenPort::new());
        let proxy_manager = Arc::new(ProxyManager::new(Arc::clone(&listen_ports)));
        let wfp = create_wfp_engine(storage.as_ref(), Arc::clone(&listen_ports)).await;
        let service_exe = std::env::current_exe()
            .unwrap_or_else(|_| std::path::PathBuf::from("wire-sentinel-service.exe"));
        let wg_impl = storage.settings.vpn_wireguard_impl().await?;
        let awg_impl = storage.settings.vpn_amnezia_impl().await?;
        let dll_path = default_dll_path();
        let vpn_factory = Arc::new(default_factory(service_exe, &wg_impl, &awg_impl, dll_path));
        let vpn = Arc::new(VpnManager::new(Arc::clone(&vpn_factory)).with_events(events.clone()));

        let profiles = storage.vpn_profiles.list().await?;
        vpn.set_profiles(profiles.clone());

        let profile_cache = Arc::new(ProfileCache::new());
        profile_cache.load(&profiles);

        let rules = storage.rules.list().await?;
        let policy_mode = storage.rules.get_policy_mode().await?;
        let ruleset = Ruleset {
            mode: policy_mode,
            rules,
            kill_switch_active: false,
        };
        let policy = Arc::new(RwLock::new(PolicyEngine::new(
            ruleset,
            Arc::clone(&profile_cache) as Arc<dyn ProfileLookup>,
        )));

        seed_dns_providers(storage.as_ref()).await?;
        let dns_provider_records = storage.dns_providers.list().await?;

        let dns_settings = storage.settings.get_dns_settings().await?;
        let dns = Arc::new(DnsLayer::new(dns_settings));
        dns.load_providers_from_records(&dns_provider_records)?;
        wire_dns_log_handler(&dns, Arc::clone(&storage), events.clone());

        let traffic = Arc::new(TrafficMonitor::new(1000));

        let app_registry = Arc::new(AppRegistryService::new(
            Arc::clone(&storage.apps) as Arc<dyn storage::AppRepository>,
            events.clone(),
        ));

        let filter_lists = Arc::new(FilterListEngine::new());
        let filter_records = storage.filter_lists.list().await?;
        let subs: Vec<_> = filter_records.iter().map(record_to_subscription).collect();
        filter_lists.replace_subscriptions(subs);
        let _ = filter_lists.reload_from_cache();
        dns.set_filter_provider(Arc::clone(&filter_lists) as Arc<dyn FilterListProvider>);

        let domain_cache = Arc::new(DomainResolverCache::new(
            Arc::clone(&storage.domain_cache) as Arc<dyn storage::DomainCacheRepository>
        ));

        let proxy = Arc::new(ProxyService::new(
            Arc::clone(&storage),
            events.clone(),
            Arc::clone(&proxy_manager),
        ));

        let mixnet_security = Arc::new(MixnetSecurityPolicy::new(events.clone()));
        let mixnet_manager = Arc::new(mixnet_core::MixnetManager::new(
            mixnet_security.to_core_policy(),
        ));
        let mixnet = Arc::new(MixnetService::new(
            Arc::clone(&storage),
            events.clone(),
            mixnet_manager,
            Arc::clone(&listen_ports),
            Arc::clone(&mixnet_security),
        ));

        let cover_traffic = Arc::new(CoverTrafficService::new(
            Arc::clone(&storage),
            events.clone(),
        ));

        let tor_process_manager = Arc::new(ProcessManager::new());
        let tor_config_store = Arc::new(TransportConfigStore::new());
        let tor_runner = Arc::new(TorSingBoxRunner::new(
            Arc::clone(&tor_process_manager),
            tor_config_store,
            resolve_singbox_exe(None),
            resolve_tor_exe(None),
        ));
        let bridge_manager = Arc::new(BridgeManager::new(Arc::clone(&tor_runner)));
        let tor = Arc::new(TorService::new(
            Arc::clone(&storage),
            events.clone(),
            tor_runner,
            bridge_manager,
        ));
        let _ = tor.load_profiles().await;

        let ndis = wfp
            .ndis_side()
            .unwrap_or_else(|| Arc::new(wfp::StubNdisEngine::new()) as Arc<dyn wfp::NdisEngine>);
        let guardian_mode = storage
            .settings
            .guardian_mode()
            .await
            .map(|s| shared_types::GuardianMode::parse(&s))
            .unwrap_or_default();
        let guardian_hybrid = Arc::new(GuardianHybridService::new(
            Arc::clone(&storage),
            Arc::clone(&ndis),
            events.clone(),
        ));
        let kernel_route_bridge =
            Arc::new(KernelRouteBridge::new(Arc::clone(&ndis), guardian_mode));
        let proxy_redirect = Arc::new(ProxyRedirectEngine::new(
            Arc::clone(&kernel_route_bridge),
            Arc::clone(&listen_ports),
        ));
        let mixnet_redirect = Arc::new(MixnetRedirectEngine::new(
            Arc::clone(&kernel_route_bridge),
            Arc::clone(&listen_ports),
        ));
        let anonymity_security = Arc::new(AnonymitySecurityPolicy::new(events.clone()));
        let anonymity_manager = Arc::new(MixnetFederationManager::new());
        let anonymity = Arc::new(AnonymityService::new(
            Arc::clone(&storage),
            events.clone(),
            Arc::clone(&listen_ports),
            Arc::clone(&anonymity_security),
            anonymity_manager,
        ));
        let anonymity_entropy = Arc::new(RouteEntropyBridge::new(events.clone()));
        let anonymity_discovery = Arc::new(AnonymityDiscoveryService::new());
        let anonymity_decoy = Arc::new(AnonymityDecoyService::new(
            anonymity_security.as_ref(),
            events.clone(),
        ));
        let privacy_analytics = Arc::new(PrivacyAnalyticsService::new(
            Arc::clone(&storage),
            events.clone(),
            Arc::clone(&mixnet),
            Arc::clone(&cover_traffic),
            Arc::clone(&anonymity),
            Arc::clone(&anonymity_entropy),
        ));
        let anonymous_routing = Arc::new(AnonymousRoutingService::new(
            Arc::clone(&mixnet_security),
            Arc::clone(&storage),
            Arc::clone(&tor),
            Arc::clone(&mixnet),
            Arc::clone(&anonymity),
            Arc::clone(&proxy),
            events.clone(),
        ));
        let kernel_telemetry = Arc::new(KernelTelemetryService::new(
            Arc::clone(&storage),
            Arc::clone(&wfp),
            Arc::clone(&ndis),
            Arc::clone(&guardian_hybrid),
        ));

        let split_tunnel = Arc::new(
            SplitTunnelEngine::new(
                Arc::clone(&wfp),
                Arc::clone(&vpn),
                Arc::clone(&domain_cache),
                Arc::clone(&storage.route_statistics)
                    as Arc<dyn storage::RouteStatisticsRepository>,
                Arc::clone(&storage.wfp_filter_state) as Arc<dyn storage::WfpFilterStateRepository>,
            )
            .with_events(events.clone())
            .with_proxy(Arc::clone(&proxy))
            .with_anonymous_routing(Arc::clone(&anonymous_routing))
            .with_anonymity(Arc::clone(&anonymity))
            .with_anonymity(Arc::clone(&anonymity))
            .with_kernel_route_bridge(Arc::clone(&kernel_route_bridge))
            .with_proxy_redirect(Arc::clone(&proxy_redirect))
            .with_mixnet_redirect(Arc::clone(&mixnet_redirect)),
        );

        let correlator = Arc::new(TrafficCorrelator::new(
            Arc::clone(&domain_cache),
            Arc::clone(&storage.correlations) as Arc<dyn storage::CorrelationRepository>,
            Arc::clone(&storage.dns_logs) as Arc<dyn storage::DnsLogRepository>,
        ));

        let audit = Arc::new(AuditRecorder::new(
            Arc::clone(&storage.audit_log) as Arc<dyn storage::AuditLogRepository>,
            events.clone(),
        ));

        let privacy = Arc::new(PrivacyScoreService::new(
            Arc::clone(&storage),
            events.clone(),
            Arc::clone(&dns),
            Arc::clone(&vpn),
        ));

        let leak_detector = Arc::new(LeakDetector::new(
            Arc::clone(&storage),
            events.clone(),
            Arc::clone(&policy),
            Arc::clone(&vpn),
        ));

        let transport = Arc::new(TransportManager::new(
            Arc::clone(&storage),
            vpn_factory.as_ref(),
            Arc::clone(&mixnet),
        ));
        let recovery = Arc::new(RecoveryService::new(Arc::clone(&storage), events.clone()));
        let performance = Arc::new(PerformanceMonitor::new(
            Arc::clone(&storage),
            events.clone(),
        ));
        let backup = Arc::new(BackupService::new(Arc::clone(&storage), events.clone()));
        let enterprise = Arc::new(LocalPolicyProvider::new(Arc::clone(&storage)));
        let update = Arc::new(UpdateManager::new(Arc::clone(&storage)));
        let diagnostics = Arc::new(DiagnosticsService::new(
            Arc::clone(&storage),
            Arc::clone(&wfp),
            Arc::clone(&vpn),
            Arc::clone(&dns),
            Arc::clone(&transport),
        ));
        let metrics = Arc::new(MetricsService::new(
            Arc::clone(&storage),
            Arc::clone(&vpn),
            Arc::clone(&transport),
        ));
        let validation = Arc::new(ValidationService::new(Arc::clone(&storage), events.clone()));
        let benchmark = Arc::new(BenchmarkService::new(Arc::clone(&storage)));
        let security_audit = Arc::new(SecurityAuditService::new(
            Arc::clone(&storage),
            events.clone(),
        ));
        let plugins = Arc::new(PluginService::new(Arc::clone(&storage), events.clone())?);
        let fault_injection = Arc::new(FaultInjectionService::new(
            Arc::clone(&recovery),
            events.clone(),
        ));
        let tailscale = Arc::new(TailscaleService::new(
            Arc::clone(&storage),
            events.clone(),
            vpn_factory.as_ref(),
        ));
        let ztna = Arc::new(ZtnaAgent::new());
        let sse = Arc::new(SseAgent::new());
        let xdr = Arc::new(XdrAgent::new(events.clone()));

        let tcp_engine = Arc::new(TcpTerminationEngine::new().with_events(events.clone()));
        let tcp_termination = Arc::new(TcpTerminationService::new(
            Arc::clone(&storage),
            Arc::clone(&tcp_engine),
        ));
        let _ = tcp_termination.reload_policy().await;

        let template_manager =
            Arc::new(SplitTunnelTemplateManager::new().with_events(events.clone()));
        let split_templates = Arc::new(SplitTemplateService::new(
            Arc::clone(&storage),
            Arc::clone(&template_manager),
        ));
        let _ = split_templates.reload().await;

        let exit_failover = Arc::new(ExitFailoverService::new());

        Ok(Self {
            storage,
            events,
            policy,
            profile_cache,
            wfp,
            vpn_factory,
            vpn,
            traffic,
            dns,
            app_registry,
            filter_lists,
            filter_scheduler: parking_lot::Mutex::new(None),
            correlator,
            domain_cache,
            split_tunnel,
            audit,
            privacy,
            leak_detector,
            transport,
            recovery,
            performance,
            diagnostics,
            backup,
            enterprise,
            update,
            metrics,
            validation,
            benchmark,
            security_audit,
            plugins,
            fault_injection,
            tailscale,
            tor,
            proxy,
            mixnet_security,
            mixnet,
            cover_traffic,
            anonymous_routing,
            privacy_analytics,
            guardian_hybrid,
            kernel_telemetry,
            kernel_route_bridge,
            proxy_redirect,
            mixnet_redirect,
            anonymity_security,
            anonymity,
            anonymity_entropy,
            anonymity_discovery,
            anonymity_decoy,
            ztna,
            sse,
            xdr,
            tcp_termination,
            split_templates,
            exit_failover,
            api_token,
        })
    }

    pub fn start_domain_cache_purge(&self, shutdown: tokio::sync::watch::Receiver<bool>) {
        Arc::clone(&self.domain_cache).start_purge_task(shutdown);
    }

    pub async fn start_filter_scheduler(&self) {
        if let Ok(scheduler) = FilterUpdateScheduler::new().await {
            let subs = self.filter_lists.subscriptions();
            let _ = scheduler
                .schedule_all(Arc::clone(&self.filter_lists), &subs)
                .await;
            let _ = scheduler.start().await;
            *self.filter_scheduler.lock() = Some(scheduler);
        }
    }
}
