use core_service::deps::ServiceDeps;
use event_bus::EventBus;
use parking_lot::RwLock;
use std::io::Read;
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
async fn diagnostics_bundle_contains_no_secrets() {
    let deps = test_deps().await;
    let bytes = deps
        .diagnostics
        .export_bundle()
        .await
        .expect("export bundle");

    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).expect("zip archive");

    let forbidden = [
        "test-token",
        "PrivateKey",
        "PresharedKey",
        "api-token",
        ".conf.dpapi",
        "password",
    ];

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).expect("zip entry");
        let mut content = String::new();
        file.read_to_string(&mut content)
            .expect("read zip entry");

        for needle in forbidden {
            assert!(
                !content.to_ascii_lowercase().contains(&needle.to_ascii_lowercase()),
                "diagnostics bundle must not contain '{needle}' in {}",
                file.name()
            );
        }
    }

    assert!(archive.len() >= 2, "expected health.json and version.json");
}
