use anonymity_federation::MixnetFederationManager;
use core_service::anonymity::AnonymityService;
use core_service::anonymity_entropy::RouteEntropyBridge;
use core_service::anonymity_security::AnonymitySecurityPolicy;
use core_service::cover_traffic::CoverTrafficService;
use core_service::mixnet::MixnetService;
use core_service::mixnet_security::MixnetSecurityPolicy;
use core_service::privacy_analytics::PrivacyAnalyticsService;
use event_bus::EventBus;
use mixnet_core::MixnetManager;
use proxy_engine::ProxyListenPort;
use std::sync::Arc;
use storage::{init_pool_in_memory, Storage};

#[tokio::test]
async fn privacy_analytics_snapshot_has_phase10_fields() {
    let pool = init_pool_in_memory().await.unwrap();
    let storage = Arc::new(Storage::new(pool));
    let events = EventBus::new();
    let listen_ports = Arc::new(ProxyListenPort::new());
    let mixnet_security = Arc::new(MixnetSecurityPolicy::new(events.clone()));
    let mixnet_manager = Arc::new(MixnetManager::new(mixnet_security.to_core_policy()));
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
    let anonymity_security = Arc::new(AnonymitySecurityPolicy::new(events.clone()));
    let anonymity = Arc::new(AnonymityService::new(
        Arc::clone(&storage),
        events.clone(),
        Arc::clone(&listen_ports),
        anonymity_security,
        Arc::new(MixnetFederationManager::new()),
    ));
    let entropy = Arc::new(RouteEntropyBridge::new(events.clone()));
    let service = PrivacyAnalyticsService::new(
        Arc::clone(&storage),
        events,
        mixnet,
        cover_traffic,
        anonymity,
        entropy,
    );

    let snapshot = service.calculate().await.unwrap();
    assert!(snapshot.anonymity_score <= 100);
    assert!(snapshot.route_entropy <= 100.0);
    assert!(snapshot.path_diversity <= 100.0);
    assert!(snapshot.cover_traffic_effectiveness <= 100.0);
}
