use shared_types::WireSentinelError;
use sha2::{Digest, Sha256};
use std::path::Path;

/// Compute SHA256 hash of executable file.
pub fn file_sha256(path: &Path) -> Result<String, WireSentinelError> {
    let data = std::fs::read(path).map_err(WireSentinelError::Io)?;
    let hash = Sha256::digest(&data);
    Ok(hex::encode(hash))
}

/// Extract publisher from PE metadata (MVP: None on non-Windows or failure).
pub fn publisher_for_exe(_path: &Path) -> Option<String> {
    None
}

/// Icon path placeholder.
pub fn icon_path_for_exe(path: &Path) -> Option<std::path::PathBuf> {
    Some(path.with_extension("ico"))
}
