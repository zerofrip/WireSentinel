//! Phase 18.5 WireSock API routes.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use shared_types::{
    HandshakeProxySettings, SplitTemplateModeSettings, SplitTunnelTemplate, TcpTerminationRule,
    TcpTerminationSettings,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::api::AppState;

pub fn routes() -> axum::Router<Arc<AppState>> {
    axum::Router::new()
        .route(
            "/tcp-termination/settings",
            axum::routing::get(get_tcp_settings).put(put_tcp_settings),
        )
        .route(
            "/tcp-termination/rules",
            axum::routing::get(list_tcp_rules).post(create_tcp_rule),
        )
        .route(
            "/tcp-termination/rules/:id",
            axum::routing::put(update_tcp_rule).delete(delete_tcp_rule),
        )
        .route(
            "/split-templates",
            axum::routing::get(list_split_templates).post(create_split_template),
        )
        .route(
            "/split-templates/mode",
            axum::routing::get(get_split_mode).put(put_split_mode),
        )
        .route(
            "/split-templates/:id",
            axum::routing::put(update_split_template).delete(delete_split_template),
        )
        .route(
            "/diagnostics/wiresock",
            axum::routing::get(get_wiresock_diagnostics),
        )
}

async fn get_tcp_settings(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.storage.tcp_termination.get_settings().await {
        Ok(s) => Json(s).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn put_tcp_settings(
    State(state): State<Arc<AppState>>,
    Json(mut settings): Json<TcpTerminationSettings>,
) -> impl IntoResponse {
    settings.updated_at = Utc::now();
    if let Err(e) = state
        .deps
        .storage
        .tcp_termination
        .set_settings(&settings)
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    let _ = state.deps.tcp_termination.reload_policy().await;
    Json(settings).into_response()
}

async fn list_tcp_rules(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.storage.tcp_termination.list_rules().await {
        Ok(rules) => Json(rules).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn create_tcp_rule(
    State(state): State<Arc<AppState>>,
    Json(mut rule): Json<TcpTerminationRule>,
) -> impl IntoResponse {
    let now = Utc::now();
    rule.id = Uuid::new_v4();
    rule.created_at = now;
    rule.updated_at = now;
    if let Err(e) = state.deps.storage.tcp_termination.insert_rule(&rule).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    let _ = state.deps.tcp_termination.reload_policy().await;
    (StatusCode::CREATED, Json(rule)).into_response()
}

async fn update_tcp_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(mut rule): Json<TcpTerminationRule>,
) -> impl IntoResponse {
    rule.id = id;
    rule.updated_at = Utc::now();
    if let Err(e) = state.deps.storage.tcp_termination.update_rule(&rule).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    let _ = state.deps.tcp_termination.reload_policy().await;
    Json(rule).into_response()
}

async fn delete_tcp_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.deps.storage.tcp_termination.delete_rule(id).await {
        Ok(true) => {
            let _ = state.deps.tcp_termination.reload_policy().await;
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn list_split_templates(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.split_templates.list().await {
        Ok(list) => Json(list).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn create_split_template(
    State(state): State<Arc<AppState>>,
    Json(mut template): Json<SplitTunnelTemplate>,
) -> impl IntoResponse {
    let now = Utc::now();
    template.id = Uuid::new_v4();
    template.created_at = now;
    template.updated_at = now;
    if let Err(e) = state.deps.split_templates.upsert(template.clone()).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    (StatusCode::CREATED, Json(template)).into_response()
}

async fn update_split_template(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(mut template): Json<SplitTunnelTemplate>,
) -> impl IntoResponse {
    template.id = id;
    template.updated_at = Utc::now();
    if let Err(e) = state.deps.split_templates.upsert(template.clone()).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    Json(template).into_response()
}

async fn delete_split_template(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.deps.split_templates.delete(id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_split_mode(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.split_templates.get_mode().await {
        Ok(mode) => Json(mode).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn put_split_mode(
    State(state): State<Arc<AppState>>,
    Json(mut settings): Json<SplitTemplateModeSettings>,
) -> impl IntoResponse {
    settings.updated_at = Utc::now();
    if let Err(e) = state.deps.split_templates.set_mode(settings.clone()).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    Json(settings).into_response()
}

#[derive(serde::Serialize)]
struct WiresockDiagnostics {
    tcp_sessions: Vec<shared_types::TcpConnectionSnapshot>,
    template_trace: Option<shared_types::TemplateResolutionTrace>,
    handshake_proxy_profiles: Vec<HandshakeProxyProfileStatus>,
}

#[derive(serde::Serialize)]
struct HandshakeProxyProfileStatus {
    profile_id: Uuid,
    profile_name: String,
    settings: Option<HandshakeProxySettings>,
}

async fn get_wiresock_diagnostics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let tcp_sessions = state.deps.tcp_termination.engine().enumerate();
    let template_trace = state.deps.split_templates.last_trace();
    let handshake_proxy_profiles = state
        .deps
        .vpn
        .profiles()
        .into_iter()
        .map(|p| HandshakeProxyProfileStatus {
            profile_id: p.id,
            profile_name: p.name,
            settings: p.handshake_proxy,
        })
        .collect();
    Json(WiresockDiagnostics {
        tcp_sessions,
        template_trace,
        handshake_proxy_profiles,
    })
    .into_response()
}
