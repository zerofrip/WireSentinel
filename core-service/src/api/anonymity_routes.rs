//! Phase 13 anonymity platform REST handlers.

use crate::api::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use shared_types::{AnonymousService, EntropySnapshot, FederatedMixnetConfig};
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Deserialize, ToSchema)]
pub struct DecoySimulateRequest {
    pub target: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AnonymityProfileAction {
    pub profile_id: Uuid,
}

#[utoipa::path(get, path = "/api/v1/anonymity", responses((status = 200, body = shared_types::AnonymityStatus)))]
pub async fn get_anonymity_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.anonymity.status().await {
        Ok(status) => Json(status).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/anonymity/entropy", responses((status = 200, body = shared_types::EntropySnapshot)))]
pub async fn get_anonymity_entropy(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let route_stats = match state
        .deps
        .storage
        .route_statistics
        .list(shared_types::RouteStatisticsQuery {
            app_id: None,
            domain: None,
            route_type: None,
            limit: 500,
        })
        .await
    {
        Ok(stats) => stats,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let route_types: Vec<&str> = route_stats.iter().map(|s| s.route_type.as_str()).collect();
    let status = state.deps.anonymity.status().await.unwrap_or_default();
    let score = state.deps.anonymity_entropy.score_route_types(&route_types);
    let anonymity_set_estimate = state
        .deps
        .anonymity_entropy
        .estimate_from_counts(status.active_providers, status.federated_active);
    Json(EntropySnapshot {
        score,
        anonymity_set_estimate,
        captured_at: Utc::now(),
    })
    .into_response()
}

#[utoipa::path(get, path = "/api/v1/anonymity/services", responses((status = 200, body = [shared_types::AnonymousService])))]
pub async fn list_anonymous_services(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.storage.anonymous_services.list().await {
        Ok(services) => Json(services).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/anonymity/services", responses((status = 200, body = shared_types::AnonymousService)))]
pub async fn create_anonymous_service(
    State(state): State<Arc<AppState>>,
    Json(mut service): Json<AnonymousService>,
) -> impl IntoResponse {
    let now = Utc::now();
    if service.id.is_nil() {
        service.id = Uuid::new_v4();
    }
    if service.created_at.timestamp() == 0 {
        service.created_at = now;
    }
    service.updated_at = now;
    if let Err(e) = state
        .deps
        .anonymity_security
        .validate_provider(&service.provider, service.profile_id)
    {
        return (StatusCode::BAD_REQUEST, e.to_string()).into_response();
    }
    match state.deps.storage.anonymous_services.insert(&service).await {
        Ok(()) => Json(service).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/anonymity/decoy/simulate", responses((status = 200)))]
pub async fn simulate_decoy_route(
    State(state): State<Arc<AppState>>,
    Json(body): Json<DecoySimulateRequest>,
) -> impl IntoResponse {
    match state.deps.anonymity_decoy.create_route(body.target) {
        Ok(route) => match state.deps.anonymity_decoy.simulate(&route) {
            Ok(hops) => Json(serde_json::json!({
                "route_id": route.id,
                "simulated_hops": hops,
            }))
            .into_response(),
            Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/privacy/anonymity", responses((status = 200, body = shared_types::PrivacyAnalyticsSnapshot)))]
pub async fn get_privacy_anonymity_analytics(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.deps.privacy_analytics.calculate().await {
        Ok(snapshot) => Json(snapshot).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/anonymity/katzenpost", responses((status = 200, body = [shared_types::KatzenpostProfile])))]
pub async fn list_katzenpost_profiles(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.anonymity.list_katzenpost().await {
        Ok(profiles) => Json(profiles).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/anonymity/loopix", responses((status = 200, body = [shared_types::LoopixProfile])))]
pub async fn list_loopix_profiles(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.anonymity.list_loopix().await {
        Ok(profiles) => Json(profiles).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/anonymity/katzenpost/start", responses((status = 200, body = shared_types::KatzenpostProfile)))]
pub async fn start_katzenpost(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AnonymityProfileAction>,
) -> impl IntoResponse {
    match state.deps.anonymity.start_katzenpost(body.profile_id).await {
        Ok(profile) => Json(profile).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/anonymity/loopix/start", responses((status = 200, body = shared_types::LoopixProfile)))]
pub async fn start_loopix(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AnonymityProfileAction>,
) -> impl IntoResponse {
    match state.deps.anonymity.start_loopix(body.profile_id).await {
        Ok(profile) => Json(profile).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[allow(dead_code)]
pub async fn start_federated_mixnet(
    State(state): State<Arc<AppState>>,
    Json(config): Json<FederatedMixnetConfig>,
) -> impl IntoResponse {
    match state.deps.anonymity.start_federated(config).await {
        Ok(port) => Json(serde_json::json!({ "listen_port": port })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[allow(dead_code)]
pub async fn get_anonymous_service(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.deps.storage.anonymous_services.get(id).await {
        Ok(Some(service)) => Json(service).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "service not found".to_string()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
