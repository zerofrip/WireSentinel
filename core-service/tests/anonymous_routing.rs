use anonymity_federation::MixnetFederationManager;
use core_service::anonymity::AnonymityService;
use core_service::anonymity_security::AnonymitySecurityPolicy;
use core_service::anonymous_routing::AnonymousRoutingService;
use core_service::domain_cache::DomainResolverCache;
use core_service::mixnet::MixnetService;
use core_service::mixnet_security::MixnetSecurityPolicy;
use core_service::proxy::ProxyService;
use core_service::split_tunnel::SplitTunnelEngine;
use core_service::tor::TorService;
use event_bus::EventBus;
use mixnet_core::MixnetManager;
use policy_engine::Decision;
use proxy_engine::{ProxyListenPort, ProxyManager};
use shared_types::{
    AnonymousRoute, AppIdentity, AppRecord, MixnetProfile, MixnetProvider, TrafficRoute, Verdict,
};
use std::path::PathBuf;
use std::sync::Arc;
use storage::{init_pool_in_memory, Storage};
use uuid::Uuid;
use vpn_engine::{default_dll_path, default_factory, VpnManager};
use wfp::{UserspaceWfpEngine, WfpEngine};

fn build_engine(
    storage: Arc<Storage>,
    wfp: Arc<dyn WfpEngine>,
    vpn: Arc<VpnManager>,
    domain_cache: Arc<DomainResolverCache>,
    anonymous_routing: Arc<AnonymousRoutingService>,
) -> SplitTunnelEngine {
    SplitTunnelEngine::new(
        wfp,
        vpn,
        domain_cache,
        Arc::clone(&storage.route_statistics) as Arc<dyn storage::RouteStatisticsRepository>,
        Arc::clone(&storage.wfp_filter_state) as Arc<dyn storage::WfpFilterStateRepository>,
    )
    .with_anonymous_routing(anonymous_routing)
}

#[tokio::test]
async fn anonymous_route_does_not_fall_back_to_direct() {
    let pool = init_pool_in_memory().await.unwrap();
    let storage = Arc::new(Storage::new(pool));
    let wfp = Arc::new(UserspaceWfpEngine::new()) as Arc<dyn WfpEngine>;
    let factory = Arc::new(default_factory(
        PathBuf::from("wire-sentinel-service.exe"),
        "native",
        "scm",
        default_dll_path(),
    ));
    let vpn = Arc::new(VpnManager::new(factory));
    let domain_cache = Arc::new(DomainResolverCache::new(
        Arc::clone(&storage.domain_cache) as Arc<dyn storage::DomainCacheRepository>
    ));

    let listen_ports = Arc::new(ProxyListenPort::new());
    let proxy_manager = Arc::new(ProxyManager::new(Arc::clone(&listen_ports)));
    let events = EventBus::new();
    let proxy = Arc::new(ProxyService::new(
        Arc::clone(&storage),
        events.clone(),
        proxy_manager,
    ));

    let mixnet_security = Arc::new(MixnetSecurityPolicy::new(events.clone()));
    let mixnet_manager = Arc::new(MixnetManager::new(mixnet_security.to_core_policy()));
    let mixnet = Arc::new(MixnetService::new(
        Arc::clone(&storage),
        events.clone(),
        mixnet_manager,
        Arc::clone(&listen_ports),
        Arc::clone(&mixnet_security),
    ));
    let tor = Arc::new(TorService::new(Arc::clone(&storage), events.clone()));
    let anonymity_security = Arc::new(AnonymitySecurityPolicy::new(events.clone()));
    let anonymity = Arc::new(AnonymityService::new(
        Arc::clone(&storage),
        events.clone(),
        Arc::clone(&listen_ports),
        anonymity_security,
        Arc::new(MixnetFederationManager::new()),
    ));
    let anonymous_routing = Arc::new(AnonymousRoutingService::new(
        Arc::clone(&mixnet_security),
        Arc::clone(&storage),
        tor,
        mixnet,
        anonymity,
        proxy,
        events,
    ));

    let engine = build_engine(
        Arc::clone(&storage),
        Arc::clone(&wfp),
        vpn,
        domain_cache,
        anonymous_routing,
    );

    let profile_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let profile = MixnetProfile {
        id: profile_id,
        name: "test-mixnet".into(),
        provider: MixnetProvider::Nym,
        gateway_id: Some("gateway-1".into()),
        config_json: None,
        enabled: true,
        active: false,
        latency_ms: None,
        last_health_at: None,
        last_error: None,
        created_at: now,
        updated_at: now,
    };
    storage.mixnet_profiles.insert(&profile).await.unwrap();

    let record = AppRecord::new(PathBuf::from("C:\\anon-app.exe"));
    let app = AppIdentity::new(77, record);
    let anonymous_route = TrafficRoute::Anonymous(AnonymousRoute::FutureMixnet(profile_id));
    let decision = Decision {
        route: anonymous_route.clone(),
        verdict: Verdict::allow("test"),
        matched_rule_id: None,
    };

    let route = engine.enforce(&decision, &app).await.unwrap();
    assert_eq!(route, anonymous_route);
    assert_ne!(route, TrafficRoute::Direct);
}
