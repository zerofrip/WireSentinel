use axum_test::TestServer;
use core_service::api::routes::router;
use core_service::api::AppState;
use core_service::deps::ServiceDeps;
use event_bus::EventBus;
use std::sync::Arc;
use storage::{init_pool_in_memory, Storage};

async fn test_app() -> AppState {
    let pool = init_pool_in_memory().await.unwrap();
    let storage = Arc::new(Storage::new(pool));
    let events = EventBus::new();
    let deps = Arc::new(
        ServiceDeps::build(storage, events, Arc::new(parking_lot::RwLock::new("test".into())))
            .await
            .unwrap(),
    );
    AppState { deps }
}

#[tokio::test]
async fn kernel_telemetry_route_returns_json() {
    let state = test_app().await;
    let app = router(state);
    let server = TestServer::new(app).unwrap();
    let resp = server
        .get("/api/v1/kernel/telemetry")
        .add_header("Authorization", "Bearer test")
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(body.get("classify_count").is_some());
    assert!(body.get("packets_per_sec").is_some());
}

#[tokio::test]
async fn kernel_statistics_route_returns_json() {
    let state = test_app().await;
    let app = router(state);
    let server = TestServer::new(app).unwrap();
    let resp = server
        .get("/api/v1/kernel/statistics")
        .add_header("Authorization", "Bearer test")
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(body.get("wfp_filter_count").is_some());
}
