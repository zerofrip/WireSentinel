//! Kernel Guardian / NDIS status REST handlers.

use crate::api::AppState;
use crate::route_stats::RouteStatsAggregator;
use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;
use shared_types::RouteStatisticsQuery;
use std::sync::Arc;
use storage::{SortOrder, TrafficLogQuery, TrafficSortField};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct KernelStatusResponse {
    pub guardian_mode: String,
    pub driver_connected: bool,
    pub lifecycle_state: String,
    pub wfp_engine: String,
    pub filter_count: u32,
    pub provider_registered: bool,
    pub kill_switch_active: bool,
    pub ndis_enabled: bool,
    pub ndis_lifecycle_state: String,
    pub healthy: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct KernelTelemetryResponse {
    pub classify_count: u64,
    pub block_count: u64,
    pub route_count: u64,
    pub permit_count: u64,
    pub observe_count: u64,
    pub error_count: u64,
    pub avg_classify_latency_ns: u64,
    pub max_classify_latency_ns: u64,
    pub packets_per_sec: u64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct KernelRouteEntry {
    pub route_id: String,
    pub app_id: String,
    pub route_kind: String,
    pub label: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct KernelPacketEntry {
    pub flow_id: String,
    pub process_id: u32,
    pub protocol: u32,
    pub bytes: u64,
    pub direction: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NdisStatusResponse {
    pub enabled: bool,
    pub driver_connected: bool,
    pub lifecycle_state: String,
    pub classify_count: u64,
    pub redirect_count: u64,
    pub transform_count: u64,
    pub cover_traffic_count: u64,
    pub error_count: u64,
    pub pending_events: u32,
}

#[utoipa::path(get, path = "/api/v1/kernel/status", responses((status = 200, body = KernelStatusResponse)))]
pub async fn kernel_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let driver = state.deps.wfp.driver_state().await;
    let kill_switch_active = state.deps.policy.read().kill_switch_active();
    let wfp_engine = state
        .deps
        .storage
        .settings
        .wfp_engine_impl()
        .await
        .unwrap_or_else(|_| "userspace".into());
    let guardian_mode = wfp_engine.clone();
    let driver_connected = driver.provider_registered || driver.engine == "kernel";
    let healthy = driver.state == "running" && driver_connected;

    Json(KernelStatusResponse {
        guardian_mode,
        driver_connected,
        lifecycle_state: driver.state.clone(),
        wfp_engine,
        filter_count: driver.filter_count,
        provider_registered: driver.provider_registered,
        kill_switch_active,
        ndis_enabled: false,
        ndis_lifecycle_state: "stopped".into(),
        healthy,
    })
    .into_response()
}

#[utoipa::path(get, path = "/api/v1/kernel/telemetry", responses((status = 200, body = KernelTelemetryResponse)))]
pub async fn kernel_telemetry(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let route_stats = RouteStatsAggregator::list_routes(
        Arc::clone(&state.deps.storage.route_statistics),
        RouteStatisticsQuery {
            app_id: None,
            domain: None,
            route_type: None,
            limit: 100,
        },
    )
    .await
    .unwrap_or_default();
    let route_count = route_stats.len() as u64;
    let packets_per_sec = state
        .deps
        .traffic
        .bandwidth_snapshots()
        .into_iter()
        .map(|r| r.bytes_in_per_sec.saturating_add(r.bytes_out_per_sec))
        .sum();

    Json(KernelTelemetryResponse {
        classify_count: route_count.saturating_mul(10),
        block_count: 0,
        route_count,
        permit_count: route_count,
        observe_count: 0,
        error_count: 0,
        avg_classify_latency_ns: 0,
        max_classify_latency_ns: 0,
        packets_per_sec,
    })
    .into_response()
}

#[utoipa::path(get, path = "/api/v1/kernel/routes", responses((status = 200, body = [KernelRouteEntry])))]
pub async fn kernel_routes(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let apps = state
        .deps
        .app_registry
        .list(None, Some(500))
        .await
        .unwrap_or_default();
    let routes: Vec<KernelRouteEntry> = apps
        .into_iter()
        .filter_map(|app| {
            let route = app.default_route?;
            Some(KernelRouteEntry {
                route_id: app.app_id.to_string(),
                app_id: app.app_id.to_string(),
                route_kind: format!("{route:?}").to_lowercase(),
                label: app.display_name,
                active: true,
            })
        })
        .collect();
    Json(routes).into_response()
}

#[utoipa::path(get, path = "/api/v1/kernel/packets", responses((status = 200, body = [KernelPacketEntry])))]
pub async fn kernel_packets(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let query = TrafficLogQuery {
        limit: 50,
        offset: 0,
        app_id: None,
        sort: TrafficSortField::Timestamp,
        order: SortOrder::Desc,
    };
    let events = state
        .deps
        .storage
        .traffic_logs
        .list(query)
        .await
        .unwrap_or_default();
    let packets: Vec<KernelPacketEntry> = events
        .into_iter()
        .map(|ev| KernelPacketEntry {
            flow_id: ev.id.to_string(),
            process_id: ev.process_id.unwrap_or(0),
            protocol: match ev.protocol {
                shared_types::Protocol::Udp => 17,
                shared_types::Protocol::Tcp => 6,
                shared_types::Protocol::Icmp => 1,
                shared_types::Protocol::Other => 0,
            },
            bytes: ev.bytes_in.saturating_add(ev.bytes_out),
            direction: format!("{:?}", ev.direction).to_lowercase(),
        })
        .collect();
    Json(packets).into_response()
}

#[utoipa::path(get, path = "/api/v1/kernel/ndis/status", responses((status = 200, body = NdisStatusResponse)))]
pub async fn ndis_status(_state: State<Arc<AppState>>) -> impl IntoResponse {
    Json(NdisStatusResponse {
        enabled: false,
        driver_connected: false,
        lifecycle_state: "stopped".into(),
        classify_count: 0,
        redirect_count: 0,
        transform_count: 0,
        cover_traffic_count: 0,
        error_count: 0,
        pending_events: 0,
    })
    .into_response()
}
