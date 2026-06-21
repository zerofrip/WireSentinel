use core_service::domain_cache::DomainResolverCache;
use core_service::proxy::ProxyService;
use core_service::split_tunnel::SplitTunnelEngine;
use event_bus::EventBus;
use policy_engine::Decision;
use proxy_engine::{ProxyListenPort, ProxyManager};
use shared_types::{
    AppIdentity, AppRecord, ProxyKind, ProxyProfile, TrafficRoute, Verdict,
};
use std::path::PathBuf;
use std::sync::Arc;
use storage::{init_pool_in_memory, Storage};
use uuid::Uuid;
use vpn_engine::{default_dll_path, default_factory, VpnManager};
use wfp::{UserspaceWfpEngine, WfpEngine};

#[tokio::test]
async fn proxy_route_uses_wfp_route_connection() {
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
        Arc::clone(&storage.domain_cache) as Arc<dyn storage::DomainCacheRepository>,
    ));
    let listen_ports = Arc::new(ProxyListenPort::new());
    let proxy_manager = Arc::new(ProxyManager::new(Arc::clone(&listen_ports)));
    let events = EventBus::new();
    let proxy = Arc::new(ProxyService::new(
        Arc::clone(&storage),
        events,
        proxy_manager,
    ));

    let engine = SplitTunnelEngine::new(
        Arc::clone(&wfp),
        vpn,
        domain_cache,
        Arc::clone(&storage.route_statistics) as Arc<dyn storage::RouteStatisticsRepository>,
        Arc::clone(&storage.wfp_filter_state) as Arc<dyn storage::WfpFilterStateRepository>,
    )
    .with_proxy(Arc::clone(&proxy));

    let profile_id = Uuid::new_v4();
    let profile = ProxyProfile {
        id: profile_id,
        name: "test-socks".into(),
        kind: ProxyKind::Socks5,
        host: "127.0.0.1".into(),
        port: 1080,
        username: None,
        password_encrypted: None,
        enabled: true,
        active: false,
        latency_ms: None,
        last_health_at: None,
        last_error: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    storage.proxy_profiles.insert(&profile).await.unwrap();

    let record = AppRecord::new(PathBuf::from("C:\\proxy-app.exe"));
    let app = AppIdentity::new(99, record);
    let decision = Decision {
        route: TrafficRoute::Proxy(profile_id),
        verdict: Verdict::allow("test"),
        matched_rule_id: None,
    };

    let route = engine.enforce(&decision, &app).await.unwrap();
    assert_eq!(route, TrafficRoute::Proxy(profile_id));
}

#[tokio::test]
async fn proxy_chain_route_is_not_direct_fallback() {
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
        Arc::clone(&storage.domain_cache) as Arc<dyn storage::DomainCacheRepository>,
    ));
    let listen_ports = Arc::new(ProxyListenPort::new());
    let proxy_manager = Arc::new(ProxyManager::new(Arc::clone(&listen_ports)));
    let events = EventBus::new();
    let proxy = Arc::new(ProxyService::new(
        Arc::clone(&storage),
        events,
        proxy_manager,
    ));

    let engine = SplitTunnelEngine::new(
        Arc::clone(&wfp),
        vpn,
        domain_cache,
        Arc::clone(&storage.route_statistics) as Arc<dyn storage::RouteStatisticsRepository>,
        Arc::clone(&storage.wfp_filter_state) as Arc<dyn storage::WfpFilterStateRepository>,
    )
    .with_proxy(Arc::clone(&proxy));

    let chain_id = Uuid::new_v4();
    let decision = Decision {
        route: TrafficRoute::ProxyChain(chain_id),
        verdict: Verdict::allow("test"),
        matched_rule_id: None,
    };
    let record = AppRecord::new(PathBuf::from("C:\\chain-app.exe"));
    let app = AppIdentity::new(100, record);

    let result = engine.enforce(&decision, &app).await;
    assert!(result.is_err() || result.unwrap() == TrafficRoute::ProxyChain(chain_id));
}
