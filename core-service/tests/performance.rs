use core_service::performance::PerformanceMonitor;
use event_bus::EventBus;
use std::sync::Arc;
use storage::{init_pool_in_memory, Storage};

#[tokio::test]
async fn performance_snapshot_fixture() {
    let pool = init_pool_in_memory().await.expect("pool");
    let storage = Arc::new(Storage::new(pool));
    let monitor = PerformanceMonitor::new(Arc::clone(&storage), EventBus::new());

    core_service::performance::record_api_latency_ms(4.5);
    core_service::performance::record_wfp_latency_ms(2.0);
    core_service::performance::record_event_published();

    let snapshot = monitor.record().await.expect("record snapshot");
    assert_eq!(snapshot.api_latency_ms, 4.0);
    assert_eq!(snapshot.wfp_latency_ms, 2.0);
    assert_eq!(snapshot.event_throughput, 1.0);
    assert!(snapshot.memory_bytes > 0);

    let latest = monitor.latest().await.expect("latest").expect("snapshot");
    assert_eq!(latest.id, snapshot.id);

    let recent = monitor.list_recent(5).await.expect("list recent");
    assert_eq!(recent.len(), 1);
}
