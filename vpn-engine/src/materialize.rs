//! Materialize VPN config blobs from db:// references to on-disk .conf files.

use crate::conf::{parse_conf, write_conf};
use shared_types::{Result, VPNProfile, WireSentinelError};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const DB_SCHEME: &str = "db://";

pub fn vpn_config_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var("PROGRAMDATA")
            .map(|p| PathBuf::from(p).join("WireSentinel").join("vpn"))
            .unwrap_or_else(|_| PathBuf::from(r"C:\ProgramData\WireSentinel\vpn"))
    } else {
        PathBuf::from("/tmp/WireSentinel/vpn")
    }
}

pub fn is_db_path(path: &Path) -> bool {
    path.to_string_lossy().starts_with(DB_SCHEME)
}

/// Resolve profile config path, materializing blob to disk when needed.
pub fn materialize_profile_config(profile: &VPNProfile, blob: Option<&[u8]>) -> Result<PathBuf> {
    if !is_db_path(&profile.config_path) {
        return Ok(profile.config_path.clone());
    }

    let blob = blob
        .ok_or_else(|| WireSentinelError::Vpn("config blob required for db:// profile".into()))?;
    let plaintext = String::from_utf8(blob.to_vec())
        .map_err(|e| WireSentinelError::Vpn(format!("invalid config blob: {e}")))?;

    let config = parse_conf(&plaintext);
    let out = write_conf(&config);

    let dir = vpn_config_dir();
    std::fs::create_dir_all(&dir).map_err(|e| WireSentinelError::Vpn(e.to_string()))?;
    let path = dir.join(format!("{}.conf", profile.id));
    std::fs::write(&path, out).map_err(|e| WireSentinelError::Vpn(e.to_string()))?;
    Ok(path)
}

#[allow(dead_code)]
pub fn profile_id_from_db_path(path: &Path) -> Option<Uuid> {
    let s = path.to_string_lossy();
    if !s.starts_with(DB_SCHEME) {
        return None;
    }
    Uuid::parse_str(s.trim_start_matches(DB_SCHEME)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::VpnBackendKind;

    #[test]
    fn materialize_writes_conf_file() {
        let profile = VPNProfile::new(
            "test".into(),
            VpnBackendKind::WireGuardNt,
            PathBuf::from(format!("db://{}", Uuid::new_v4())),
        );
        let blob = b"[Interface]\nPrivateKey = abc\nAddress = 10.0.0.2/32\n";
        let path = materialize_profile_config(&profile, Some(blob)).unwrap();
        assert!(path.exists());
        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("PrivateKey = abc"));
    }
}
