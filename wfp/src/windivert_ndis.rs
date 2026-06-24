//! WinDivert-backed [`NdisEngine`] for the signed enforcement stack.

use async_trait::async_trait;
use ndis_controller::NdisTelemetrySummaryV2;
use shared_types::{AppIdentity, NdisHealth, Result, TrafficRoute, TunnelIface};
use std::sync::Arc;
use uuid::Uuid;
use windivert_engine::{WinDivertEngine, WinDivertEngineApi};

pub struct WinDivertNdisEngine {
    inner: Arc<WinDivertEngine>,
}

impl Default for WinDivertNdisEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl WinDivertNdisEngine {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(WinDivertEngine::new()),
        }
    }

    pub fn inner(&self) -> Arc<WinDivertEngine> {
        Arc::clone(&self.inner)
    }
}

#[async_trait]
impl crate::NdisEngine for WinDivertNdisEngine {
    async fn init(&self) -> Result<()> {
        self.inner.init().await
    }

    async fn shutdown(&self) -> Result<()> {
        self.inner.shutdown().await
    }

    async fn sync_route(
        &self,
        app: &AppIdentity,
        route: &TrafficRoute,
        tunnel: Option<TunnelIface>,
    ) -> Result<()> {
        self.inner.sync_route(app, route, tunnel).await
    }

    async fn clear_route(&self, app_id: Uuid) -> Result<()> {
        self.inner.clear_route(app_id).await
    }

    async fn health(&self) -> NdisHealth {
        self.inner.health().await
    }

    async fn telemetry_summary(&self) -> Result<NdisTelemetrySummaryV2> {
        let snap = self.inner.telemetry().snapshot();
        Ok(NdisTelemetrySummaryV2 {
            version: ndis_controller::NDIS_TELEMETRY_VERSION,
            classify_count: snap.packets_seen,
            redirect_count: snap.redirect_count,
            transform_count: snap.transform_count,
            cover_traffic_count: snap.cover_traffic_count,
            error_count: snap.error_count,
            ..Default::default()
        })
    }
}

pub fn create_windivert_ndis_engine() -> Arc<dyn crate::NdisEngine> {
    Arc::new(WinDivertNdisEngine::new())
}
