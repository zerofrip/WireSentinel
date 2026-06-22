use crate::engine::{RouteEnforcer, WfpEngine};
use async_trait::async_trait;
use parking_lot::RwLock;
use shared_types::{AppIdentity, Result, RuleAction, TrafficRoute, TunnelIface};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

/// Non-Windows stub for development and CI.
#[cfg_attr(windows, allow(dead_code))]
pub struct StubWfpEngine {
    kill_switch: AtomicBool,
    routes: RwLock<HashMap<Uuid, TrafficRoute>>,
}

#[cfg_attr(windows, allow(dead_code))]
impl StubWfpEngine {
    pub fn new() -> Self {
        Self {
            kill_switch: AtomicBool::new(false),
            routes: RwLock::new(HashMap::new()),
        }
    }

    fn store_route(&self, app_id: Uuid, route: TrafficRoute) {
        self.routes.write().insert(app_id, route);
    }
}

impl Default for StubWfpEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WfpEngine for StubWfpEngine {
    async fn init(&self) -> Result<()> {
        info!("stub WFP engine initialized");
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        self.routes.write().clear();
        Ok(())
    }

    async fn apply_route(&self, app: &AppIdentity, route: &TrafficRoute) -> Result<()> {
        debug!(app = %app.display_name(), ?route, "stub apply route");
        self.store_route(app.id(), route.clone());
        Ok(())
    }

    async fn remove_app_rule(&self, app_id: Uuid) -> Result<()> {
        self.routes.write().remove(&app_id);
        Ok(())
    }

    async fn apply_kill_switch(&self, active: bool) -> Result<()> {
        self.kill_switch.store(active, Ordering::SeqCst);
        info!(active, "stub kill switch");
        Ok(())
    }

    async fn sync_rules(&self, rules: &[(Uuid, PathBuf, RuleAction)]) -> Result<()> {
        let mut map = self.routes.write();
        map.clear();
        for (id, _path, action) in rules {
            let route = match action {
                RuleAction::Block => TrafficRoute::Blocked,
                RuleAction::Allow | RuleAction::RouteDirect | RuleAction::LogOnly => {
                    TrafficRoute::Direct
                }
                RuleAction::RouteViaVpn(pid) => TrafficRoute::WireGuard(*pid),
                RuleAction::RouteViaTailnet(pid) => TrafficRoute::Tailnet(*pid),
                RuleAction::RouteViaTor(pid) => TrafficRoute::Tor(*pid),
                RuleAction::RouteViaProxy(pid) => TrafficRoute::Proxy(*pid),
                RuleAction::RouteViaProxyChain(pid) => TrafficRoute::ProxyChain(*pid),
                RuleAction::RouteViaChain(pid) => TrafficRoute::Chain(*pid),
                RuleAction::RouteViaAnonymous(route) => TrafficRoute::Anonymous(route.clone()),
                RuleAction::RouteViaMixnet(id) => {
                    TrafficRoute::Anonymous(shared_types::AnonymousRoute::FutureMixnet(*id))
                }
                RuleAction::SegmentDeny(_) => TrafficRoute::Blocked,
            };
            map.insert(*id, route);
        }
        Ok(())
    }

    async fn allow_connection(&self, app: &AppIdentity, route: &TrafficRoute) -> Result<()> {
        debug!(app = %app.display_name(), ?route, "stub allow connection");
        self.store_route(app.id(), route.clone());
        Ok(())
    }

    async fn block_connection(&self, app: &AppIdentity) -> Result<()> {
        debug!(app = %app.display_name(), "stub block connection");
        self.store_route(app.id(), TrafficRoute::Blocked);
        Ok(())
    }

    async fn route_connection(
        &self,
        app: &AppIdentity,
        route: &TrafficRoute,
        tunnel: Option<TunnelIface>,
    ) -> Result<()> {
        debug!(app = %app.display_name(), ?route, ?tunnel, "stub route connection");
        self.store_route(app.id(), route.clone());
        Ok(())
    }
}

#[async_trait]
impl RouteEnforcer for StubWfpEngine {
    async fn apply_route(&self, app: &AppIdentity, route: &TrafficRoute) -> Result<()> {
        WfpEngine::apply_route(self, app, route).await
    }

    async fn apply_kill_switch(&self, active: bool) -> Result<()> {
        WfpEngine::apply_kill_switch(self, active).await
    }
}

/// Non-Windows stub for `KernelCalloutEngine`.
#[cfg_attr(windows, allow(dead_code))]
pub struct KernelCalloutEngineStub;

#[cfg_attr(windows, allow(dead_code))]
impl KernelCalloutEngineStub {
    pub fn new() -> Self {
        Self
    }

    pub fn set_listen_ports(&self, _ports: Arc<proxy_engine::ProxyListenPort>) {}
}

impl Default for KernelCalloutEngineStub {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WfpEngine for KernelCalloutEngineStub {
    async fn init(&self) -> Result<()> {
        Err(shared_types::WireSentinelError::Wfp(
            "kernel callout requires Windows".into(),
        ))
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn apply_route(&self, _: &AppIdentity, _: &TrafficRoute) -> Result<()> {
        Err(shared_types::WireSentinelError::Wfp(
            "kernel callout requires Windows".into(),
        ))
    }

    async fn remove_app_rule(&self, _: Uuid) -> Result<()> {
        Ok(())
    }

    async fn apply_kill_switch(&self, _: bool) -> Result<()> {
        Err(shared_types::WireSentinelError::Wfp(
            "kernel callout requires Windows".into(),
        ))
    }

    async fn sync_rules(&self, _: &[(Uuid, PathBuf, RuleAction)]) -> Result<()> {
        Err(shared_types::WireSentinelError::Wfp(
            "kernel callout requires Windows".into(),
        ))
    }
}

#[async_trait]
impl RouteEnforcer for KernelCalloutEngineStub {
    async fn apply_route(&self, app: &AppIdentity, route: &TrafficRoute) -> Result<()> {
        WfpEngine::apply_route(self, app, route).await
    }

    async fn apply_kill_switch(&self, active: bool) -> Result<()> {
        WfpEngine::apply_kill_switch(self, active).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn block_connection_records_blocked_route() {
        let wfp = StubWfpEngine::new();
        let record = shared_types::AppRecord::new(std::path::PathBuf::from("C:\\app.exe"));
        let app = AppIdentity::new(1, record);
        wfp.block_connection(&app).await.unwrap();
        assert_eq!(
            wfp.routes.read().get(&app.id()),
            Some(&TrafficRoute::Blocked)
        );
    }

    #[tokio::test]
    async fn allow_connection_records_direct_route() {
        let wfp = StubWfpEngine::new();
        let record = shared_types::AppRecord::new(std::path::PathBuf::from("C:\\app.exe"));
        let app = AppIdentity::new(1, record);
        wfp.allow_connection(&app, &TrafficRoute::Direct)
            .await
            .unwrap();
        assert_eq!(
            wfp.routes.read().get(&app.id()),
            Some(&TrafficRoute::Direct)
        );
    }
}
