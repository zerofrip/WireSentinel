//! NDIS LWF engine client — complements Guardian WFP callouts.

use async_trait::async_trait;
use ndis_controller::{
    NdisClient, NdisDriverStateV2, NdisLifecycleState, NdisRouteAssignmentV2,
    NdisTelemetrySummaryV2,
};
use parking_lot::Mutex;
use shared_types::{AppIdentity, NdisHealth, Result, TrafficRoute, TunnelIface, WireSentinelError};
use std::sync::Arc;
use uuid::Uuid;

/// NDIS device handle is opened per service thread.
struct SendNdisClient(NdisClient);
unsafe impl Send for SendNdisClient {}
unsafe impl Sync for SendNdisClient {}

impl SendNdisClient {
    fn inner(&self) -> &NdisClient {
        &self.0
    }
}

#[async_trait]
pub trait NdisEngine: Send + Sync {
    async fn init(&self) -> Result<()>;
    async fn shutdown(&self) -> Result<()>;
    async fn sync_route(
        &self,
        app: &AppIdentity,
        route: &TrafficRoute,
        tunnel: Option<TunnelIface>,
    ) -> Result<()>;
    async fn clear_route(&self, app_id: Uuid) -> Result<()>;
    async fn health(&self) -> NdisHealth;
    async fn telemetry_summary(&self) -> Result<NdisTelemetrySummaryV2>;
}

fn map_lifecycle(state: u32) -> &'static str {
    match state {
        x if x == NdisLifecycleState::Running as u32 => "running",
        x if x == NdisLifecycleState::Starting as u32 => "starting",
        x if x == NdisLifecycleState::Paused as u32 => "paused",
        x if x == NdisLifecycleState::Failed as u32 => "failed",
        _ => "stopped",
    }
}

fn health_from_state(state: &NdisDriverStateV2, message: Option<String>) -> NdisHealth {
    let lifecycle = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(state.lifecycle_state)) };
    let filter_attached =
        unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(state.filter_attached)) } != 0;
    let active_route_count =
        unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(state.active_route_count)) };
    let active_redirect_count =
        unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(state.active_redirect_count)) };
    NdisHealth {
        available: lifecycle == NdisLifecycleState::Running as u32,
        state: map_lifecycle(lifecycle).into(),
        filter_attached,
        active_route_count,
        active_redirect_count,
        classify_count: 0,
        error_count: 0,
        message,
        checked_at: chrono::Utc::now(),
    }
}

fn route_assignment(
    app: &AppIdentity,
    route: &TrafficRoute,
    tunnel: Option<&TunnelIface>,
) -> NdisRouteAssignmentV2 {
    let flow_id = app.id();
    let app_id = app.id();
    match route {
        TrafficRoute::Proxy(id) => {
            let socks = tunnel.and_then(|t| t.socks_port).unwrap_or(0);
            NdisRouteAssignmentV2::new_proxy(flow_id, app_id, id.as_u128() as u64, socks)
        }
        TrafficRoute::ProxyChain(id) => {
            let socks = tunnel.and_then(|t| t.socks_port).unwrap_or(0);
            NdisRouteAssignmentV2::new_proxy(flow_id, app_id, id.as_u128() as u64, socks)
        }
        TrafficRoute::WireGuard(id) | TrafficRoute::AmneziaWG(id) => {
            NdisRouteAssignmentV2::new_vpn(
                flow_id,
                app_id,
                id.as_u128() as u64,
                tunnel.map(|t| t.luid).unwrap_or(0),
            )
        }
        TrafficRoute::Blocked => NdisRouteAssignmentV2 {
            version: ndis_controller::NDIS_ROUTE_ASSIGNMENT_VERSION,
            flow_id: ndis_controller::uuid_to_bytes(flow_id),
            app_id: ndis_controller::uuid_to_bytes(app_id),
            route_kind: ndis_controller::NdisRouteKind::Blocked as u32,
            profile_id: 0,
            interface_luid: 0,
            socks_port: 0,
            protocol: 0,
            reserved: [0; 4],
        },
        _ => {
            let profile_id = route
                .profile_id()
                .map(|id| id.as_u128() as u64)
                .unwrap_or(0);
            NdisRouteAssignmentV2::new_vpn(
                flow_id,
                app_id,
                profile_id,
                tunnel.map(|t| t.luid).unwrap_or(0),
            )
        }
    }
}

/// NDIS driver client (Windows) with in-memory fallback for CI.
pub struct NdisCalloutEngine {
    client: Mutex<Option<SendNdisClient>>,
    stub_routes: Mutex<std::collections::HashMap<Uuid, TrafficRoute>>,
}

impl Default for NdisCalloutEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl NdisCalloutEngine {
    pub fn new() -> Self {
        Self {
            client: Mutex::new(None),
            stub_routes: Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait]
impl NdisEngine for NdisCalloutEngine {
    async fn init(&self) -> Result<()> {
        match NdisClient::connect() {
            Ok(client) => {
                *self.client.lock() = Some(SendNdisClient(client));
                Ok(())
            }
            Err(e) => {
                tracing::warn!(error = %e, "NDIS driver unavailable — using stub routing state");
                Ok(())
            }
        }
    }

    async fn shutdown(&self) -> Result<()> {
        *self.client.lock() = None;
        self.stub_routes.lock().clear();
        Ok(())
    }

    async fn sync_route(
        &self,
        app: &AppIdentity,
        route: &TrafficRoute,
        tunnel: Option<TunnelIface>,
    ) -> Result<()> {
        self.stub_routes.lock().insert(app.id(), route.clone());
        if let Some(client) = self.client.lock().as_ref() {
            let assignment = route_assignment(app, route, tunnel.as_ref());
            client
                .inner()
                .set_route(&assignment)
                .map_err(|e| WireSentinelError::Wfp(format!("ndis set route: {e}")))?;
        }
        Ok(())
    }

    async fn clear_route(&self, app_id: Uuid) -> Result<()> {
        self.stub_routes.lock().remove(&app_id);
        if let Some(client) = self.client.lock().as_ref() {
            client
                .inner()
                .clear_route(app_id)
                .map_err(|e| WireSentinelError::Wfp(format!("ndis clear route: {e}")))?;
        }
        Ok(())
    }

    async fn health(&self) -> NdisHealth {
        if let Some(client) = self.client.lock().as_ref() {
            match client.inner().driver_state() {
                Ok(state) => return health_from_state(&state, None),
                Err(e) => {
                    return NdisHealth {
                        available: false,
                        state: "error".into(),
                        filter_attached: false,
                        active_route_count: 0,
                        active_redirect_count: 0,
                        classify_count: 0,
                        error_count: 1,
                        message: Some(e.to_string()),
                        checked_at: chrono::Utc::now(),
                    };
                }
            }
        }
        let routes = self.stub_routes.lock().len() as u32;
        NdisHealth {
            available: false,
            state: "stub".into(),
            filter_attached: false,
            active_route_count: routes,
            active_redirect_count: 0,
            classify_count: 0,
            error_count: 0,
            message: Some("NDIS driver not connected".into()),
            checked_at: chrono::Utc::now(),
        }
    }

    async fn telemetry_summary(&self) -> Result<NdisTelemetrySummaryV2> {
        if let Some(client) = self.client.lock().as_ref() {
            return client
                .inner()
                .telemetry_summary()
                .map_err(|e| WireSentinelError::Wfp(format!("ndis telemetry: {e}")));
        }
        Ok(NdisTelemetrySummaryV2 {
            version: ndis_controller::NDIS_TELEMETRY_VERSION,
            ..Default::default()
        })
    }
}

/// Non-Windows / offline stub for CI.
pub struct StubNdisEngine {
    routes: parking_lot::RwLock<std::collections::HashMap<Uuid, TrafficRoute>>,
}

impl Default for StubNdisEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl StubNdisEngine {
    pub fn new() -> Self {
        Self {
            routes: parking_lot::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait]
impl NdisEngine for StubNdisEngine {
    async fn init(&self) -> Result<()> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        self.routes.write().clear();
        Ok(())
    }

    async fn sync_route(
        &self,
        app: &AppIdentity,
        route: &TrafficRoute,
        _tunnel: Option<TunnelIface>,
    ) -> Result<()> {
        self.routes.write().insert(app.id(), route.clone());
        Ok(())
    }

    async fn clear_route(&self, app_id: Uuid) -> Result<()> {
        self.routes.write().remove(&app_id);
        Ok(())
    }

    async fn health(&self) -> NdisHealth {
        NdisHealth {
            available: false,
            state: "stub".into(),
            filter_attached: false,
            active_route_count: self.routes.read().len() as u32,
            active_redirect_count: 0,
            classify_count: 0,
            error_count: 0,
            message: Some("NDIS stub engine".into()),
            checked_at: chrono::Utc::now(),
        }
    }

    async fn telemetry_summary(&self) -> Result<NdisTelemetrySummaryV2> {
        Ok(NdisTelemetrySummaryV2 {
            version: ndis_controller::NDIS_TELEMETRY_VERSION,
            ..Default::default()
        })
    }
}

/// Probe whether the NDIS LWF driver is loaded and reachable.
pub fn ndis_driver_available() -> std::result::Result<String, String> {
    match NdisClient::connect() {
        Ok(client) => {
            let state = client.driver_state().map_err(|e| e.to_string())?;
            let major =
                unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(state.version_major)) };
            let minor =
                unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(state.version_minor)) };
            let patch =
                unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(state.version_patch)) };
            let lifecycle =
                unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(state.lifecycle_state)) };
            Ok(format!(
                "NDIS driver v{major}.{minor}.{patch} state={lifecycle}"
            ))
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn create_ndis_engine() -> Arc<dyn NdisEngine> {
    Arc::new(NdisCalloutEngine::new())
}
