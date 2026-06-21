use async_trait::async_trait;
use shared_types::{AppIdentity, Result, RuleAction, TrafficRoute, TunnelIface, WireSentinelError};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum WfpEventKind {
    RouteApplied { app_id: Uuid, route: TrafficRoute },
    KillSwitchChanged { active: bool },
    Error { message: String },
}

#[derive(Debug, Clone)]
pub struct WfpEvent {
    pub kind: WfpEventKind,
}

/// Enforces routing decisions at the WFP layer.
#[async_trait]
pub trait RouteEnforcer: Send + Sync {
    async fn apply_route(&self, app: &AppIdentity, route: &TrafficRoute) -> Result<()>;

    async fn apply_kill_switch(&self, active: bool) -> Result<()>;

    /// Phase 2 placeholder — `FWPM_LAYER_ALE_AUTH_CONNECT` (Phase 3 kernel callout).
    async fn apply_ale_auth_connect(
        &self,
        _app: &AppIdentity,
        _route: &TrafficRoute,
    ) -> Result<()> {
        Err(WireSentinelError::Wfp(
            "ALE_AUTH_CONNECT not implemented (Phase 3)".into(),
        ))
    }

    /// Phase 2 placeholder — `FWPM_LAYER_ALE_FLOW_ESTABLISHED` (Phase 3 kernel callout).
    async fn apply_ale_flow_established(
        &self,
        _app: &AppIdentity,
        _route: &TrafficRoute,
    ) -> Result<()> {
        Err(WireSentinelError::Wfp(
            "ALE_FLOW_ESTABLISHED not implemented (Phase 3)".into(),
        ))
    }
}

/// WFP engine trait — implemented by userspace (Phase 1) and kernel callout (Phase 2).
#[async_trait]
pub trait WfpEngine: Send + Sync {
    async fn init(&self) -> Result<()>;
    async fn shutdown(&self) -> Result<()>;
    async fn apply_route(&self, app: &AppIdentity, route: &TrafficRoute) -> Result<()>;
    async fn remove_app_rule(&self, app_id: Uuid) -> Result<()>;
    async fn apply_kill_switch(&self, active: bool) -> Result<()>;
    async fn sync_rules(&self, rules: &[(Uuid, PathBuf, RuleAction)]) -> Result<()>;

    async fn allow_connection(&self, app: &AppIdentity, route: &TrafficRoute) -> Result<()> {
        self.route_connection(app, route, None).await
    }

    async fn block_connection(&self, app: &AppIdentity) -> Result<()> {
        self.route_connection(app, &TrafficRoute::Blocked, None)
            .await
    }

    async fn route_connection(
        &self,
        app: &AppIdentity,
        route: &TrafficRoute,
        tunnel: Option<TunnelIface>,
    ) -> Result<()> {
        let _ = tunnel;
        self.apply_route(app, route).await
    }

    /// Filter IDs last applied for an app (userspace engine).
    fn filter_ids_for_app(&self, _app_id: Uuid) -> Vec<u64> {
        Vec::new()
    }

    /// Total tracked filter count.
    fn tracked_filter_count(&self) -> u32 {
        0
    }

    /// Remove orphan filters not in known_ids. Returns removed count.
    async fn reconcile_filters(&self, _known_ids: &[u64]) -> Result<u32> {
        Ok(0)
    }

    /// Current driver state snapshot.
    async fn driver_state(&self) -> shared_types::DriverState {
        shared_types::DriverState {
            engine: "unknown".into(),
            state: "unknown".into(),
            filter_count: self.tracked_filter_count(),
            provider_registered: false,
            message: None,
        }
    }

    /// NDIS sidecar when guardian mode is `ndis` or `hybrid`.
    fn ndis_side(&self) -> Option<std::sync::Arc<dyn crate::ndis::NdisEngine>> {
        None
    }
}
