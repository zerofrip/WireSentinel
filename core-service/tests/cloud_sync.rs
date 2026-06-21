use core_service::cloud_sync::{CloudSyncAgent, CloudSyncConfig};
use shared_types::SyncMode;
use std::sync::Arc;
use storage::{init_pool_in_memory, Storage};
use uuid::Uuid;

#[tokio::test]
async fn cloud_sync_config_load_save_round_trip() {
    let pool = init_pool_in_memory().await.expect("pool");
    let storage = Arc::new(Storage::new(pool));

    let cfg = CloudSyncConfig {
        enabled: true,
        cloud_url: "https://cloud.example.test".into(),
        tenant_id: Some(Uuid::new_v4()),
        device_id: Some("device-abc".into()),
        sync_mode: SyncMode::Bidirectional,
        interval_secs: 120,
        api_token: Some("test-token".into()),
    };

    CloudSyncAgent::save_config(&storage, &cfg)
        .await
        .expect("save");

    let loaded = CloudSyncAgent::load_config(&storage)
        .await
        .expect("load");

    assert_eq!(loaded, cfg);
}

#[tokio::test]
async fn cloud_sync_config_load_default_when_missing() {
    let pool = init_pool_in_memory().await.expect("pool");
    let storage = Storage::new(pool);

    let loaded = CloudSyncAgent::load_config(&storage)
        .await
        .expect("load");

    assert_eq!(loaded, CloudSyncConfig::default());
}
