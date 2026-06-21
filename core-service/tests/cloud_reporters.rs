use core_service::cloud_backup_reporter::{CloudBackupReporter, CloudBackupReporterConfig};
use core_service::cloud_telemetry_reporter::{
    CloudTelemetryReporter, CloudTelemetryReporterConfig,
};
use core_service::cloud_usage_reporter::{CloudUsageReporter, CloudUsageReporterConfig};
use std::sync::Arc;
use storage::{init_pool_in_memory, Storage};
use uuid::Uuid;

#[tokio::test]
async fn cloud_usage_reporter_config_round_trip() {
    let pool = init_pool_in_memory().await.expect("pool");
    let storage = Arc::new(Storage::new(pool));
    let cfg = CloudUsageReporterConfig {
        enabled: true,
        cloud_url: "https://cloud.example.test".into(),
        tenant_id: Some(Uuid::new_v4()),
        device_id: Some("device-1".into()),
        interval_secs: 180,
        api_token: Some("token".into()),
    };
    CloudUsageReporter::save_config(&storage, &cfg)
        .await
        .expect("save");
    let loaded = CloudUsageReporter::load_config(&storage)
        .await
        .expect("load");
    assert_eq!(loaded, cfg);
}

#[tokio::test]
async fn cloud_telemetry_reporter_config_round_trip() {
    let pool = init_pool_in_memory().await.expect("pool");
    let storage = Arc::new(Storage::new(pool));
    let cfg = CloudTelemetryReporterConfig {
        enabled: true,
        cloud_url: "https://cloud.example.test".into(),
        tenant_id: Some(Uuid::new_v4()),
        controller_id: Some("ctrl-1".into()),
        interval_secs: 90,
        api_token: None,
    };
    CloudTelemetryReporter::save_config(&storage, &cfg)
        .await
        .expect("save");
    let loaded = CloudTelemetryReporter::load_config(&storage)
        .await
        .expect("load");
    assert_eq!(loaded, cfg);
}

#[tokio::test]
async fn cloud_backup_reporter_config_defaults() {
    let pool = init_pool_in_memory().await.expect("pool");
    let storage = Storage::new(pool);
    let loaded = CloudBackupReporter::load_config(&storage)
        .await
        .expect("load");
    assert_eq!(loaded, CloudBackupReporterConfig::default());
}
