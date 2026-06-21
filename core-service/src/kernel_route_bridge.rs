//! Sync per-app routes to the NDIS LWF driver when guardian mode uses NDIS.

use shared_types::{AppIdentity, GuardianMode, Result, TrafficRoute, TunnelIface};
use std::sync::Arc;
use wfp::NdisEngine;

pub struct KernelRouteBridge {
    ndis: Arc<dyn NdisEngine>,
    mode: GuardianMode,
}

impl KernelRouteBridge {
    pub fn new(ndis: Arc<dyn NdisEngine>, mode: GuardianMode) -> Self {
        Self { ndis, mode }
    }

    pub fn mode(&self) -> GuardianMode {
        self.mode
    }

    pub fn is_active(&self) -> bool {
        self.mode.uses_ndis()
    }

    pub async fn sync_enforce(
        &self,
        app: &AppIdentity,
        route: &TrafficRoute,
        tunnel: Option<TunnelIface>,
    ) -> Result<()> {
        if !self.is_active() {
            return Ok(());
        }
        match route {
            TrafficRoute::Blocked => self.ndis.clear_route(app.id()).await,
            _ => self.ndis.sync_route(app, route, tunnel).await,
        }
    }
}
