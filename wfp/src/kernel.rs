//! Kernel Guardian driver client — implements `WfpEngine` via IOCTL.

use crate::engine::{RouteEnforcer, WfpEngine};
use async_trait::async_trait;
use guardian_controller::{
    uuid_to_bytes, wide_path_bytes, GuardianAppPolicyV1, GuardianClient, GuardianDriverStateV1,
    GuardianKillSwitchV1, GuardianLifecycleState, GuardianPolicyAction, GuardianPolicyMatchKind,
    GuardianRouteAssignmentV1, GuardianRouteKind, GUARDIAN_APP_POLICY_VERSION,
    GUARDIAN_ROUTE_ASSIGNMENT_VERSION,
};
use parking_lot::{Mutex, RwLock};
use proxy_engine::ProxyListenPort;
use shared_types::{
    AppIdentity, DriverState, Result, RuleAction, TrafficRoute, TunnelIface, WireSentinelError,
};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

/// WFP engine backed by Guardian.sys.
pub struct KernelCalloutEngine {
    client: Mutex<Option<GuardianClient>>,
    listen_ports: RwLock<Option<Arc<ProxyListenPort>>>,
}

impl Default for KernelCalloutEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl KernelCalloutEngine {
    pub fn new() -> Self {
        Self {
            client: Mutex::new(None),
            listen_ports: RwLock::new(None),
        }
    }

    pub fn set_listen_ports(&self, ports: Arc<ProxyListenPort>) {
        *self.listen_ports.write() = Some(ports);
    }

    fn socks_port_for_route(&self, route: &TrafficRoute, tunnel: Option<&TunnelIface>) -> u16 {
        if let Some(port) = tunnel.and_then(|t| t.socks_port) {
            return port;
        }
        let Some(id) = route.profile_id() else {
            return 0;
        };
        self.listen_ports
            .read()
            .as_ref()
            .and_then(|ports| ports.get(id))
            .unwrap_or(0)
    }

    fn with_client<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&GuardianClient) -> Result<T>,
    {
        let guard = self.client.lock();
        let client = guard
            .as_ref()
            .ok_or_else(|| WireSentinelError::Wfp("Guardian driver not initialized".into()))?;
        f(client)
    }
}

fn map_rule_action(action: &RuleAction) -> u32 {
    match action {
        RuleAction::Block => GuardianPolicyAction::Block as u32,
        RuleAction::RouteViaVpn(_)
        | RuleAction::RouteViaTailnet(_)
        | RuleAction::RouteViaTor(_)
        | RuleAction::RouteViaProxy(_)
        | RuleAction::RouteViaProxyChain(_)
        | RuleAction::RouteViaChain(_)
        | RuleAction::RouteViaAnonymous(_)
        | RuleAction::RouteViaMixnet(_) => GuardianPolicyAction::Route as u32,
        RuleAction::LogOnly => GuardianPolicyAction::Observe as u32,
        _ => GuardianPolicyAction::Allow as u32,
    }
}

fn map_traffic_route(route: &TrafficRoute) -> (u32, u64) {
    match route {
        TrafficRoute::Blocked => (GuardianRouteKind::Blocked as u32, 0),
        TrafficRoute::Direct => (GuardianRouteKind::Direct as u32, 0),
        TrafficRoute::WireGuard(id) | TrafficRoute::AmneziaWG(id) => {
            (GuardianRouteKind::Vpn as u32, id.as_u128() as u64)
        }
        TrafficRoute::Tailnet(id) => (GuardianRouteKind::Tailnet as u32, id.as_u128() as u64),
        TrafficRoute::Tor(id) => (GuardianRouteKind::Tor as u32, id.as_u128() as u64),
        TrafficRoute::Anonymous(_) => {
            let id = route.profile_id().unwrap_or_else(Uuid::nil);
            (GuardianRouteKind::Anonymous as u32, id.as_u128() as u64)
        }
        TrafficRoute::Proxy(id) => (GuardianRouteKind::Proxy as u32, id.as_u128() as u64),
        TrafficRoute::ProxyChain(id) => (GuardianRouteKind::Chain as u32, id.as_u128() as u64),
        TrafficRoute::Chain(id) => (GuardianRouteKind::Chain as u32, id.as_u128() as u64),
        TrafficRoute::Katzenpost(id) => (GuardianRouteKind::Katzenpost as u32, id.as_u128() as u64),
        TrafficRoute::Loopix(id) => (GuardianRouteKind::Loopix as u32, id.as_u128() as u64),
        TrafficRoute::FederatedMixnet(id) => (
            GuardianRouteKind::FederatedMixnet as u32,
            id.as_u128() as u64,
        ),
    }
}

fn build_app_policy(
    app: &AppIdentity,
    route: &TrafficRoute,
    tunnel: Option<&TunnelIface>,
) -> GuardianAppPolicyV1 {
    let exe = app.exe_path().to_string_lossy().into_owned();
    let (exe_path, len) = wide_path_bytes(&exe);
    let (_route_kind, profile_id) = map_traffic_route(route);
    let action = match route {
        TrafficRoute::Blocked => GuardianPolicyAction::Block as u32,
        _ => GuardianPolicyAction::Route as u32,
    };

    GuardianAppPolicyV1 {
        version: GUARDIAN_APP_POLICY_VERSION,
        app_id: uuid_to_bytes(app.id()),
        action,
        match_kind: GuardianPolicyMatchKind::ProcessPath as u32,
        exe_path_length_chars: len,
        exe_path,
        exe_hash_sha256: [0; 32],
        sid_length_bytes: 0,
        sid_bytes: [0; 68],
        package_family_name: [0; 128],
        profile_id,
        interface_luid: tunnel.map(|t| t.luid).unwrap_or(0),
    }
}

fn driver_state_from_kernel(state: &GuardianDriverStateV1) -> DriverState {
    let lifecycle = match state.lifecycle_state {
        x if x == GuardianLifecycleState::Running as u32 => "running",
        x if x == GuardianLifecycleState::Recovering as u32 => "recovering",
        x if x == GuardianLifecycleState::Failed as u32 => "failed",
        _ => "stopped",
    };
    DriverState {
        engine: "kernel".into(),
        state: lifecycle.into(),
        filter_count: state.filter_count,
        provider_registered: state.callouts_registered > 0,
        message: None,
    }
}

#[async_trait]
impl WfpEngine for KernelCalloutEngine {
    async fn init(&self) -> Result<()> {
        let client = GuardianClient::connect()
            .map_err(|e| WireSentinelError::Wfp(format!("Guardian driver connect failed: {e}")))?;
        let _ = client
            .reconcile()
            .map_err(|e| WireSentinelError::Wfp(format!("Guardian reconcile failed: {e}")))?;
        *self.client.lock() = Some(client);
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        *self.client.lock() = None;
        Ok(())
    }

    async fn apply_route(&self, app: &AppIdentity, route: &TrafficRoute) -> Result<()> {
        self.route_connection(app, route, None).await
    }

    async fn remove_app_rule(&self, app_id: Uuid) -> Result<()> {
        self.with_client(|c| {
            c.clear_route(app_id)
                .map_err(|e| WireSentinelError::Wfp(format!("clear route: {e}")))
        })
    }

    async fn apply_kill_switch(&self, active: bool) -> Result<()> {
        self.with_client(|c| {
            let cfg = if active {
                GuardianKillSwitchV1::block_all()
            } else {
                GuardianKillSwitchV1::restore()
            };
            c.set_kill_switch(&cfg)
                .map_err(|e| WireSentinelError::Wfp(format!("kill switch: {e}")))
        })
    }

    async fn sync_rules(&self, rules: &[(Uuid, PathBuf, RuleAction)]) -> Result<()> {
        for (app_id, path, action) in rules {
            let exe = path.to_string_lossy();
            let (exe_path, len) = wide_path_bytes(&exe);
            let policy = GuardianAppPolicyV1 {
                version: GUARDIAN_APP_POLICY_VERSION,
                app_id: uuid_to_bytes(*app_id),
                action: map_rule_action(action),
                match_kind: GuardianPolicyMatchKind::ProcessPath as u32,
                exe_path_length_chars: len,
                exe_path,
                exe_hash_sha256: [0; 32],
                sid_length_bytes: 0,
                sid_bytes: [0; 68],
                package_family_name: [0; 128],
                profile_id: 0,
                interface_luid: 0,
            };
            self.with_client(|c| {
                c.set_app_policy(&policy)
                    .map_err(|e| WireSentinelError::Wfp(format!("sync policy: {e}")))
            })?;
        }
        Ok(())
    }

    async fn route_connection(
        &self,
        app: &AppIdentity,
        route: &TrafficRoute,
        tunnel: Option<TunnelIface>,
    ) -> Result<()> {
        let policy = build_app_policy(app, route, tunnel.as_ref());
        let socks_port = self.socks_port_for_route(route, tunnel.as_ref());
        self.with_client(|c| {
            c.set_app_policy(&policy)
                .map_err(|e| WireSentinelError::Wfp(format!("set policy: {e}")))?;

            let assignment = match route {
                TrafficRoute::Proxy(id) => {
                    GuardianRouteAssignmentV1::new_proxy(app.id(), id.as_u128() as u64, socks_port)
                }
                TrafficRoute::ProxyChain(id) => GuardianRouteAssignmentV1::new_proxy_chain(
                    app.id(),
                    id.as_u128() as u64,
                    socks_port,
                ),
                TrafficRoute::Anonymous(_) => {
                    let profile_id = route
                        .profile_id()
                        .map(|id| id.as_u128() as u64)
                        .unwrap_or(0);
                    GuardianRouteAssignmentV1 {
                        version: GUARDIAN_ROUTE_ASSIGNMENT_VERSION,
                        app_id: uuid_to_bytes(app.id()),
                        route_kind: GuardianRouteKind::Anonymous as u32,
                        profile_id,
                        interface_luid: tunnel.map(|t| t.luid).unwrap_or(0),
                        socks_port,
                        reserved: [0; 6],
                    }
                }
                _ => {
                    let (route_kind, profile_id) = map_traffic_route(route);
                    GuardianRouteAssignmentV1 {
                        version: GUARDIAN_ROUTE_ASSIGNMENT_VERSION,
                        app_id: uuid_to_bytes(app.id()),
                        route_kind,
                        profile_id,
                        interface_luid: tunnel.map(|t| t.luid).unwrap_or(0),
                        socks_port,
                        reserved: [0; 6],
                    }
                }
            };
            c.set_route(&assignment)
                .map_err(|e| WireSentinelError::Wfp(format!("set route: {e}")))
        })
    }

    async fn driver_state(&self) -> DriverState {
        self.with_client(|c| {
            c.driver_state()
                .map(|s| driver_state_from_kernel(&s))
                .map_err(|e| WireSentinelError::Wfp(format!("driver state: {e}")))
        })
        .unwrap_or(DriverState {
            engine: "kernel".into(),
            state: "unavailable".into(),
            filter_count: 0,
            provider_registered: false,
            message: Some("Guardian not connected".into()),
        })
    }
}

#[async_trait]
impl RouteEnforcer for KernelCalloutEngine {
    async fn apply_route(&self, app: &AppIdentity, route: &TrafficRoute) -> Result<()> {
        WfpEngine::apply_route(self, app, route).await
    }

    async fn apply_kill_switch(&self, active: bool) -> Result<()> {
        WfpEngine::apply_kill_switch(self, active).await
    }
}
