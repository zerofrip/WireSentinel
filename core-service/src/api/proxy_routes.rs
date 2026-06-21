//! Proxy and proxy-chain REST handlers.

use crate::api::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

#[utoipa::path(get, path = "/api/v1/proxies", responses((status = 200, body = [shared_types::ProxyProfile])))]
pub async fn list_proxies(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.proxy.list().await {
        Ok(profiles) => Json(profiles).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/proxies/{id}", responses((status = 200, body = shared_types::ProxyProfile)))]
pub async fn get_proxy(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.deps.proxy.get(id).await {
        Ok(Some(profile)) => Json(profile).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "proxy not found".to_string()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/proxies", responses((status = 200, body = shared_types::ProxyProfile)))]
pub async fn create_proxy(
    State(state): State<Arc<AppState>>,
    Json(mut profile): Json<shared_types::ProxyProfile>,
) -> impl IntoResponse {
    let now = Utc::now();
    if profile.id.is_nil() {
        profile.id = Uuid::new_v4();
    }
    if profile.created_at.timestamp() == 0 {
        profile.created_at = now;
    }
    profile.updated_at = now;
    match state.deps.proxy.create(profile).await {
        Ok(profile) => Json(profile).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(put, path = "/api/v1/proxies/{id}", responses((status = 200, body = shared_types::ProxyProfile)))]
pub async fn update_proxy(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(mut profile): Json<shared_types::ProxyProfile>,
) -> impl IntoResponse {
    profile.id = id;
    match state.deps.proxy.update(profile).await {
        Ok(profile) => Json(profile).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(delete, path = "/api/v1/proxies/{id}", responses((status = 200)))]
pub async fn delete_proxy(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.deps.proxy.delete(id).await {
        Ok(true) => Json(serde_json::json!({"ok": true})).into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/proxies/{id}/connect", responses((status = 200, body = shared_types::ProxyProfile)))]
pub async fn connect_proxy(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.deps.proxy.connect(id).await {
        Ok(profile) => Json(profile).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/proxies/{id}/disconnect", responses((status = 200)))]
pub async fn disconnect_proxy(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.deps.proxy.disconnect(id).await {
        Ok(()) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/proxies/{id}/health", responses((status = 200)))]
pub async fn health_proxy(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.deps.proxy.health_check(id).await {
        Ok(health) => Json(health).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/proxies/{id}/latency", responses((status = 200)))]
pub async fn latency_proxy(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.deps.proxy.measure_latency(id).await {
        Ok(latency_ms) => Json(serde_json::json!({ "latency_ms": latency_ms })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/proxy-chains", responses((status = 200, body = [shared_types::ProxyChain])))]
pub async fn list_proxy_chains(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.proxy.list_chains().await {
        Ok(chains) => Json(chains).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/proxy-chains/{id}", responses((status = 200, body = shared_types::ProxyChain)))]
pub async fn get_proxy_chain(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.deps.proxy.get_chain(id).await {
        Ok(Some(chain)) => Json(chain).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "proxy chain not found".to_string()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/proxy-chains", responses((status = 200, body = shared_types::ProxyChain)))]
pub async fn create_proxy_chain(
    State(state): State<Arc<AppState>>,
    Json(chain): Json<shared_types::ProxyChain>,
) -> impl IntoResponse {
    match state.deps.proxy.create_chain(chain).await {
        Ok(chain) => Json(chain).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(put, path = "/api/v1/proxy-chains/{id}", responses((status = 200, body = shared_types::ProxyChain)))]
pub async fn update_proxy_chain(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(mut chain): Json<shared_types::ProxyChain>,
) -> impl IntoResponse {
    chain.id = id;
    match state.deps.proxy.update_chain(chain).await {
        Ok(chain) => Json(chain).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(delete, path = "/api/v1/proxy-chains/{id}", responses((status = 200)))]
pub async fn delete_proxy_chain(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.deps.proxy.delete_chain(id).await {
        Ok(true) => Json(serde_json::json!({"ok": true})).into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/proxy-chains/{id}/start", responses((status = 200, body = shared_types::ProxyChain)))]
pub async fn start_proxy_chain(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.deps.proxy.start_chain(id).await {
        Ok(chain) => Json(chain).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/proxy-chains/{id}/stop", responses((status = 200)))]
pub async fn stop_proxy_chain(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.deps.proxy.stop_chain(id).await {
        Ok(()) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
