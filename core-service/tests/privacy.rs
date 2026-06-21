use core_service::deps::ServiceDeps;
use event_bus::EventBus;
use parking_lot::RwLock;
use std::sync::Arc;
use storage::{init_pool_in_memory, Storage};

async fn test_deps() -> Arc<ServiceDeps> {
    let pool = init_pool_in_memory().await.expect("pool");
    let storage = Arc::new(Storage::new(pool));
    let events = EventBus::new();
    let token = Arc::new(RwLock::new("test-token".to_string()));
    Arc::new(
        ServiceDeps::build(storage, events, token)
            .await
            .expect("deps"),
    )
}

#[tokio::test]
async fn privacy_score_persisted_and_emits_event() {
    let deps = test_deps().await;
    let mut rx = deps.events.subscribe();

    let snapshot = deps.privacy.calculate().await.expect("calculate");
    assert!(snapshot.score <= 100);

    let latest = deps
        .storage
        .privacy_snapshots
        .latest()
        .await
        .expect("latest")
        .expect("snapshot exists");
    assert_eq!(latest.id, snapshot.id);

    let event = rx.try_recv().expect("event received");
    match event {
        shared_types::ServiceEvent::PrivacyScoreUpdated { snapshot: s, .. } => {
            assert_eq!(s.id, snapshot.id);
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[tokio::test]
async fn privacy_score_components_bounded() {
    let deps = test_deps().await;
    let snapshot = deps.privacy.calculate().await.expect("calculate");
    let c = snapshot.components;
    assert!(c.encrypted_dns <= 100);
    assert!(c.blocked_trackers <= 100);
    assert!(c.vpn_coverage <= 100);
    assert!(c.route_leakage <= 100);
    assert!(c.dns_leakage <= 100);
}
