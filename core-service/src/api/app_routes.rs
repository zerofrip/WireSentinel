use crate::api::AppState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;
use shared_types::{AppExitConfig, AppSummary, TrafficRoute};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct SetAppRouteBody {
    pub app_id: Uuid,
    #[serde(default)]
    pub route: Option<TrafficRoute>,
    #[serde(default)]
    pub exit_config: Option<AppExitConfig>,
}

#[utoipa::path(get, path = "/api/v1/apps", responses((status = 200, body = [AppSummary])))]
pub async fn list_apps(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.app_registry.list(None, Some(500)).await {
        Ok(apps) => {
            let summaries: Vec<AppSummary> = apps.into_iter().map(AppSummary::from).collect();
            Json(summaries).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/apps", responses((status = 200, body = AppSummary)))]
pub async fn set_app_route(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetAppRouteBody>,
) -> impl IntoResponse {
    let old_route = state
        .deps
        .app_registry
        .get(body.app_id)
        .await
        .ok()
        .flatten()
        .and_then(|a| a.effective_exit_config())
        .and_then(|c| c.routes.first().cloned());

    let exit_config = if let Some(config) = body.exit_config {
        Some(config)
    } else {
        body.route.map(AppExitConfig::from_single)
    };

    let new_route = exit_config.as_ref().and_then(|c| c.routes.first().cloned());

    match state
        .deps
        .app_registry
        .set_exit_config(body.app_id, exit_config)
        .await
    {
        Ok(Some(app)) => {
            state.deps.exit_failover.reset_index(body.app_id);
            let _ = state
                .deps
                .audit
                .record_route_changed(body.app_id, old_route, new_route, None)
                .await;
            Json(AppSummary::from(app)).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
