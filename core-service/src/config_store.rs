//! DPAPI-backed configuration storage.

use shared_types::{AppConfig, Result, WireSentinelError};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

pub fn config_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var("PROGRAMDATA")
            .map(|p| PathBuf::from(p).join("WireSentinel"))
            .unwrap_or_else(|_| PathBuf::from(r"C:\ProgramData\WireSentinel"))
    } else {
        PathBuf::from("/tmp/WireSentinel")
    }
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn tunnels_dir() -> PathBuf {
    config_dir().join("tunnels")
}

pub struct ConfigStore;

impl ConfigStore {
    pub fn ensure_dirs() -> Result<()> {
        std::fs::create_dir_all(config_dir()).map_err(WireSentinelError::Io)?;
        std::fs::create_dir_all(tunnels_dir()).map_err(WireSentinelError::Io)?;
        Ok(())
    }

    pub fn load() -> Result<AppConfig> {
        Self::ensure_dirs()?;
        let path = config_path();
        if !path.exists() {
            info!("no config found, using defaults");
            return Ok(AppConfig::default());
        }
        let data = std::fs::read_to_string(&path).map_err(WireSentinelError::Io)?;
        serde_json::from_str(&data).map_err(WireSentinelError::Serde)
    }

    pub fn save(config: &AppConfig) -> Result<()> {
        Self::ensure_dirs()?;
        let data = serde_json::to_string_pretty(config).map_err(WireSentinelError::Serde)?;
        std::fs::write(config_path(), data).map_err(WireSentinelError::Io)?;
        Ok(())
    }

    /// Encrypt plaintext with DPAPI (Windows) or store as-is (dev).
    pub fn encrypt_secret(plaintext: &[u8]) -> Result<Vec<u8>> {
        #[cfg(windows)]
        {
            use windows::Win32::Security::Cryptography::{
                CryptProtectData, CRYPTPROTECT_LOCAL_MACHINE, CRYPT_INTEGER_BLOB,
            };

            let mut input = CRYPT_INTEGER_BLOB {
                cbData: plaintext.len() as u32,
                pbData: plaintext.as_ptr() as *mut u8,
            };
            let mut output = CRYPT_INTEGER_BLOB::default();

            unsafe {
                CryptProtectData(
                    &mut input,
                    None,
                    None,
                    None,
                    None,
                    CRYPTPROTECT_LOCAL_MACHINE,
                    &mut output,
                )
                .map_err(|e| WireSentinelError::Config(format!("CryptProtectData: {e}")))?;

                let encrypted =
                    std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
                Ok(encrypted)
            }
        }
        #[cfg(not(windows))]
        {
            Ok(plaintext.to_vec())
        }
    }

    pub fn decrypt_secret(ciphertext: &[u8]) -> Result<Vec<u8>> {
        #[cfg(windows)]
        {
            use windows::Win32::Security::Cryptography::{
                CryptUnprotectData, CRYPT_INTEGER_BLOB,
            };

            let mut input = CRYPT_INTEGER_BLOB {
                cbData: ciphertext.len() as u32,
                pbData: ciphertext.as_ptr() as *mut u8,
            };
            let mut output = CRYPT_INTEGER_BLOB::default();

            unsafe {
                CryptUnprotectData(&mut input, None, None, None, None, 0, &mut output)
                    .map_err(|e| WireSentinelError::Config(format!("CryptUnprotectData: {e}")))?;

                let decrypted =
                    std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
                Ok(decrypted)
            }
        }
        #[cfg(not(windows))]
        {
            Ok(ciphertext.to_vec())
        }
    }

    pub fn save_tunnel_config(name: &str, plaintext: &str) -> Result<PathBuf> {
        let encrypted = Self::encrypt_secret(plaintext.as_bytes())?;
        let path = tunnels_dir().join(format!("{name}.conf.dpapi"));
        std::fs::write(&path, encrypted).map_err(WireSentinelError::Io)?;
        info!(path = %path.display(), "saved encrypted tunnel config");
        Ok(path)
    }

    pub fn load_tunnel_config(path: &Path) -> Result<String> {
        let encrypted = std::fs::read(path).map_err(WireSentinelError::Io)?;
        let decrypted = Self::decrypt_secret(&encrypted)?;
        String::from_utf8(decrypted)
            .map_err(|e| WireSentinelError::Config(format!("invalid UTF-8 in tunnel config: {e}")))
    }
}
