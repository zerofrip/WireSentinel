//! Tailnet profile join/leave API handlers.

use crate::api::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use shared_types::{TailnetProfile, TailscaleStatus};
use std::sync::Arc;
use uuid::Uuid;

#[utoipa::path(
    post,
    path = "/api/v1/tailnet/profiles/{id}/join",
    responses((status = 200, body = TailnetProfile))
)]
pub async fn tailnet_join(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.deps.tailscale.join(id).await {
        Ok(profile) => Json(profile).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/tailnet/profiles/{id}/leave",
    responses((status = 200, body = TailscaleStatus))
)]
pub async fn tailnet_leave(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Response {
    if let Err(e) = state.deps.tailscale.leave(id, "user requested").await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    match state.deps.tailscale.status().await {
        Ok(status) => Json(status).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
