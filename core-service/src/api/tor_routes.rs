//! Tor profile start/stop API handlers.

use crate::api::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use shared_types::TorProfile;
use std::sync::Arc;
use uuid::Uuid;

#[utoipa::path(
    post,
    path = "/api/v1/tor/profiles/{id}/start",
    responses((status = 200, body = TorProfile))
)]
pub async fn tor_start(State(state): State<Arc<AppState>>, Path(id): Path<Uuid>) -> Response {
    match state.deps.tor.start(id).await {
        Ok(profile) => Json(profile).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/tor/profiles/{id}/stop",
    responses((status = 200, body = shared_types::TorStatus))
)]
pub async fn tor_stop(State(state): State<Arc<AppState>>, Path(id): Path<Uuid>) -> Response {
    if let Err(e) = state.deps.tor.stop(id, "user requested").await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    match state.deps.tor.status().await {
        Ok(status) => Json(status).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
