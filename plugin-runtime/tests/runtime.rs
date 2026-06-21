//! Integration tests for plugin-runtime Phase 7A.

use plugin_runtime::{
    hex_sha256, PluginLoader, PluginManager, PluginRegistry, PluginSecurityEnforcer,
    PluginSecurityPolicy, WireSentinelPlugin,
};
use shared_types::{
    PluginCapability, PluginFormat, PluginManifest, PluginPermission, PluginRecord, PluginState,
};
use std::sync::Arc;
use uuid::Uuid;

fn minimal_wasm_bytes() -> Vec<u8> {
    br#"
        (module
          (func (export "plugin_init") (result i32)
            i32.const 0
          )
        )
    "#
    .to_vec()
}

fn sample_manifest(id: Uuid, path: &str) -> PluginManifest {
    PluginManifest {
        id,
        name: "test-filter".into(),
        version: "0.1.0".into(),
        format: PluginFormat::Wasm,
        capabilities: vec![PluginCapability::FilterEngine],
        permissions: vec![PluginPermission::FilterDomains],
        min_core_version: "0.1.0".into(),
        path: path.into(),
        sha256: None,
    }
}

#[test]
fn security_enforcer_rejects_spawn_process() {
    let enforcer = PluginSecurityEnforcer::new(PluginSecurityPolicy::default());
    let manifest = PluginManifest {
        id: Uuid::new_v4(),
        name: "bad".into(),
        version: "0.1.0".into(),
        format: PluginFormat::Wasm,
        capabilities: vec![],
        permissions: vec![PluginPermission::SpawnProcess],
        min_core_version: "0.1.0".into(),
        path: "/tmp/test.wasm".into(),
        sha256: None,
    };
    assert!(enforcer.validate_manifest(&manifest).is_err());
}

#[test]
fn wasm_loader_compiles_minimal_module() {
    let dir = tempfile::tempdir().unwrap();
    let wasm_path = dir.path().join("sample.wasm");
    let bytes = minimal_wasm_bytes();
    std::fs::write(&wasm_path, &bytes).unwrap();

    let id = Uuid::new_v4();
    let mut manifest = sample_manifest(id, wasm_path.to_str().unwrap());
    manifest.sha256 = Some(hex_sha256(&bytes));

    let record = PluginRecord {
        id,
        manifest,
        state: PluginState::Installed,
        error_message: None,
        installed_at: chrono::Utc::now(),
        loaded_at: None,
    };

    let loader = PluginLoader::new(PluginSecurityPolicy::default()).unwrap();
    let plugin = loader.load(&record).unwrap();
    assert_eq!(plugin.id(), id);
    assert_eq!(plugin.manifest().name, "test-filter");
}

#[test]
fn registry_tracks_loaded_plugins() {
    let mut registry = PluginRegistry::new();
    let id = Uuid::new_v4();
    let record = PluginRecord {
        id,
        manifest: sample_manifest(id, "sample.wasm"),
        state: PluginState::Loaded,
        error_message: None,
        installed_at: chrono::Utc::now(),
        loaded_at: Some(chrono::Utc::now()),
    };

    struct DummyPlugin(PluginManifest);
    impl WireSentinelPlugin for DummyPlugin {
        fn id(&self) -> Uuid {
            self.0.id
        }
        fn manifest(&self) -> &PluginManifest {
            &self.0
        }
        fn capabilities(&self) -> &[PluginCapability] {
            &self.0.capabilities
        }
    }

    let plugin: Arc<dyn WireSentinelPlugin> = Arc::new(DummyPlugin(record.manifest.clone()));
    registry.register_loaded(plugin, record);
    assert!(registry.is_loaded(id));
    assert_eq!(registry.records().len(), 1);
}

#[test]
fn manager_discovers_manifest_json() {
    let dir = tempfile::tempdir().unwrap();
    let id = Uuid::new_v4();
    let manifest = sample_manifest(id, "sample.wasm");
    std::fs::write(
        dir.path().join("sample.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();
    std::fs::write(dir.path().join("sample.wasm"), b"\0asm\x01\0\0\0").unwrap();

    let mut manager =
        PluginManager::new(dir.path().to_path_buf(), PluginSecurityPolicy::default()).unwrap();
    let discovered = manager.discover().unwrap();
    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].id, id);
}
