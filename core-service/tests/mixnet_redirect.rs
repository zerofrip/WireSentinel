use core_service::kernel_route_bridge::KernelRouteBridge;
use core_service::mixnet_redirect::MixnetRedirectEngine;
use proxy_engine::ProxyListenPort;
use shared_types::{AnonymousRoute, AppIdentity, AppRecord, GuardianMode, TrafficRoute};
use std::path::PathBuf;
use std::sync::Arc;
use wfp::{NdisEngine, StubNdisEngine};

#[tokio::test]
async fn mixnet_redirect_syncs_ndis_for_anonymous_route() {
    let ndis = Arc::new(StubNdisEngine::new());
    let bridge = Arc::new(KernelRouteBridge::new(
        Arc::clone(&ndis) as Arc<dyn wfp::NdisEngine>,
        GuardianMode::Hybrid,
    ));
    let listen_ports = Arc::new(ProxyListenPort::new());
    let engine = MixnetRedirectEngine::new(bridge, listen_ports);

    let record = AppRecord::new(PathBuf::from("C:\\anon.exe"));
    let app = AppIdentity::new(99, record);
    let route = TrafficRoute::Anonymous(AnonymousRoute::FutureMixnet(uuid::Uuid::new_v4()));

    engine.apply(&app, &route, None).await.unwrap();
    assert_eq!(ndis.health().await.active_route_count, 1);
}

#[tokio::test]
async fn mixnet_redirect_skips_blocked_route() {
    let ndis = Arc::new(StubNdisEngine::new());
    let bridge = Arc::new(KernelRouteBridge::new(
        Arc::clone(&ndis) as Arc<dyn wfp::NdisEngine>,
        GuardianMode::Hybrid,
    ));
    let engine = MixnetRedirectEngine::new(bridge, Arc::new(ProxyListenPort::new()));
    let record = AppRecord::new(PathBuf::from("C:\\blocked.exe"));
    let app = AppIdentity::new(2, record);

    engine
        .apply(&app, &TrafficRoute::Blocked, None)
        .await
        .unwrap();
    assert_eq!(ndis.health().await.active_route_count, 0);
}
