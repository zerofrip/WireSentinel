use core_service::deps::ServiceDeps;
use event_bus::EventBus;
use parking_lot::RwLock;
use std::sync::Arc;
use storage::{init_pool_in_memory, Storage};

async fn test_deps() -> Arc<ServiceDeps> {
    let pool = init_pool_in_memory().await.expect("pool");
    let storage = Arc::new(Storage::new(pool));
    let events = EventBus::new();
    let token = Arc::new(RwLock::new("test-token".to_string()));
    Arc::new(
        ServiceDeps::build(storage, events, token)
            .await
            .expect("deps"),
    )
}

#[tokio::test]
async fn fault_inject_dns_crash_verifies_recovery() {
    let deps = test_deps().await;
    let verified = deps
        .fault_injection
        .inject_and_verify(
            "dns_crash",
            &deps.vpn,
            &deps.transport,
            deps.wfp.as_ref(),
            deps.dns.settings().enabled,
        )
        .await
        .expect("inject");
    assert!(verified);
}
