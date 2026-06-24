//! Enforcement backend settings API.

use crate::api::AppState;
use crate::enforcement;
use axum::{extract::State, response::IntoResponse, Json};
use shared_types::{
    EnforcementBackend, EnforcementComponentsHealth, EnforcementSettingsResponse,
    SetEnforcementBackendRequest,
};
use std::sync::Arc;

fn component_status(ok: bool, unused: bool) -> String {
    if unused {
        "not_used".into()
    } else if ok {
        "healthy".into()
    } else {
        "unavailable".into()
    }
}

pub async fn get_enforcement_settings(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mapping = enforcement::resolve_mapping(&state.deps.storage)
        .await
        .unwrap_or_else(|_| {
            shared_types::EnforcementMapping::from_backend(EnforcementBackend::Signed)
        });
    let wfp_engine = state
        .deps
        .storage
        .settings
        .wfp_engine_impl()
        .await
        .unwrap_or_else(|_| mapping.wfp_engine_impl.to_string());
    let driver = state.deps.wfp.driver_state().await;
    let ndis_health = if let Some(ndis) = state.deps.wfp.ndis_side() {
        ndis.health().await
    } else {
        shared_types::NdisHealth {
            available: false,
            state: "not_used".into(),
            filter_attached: false,
            active_route_count: 0,
            active_redirect_count: 0,
            classify_count: 0,
            error_count: 0,
            message: None,
            checked_at: chrono::Utc::now(),
        }
    };
    let signed = mapping.backend == EnforcementBackend::Signed;
    let guardian_probe = wfp::kernel_driver_available().ok();
    let windivert_probe = windivert_engine::windivert_available().ok();
    let transports = state.deps.transport.status().await.unwrap_or_default();
    let singbox_running = transports.iter().any(|t| {
        matches!(
            t.kind,
            shared_types::TransportProfileKind::SingBox
        ) && matches!(t.state, shared_types::TransportState::Running)
    });

    let response = EnforcementSettingsResponse {
        enforcement_backend: mapping.backend,
        guardian_mode: mapping.guardian_mode,
        wfp_engine_impl: wfp_engine,
        components: EnforcementComponentsHealth {
            wfp: component_status(driver.state == "running", false),
            wireguard: component_status(true, false),
            windivert: if signed {
                windivert_probe
                    .map(|_| "healthy".into())
                    .unwrap_or_else(|| "unavailable".into())
            } else {
                "not_used".into()
            },
            singbox: if singbox_running {
                "running".into()
            } else {
                "stopped".into()
            },
            guardian: if signed {
                "not_used".into()
            } else {
                guardian_probe
                    .map(|_| "healthy".into())
                    .unwrap_or_else(|| "unavailable".into())
            },
            ndis: if signed {
                if ndis_health.available {
                    "healthy".into()
                } else {
                    "unavailable".into()
                }
            } else if ndis_health.available {
                "healthy".into()
            } else {
                "unavailable".into()
            },
        },
        restart_required: false,
    };
    Json(response).into_response()
}

pub async fn set_enforcement_settings(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetEnforcementBackendRequest>,
) -> impl IntoResponse {
    match enforcement::apply_backend(&state.deps.storage, body.enforcement_backend).await {
        Ok(()) => Json(EnforcementSettingsResponse {
            enforcement_backend: body.enforcement_backend,
            guardian_mode: shared_types::EnforcementMapping::from_backend(body.enforcement_backend)
                .guardian_mode,
            wfp_engine_impl: shared_types::EnforcementMapping::from_backend(
                body.enforcement_backend,
            )
            .wfp_engine_impl
            .to_string(),
            components: EnforcementComponentsHealth {
                wfp: "pending".into(),
                wireguard: "available".into(),
                windivert: "pending".into(),
                singbox: "pending".into(),
                guardian: "pending".into(),
                ndis: "pending".into(),
            },
            restart_required: true,
        })
        .into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
