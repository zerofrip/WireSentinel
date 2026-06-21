use crate::loader::NativePluginLoader;
use crate::registry::PluginRegistry;
use crate::wasm_loader::PluginLoader;
use chrono::Utc;
use shared_types::{
    PluginCapability, PluginFormat, PluginManifest, PluginRecord, PluginSecurityPolicy,
    PluginState, Result, WireSentinelError,
};
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use uuid::Uuid;

/// Default directory for installed plugin artifacts.
pub fn plugins_data_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var("PROGRAMDATA")
            .map(|p| PathBuf::from(p).join("WireSentinel").join("plugins"))
            .unwrap_or_else(|_| PathBuf::from(r"C:\ProgramData\WireSentinel\plugins"))
    } else {
        PathBuf::from("/tmp/WireSentinel/plugins")
    }
}

/// High-level plugin lifecycle manager.
pub struct PluginManager {
    data_dir: PathBuf,
    registry: PluginRegistry,
    wasm_loader: PluginLoader,
    native_loader: NativePluginLoader,
}

impl PluginManager {
    pub fn new(data_dir: PathBuf, policy: PluginSecurityPolicy) -> Result<Self> {
        std::fs::create_dir_all(&data_dir).map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(Self {
            data_dir,
            registry: PluginRegistry::new(),
            wasm_loader: PluginLoader::new(policy.clone())?,
            native_loader: NativePluginLoader::with_policy(policy.require_signature),
        })
    }

    pub fn seed_records(&mut self, records: Vec<PluginRecord>) {
        for record in records {
            self.registry.upsert_record(record);
        }
    }

    pub fn list(&self) -> Vec<PluginRecord> {
        self.registry.records()
    }

    pub fn list_anonymity_providers(&self) -> Vec<PluginRecord> {
        self.registry
            .records()
            .into_iter()
            .filter(|record| {
                record
                    .manifest
                    .capabilities
                    .iter()
                    .any(|cap| *cap == PluginCapability::AnonymityBackend)
            })
            .collect()
    }

    pub fn discover(&mut self) -> Result<Vec<PluginRecord>> {
        let mut discovered = Vec::new();
        if !self.data_dir.exists() {
            return Ok(discovered);
        }

        for entry in std::fs::read_dir(&self.data_dir)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
        {
            let entry = entry.map_err(|e| WireSentinelError::Config(e.to_string()))?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            let mut manifest = match parse_manifest_file(&path) {
                Ok(m) => m,
                Err(e) => {
                    warn!(error = %e, path = %path.display(), "invalid plugin manifest");
                    continue;
                }
            };

            let mut wasm_path = PathBuf::from(&manifest.path);
            if !wasm_path.is_absolute() {
                wasm_path = path
                    .parent()
                    .unwrap_or(&self.data_dir)
                    .join(wasm_path);
                manifest.path = wasm_path.to_string_lossy().into_owned();
            }

            if let Err(e) = self.wasm_loader.enforcer().validate_manifest(&manifest) {
                warn!(error = %e, plugin = %manifest.name, "plugin manifest rejected by policy");
                continue;
            }

            if manifest.format == PluginFormat::Wasm && !wasm_path.exists() {
                warn!(path = %wasm_path.display(), "plugin wasm missing");
                continue;
            }

            let record = PluginRecord {
                id: manifest.id,
                manifest,
                state: PluginState::Installed,
                error_message: None,
                installed_at: Utc::now(),
                loaded_at: None,
            };
            self.registry.upsert_record(record.clone());
            discovered.push(record);
        }

        info!(count = discovered.len(), "plugin discovery completed");
        Ok(discovered)
    }

    pub fn load(&mut self, id: Uuid) -> Result<PluginRecord> {
        let record = self
            .registry
            .record(id)
            .cloned()
            .ok_or_else(|| WireSentinelError::Other(format!("plugin not registered: {id}")))?;

        if self.registry.is_loaded(id) {
            return Ok(record);
        }

        match record.manifest.format {
            PluginFormat::Wasm => {
                let plugin = self.wasm_loader.load(&record)?;
                let mut loaded = record;
                loaded.state = PluginState::Loaded;
                loaded.loaded_at = Some(Utc::now());
                loaded.error_message = None;
                self.registry.register_loaded(plugin, loaded.clone());
                Ok(loaded)
            }
            PluginFormat::Native => {
                let _handle = self.native_loader.load(record.manifest.clone())?;
                let mut loaded = record;
                loaded.state = PluginState::Loaded;
                loaded.loaded_at = Some(Utc::now());
                loaded.error_message = None;
                self.registry.upsert_record(loaded.clone());
                Ok(loaded)
            }
        }
    }

    pub fn unload(&mut self, id: Uuid) -> Result<PluginRecord> {
        let record = self
            .registry
            .record(id)
            .cloned()
            .ok_or_else(|| WireSentinelError::Other(format!("plugin not registered: {id}")))?;

        self.registry.unregister_loaded(id);
        let mut unloaded = record;
        unloaded.state = PluginState::Unloaded;
        unloaded.loaded_at = None;
        self.registry.upsert_record(unloaded.clone());
        Ok(unloaded)
    }

    pub fn mark_failed(&mut self, id: Uuid, error: String) -> Result<PluginRecord> {
        let record = self
            .registry
            .record(id)
            .cloned()
            .ok_or_else(|| WireSentinelError::Other(format!("plugin not registered: {id}")))?;

        self.registry.unregister_loaded(id);
        let mut failed = record;
        failed.state = PluginState::Failed;
        failed.error_message = Some(error);
        failed.loaded_at = None;
        self.registry.upsert_record(failed.clone());
        Ok(failed)
    }
}

fn parse_manifest_file(path: &Path) -> Result<PluginManifest> {
    let raw = std::fs::read_to_string(path).map_err(|e| WireSentinelError::Config(e.to_string()))?;
    let mut manifest: PluginManifest = serde_json::from_str(&raw).map_err(WireSentinelError::Serde)?;

    if manifest.path.is_empty() {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("plugin")
            .strip_suffix(".plugin")
            .unwrap_or("plugin");
        manifest.path = path
            .with_file_name(format!("{stem}.wasm"))
            .to_string_lossy()
            .into_owned();
    }

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::PluginCapability;

    #[test]
    fn discovers_manifest_json() {
        let dir = std::env::temp_dir().join(format!("ws-plugins-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let manifest = PluginManifest {
            id: Uuid::new_v4(),
            name: "test".into(),
            version: "0.1.0".into(),
            format: PluginFormat::Wasm,
            capabilities: vec![PluginCapability::FilterEngine],
            permissions: vec![],
            min_core_version: "0.1.0".into(),
            path: "test.wasm".into(),
            sha256: None,
        };
        std::fs::write(
            dir.join("test.json"),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.join("test.wasm"), b"\0asm\x01\0\0\0").unwrap();
        let mut manager = PluginManager::new(dir.clone(), PluginSecurityPolicy::default()).unwrap();
        assert_eq!(manager.discover().unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }
}
