use crate::traits::{
    DnsProviderPlugin, FilterEnginePlugin, MetricsProviderPlugin, PolicyProviderPlugin,
    TransformModulePlugin, TransportBackendPlugin, WireSentinelPlugin,
};
use shared_types::{
    PluginManifest, PluginPermission, PluginRecord, PluginSecurityPolicy, Result, WireSentinelError,
};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;
use wasmtime::{Config, Engine, Instance, Module, Store};

pub fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Runtime enforcement wrapper around the shared policy model.
#[derive(Debug, Clone)]
pub struct PluginSecurityEnforcer {
    policy: PluginSecurityPolicy,
}

impl PluginSecurityEnforcer {
    pub fn new(policy: PluginSecurityPolicy) -> Self {
        Self { policy }
    }

    pub fn policy(&self) -> &PluginSecurityPolicy {
        &self.policy
    }

    pub fn validate_manifest(&self, manifest: &PluginManifest) -> Result<()> {
        if self.policy.require_signature && manifest.sha256.is_none() {
            return Err(WireSentinelError::Other(
                "plugin signature (sha256) required but missing".into(),
            ));
        }

        if !manifest_path_allowed(&self.policy, Path::new(&manifest.path)) {
            return Err(WireSentinelError::Other(format!(
                "path not allowed: {}",
                manifest.path
            )));
        }

        for permission in &manifest.permissions {
            if !permission_allowed(*permission) {
                return Err(WireSentinelError::Other(format!(
                    "permission {:?} not allowed in Phase 7A sandbox",
                    permission
                )));
            }
        }

        Ok(())
    }

    pub fn validate_wasm_bytes(
        &self,
        path: &Path,
        bytes: &[u8],
        manifest: &PluginManifest,
    ) -> Result<()> {
        self.validate_manifest(manifest)?;

        if let Some(expected) = &manifest.sha256 {
            let actual = hex_sha256(bytes);
            if !actual.eq_ignore_ascii_case(expected) {
                return Err(WireSentinelError::Other(format!(
                    "sha256 mismatch for {}",
                    path.display()
                )));
            }
        }

        Ok(())
    }
}

fn permission_allowed(permission: PluginPermission) -> bool {
    matches!(
        permission,
        PluginPermission::ReadConfig
            | PluginPermission::FilterDomains
            | PluginPermission::DnsResolve
            | PluginPermission::Network
    )
}

fn manifest_path_allowed(policy: &PluginSecurityPolicy, path: &Path) -> bool {
    if policy.allowed_paths.is_empty() {
        return true;
    }
    let path_str = path.to_string_lossy();
    policy
        .allowed_paths
        .iter()
        .any(|allowed| path_str.starts_with(allowed))
}

struct WasmPlugin {
    manifest: PluginManifest,
    _store: Store<()>,
    _instance: Instance,
}

impl WireSentinelPlugin for WasmPlugin {
    fn id(&self) -> Uuid {
        self.manifest.id
    }

    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn capabilities(&self) -> &[shared_types::PluginCapability] {
        &self.manifest.capabilities
    }
}

impl FilterEnginePlugin for WasmPlugin {
    fn parse_domains(&self, _content: &str) -> HashSet<String> {
        HashSet::new()
    }

    fn is_blocked(&self, _domain: &str, _blocked: &HashSet<String>) -> bool {
        false
    }
}

impl DnsProviderPlugin for WasmPlugin {
    fn resolve(&self, _qname: &str) -> Option<String> {
        None
    }
}

impl TransportBackendPlugin for WasmPlugin {
    fn backend_name(&self) -> &str {
        &self.manifest.name
    }
}

impl TransformModulePlugin for WasmPlugin {
    fn transform(&self, payload: &[u8]) -> Vec<u8> {
        payload.to_vec()
    }
}

impl PolicyProviderPlugin for WasmPlugin {
    fn policy_hint(&self, _app_id: Uuid) -> Option<String> {
        None
    }
}

impl MetricsProviderPlugin for WasmPlugin {
    fn collect_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "plugin_id": self.manifest.id,
            "name": self.manifest.name,
        })
    }
}

/// wasmtime-backed WASM plugin loader.
pub struct PluginLoader {
    engine: Engine,
    enforcer: PluginSecurityEnforcer,
}

impl PluginLoader {
    pub fn new(policy: PluginSecurityPolicy) -> Result<Self> {
        let mut config = Config::new();
        config.consume_fuel(true);
        let engine = Engine::new(&config)
            .map_err(|e| WireSentinelError::Other(format!("wasmtime engine: {e}")))?;
        Ok(Self {
            engine,
            enforcer: PluginSecurityEnforcer::new(policy),
        })
    }

    pub fn enforcer(&self) -> &PluginSecurityEnforcer {
        &self.enforcer
    }

    pub fn load(&self, record: &PluginRecord) -> Result<Arc<dyn WireSentinelPlugin>> {
        let manifest = &record.manifest;
        let path = PathBuf::from(&manifest.path);
        if !path.exists() {
            return Err(WireSentinelError::Other(format!(
                "wasm file missing: {}",
                path.display()
            )));
        }

        let bytes = std::fs::read(&path).map_err(WireSentinelError::Io)?;
        self.enforcer
            .validate_wasm_bytes(&path, &bytes, manifest)?;

        let module = Module::new(&self.engine, &bytes)
            .map_err(|e| WireSentinelError::Other(format!("wasm compile: {e}")))?;
        let mut store = Store::new(&self.engine, ());
        store
            .set_fuel(self.enforcer.policy().max_fuel)
            .map_err(|e| WireSentinelError::Other(format!("fuel limit: {e}")))?;
        let instance = Instance::new(&mut store, &module, &[])
            .map_err(|e| WireSentinelError::Other(format!("wasm instantiate: {e}")))?;

        if let Ok(init) = instance.get_typed_func::<(), i32>(&mut store, "plugin_init") {
            init.call(&mut store, ())
                .map_err(|e| WireSentinelError::Other(format!("plugin_init: {e}")))?;
        }

        Ok(Arc::new(WasmPlugin {
            manifest: manifest.clone(),
            _store: store,
            _instance: instance,
        }))
    }
}
