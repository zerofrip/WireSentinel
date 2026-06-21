use core_service::kernel_route_bridge::KernelRouteBridge;
use core_service::proxy_redirect::ProxyRedirectEngine;
use proxy_engine::ProxyListenPort;
use shared_types::{AppIdentity, AppRecord, GuardianMode, TrafficRoute};
use std::path::PathBuf;
use std::sync::Arc;
use wfp::{NdisEngine, StubNdisEngine};

#[tokio::test]
async fn proxy_redirect_syncs_ndis_for_proxy_route() {
    let ndis = Arc::new(StubNdisEngine::new());
    let bridge = Arc::new(KernelRouteBridge::new(
        Arc::clone(&ndis) as Arc<dyn wfp::NdisEngine>,
        GuardianMode::Ndis,
    ));
    let listen_ports = Arc::new(ProxyListenPort::new());
    let profile_id = uuid::Uuid::new_v4();
    listen_ports.set(profile_id, 1080);
    let engine = ProxyRedirectEngine::new(bridge, listen_ports);

    let record = AppRecord::new(PathBuf::from("C:\\proxy.exe"));
    let app = AppIdentity::new(42, record);
    let route = TrafficRoute::Proxy(profile_id);

    engine.apply(&app, &route, None).await.unwrap();
    let health = ndis.health().await;
    assert_eq!(health.active_route_count, 1);
}

#[tokio::test]
async fn proxy_redirect_skips_direct_route() {
    let ndis = Arc::new(StubNdisEngine::new());
    let bridge = Arc::new(KernelRouteBridge::new(
        Arc::clone(&ndis) as Arc<dyn wfp::NdisEngine>,
        GuardianMode::Ndis,
    ));
    let engine = ProxyRedirectEngine::new(bridge, Arc::new(ProxyListenPort::new()));
    let record = AppRecord::new(PathBuf::from("C:\\direct.exe"));
    let app = AppIdentity::new(1, record);

    engine
        .apply(&app, &TrafficRoute::Direct, None)
        .await
        .unwrap();
    assert_eq!(ndis.health().await.active_route_count, 0);
}
