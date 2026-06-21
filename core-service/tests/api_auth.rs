use axum::body::Body;
use axum::http::{Request, StatusCode};
use core_service::api::routes;
use core_service::api::AppState;
use core_service::deps::ServiceDeps;
use event_bus::EventBus;
use parking_lot::RwLock;
use std::sync::Arc;
use storage::{init_pool_in_memory, Storage};
use tower::ServiceExt;

async fn test_state() -> AppState {
    let pool = init_pool_in_memory().await.expect("pool");
    let storage = Arc::new(Storage::new(pool));
    let events = EventBus::new();
    let token = Arc::new(RwLock::new("test-token".to_string()));
    let deps = Arc::new(
        ServiceDeps::build(storage, events, token)
            .await
            .expect("deps"),
    );
    AppState { deps }
}

#[tokio::test]
async fn status_requires_bearer() {
    let state = test_state().await;
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn status_with_valid_token() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/status")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn traffic_logs_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/traffic/logs?limit=10")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn filter_lists_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/filter-lists")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn statistics_routes_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/statistics/routes?limit=10")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn audit_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/audit?limit=10")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn statistics_blocked_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/statistics/blocked?limit=10")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn privacy_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/privacy")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn transports_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/transports")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn transports_status_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/transports/status")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn chains_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/chains")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn dns_providers_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/dns/providers")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn leaks_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/leaks?limit=10")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn logs_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/logs?limit=10")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn diagnostics_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/diagnostics")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn diagnostics_export_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/diagnostics/export")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn backup_export_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/backup/export")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn backup_import_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let export = app_export_json(state, &token).await;

    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let body = format!(r#"{{"data":{}}}"#, serde_json::to_string(&export).unwrap());
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/backup/import")
                .header("Authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn metrics_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/metrics")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn update_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/update")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn update_check_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/update/check")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn enterprise_policy_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/enterprise/policy")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn performance_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/performance?limit=5")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

async fn app_export_json(state: AppState, token: &str) -> String {
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/backup/export")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(body.to_vec()).unwrap()
}

#[tokio::test]
async fn validation_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/validation")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn benchmark_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/benchmark?limit=5")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn security_audit_endpoints() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);

    let list = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/security/audit")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list.status(), StatusCode::OK);

    let run = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/security/audit/run")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(run.status(), StatusCode::OK);
}

#[tokio::test]
async fn fault_inject_endpoint() {
    let state = test_state().await;
    let token = state.deps.api_token.read().clone();
    let app = routes::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/fault/inject")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"scenario":"dns_crash"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
