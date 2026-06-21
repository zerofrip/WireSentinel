use core_service::anonymity::AnonymityService;
use core_service::anonymity_entropy::RouteEntropyBridge;
use core_service::anonymity_security::AnonymitySecurityPolicy;
use core_service::cover_traffic::CoverTrafficService;
use core_service::mixnet::MixnetService;
use core_service::mixnet_security::MixnetSecurityPolicy;
use core_service::privacy_analytics::PrivacyAnalyticsService;
use anonymity_federation::MixnetFederationManager;
use event_bus::EventBus;
use proxy_engine::ProxyListenPort;
use std::sync::Arc;
use storage::{init_pool_in_memory, Storage};

#[tokio::test]
async fn anonymity_status_defaults_to_inactive() {
    let pool = init_pool_in_memory().await.unwrap();
    let storage = Arc::new(Storage::new(pool));
    let events = EventBus::new();
    let listen_ports = Arc::new(ProxyListenPort::new());
    let security = Arc::new(AnonymitySecurityPolicy::new(events.clone()));
    let service = AnonymityService::new(
        Arc::clone(&storage),
        events,
        listen_ports,
        security,
        Arc::new(MixnetFederationManager::new()),
    );

    let status = service.status().await.unwrap();
    assert!(!status.katzenpost_active);
    assert!(!status.loopix_active);
    assert_eq!(status.active_providers, 0);
}

#[test]
fn entropy_bridge_scores_route_mix() {
    let events = EventBus::new();
    let bridge = RouteEntropyBridge::new(events);
    let score = bridge.score_route_types(&["direct", "tor", "katzenpost"]);
    assert!(score >= 0.0);
    assert!(bridge.estimate_from_counts(2, false) > 0.0);
}

#[tokio::test]
async fn privacy_analytics_includes_phase13_optional_fields() {
    let pool = init_pool_in_memory().await.unwrap();
    let storage = Arc::new(Storage::new(pool));
    let events = EventBus::new();
    let listen_ports = Arc::new(ProxyListenPort::new());
    let mixnet_security = Arc::new(MixnetSecurityPolicy::new(events.clone()));
    let mixnet_manager = Arc::new(mixnet_core::MixnetManager::new(mixnet_security.to_core_policy()));
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
        Arc::clone(&anonymity_security),
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
    assert!(snapshot.anonymity_set_estimate.is_some());
    assert!(snapshot.cover_traffic_efficiency.is_some());
    assert!(snapshot.mixnet_diversity.is_some());
    assert!(snapshot.federation_diversity.is_some());
}
