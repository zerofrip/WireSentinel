use core_service::guardian_hybrid::GuardianHybridService;
use core_service::kernel_telemetry::KernelTelemetryService;
use event_bus::EventBus;
use shared_types::GuardianMode;
use std::sync::Arc;
use storage::{init_pool_in_memory, Storage};
use wfp::{StubNdisEngine, UserspaceWfpEngine, WfpEngine};

fn test_hybrid(
    storage: Arc<Storage>,
    ndis: Arc<dyn wfp::NdisEngine>,
) -> Arc<GuardianHybridService> {
    Arc::new(GuardianHybridService::new(storage, ndis, EventBus::new()))
}

#[tokio::test]
async fn kernel_telemetry_collects_wfp_mode_snapshot() {
    let pool = init_pool_in_memory().await.unwrap();
    let storage = Arc::new(Storage::new(pool));
    let wfp = Arc::new(UserspaceWfpEngine::new()) as Arc<dyn WfpEngine>;
    let ndis = Arc::new(StubNdisEngine::new()) as Arc<dyn wfp::NdisEngine>;
    let hybrid = test_hybrid(Arc::clone(&storage), Arc::clone(&ndis));
    let telemetry = KernelTelemetryService::new(storage, wfp, ndis, hybrid);

    let snapshot = telemetry.collect().await.unwrap();
    assert_eq!(snapshot.guardian_mode, GuardianMode::Wfp);
    assert!(snapshot.guardian.is_some());
}

#[tokio::test]
async fn kernel_telemetry_statistics_aggregate_counts() {
    let pool = init_pool_in_memory().await.unwrap();
    let storage = Arc::new(Storage::new(pool));
    let wfp = Arc::new(UserspaceWfpEngine::new()) as Arc<dyn WfpEngine>;
    let ndis = Arc::new(StubNdisEngine::new()) as Arc<dyn wfp::NdisEngine>;
    let hybrid = test_hybrid(Arc::clone(&storage), Arc::clone(&ndis));
    let telemetry = KernelTelemetryService::new(storage, wfp, ndis, hybrid);

    let stats = telemetry.statistics().await.unwrap();
    assert_eq!(stats.guardian_mode, GuardianMode::Wfp);
}

#[tokio::test]
async fn kernel_telemetry_persist_snapshot_writes_row() {
    let pool = init_pool_in_memory().await.unwrap();
    let storage = Arc::new(Storage::new(pool));
    let wfp = Arc::new(UserspaceWfpEngine::new()) as Arc<dyn WfpEngine>;
    let ndis = Arc::new(StubNdisEngine::new()) as Arc<dyn wfp::NdisEngine>;
    let hybrid = test_hybrid(Arc::clone(&storage), Arc::clone(&ndis));
    let telemetry = KernelTelemetryService::new(Arc::clone(&storage), wfp, ndis, hybrid);

    telemetry.persist_snapshot().await.unwrap();
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM kernel_telemetry_snapshots")
        .fetch_one(&storage.pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);
}
