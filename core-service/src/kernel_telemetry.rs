//! Bridges Guardian WFP and NDIS telemetry into shared Phase 12 DTOs.

use chrono::Utc;
use shared_types::{
    GuardianMode, KernelStatistics, KernelTelemetryV2, Result, WireSentinelError,
};
use std::sync::Arc;
use storage::Storage;
use uuid::Uuid;
use wfp::{NdisEngine, WfpEngine};

use crate::guardian_hybrid::GuardianHybridService;

pub struct KernelTelemetryService {
    storage: Arc<Storage>,
    wfp: Arc<dyn WfpEngine>,
    ndis: Arc<dyn NdisEngine>,
    guardian_hybrid: Arc<GuardianHybridService>,
}

impl KernelTelemetryService {
    pub fn new(
        storage: Arc<Storage>,
        wfp: Arc<dyn WfpEngine>,
        ndis: Arc<dyn NdisEngine>,
        guardian_hybrid: Arc<GuardianHybridService>,
    ) -> Self {
        Self {
            storage,
            wfp,
            ndis,
            guardian_hybrid,
        }
    }

    pub async fn collect(&self) -> Result<KernelTelemetryV2> {
        let mode = self.guardian_hybrid.guardian_mode().await;
        let guardian = if mode == GuardianMode::Wfp || mode == GuardianMode::Hybrid {
            Some(self.wfp.driver_state().await)
        } else {
            None
        };
        let ndis = if mode.uses_ndis() {
            Some(self.ndis.health().await)
        } else {
            None
        };
        let summary = self
            .ndis
            .telemetry_summary()
            .await
            .unwrap_or_default();
        let classify_count = unsafe {
            std::ptr::read_unaligned(std::ptr::addr_of!(summary.classify_count))
        };
        let redirect_count = unsafe {
            std::ptr::read_unaligned(std::ptr::addr_of!(summary.redirect_count))
        };
        let transform_count = unsafe {
            std::ptr::read_unaligned(std::ptr::addr_of!(summary.transform_count))
        };
        let cover_traffic_count = unsafe {
            std::ptr::read_unaligned(std::ptr::addr_of!(summary.cover_traffic_count))
        };
        let error_count = unsafe {
            std::ptr::read_unaligned(std::ptr::addr_of!(summary.error_count))
        };
        let dropped_count = unsafe {
            std::ptr::read_unaligned(std::ptr::addr_of!(summary.dropped_count))
        };

        Ok(KernelTelemetryV2 {
            guardian_mode: mode,
            guardian,
            ndis,
            classify_count,
            redirect_count,
            transform_count,
            cover_traffic_count,
            error_count,
            dropped_count,
            captured_at: Utc::now(),
        })
    }

    pub async fn statistics(&self) -> Result<KernelStatistics> {
        let telemetry = self.collect().await?;
        let wfp_filter_count = telemetry
            .guardian
            .as_ref()
            .map(|g| g.filter_count)
            .unwrap_or(0);
        let ndis_route_count = telemetry
            .ndis
            .as_ref()
            .map(|n| n.active_route_count)
            .unwrap_or(0);
        let ndis_redirect_count = telemetry
            .ndis
            .as_ref()
            .map(|n| n.active_redirect_count)
            .unwrap_or(0);
        let telemetry_events = telemetry.classify_count
            + telemetry.redirect_count
            + telemetry.transform_count
            + telemetry.cover_traffic_count;
        Ok(KernelStatistics {
            guardian_mode: telemetry.guardian_mode,
            wfp_filter_count,
            ndis_route_count,
            ndis_redirect_count,
            telemetry_events,
            security_violations: telemetry.error_count,
            captured_at: Utc::now(),
        })
    }

    pub async fn persist_snapshot(&self) -> Result<()> {
        let telemetry = self.collect().await?;
        let statistics = self.statistics().await?;
        let telemetry_json =
            serde_json::to_string(&telemetry).map_err(WireSentinelError::Serde)?;
        let statistics_json =
            serde_json::to_string(&statistics).map_err(WireSentinelError::Serde)?;
        sqlx::query(
            "INSERT INTO kernel_telemetry_snapshots (id, guardian_mode, telemetry_json, statistics_json, captured_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(telemetry.guardian_mode.as_str())
        .bind(telemetry_json)
        .bind(statistics_json)
        .bind(telemetry.captured_at.to_rfc3339())
        .execute(&self.storage.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }
}
