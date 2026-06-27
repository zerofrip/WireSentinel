use sha2::{Digest, Sha256};
use shared_types::WireSentinelError;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Compute SHA256 hash of an executable, streaming in fixed-size chunks so the
/// peak memory stays bounded regardless of the binary size (browsers and games
/// can be hundreds of MB, which previously caused large per-connection allocations).
pub fn file_sha256(path: &Path) -> Result<String, WireSentinelError> {
    let mut file = File::open(path).map_err(WireSentinelError::Io)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buf).map_err(WireSentinelError::Io)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Extract publisher from PE metadata (MVP: None on non-Windows or failure).
pub fn publisher_for_exe(_path: &Path) -> Option<String> {
    None
}

/// Icon path placeholder.
pub fn icon_path_for_exe(path: &Path) -> Option<std::path::PathBuf> {
    Some(path.with_extension("ico"))
}
