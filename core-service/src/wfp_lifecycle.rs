//! WFP lifecycle: orphan filter reconcile and driver state events.

use chrono::Utc;
use event_bus::EventBus;
use shared_types::{Result, ServiceEventInner};
use std::sync::Arc;
use storage::Storage;
use tracing::info;
use wfp::WfpEngine;

pub struct WfpLifecycleManager;

impl WfpLifecycleManager {
    pub async fn reconcile(
        wfp: Arc<dyn WfpEngine>,
        storage: Arc<Storage>,
        events: EventBus,
    ) -> Result<u32> {
        let records = storage.wfp_filter_state.list_all().await?;
        let known_ids: Vec<u64> = records
            .iter()
            .map(|r| r.filter_id)
            .filter(|&id| id != 0)
            .collect();
        let removed = wfp.reconcile_filters(&known_ids).await?;
        if removed > 0 {
            info!(removed, "reconciled orphan WFP filters");
        }
        let state = wfp.driver_state().await;
        events.publish(
            ServiceEventInner::DriverStateChanged { state: state.clone() }.with_timestamp(Utc::now()),
        );
        if state.engine == "kernel" && state.state == "running" {
            events.publish(
                ServiceEventInner::DriverRecovered {
                    recovery_generation: 0,
                }
                .with_timestamp(Utc::now()),
            );
        }
        Ok(removed)
    }
}
