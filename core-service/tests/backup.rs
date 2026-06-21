use core_service::backup::BackupService;
use event_bus::EventBus;
use shared_types::{DnsSettings, EnterprisePolicy, Rule, RuleAction};
use std::sync::Arc;
use storage::{init_pool_in_memory, Storage};
use uuid::Uuid;

#[tokio::test]
async fn backup_export_import_round_trip() {
    let pool = init_pool_in_memory().await.expect("pool");
    let storage = Arc::new(Storage::new(pool));
    let backup = BackupService::new(Arc::clone(&storage), EventBus::new());

    let rule = Rule::for_domain("example.com", 10, RuleAction::Block);
    storage.rules.insert(&rule).await.expect("insert rule");

    let dns = DnsSettings::default();
    storage
        .settings
        .set_dns_settings(&dns)
        .await
        .expect("set dns");

    let policy = EnterprisePolicy {
        id: Uuid::new_v4(),
        version: 1,
        policy_json: serde_json::json!({ "telemetry": false }),
        locked_keys: vec!["telemetry".into()],
        updated_at: chrono::Utc::now(),
    };
    storage
        .enterprise_policy
        .upsert(&policy)
        .await
        .expect("upsert policy");

    let (_, json) = backup.export_json().await.expect("export");
    assert!(json.contains("example.com"));

    backup.import_json(&json).await.expect("import");

    let rules = storage.rules.list().await.expect("list rules");
    assert!(rules
        .iter()
        .any(|r| matches!(r.scope, shared_types::RuleScope::Domain(ref d) if d == "example.com")));

    let manifests = storage
        .backup_manifest
        .list_recent(10)
        .await
        .expect("manifests");
    assert!(manifests.len() >= 2);
    assert!(manifests.iter().any(|m| m.operation == "export"));
    assert!(manifests.iter().any(|m| m.operation == "import"));
}
