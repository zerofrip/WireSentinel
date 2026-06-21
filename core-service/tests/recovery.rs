use core_service::recovery::RecoveryService;
use event_bus::EventBus;
use std::sync::Arc;
use storage::{init_pool_in_memory, Storage};
use uuid::Uuid;

#[tokio::test]
async fn runtime_state_snapshot_save_and_load() {
    let pool = init_pool_in_memory().await.expect("pool");
    let storage = Arc::new(Storage::new(pool));
    let recovery = RecoveryService::new(Arc::clone(&storage), EventBus::new());

    let profile_id = Uuid::new_v4();
    recovery
        .persist_vpn(profile_id, true)
        .await
        .expect("persist vpn");

    let records = storage
        .runtime_state
        .list_by_scope("vpn")
        .await
        .expect("list vpn scope");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].entity_id, profile_id.to_string());

    let state: serde_json::Value =
        serde_json::from_str(&records[0].state_json).expect("state json");
    assert_eq!(state["connected"], true);

    recovery
        .persist_vpn(profile_id, false)
        .await
        .expect("persist vpn disconnected");
    let updated = storage
        .runtime_state
        .list_by_scope("vpn")
        .await
        .expect("list vpn scope");
    assert_eq!(updated.len(), 1);
    let state: serde_json::Value =
        serde_json::from_str(&updated[0].state_json).expect("state json");
    assert_eq!(state["connected"], false);
}
