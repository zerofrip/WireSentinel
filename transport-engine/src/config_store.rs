use shared_types::{Result, WireSentinelError};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Returns `%ProgramData%/WireSentinel/transports` on Windows, `/tmp/WireSentinel/transports` elsewhere.
pub fn transports_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var("PROGRAMDATA")
            .map(|p| PathBuf::from(p).join("WireSentinel").join("transports"))
            .unwrap_or_else(|_| PathBuf::from(r"C:\ProgramData\WireSentinel\transports"))
    } else {
        PathBuf::from("/tmp/WireSentinel/transports")
    }
}

/// Persists generated transport configs as `{id}.json`.
pub struct TransportConfigStore {
    base_dir: PathBuf,
}

impl TransportConfigStore {
    pub fn new() -> Self {
        Self {
            base_dir: transports_dir(),
        }
    }

    pub fn with_dir(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn path_for(&self, id: Uuid) -> PathBuf {
        self.base_dir.join(format!("{id}.json"))
    }

    pub fn write_json(&self, id: Uuid, value: &serde_json::Value) -> Result<PathBuf> {
        std::fs::create_dir_all(&self.base_dir).map_err(WireSentinelError::Io)?;
        let path = self.path_for(id);
        let bytes = serde_json::to_vec_pretty(value).map_err(WireSentinelError::Serde)?;
        std::fs::write(&path, bytes).map_err(WireSentinelError::Io)?;
        Ok(path)
    }

    pub fn read_json(&self, id: Uuid) -> Result<serde_json::Value> {
        let path = self.path_for(id);
        let bytes = std::fs::read(&path).map_err(WireSentinelError::Io)?;
        serde_json::from_slice(&bytes).map_err(WireSentinelError::Serde)
    }

    pub fn delete(&self, id: Uuid) -> Result<bool> {
        let path = self.path_for(id);
        if !path.exists() {
            return Ok(false);
        }
        std::fs::remove_file(&path).map_err(WireSentinelError::Io)?;
        Ok(true)
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }
}

impl Default for TransportConfigStore {
    fn default() -> Self {
        Self::new()
    }
}
