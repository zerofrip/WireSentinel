use core_service::domain_cache::DomainResolverCache;
use core_service::split_tunnel::SplitTunnelEngine;
use policy_engine::Decision;
use shared_types::{AppIdentity, AppRecord, TrafficRoute, Verdict};
use std::path::PathBuf;
use std::sync::Arc;
use storage::{init_pool_in_memory, Storage};
use uuid::Uuid;
use vpn_engine::{default_dll_path, default_factory, VpnManager};
use wfp::{UserspaceWfpEngine, WfpEngine};

#[tokio::test]
async fn enforce_blocked_applies_wfp_block() {
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
    let engine = SplitTunnelEngine::new(
        Arc::clone(&wfp),
        vpn,
        domain_cache,
        Arc::clone(&storage.route_statistics) as Arc<dyn storage::RouteStatisticsRepository>,
        Arc::clone(&storage.wfp_filter_state) as Arc<dyn storage::WfpFilterStateRepository>,
    );

    let record = AppRecord::new(PathBuf::from("C:\\blocked.exe"));
    let app = AppIdentity::new(42, record);
    let decision = Decision {
        route: TrafficRoute::Blocked,
        verdict: Verdict::block("test rule"),
        matched_rule_id: Some(Uuid::new_v4()),
    };

    let route = engine.enforce(&decision, &app).await.unwrap();
    assert_eq!(route, TrafficRoute::Blocked);
}
