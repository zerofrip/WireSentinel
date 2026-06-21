//! Mixnet, anonymous routing, cover traffic, and privacy analytics REST handlers.

use crate::api::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use shared_types::{
    AnonymousChain, CoverTrafficProfile, CoverTrafficSettings, MixnetRoute, ServiceEvent,
    ServiceEventInner, TrafficRoute,
};
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Deserialize, ToSchema)]
pub struct MixnetProfileAction {
    pub profile_id: Uuid,
}

#[utoipa::path(get, path = "/api/v1/mixnet", responses((status = 200, body = [shared_types::MixnetProfile])))]
pub async fn list_mixnet_profiles(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.mixnet.list().await {
        Ok(profiles) => Json(profiles).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/mixnet/status", responses((status = 200, body = shared_types::MixnetStatus)))]
pub async fn mixnet_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.mixnet.status().await {
        Ok(status) => Json(status).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/mixnet/routes", responses((status = 200, body = [shared_types::MixnetRoute])))]
pub async fn mixnet_routes(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut routes: Vec<MixnetRoute> = Vec::new();
    if let Ok(profiles) = state.deps.mixnet.list().await {
        for profile in profiles.into_iter().filter(|p| p.active) {
            if let Ok(mut profile_routes) = state.deps.mixnet.list_routes(profile.id).await {
                routes.append(&mut profile_routes);
            }
        }
    }
    Json(routes).into_response()
}

#[utoipa::path(post, path = "/api/v1/mixnet/start", responses((status = 200, body = shared_types::MixnetProfile)))]
pub async fn start_mixnet(
    State(state): State<Arc<AppState>>,
    Json(body): Json<MixnetProfileAction>,
) -> impl IntoResponse {
    match state.deps.mixnet.start(body.profile_id).await {
        Ok(profile) => Json(profile).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/mixnet/stop", responses((status = 200)))]
pub async fn stop_mixnet(
    State(state): State<Arc<AppState>>,
    Json(body): Json<MixnetProfileAction>,
) -> impl IntoResponse {
    match state.deps.mixnet.stop(body.profile_id, "user requested").await {
        Ok(()) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/anonymous-routes", responses((status = 200, body = [shared_types::AnonymousChain])))]
pub async fn list_anonymous_routes(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.storage.anonymous_chains.list().await {
        Ok(chains) => Json(chains).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/anonymous-routes/{id}", responses((status = 200, body = shared_types::AnonymousChain)))]
pub async fn get_anonymous_route(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.deps.storage.anonymous_chains.get(id).await {
        Ok(Some(chain)) => Json(chain).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "anonymous chain not found".to_string()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/anonymous-routes", responses((status = 200, body = shared_types::AnonymousChain)))]
pub async fn create_anonymous_route(
    State(state): State<Arc<AppState>>,
    Json(mut chain): Json<AnonymousChain>,
) -> impl IntoResponse {
    let now = Utc::now();
    if chain.id.is_nil() {
        chain.id = Uuid::new_v4();
    }
    if chain.created_at.timestamp() == 0 {
        chain.created_at = now;
    }
    chain.updated_at = now;
    match state.deps.storage.anonymous_chains.insert(&chain).await {
        Ok(()) => Json(chain).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(put, path = "/api/v1/anonymous-routes/{id}", responses((status = 200, body = shared_types::AnonymousChain)))]
pub async fn update_anonymous_route(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(mut chain): Json<AnonymousChain>,
) -> impl IntoResponse {
    chain.id = id;
    chain.updated_at = Utc::now();
    match state.deps.storage.anonymous_chains.update(&chain).await {
        Ok(()) => Json(chain).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(delete, path = "/api/v1/anonymous-routes/{id}", responses((status = 200)))]
pub async fn delete_anonymous_route(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.deps.storage.anonymous_chains.delete(id).await {
        Ok(true) => Json(serde_json::json!({"ok": true})).into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/anonymous-routes/{id}/start", responses((status = 200, body = shared_types::AnonymousChain)))]
pub async fn start_anonymous_route(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state
        .deps
        .anonymous_routing
        .ensure_ready(&TrafficRoute::Chain(id))
        .await
    {
        Ok(_port) => match state.deps.storage.anonymous_chains.get(id).await {
            Ok(Some(chain)) => Json(chain).into_response(),
            Ok(None) => (StatusCode::NOT_FOUND, "anonymous chain not found".to_string()).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/anonymous-routes/{id}/stop", responses((status = 200)))]
pub async fn stop_anonymous_route(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    state.deps.events.publish(ServiceEvent::now(ServiceEventInner::AnonymousChainStopped {
        chain_id: id,
        reason: "user requested".to_string(),
    }));
    Json(serde_json::json!({"ok": true})).into_response()
}

#[utoipa::path(get, path = "/api/v1/privacy/analytics", responses((status = 200, body = shared_types::PrivacyAnalyticsSnapshot)))]
pub async fn get_privacy_analytics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.storage.privacy_analytics.latest().await {
        Ok(Some(snapshot)) => Json(snapshot).into_response(),
        Ok(None) => match state.deps.privacy_analytics.calculate().await {
            Ok(snapshot) => Json(snapshot).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/cover-traffic/settings", responses((status = 200, body = shared_types::CoverTrafficSettings)))]
pub async fn get_cover_traffic_settings(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.cover_traffic.list().await {
        Ok(settings) => {
            if let Some(first) = settings.into_iter().next() {
                Json(first).into_response()
            } else {
                let now = Utc::now();
                Json(CoverTrafficSettings {
                    id: Uuid::new_v4(),
                    mixnet_profile_id: None,
                    profile: CoverTrafficProfile::Disabled,
                    enabled: false,
                    rate_bps: None,
                    created_at: now,
                    updated_at: now,
                })
                .into_response()
            }
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(put, path = "/api/v1/cover-traffic/settings", responses((status = 200, body = shared_types::CoverTrafficSettings)))]
pub async fn update_cover_traffic_settings(
    State(state): State<Arc<AppState>>,
    Json(settings): Json<CoverTrafficSettings>,
) -> impl IntoResponse {
    match state.deps.cover_traffic.upsert(settings).await {
        Ok(settings) => Json(settings).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
