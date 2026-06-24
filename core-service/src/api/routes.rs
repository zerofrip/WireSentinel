use crate::api::anonymity_routes::{
    create_anonymous_service, get_anonymity_entropy, get_anonymity_status,
    get_privacy_anonymity_analytics, list_anonymous_services, list_katzenpost_profiles,
    list_loopix_profiles, simulate_decoy_route, start_katzenpost, start_loopix,
};
use crate::api::app_routes::{list_apps, set_app_route};
use crate::api::kernel_routes::{
    kernel_packets, kernel_routes, kernel_status, kernel_telemetry, ndis_status,
};
use crate::api::middleware::extract_bearer;
use crate::api::mixnet_routes::{
    create_anonymous_route, delete_anonymous_route, get_anonymous_route,
    get_cover_traffic_settings, get_privacy_analytics, list_anonymous_routes, list_mixnet_profiles,
    mixnet_routes, mixnet_status, start_anonymous_route, start_mixnet, stop_anonymous_route,
    stop_mixnet, update_anonymous_route, update_cover_traffic_settings,
};
use crate::api::openapi::ApiDoc;
use crate::api::proxy_routes::{
    connect_proxy, create_proxy, create_proxy_chain, delete_proxy, delete_proxy_chain,
    disconnect_proxy, get_proxy, get_proxy_chain, health_proxy, latency_proxy, list_proxies,
    list_proxy_chains, start_proxy_chain, stop_proxy_chain, update_proxy, update_proxy_chain,
};
use crate::api::settings_routes::{get_enforcement_settings, set_enforcement_settings};
use crate::api::AppState;
use crate::enterprise::PolicyProvider;
use crate::route_stats::RouteStatsAggregator;
use crate::tor::{BridgeTestRequest, BridgeTestResponse};
use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use chrono::Utc;
use filter_lists::filters_cache_dir;
use serde::Deserialize;
use shared_types::{
    AuditLogQuery, ChainProfile, DnsProviderRecord, DnsSettings, EnterprisePolicy,
    FilterListRecord, KernelStatistics, KernelTelemetryV2, LeakIncident, LogLevel, PolicyMode,
    PrivacyScoreSnapshot, RouteStatisticsQuery, Rule, SecurityAuditEntry, SecurityFinding,
    ServiceEventInner, ServiceStatus, TailscaleStatus, TorStatus, TransportProfile,
    TransportStatusRecord, VPNProfile, ValidationReport,
};
use std::sync::Arc;
use storage::{
    CorrelationQuery, DnsLogQuery, DnsSortField, SortOrder, TrafficLogQuery, TrafficSortField,
};
use tower_http::cors::{Any, CorsLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;
use vpn_engine::{detect_backend, materialize_profile_config};

pub fn router(state: AppState) -> Router {
    let shared = Arc::new(state);
    let api = Router::new()
        .route("/status", get(status))
        .route("/apps", get(list_apps).post(set_app_route))
        .route("/traffic", get(list_traffic))
        .route("/traffic/logs", get(traffic_logs))
        .route("/traffic/export", get(export_traffic))
        .route("/traffic/top-domains", get(top_domains))
        .route("/rules", get(list_rules).post(add_rule))
        .route("/rules/{id}", put(update_rule).delete(delete_rule))
        .route("/rules/kill-switch", post(set_kill_switch))
        .route("/rules/mode", get(get_policy_mode).put(set_policy_mode))
        .route("/statistics/routes", get(route_statistics))
        .route("/statistics/blocked", get(blocked_statistics))
        .route("/audit", get(audit_log))
        .route("/vpn", get(list_vpn).post(add_vpn))
        .route("/vpn/{id}/connect", post(vpn_connect))
        .route("/vpn/{id}/disconnect", post(vpn_disconnect))
        .route("/vpn/{id}/status", get(vpn_status))
        .route("/dns", get(get_dns).put(set_dns))
        .route("/dns/settings", get(get_dns).put(set_dns))
        .route("/dns/logs", get(dns_logs))
        .route("/dns/resolve", post(dns_resolve))
        .route(
            "/filter-lists",
            get(list_filter_lists).post(add_filter_list),
        )
        .route(
            "/filter-lists/{id}",
            put(update_filter_list).delete(delete_filter_list),
        )
        .route("/filter-lists/{id}/update", post(refresh_filter_list))
        .route("/correlations", get(list_correlations))
        .route("/privacy", get(get_privacy))
        .route("/leaks", get(list_leaks))
        .route("/transports", get(list_transports))
        .route("/transports/status", get(transports_status))
        .route("/chains", get(list_chains).post(create_chain))
        .route("/chains/{id}/start", post(start_chain))
        .route("/chains/{id}/stop", post(stop_chain))
        .route("/plugins", get(list_plugins))
        .merge(crate::api::wiresock::routes())
        .route("/plugins/load", post(load_plugin))
        .route("/plugins/unload", post(unload_plugin))
        .route("/tailnet", get(list_tailnet).post(upsert_tailnet))
        .route("/tailnet/status", get(tailnet_status))
        .route("/tor", get(list_tor).post(upsert_tor))
        .route("/tor/status", get(tor_status))
        .route("/bridges", get(list_bridges).post(create_bridge))
        .route("/bridges/test", post(test_bridge))
        .route("/proxies", get(list_proxies).post(create_proxy))
        .route(
            "/proxies/{id}",
            get(get_proxy).put(update_proxy).delete(delete_proxy),
        )
        .route("/proxies/{id}/connect", post(connect_proxy))
        .route("/proxies/{id}/disconnect", post(disconnect_proxy))
        .route("/proxies/{id}/health", post(health_proxy))
        .route("/proxies/{id}/latency", post(latency_proxy))
        .route(
            "/proxy-chains",
            get(list_proxy_chains).post(create_proxy_chain),
        )
        .route(
            "/proxy-chains/{id}",
            get(get_proxy_chain)
                .put(update_proxy_chain)
                .delete(delete_proxy_chain),
        )
        .route("/proxy-chains/{id}/start", post(start_proxy_chain))
        .route("/proxy-chains/{id}/stop", post(stop_proxy_chain))
        .route("/mixnet", get(list_mixnet_profiles))
        .route("/mixnet/status", get(mixnet_status))
        .route("/mixnet/routes", get(mixnet_routes))
        .route("/mixnet/start", post(start_mixnet))
        .route("/mixnet/stop", post(stop_mixnet))
        .route("/kernel/status", get(kernel_status))
        .route("/kernel/telemetry", get(kernel_telemetry))
        .route("/kernel/routes", get(kernel_routes))
        .route("/kernel/packets", get(kernel_packets))
        .route("/kernel/ndis/status", get(ndis_status))
        .route(
            "/anonymous-routes",
            get(list_anonymous_routes).post(create_anonymous_route),
        )
        .route(
            "/anonymous-routes/{id}",
            get(get_anonymous_route)
                .put(update_anonymous_route)
                .delete(delete_anonymous_route),
        )
        .route("/anonymous-routes/{id}/start", post(start_anonymous_route))
        .route("/anonymous-routes/{id}/stop", post(stop_anonymous_route))
        .route("/privacy/analytics", get(get_privacy_analytics))
        .route("/privacy/anonymity", get(get_privacy_anonymity_analytics))
        .route("/anonymity", get(get_anonymity_status))
        .route("/anonymity/entropy", get(get_anonymity_entropy))
        .route(
            "/anonymity/services",
            get(list_anonymous_services).post(create_anonymous_service),
        )
        .route("/anonymity/decoy/simulate", post(simulate_decoy_route))
        .route("/anonymity/katzenpost", get(list_katzenpost_profiles))
        .route("/anonymity/katzenpost/start", post(start_katzenpost))
        .route("/anonymity/loopix", get(list_loopix_profiles))
        .route("/anonymity/loopix/start", post(start_loopix))
        .route(
            "/cover-traffic/settings",
            get(get_cover_traffic_settings).put(update_cover_traffic_settings),
        )
        .route(
            "/dns/providers",
            get(list_dns_providers).put(upsert_dns_providers),
        )
        .route("/logs", get(list_logs))
        .route("/logs/download", get(download_logs))
        .route("/settings/log-level", put(set_log_level))
        .route(
            "/settings/enforcement",
            get(get_enforcement_settings).put(set_enforcement_settings),
        )
        .route("/performance", get(get_performance))
        .route("/diagnostics", get(get_diagnostics))
        .route("/diagnostics/export", post(export_diagnostics))
        .route("/backup/export", get(export_backup))
        .route("/backup/import", post(import_backup))
        .route(
            "/enterprise/policy",
            get(get_enterprise_policy).put(set_enterprise_policy),
        )
        .route("/update", get(get_update))
        .route("/update/check", post(check_update))
        .route("/metrics", get(get_metrics))
        .route("/validation", get(get_validation))
        .route("/kernel/statistics", get(get_kernel_statistics))
        .route("/benchmark", get(get_benchmark))
        .route("/security/audit", get(list_security_audit))
        .route("/security/audit/run", post(run_security_audit))
        .route("/fault/inject", post(fault_inject))
        .route("/auth/rotate", post(rotate_auth_token))
        .route("/events", get(ws_handler))
        .route_layer(middleware::from_fn_with_state(
            Arc::clone(&shared),
            crate::api::middleware::require_bearer,
        ))
        .with_state(Arc::clone(&shared));

    Router::new()
        .merge(SwaggerUi::new("/api/v1/docs").url("/api/v1/openapi.json", ApiDoc::openapi()))
        .nest("/api/v1", api)
        .layer(middleware::from_fn(crate::api::middleware::rate_limit))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
}

#[utoipa::path(get, path = "/api/v1/status", responses((status = 200, body = ServiceStatus)))]
pub async fn status(State(state): State<Arc<AppState>>) -> Json<ServiceStatus> {
    Json(state.deps.status())
}

#[derive(Deserialize)]
pub struct LimitQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Deserialize)]
pub struct TrafficLogsQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub app_id: Option<Uuid>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

#[derive(Deserialize)]
pub struct DnsLogsQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub qname: Option<String>,
    pub blocked: Option<bool>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

#[derive(Deserialize)]
pub struct CorrelationsQuery {
    pub limit: Option<u32>,
    pub app_id: Option<Uuid>,
    pub domain: Option<String>,
}

#[derive(Deserialize)]
pub struct ExportQuery {
    pub format: Option<String>,
    pub source: Option<String>,
    pub limit: Option<u32>,
    pub app_id: Option<Uuid>,
}

async fn list_traffic(
    State(state): State<Arc<AppState>>,
    Query(q): Query<LimitQuery>,
) -> impl IntoResponse {
    let snapshots = state.deps.traffic.bandwidth_snapshots();
    let limit = q.limit.unwrap_or(100) as usize;
    Json(snapshots.into_iter().take(limit).collect::<Vec<_>>()).into_response()
}

async fn export_traffic(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ExportQuery>,
) -> impl IntoResponse {
    let format = q.format.as_deref().unwrap_or("json");
    if format == "csv" {
        let query = TrafficLogQuery {
            limit: q.limit.unwrap_or(1000),
            offset: 0,
            app_id: q.app_id,
            sort: TrafficSortField::Timestamp,
            order: SortOrder::Desc,
        };
        match state.deps.storage.traffic_logs.list(query).await {
            Ok(logs) => {
                let mut csv =
                    String::from("id,app_id,timestamp,protocol,remote_addr,domain,route\n");
                for log in logs {
                    csv.push_str(&format!(
                        "{},{},{},{:?},{},{},{:?}\n",
                        log.id,
                        log.app.id(),
                        log.timestamp,
                        log.protocol,
                        log.remote_addr,
                        log.remote_domain.as_deref().unwrap_or(""),
                        log.route
                    ));
                }
                (StatusCode::OK, [("content-type", "text/csv")], csv).into_response()
            }
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    } else if q.source.as_deref() == Some("logs") {
        let query = TrafficLogQuery {
            limit: q.limit.unwrap_or(1000),
            offset: 0,
            app_id: q.app_id,
            sort: TrafficSortField::Timestamp,
            order: SortOrder::Desc,
        };
        match state.deps.storage.traffic_logs.list(query).await {
            Ok(logs) => Json(logs).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    } else {
        let snapshots = state.deps.traffic.bandwidth_snapshots();
        Json(snapshots).into_response()
    }
}

async fn traffic_logs(
    State(state): State<Arc<AppState>>,
    Query(q): Query<TrafficLogsQuery>,
) -> impl IntoResponse {
    let query = TrafficLogQuery {
        limit: q.limit.unwrap_or(100),
        offset: q.offset.unwrap_or(0),
        app_id: q.app_id,
        sort: match q.sort.as_deref() {
            Some("bytes") => TrafficSortField::Bytes,
            _ => TrafficSortField::Timestamp,
        },
        order: match q.order.as_deref() {
            Some("asc") => SortOrder::Asc,
            _ => SortOrder::Desc,
        },
    };
    match state.deps.storage.traffic_logs.list(query).await {
        Ok(logs) => Json(logs).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn top_domains(
    State(state): State<Arc<AppState>>,
    Query(q): Query<LimitQuery>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(20);
    match state.deps.storage.dns_logs.top_domains(limit).await {
        Ok(entries) => Json(entries).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn list_correlations(
    State(state): State<Arc<AppState>>,
    Query(q): Query<CorrelationsQuery>,
) -> impl IntoResponse {
    let query = CorrelationQuery {
        limit: q.limit.unwrap_or(100),
        app_id: q.app_id,
        domain: q.domain,
    };
    match state.deps.storage.correlations.list(query).await {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn list_filter_lists(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.storage.filter_lists.list().await {
        Ok(lists) => Json(lists).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn add_filter_list(
    State(state): State<Arc<AppState>>,
    Json(mut record): Json<FilterListRecord>,
) -> impl IntoResponse {
    if record.id.is_nil() {
        record.id = Uuid::new_v4();
    }
    if record.cache_path.is_none() {
        record.cache_path = Some(filters_cache_dir().join(format!("{}.cache", record.id)));
    }
    if let Err(e) = state.deps.storage.filter_lists.insert(&record).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    if let Err(e) = state.deps.sync_filter_engine().await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    Json(record).into_response()
}

async fn update_filter_list(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(mut record): Json<FilterListRecord>,
) -> impl IntoResponse {
    record.id = id;
    if let Err(e) = state.deps.storage.filter_lists.update(&record).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    if let Err(e) = state.deps.sync_filter_engine().await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    Json(record).into_response()
}

async fn delete_filter_list(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.deps.storage.filter_lists.delete(id).await {
        Ok(true) => {
            let _ = state.deps.sync_filter_engine().await;
            Json(serde_json::json!({"ok": true})).into_response()
        }
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn refresh_filter_list(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let record = match state.deps.storage.filter_lists.get(id).await {
        Ok(Some(r)) => r,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    match state.deps.filter_lists.update_one(id).await {
        Ok(()) => {
            let entry_count = state.deps.filter_lists.subscriptions().len() as u32;
            let now = Utc::now();
            let mut updated = record;
            updated.last_updated = Some(now);
            let _ = state.deps.storage.filter_lists.update(&updated).await;
            state.deps.events.publish(
                ServiceEventInner::FilterListUpdated {
                    list_id: id,
                    name: updated.name.clone(),
                    entry_count,
                }
                .with_timestamp(now),
            );
            Json(serde_json::json!({"ok": true, "entry_count": entry_count})).into_response()
        }
        Err(e) => {
            state.deps.events.publish(
                ServiceEventInner::FilterListFailed {
                    list_id: id,
                    error: e.clone(),
                }
                .with_timestamp(Utc::now()),
            );
            (StatusCode::INTERNAL_SERVER_ERROR, e).into_response()
        }
    }
}

async fn list_rules(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.storage.rules.list().await {
        Ok(rules) => Json(rules).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn add_rule(State(state): State<Arc<AppState>>, Json(rule): Json<Rule>) -> impl IntoResponse {
    if let Err(e) = state.deps.storage.rules.insert(&rule).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    state.deps.policy.write().add_rule(rule.clone());
    state
        .deps
        .events
        .publish(ServiceEventInner::RuleCreated { rule: rule.clone() }.with_timestamp(Utc::now()));
    Json(serde_json::json!({"ok": true})).into_response()
}

async fn update_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(mut rule): Json<Rule>,
) -> impl IntoResponse {
    rule.id = id;
    if let Err(e) = state.deps.storage.rules.update(&rule).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    if !state.deps.policy.write().update_rule(rule.clone()) {
        state.deps.policy.write().add_rule(rule.clone());
    }
    state
        .deps
        .events
        .publish(ServiceEventInner::RuleUpdated { rule: rule.clone() }.with_timestamp(Utc::now()));
    Json(serde_json::json!({"ok": true})).into_response()
}

async fn delete_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.deps.storage.rules.delete(id).await {
        Ok(true) => {
            state.deps.policy.write().remove_rule(id);
            state
                .deps
                .events
                .publish(ServiceEventInner::RuleDeleted { rule_id: id }.with_timestamp(Utc::now()));
            Json(serde_json::json!({"ok": true})).into_response()
        }
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct KillSwitchBody {
    active: bool,
}

async fn set_kill_switch(
    State(state): State<Arc<AppState>>,
    Json(body): Json<KillSwitchBody>,
) -> impl IntoResponse {
    let old = state.deps.policy.read().kill_switch_active();
    state.deps.policy.write().set_kill_switch(body.active);
    if let Err(e) = state.deps.wfp.apply_kill_switch(body.active).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    let driver_state = state.deps.wfp.driver_state().await;
    state.deps.events.publish(
        ServiceEventInner::DriverStateChanged {
            state: driver_state,
        }
        .with_timestamp(Utc::now()),
    );
    let _ = state
        .deps
        .audit
        .record_policy_changed(
            "kill_switch_active",
            Some(old.to_string()),
            Some(body.active.to_string()),
            None,
        )
        .await;
    Json(serde_json::json!({"ok": true, "active": body.active})).into_response()
}

#[derive(Deserialize)]
struct RouteStatsQuery {
    app_id: Option<Uuid>,
    domain: Option<String>,
    route_type: Option<String>,
    limit: Option<u32>,
}

async fn route_statistics(
    State(state): State<Arc<AppState>>,
    Query(q): Query<RouteStatsQuery>,
) -> impl IntoResponse {
    let query = RouteStatisticsQuery {
        app_id: q.app_id,
        domain: q.domain,
        route_type: q.route_type,
        limit: q.limit.unwrap_or(100),
    };
    match RouteStatsAggregator::list_routes(Arc::clone(&state.deps.storage.route_statistics), query)
        .await
    {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn blocked_statistics(
    State(state): State<Arc<AppState>>,
    Query(q): Query<LimitQuery>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(50);
    match RouteStatsAggregator::blocked_summary(
        Arc::clone(&state.deps.storage.route_statistics),
        limit,
    )
    .await
    {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct AuditQuery {
    event_type: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
}

async fn audit_log(
    State(state): State<Arc<AppState>>,
    Query(q): Query<AuditQuery>,
) -> impl IntoResponse {
    let query = AuditLogQuery {
        event_type: q.event_type,
        limit: q.limit.unwrap_or(100),
        offset: q.offset.unwrap_or(0),
    };
    match state.deps.audit.list(query).await {
        Ok(entries) => Json(entries).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_policy_mode(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.storage.rules.get_policy_mode().await {
        Ok(mode) => Json(serde_json::json!({ "mode": mode })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct PolicyModeBody {
    mode: PolicyMode,
}

async fn set_policy_mode(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PolicyModeBody>,
) -> impl IntoResponse {
    let old = state.deps.storage.rules.get_policy_mode().await.ok();
    if let Err(e) = state.deps.storage.rules.set_policy_mode(body.mode).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    state.deps.policy.write().set_mode(body.mode);
    let _ = state
        .deps
        .audit
        .record_policy_changed(
            "policy_mode",
            old.map(|m| format!("{m:?}")),
            Some(format!("{:?}", body.mode)),
            None,
        )
        .await;
    Json(serde_json::json!({"ok": true})).into_response()
}

#[utoipa::path(get, path = "/api/v1/vpn", responses((status = 200, body = [VPNProfile])))]
pub async fn list_vpn(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let profiles = state.deps.vpn.profiles();
    let mut out = Vec::new();
    for p in profiles {
        let status = state.deps.vpn.state(p.id).await;
        out.push(serde_json::json!({
            "profile": p,
            "status": status,
        }));
    }
    Json(out).into_response()
}

#[derive(Deserialize)]
pub struct AddVpnBody {
    pub name: String,
    pub config_plaintext: String,
}

async fn add_vpn(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AddVpnBody>,
) -> impl IntoResponse {
    let backend = detect_backend(&body.config_plaintext);
    let blob = body.config_plaintext.into_bytes();
    let profile = VPNProfile::new(
        body.name,
        backend,
        std::path::PathBuf::from(format!("db://{}", Uuid::new_v4())),
    );
    if let Err(e) = state
        .deps
        .storage
        .vpn_profiles
        .insert(&profile, &blob)
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    state.deps.vpn.add_profile(profile.clone());
    state.deps.profile_cache.load(&state.deps.vpn.profiles());
    Json(profile).into_response()
}

async fn vpn_connect(State(state): State<Arc<AppState>>, Path(id): Path<Uuid>) -> Response {
    let profile = match state.deps.vpn.get_profile(id) {
        Some(p) => p,
        None => return StatusCode::NOT_FOUND.into_response(),
    };
    let blob = match state.deps.storage.vpn_profiles.get_config_blob(id).await {
        Ok(blob) => blob,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let config_path = match materialize_profile_config(&profile, blob.as_deref()) {
        Ok(path) => path,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    match state.deps.vpn.connect_with_config(id, config_path).await {
        Ok(()) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn vpn_disconnect(State(state): State<Arc<AppState>>, Path(id): Path<Uuid>) -> Response {
    match state.deps.vpn.disconnect(id).await {
        Ok(()) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn vpn_status(State(state): State<Arc<AppState>>, Path(id): Path<Uuid>) -> Response {
    match state.deps.vpn.state(id).await {
        Some(s) => Json(s).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn get_dns(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(state.deps.dns.settings()).into_response()
}

async fn set_dns(
    State(state): State<Arc<AppState>>,
    Json(settings): Json<DnsSettings>,
) -> impl IntoResponse {
    match state.deps.dns.update_settings(settings.clone()) {
        Ok(()) => {
            let _ = state
                .deps
                .storage
                .settings
                .set_dns_settings(&settings)
                .await;
            Json(serde_json::json!({"ok": true})).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn dns_logs(
    State(state): State<Arc<AppState>>,
    Query(q): Query<DnsLogsQuery>,
) -> impl IntoResponse {
    let query = DnsLogQuery {
        limit: q.limit.unwrap_or(100),
        offset: q.offset.unwrap_or(0),
        qname: q.qname,
        blocked: q.blocked,
        sort: DnsSortField::Timestamp,
        order: match q.order.as_deref() {
            Some("asc") => SortOrder::Asc,
            _ => SortOrder::Desc,
        },
    };
    match state.deps.storage.dns_logs.list(query).await {
        Ok(logs) => Json(logs).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct ResolveBody {
    pub qname: String,
}

async fn dns_resolve(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ResolveBody>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();
    match state.deps.dns.resolve(&body.qname, "A").await {
        Ok(log) => {
            crate::benchmark::record_dns_latency_ms(start.elapsed().as_secs_f64() * 1000.0);
            if self_store_dns(&state).await {
                let _ = state.deps.storage.dns_logs.insert(&log).await;
            }
            let ts = Utc::now();
            if log.blocked {
                state.deps.events.publish(
                    ServiceEventInner::DnsQueryBlocked { log: log.clone() }.with_timestamp(ts),
                );
            } else {
                state.deps.events.publish(
                    ServiceEventInner::DnsQueryObserved { log: log.clone() }.with_timestamp(ts),
                );
                if let Some(cid) = log.correlation_id {
                    let _ = state
                        .deps
                        .correlator
                        .on_dns_resolved(log.app_id, &log.qname, &log.answers, cid)
                        .await;
                } else {
                    let cid = Uuid::new_v4();
                    let _ = state
                        .deps
                        .correlator
                        .on_dns_resolved(log.app_id, &log.qname, &log.answers, cid)
                        .await;
                }
            }
            Json(log).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn self_store_dns(state: &Arc<AppState>) -> bool {
    state
        .deps
        .storage
        .settings
        .store_dns_logs()
        .await
        .unwrap_or(true)
}

#[utoipa::path(get, path = "/api/v1/privacy", responses((status = 200, body = PrivacyScoreSnapshot)))]
pub async fn get_privacy(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.storage.privacy_snapshots.latest().await {
        Ok(Some(snapshot)) => Json(snapshot).into_response(),
        Ok(None) => match state.deps.privacy.calculate().await {
            Ok(snapshot) => Json(snapshot).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct LeaksQuery {
    #[serde(default = "default_leaks_limit")]
    pub limit: u32,
}

fn default_leaks_limit() -> u32 {
    50
}

#[utoipa::path(get, path = "/api/v1/leaks", params(LeaksQuery), responses((status = 200, body = [LeakIncident])))]
pub async fn list_leaks(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LeaksQuery>,
) -> impl IntoResponse {
    match state
        .deps
        .storage
        .leak_incidents
        .list_recent(query.limit)
        .await
    {
        Ok(incidents) => Json(incidents).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/transports", responses((status = 200, body = [TransportProfile])))]
pub async fn list_transports(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.transport.list_profiles().await {
        Ok(profiles) => Json(profiles).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/transports/status", responses((status = 200, body = [TransportStatusRecord])))]
pub async fn transports_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.transport.status().await {
        Ok(status) => Json(status).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/chains", responses((status = 200, body = [ChainProfile])))]
pub async fn list_chains(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.storage.chain_profiles.list().await {
        Ok(chains) => Json(chains).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/chains", responses((status = 200, body = ChainProfile)))]
pub async fn create_chain(
    State(state): State<Arc<AppState>>,
    Json(mut profile): Json<ChainProfile>,
) -> impl IntoResponse {
    if profile.id.is_nil() {
        profile.id = Uuid::new_v4();
    }
    let now = Utc::now();
    if profile.created_at.timestamp() == 0 {
        profile.created_at = now;
    }
    profile.updated_at = now;
    match state.deps.storage.chain_profiles.insert(&profile).await {
        Ok(()) => {
            state.deps.events.publish(
                ServiceEventInner::TransportChainUpdated {
                    chain: profile.clone(),
                }
                .with_timestamp(now),
            );
            Json(profile).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/chains/{id}/start", responses((status = 200)))]
pub async fn start_chain(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let chain = match state.deps.storage.chain_profiles.get(id).await {
        Ok(Some(c)) => c,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    if let Some(profile_id) = chain.obfuscation_profile_id {
        if let Ok(Some(obf)) = state
            .deps
            .storage
            .obfuscation_profiles
            .get(profile_id)
            .await
        {
            state.deps.events.publish(
                ServiceEventInner::ObfuscationProfileApplied {
                    chain_id: chain.id,
                    profile_id,
                    preset: obf.preset,
                }
                .with_timestamp(Utc::now()),
            );
        }
    }

    match state.deps.transport.start_chain(&chain).await {
        Ok(()) => {
            state.deps.events.publish(
                ServiceEventInner::TransportChainStarted {
                    chain_id: chain.id,
                    name: chain.name.clone(),
                }
                .with_timestamp(Utc::now()),
            );
            Json(serde_json::json!({"ok": true})).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/chains/{id}/stop", responses((status = 200)))]
pub async fn stop_chain(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.deps.transport.stop_chain(id).await {
        Ok(()) => {
            state.deps.events.publish(
                ServiceEventInner::TransportChainStopped {
                    chain_id: id,
                    reason: "api stop".into(),
                }
                .with_timestamp(Utc::now()),
            );
            Json(serde_json::json!({"ok": true})).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/plugins", responses((status = 200, body = [shared_types::PluginRecord])))]
pub async fn list_plugins(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.plugins.list().await {
        Ok(plugins) => Json(plugins).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct PluginActionBody {
    pub id: Uuid,
}

#[utoipa::path(post, path = "/api/v1/plugins/load", responses((status = 200, body = shared_types::PluginRecord)))]
pub async fn load_plugin(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PluginActionBody>,
) -> impl IntoResponse {
    match state.deps.plugins.load(body.id).await {
        Ok(record) => Json(record).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/plugins/unload", responses((status = 200, body = shared_types::PluginRecord)))]
pub async fn unload_plugin(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PluginActionBody>,
) -> impl IntoResponse {
    match state.deps.plugins.unload(body.id).await {
        Ok(record) => Json(record).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/tailnet", responses((status = 200, body = [shared_types::TailnetProfile])))]
pub async fn list_tailnet(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.storage.tailnet_profiles.list().await {
        Ok(profiles) => Json(profiles).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/tailnet", responses((status = 200, body = shared_types::TailnetProfile)))]
pub async fn upsert_tailnet(
    State(state): State<Arc<AppState>>,
    Json(mut profile): Json<shared_types::TailnetProfile>,
) -> impl IntoResponse {
    let now = Utc::now();
    if profile.id.is_nil() {
        profile.id = Uuid::new_v4();
    }
    if profile.created_at.timestamp() == 0 {
        profile.created_at = now;
    }
    profile.updated_at = now;
    let op = if state
        .deps
        .storage
        .tailnet_profiles
        .get(profile.id)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        state.deps.storage.tailnet_profiles.update(&profile).await
    } else {
        state.deps.storage.tailnet_profiles.insert(&profile).await
    };
    match op {
        Ok(()) => {
            state.deps.events.publish(
                ServiceEventInner::TailnetProfileUpdated {
                    profile: profile.clone(),
                }
                .with_timestamp(now),
            );
            Json(profile).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/tailnet/status", responses((status = 200, body = TailscaleStatus)))]
pub async fn tailnet_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.tailscale.status().await {
        Ok(status) => Json(status).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/tor/profiles", responses((status = 200, body = [shared_types::TorProfile])))]
pub async fn list_tor(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.storage.tor_profiles.list().await {
        Ok(profiles) => Json(profiles).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/tor/profiles", responses((status = 200, body = shared_types::TorProfile)))]
pub async fn upsert_tor(
    State(state): State<Arc<AppState>>,
    Json(mut profile): Json<shared_types::TorProfile>,
) -> impl IntoResponse {
    let now = Utc::now();
    if profile.id.is_nil() {
        profile.id = Uuid::new_v4();
    }
    if profile.created_at.timestamp() == 0 {
        profile.created_at = now;
    }
    profile.updated_at = now;
    let op = if state
        .deps
        .storage
        .tor_profiles
        .get(profile.id)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        state.deps.storage.tor_profiles.update(&profile).await
    } else {
        state.deps.storage.tor_profiles.insert(&profile).await
    };
    match op {
        Ok(()) => Json(profile).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/tor/status", responses((status = 200, body = TorStatus)))]
pub async fn tor_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.tor.status().await {
        Ok(status) => Json(status).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/bridges", responses((status = 200, body = shared_types::BridgeProfile)))]
pub async fn create_bridge(
    State(state): State<Arc<AppState>>,
    Json(mut profile): Json<shared_types::BridgeProfile>,
) -> impl IntoResponse {
    let now = Utc::now();
    if profile.id.is_nil() {
        profile.id = Uuid::new_v4();
    }
    if profile.created_at.timestamp() == 0 {
        profile.created_at = now;
    }
    profile.updated_at = now;
    match state.deps.storage.bridge_profiles.insert(&profile).await {
        Ok(()) => Json(profile).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/dns/providers", responses((status = 200, body = [DnsProviderRecord])))]
pub async fn list_dns_providers(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.storage.dns_providers.list().await {
        Ok(providers) => Json(providers).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpsertDnsProvidersBody {
    pub providers: Vec<DnsProviderRecord>,
}

#[utoipa::path(put, path = "/api/v1/dns/providers", responses((status = 200)))]
pub async fn upsert_dns_providers(
    State(state): State<Arc<AppState>>,
    Json(body): Json<UpsertDnsProvidersBody>,
) -> impl IntoResponse {
    let now = Utc::now();
    for mut provider in body.providers {
        if provider.id.is_nil() {
            provider.id = Uuid::new_v4();
        }
        if provider.created_at.timestamp() == 0 {
            provider.created_at = now;
        }
        provider.updated_at = now;
        if let Err(e) = state.deps.storage.dns_providers.upsert(&provider).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
        state.deps.events.publish(
            ServiceEventInner::DnsProviderChanged {
                provider_id: provider.id,
                provider_name: provider.name.clone(),
            }
            .with_timestamp(now),
        );
    }

    match state.deps.storage.dns_providers.list().await {
        Ok(records) => {
            if let Err(e) = state.deps.dns.load_providers_from_records(&records) {
                return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
            }
            if state.deps.dns.settings().enabled {
                let _ = state.deps.dns.update_settings(state.deps.dns.settings());
            }
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }

    Json(serde_json::json!({"ok": true})).into_response()
}

#[utoipa::path(get, path = "/api/v1/tailscale", responses((status = 200, body = TailscaleStatus)))]
pub async fn get_tailscale(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.tailscale.status().await {
        Ok(status) => Json(status).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/tor", responses((status = 200, body = TorStatus)))]
pub async fn get_tor(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.tor.status().await {
        Ok(status) => Json(status).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/bridges", responses((status = 200, body = [shared_types::BridgeProfile])))]
pub async fn list_bridges(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.tor.list_bridges().await {
        Ok(bridges) => Json(bridges).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/bridges/test", responses((status = 200, body = BridgeTestResponse)))]
pub async fn test_bridge(
    State(state): State<Arc<AppState>>,
    Json(body): Json<BridgeTestRequest>,
) -> impl IntoResponse {
    match state.deps.tor.test_bridge(body.bridge_id).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Response {
    let token = extract_bearer(&headers)
        .map(|s| s.to_string())
        .or_else(|| query.get("token").cloned());
    if crate::auth::validate_token(state.deps.api_token.read().as_str(), token.as_deref()).is_err()
    {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    ws.on_upgrade(move |socket| crate::api::ws::handle_socket(socket, state))
}

#[derive(Deserialize)]
struct LogsQuery {
    limit: Option<u32>,
    level: Option<String>,
}

async fn list_logs(Query(q): Query<LogsQuery>) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(100) as usize;
    match crate::logging::global() {
        Some(logging) => Json(logging.recent(limit, q.level.as_deref())).into_response(),
        None => Json(Vec::<shared_types::LogEntry>::new()).into_response(),
    }
}

async fn download_logs() -> impl IntoResponse {
    match crate::logging::global() {
        Some(logging) => match crate::logging::zip_log_files(logging.log_dir()) {
            Ok(bytes) => {
                (StatusCode::OK, [("content-type", "application/zip")], bytes).into_response()
            }
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(Deserialize)]
struct LogLevelBody {
    level: LogLevel,
}

async fn set_log_level(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LogLevelBody>,
) -> impl IntoResponse {
    if state.deps.enterprise.is_locked("log_level") {
        return (StatusCode::FORBIDDEN, "setting locked by enterprise policy").into_response();
    }
    if let Some(logging) = crate::logging::global() {
        if let Err(e) = logging.set_level(body.level) {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    }
    if let Err(e) = state.deps.storage.settings.set_log_level(body.level).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    Json(serde_json::json!({"ok": true})).into_response()
}

#[derive(Deserialize)]
struct PerformanceQuery {
    limit: Option<u32>,
}

async fn get_performance(
    State(state): State<Arc<AppState>>,
    Query(q): Query<PerformanceQuery>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(20);
    match state.deps.performance.list_recent(limit).await {
        Ok(snapshots) => Json(serde_json::json!({
            "latest": snapshots.first(),
            "snapshots": snapshots,
        }))
        .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_diagnostics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(state.deps.diagnostics.health().await).into_response()
}

async fn export_diagnostics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.diagnostics.export_bundle().await {
        Ok(bytes) => (StatusCode::OK, [("content-type", "application/zip")], bytes).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct BackupExportQuery {
    format: Option<String>,
}

async fn export_backup(
    State(state): State<Arc<AppState>>,
    Query(q): Query<BackupExportQuery>,
) -> impl IntoResponse {
    let format = q.format.as_deref().unwrap_or("json");
    if format == "encrypted" {
        match state.deps.backup.export_encrypted().await {
            Ok(bytes) => (
                StatusCode::OK,
                [("content-type", "application/octet-stream")],
                bytes,
            )
                .into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    } else {
        match state.deps.backup.export_json().await {
            Ok((_, json)) => {
                Json(serde_json::from_str::<serde_json::Value>(&json).unwrap_or_default())
                    .into_response()
            }
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    }
}

#[derive(Deserialize)]
struct BackupImportBody {
    format: Option<String>,
    data: String,
}

async fn import_backup(
    State(state): State<Arc<AppState>>,
    Json(body): Json<BackupImportBody>,
) -> impl IntoResponse {
    let format = body.format.as_deref().unwrap_or("json");
    let result = if format == "encrypted" {
        let bytes = match base64_decode(&body.data) {
            Ok(b) => b,
            Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
        };
        state.deps.backup.import_encrypted(&bytes).await
    } else {
        state.deps.backup.import_json(&body.data).await
    };
    match result {
        Ok(()) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_enterprise_policy(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.enterprise.load().await {
        Ok(policy) => Json(policy).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn set_enterprise_policy(
    State(state): State<Arc<AppState>>,
    Json(policy): Json<EnterprisePolicy>,
) -> impl IntoResponse {
    match state.deps.enterprise.apply(&policy).await {
        Ok(()) => {
            state.deps.events.publish(
                ServiceEventInner::SecurityAudit {
                    entry: SecurityAuditEntry {
                        action: "policy_update".into(),
                        actor: None,
                        detail: None,
                        timestamp: Utc::now(),
                    },
                }
                .with_timestamp(Utc::now()),
            );
            Json(serde_json::json!({"ok": true})).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_update(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.update.info().await {
        Ok(info) => Json(info).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn check_update(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.update.check().await {
        Ok(info) => Json(info).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct MetricsQuery {
    format: Option<String>,
}

async fn get_metrics(
    State(state): State<Arc<AppState>>,
    Query(q): Query<MetricsQuery>,
) -> impl IntoResponse {
    match state.deps.metrics.snapshot().await {
        Ok(snapshot) => {
            if q.format.as_deref() == Some("prometheus") {
                (
                    StatusCode::OK,
                    [("content-type", "text/plain; version=0.0.4")],
                    crate::metrics::MetricsService::to_prometheus(&snapshot),
                )
                    .into_response()
            } else {
                Json(snapshot).into_response()
            }
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn rotate_auth_token(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match crate::auth::rotate_token() {
        Ok(token) => {
            *state.deps.api_token.write() = token.clone();
            state.deps.events.publish(
                ServiceEventInner::SecurityAudit {
                    entry: SecurityAuditEntry {
                        action: "token_rotation".into(),
                        actor: None,
                        detail: None,
                        timestamp: Utc::now(),
                    },
                }
                .with_timestamp(Utc::now()),
            );
            let _ = crate::auth::restrict_token_acl();
            Json(serde_json::json!({"ok": true, "token": token})).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct BenchmarkQuery {
    limit: Option<u32>,
}

#[utoipa::path(get, path = "/api/v1/validation", responses((status = 200, body = ValidationReport)))]
pub async fn get_validation(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.validation.latest_report().await {
        Ok(report) => Json(report).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/kernel/telemetry", responses((status = 200, body = KernelTelemetryV2)))]
pub async fn get_kernel_telemetry(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.kernel_telemetry.collect().await {
        Ok(telemetry) => Json(telemetry).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/kernel/statistics", responses((status = 200, body = KernelStatistics)))]
pub async fn get_kernel_statistics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.kernel_telemetry.statistics().await {
        Ok(stats) => Json(stats).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/benchmark", responses((status = 200)))]
pub async fn get_benchmark(
    State(state): State<Arc<AppState>>,
    Query(q): Query<BenchmarkQuery>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(10);
    match (
        state.deps.benchmark.latest().await,
        state.deps.benchmark.list_recent(limit).await,
    ) {
        (Ok(latest), Ok(recent)) => Json(serde_json::json!({
            "latest": latest,
            "recent": recent,
        }))
        .into_response(),
        (Err(e), _) | (_, Err(e)) => {
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

#[utoipa::path(get, path = "/api/v1/security/audit", responses((status = 200, body = [SecurityFinding])))]
pub async fn list_security_audit(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.security_audit.list_findings(false).await {
        Ok(findings) => Json(findings).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/security/audit/run", responses((status = 200, body = [SecurityFinding])))]
pub async fn run_security_audit(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.deps.security_audit.run_full_audit().await {
        Ok(findings) => Json(findings).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct FaultInjectBody {
    scenario: String,
}

async fn fault_inject(
    State(state): State<Arc<AppState>>,
    Json(body): Json<FaultInjectBody>,
) -> impl IntoResponse {
    let dns_enabled = state.deps.dns.settings().enabled;
    match state
        .deps
        .fault_injection
        .inject_and_verify(
            &body.scenario,
            &state.deps.vpn,
            &state.deps.transport,
            state.deps.wfp.as_ref(),
            dns_enabled,
        )
        .await
    {
        Ok(verified) => {
            Json(serde_json::json!({ "ok": true, "verified": verified })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(input)
        .map_err(|e| format!("base64 decode: {e}"))
}
