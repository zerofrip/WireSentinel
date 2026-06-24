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
            "/split-templates/:id/clone",
            axum::routing::post(clone_split_template),
        )
        .route(
            "/vpn/:id/handshake-proxy",
            axum::routing::get(get_vpn_handshake_proxy).put(put_vpn_handshake_proxy),
        )
        .route(
            "/diagnostics/wiresock",
            axum::routing::get(get_wiresock_diagnostics),
        )
        .route(
            "/diagnostics/wiresock/template-trace",
            axum::routing::post(run_wiresock_template_trace),
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
pub struct WiresockDiagnostics {
    pub tcp_sessions: Vec<shared_types::TcpConnectionSnapshot>,
    pub template_trace: Option<shared_types::TemplateResolutionTrace>,
    pub handshake_proxy_profiles: Vec<HandshakeProxyProfileStatus>,
}

#[derive(serde::Serialize)]
pub struct HandshakeProxyProfileStatus {
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

async fn run_wiresock_template_trace(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let trace = state.deps.split_templates.manager().resolve_trace();
    state.deps.split_templates.store_trace(trace.clone());
    Json(trace).into_response()
}

#[derive(serde::Deserialize)]
struct CloneTemplateBody {
    name: Option<String>,
}

async fn clone_split_template(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<CloneTemplateBody>,
) -> impl IntoResponse {
    let templates = match state.deps.split_templates.list().await {
        Ok(t) => t,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let Some(source) = templates.into_iter().find(|t| t.id == id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let now = Utc::now();
    let mut clone = source;
    clone.id = Uuid::new_v4();
    clone.name = body
        .name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| format!("{} (copy)", clone.name));
    clone.created_at = now;
    clone.updated_at = now;
    if let Err(e) = state.deps.split_templates.upsert(clone.clone()).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    (StatusCode::CREATED, Json(clone)).into_response()
}

async fn get_vpn_handshake_proxy(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.deps.vpn.get_profile(id) {
        Some(profile) => Json(
            profile
                .handshake_proxy
                .unwrap_or_default(),
        )
        .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn put_vpn_handshake_proxy(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(settings): Json<HandshakeProxySettings>,
) -> impl IntoResponse {
    let mut profiles = match state.deps.storage.vpn_profiles.list().await {
        Ok(p) => p,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let Some(profile) = profiles.iter_mut().find(|p| p.id == id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    profile.handshake_proxy = Some(settings.clone());
    if let Err(e) = state.deps.storage.vpn_profiles.update(profile, None).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    state.deps.vpn.set_profiles(profiles);
    Json(settings).into_response()
}
