//! Hybrid WFP engine — Guardian kernel callouts plus NDIS LWF datapath.

use crate::engine::WfpEngine;
use crate::ndis::NdisEngine;
use async_trait::async_trait;
use shared_types::{AppIdentity, DriverState, Result, RuleAction, TrafficRoute, TunnelIface};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

pub struct HybridWfpEngine {
    wfp: Arc<dyn WfpEngine>,
    ndis: Arc<dyn NdisEngine>,
    sync_ndis: bool,
}

impl HybridWfpEngine {
    pub fn new(wfp: Arc<dyn WfpEngine>, ndis: Arc<dyn NdisEngine>) -> Self {
        Self {
            wfp,
            ndis,
            sync_ndis: true,
        }
    }

    pub fn wfp_only(wfp: Arc<dyn WfpEngine>) -> Self {
        Self {
            wfp,
            ndis: Arc::new(crate::ndis::StubNdisEngine::new()),
            sync_ndis: false,
        }
    }

    pub fn ndis_primary(wfp: Arc<dyn WfpEngine>, ndis: Arc<dyn NdisEngine>) -> Self {
        Self {
            wfp,
            ndis,
            sync_ndis: true,
        }
    }

    pub fn ndis_engine(&self) -> Arc<dyn NdisEngine> {
        Arc::clone(&self.ndis)
    }
}

#[async_trait]
impl WfpEngine for HybridWfpEngine {
    async fn init(&self) -> Result<()> {
        self.wfp.init().await?;
        if self.sync_ndis {
            self.ndis.init().await?;
        }
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        if self.sync_ndis {
            let _ = self.ndis.shutdown().await;
        }
        self.wfp.shutdown().await
    }

    async fn apply_route(&self, app: &AppIdentity, route: &TrafficRoute) -> Result<()> {
        self.route_connection(app, route, None).await
    }

    async fn remove_app_rule(&self, app_id: Uuid) -> Result<()> {
        if self.sync_ndis {
            let _ = self.ndis.clear_route(app_id).await;
        }
        self.wfp.remove_app_rule(app_id).await
    }

    async fn apply_kill_switch(&self, active: bool) -> Result<()> {
        self.wfp.apply_kill_switch(active).await
    }

    async fn sync_rules(&self, rules: &[(Uuid, PathBuf, RuleAction)]) -> Result<()> {
        // SegmentDeny and other ZTNA segment actions pass through to the underlying WFP engine.
        self.wfp.sync_rules(rules).await
    }

    async fn route_connection(
        &self,
        app: &AppIdentity,
        route: &TrafficRoute,
        tunnel: Option<TunnelIface>,
    ) -> Result<()> {
        self.wfp
            .route_connection(app, route, tunnel.clone())
            .await?;
        if self.sync_ndis {
            self.ndis.sync_route(app, route, tunnel).await?;
        }
        Ok(())
    }

    fn filter_ids_for_app(&self, app_id: Uuid) -> Vec<u64> {
        self.wfp.filter_ids_for_app(app_id)
    }

    fn tracked_filter_count(&self) -> u32 {
        self.wfp.tracked_filter_count()
    }

    async fn reconcile_filters(&self, known_ids: &[u64]) -> Result<u32> {
        self.wfp.reconcile_filters(known_ids).await
    }

    async fn driver_state(&self) -> DriverState {
        self.wfp.driver_state().await
    }

    fn ndis_side(&self) -> Option<Arc<dyn NdisEngine>> {
        if self.sync_ndis {
            Some(self.ndis_engine())
        } else {
            None
        }
    }
}
