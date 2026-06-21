//! API bearer token — stored in DPAPI-protected file, read by UI locally.

use shared_types::WireSentinelError;
use storage::data_dir;
use std::path::PathBuf;
use uuid::Uuid;

pub fn token_path() -> PathBuf {
    data_dir().join(".api-token")
}

pub fn load_or_create_token() -> Result<String, WireSentinelError> {
    std::fs::create_dir_all(data_dir()).map_err(WireSentinelError::Io)?;
    let path = token_path();
    if path.exists() {
        let encrypted = std::fs::read(&path).map_err(WireSentinelError::Io)?;
        let plain = decrypt_token(&encrypted)?;
        let _ = restrict_token_acl();
        return String::from_utf8(plain)
            .map_err(|e| WireSentinelError::Config(format!("invalid token utf8: {e}")));
    }
    let token = Uuid::new_v4().to_string();
    save_token(&token)?;
    let _ = restrict_token_acl();
    Ok(token)
}

pub fn save_token(token: &str) -> Result<(), WireSentinelError> {
    let encrypted = encrypt_token(token.as_bytes())?;
    std::fs::write(token_path(), encrypted).map_err(WireSentinelError::Io)
}

fn encrypt_token(plaintext: &[u8]) -> Result<Vec<u8>, WireSentinelError> {
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
            Ok(std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec())
        }
    }
    #[cfg(not(windows))]
    {
        Ok(plaintext.to_vec())
    }
}

fn decrypt_token(ciphertext: &[u8]) -> Result<Vec<u8>, WireSentinelError> {
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
            Ok(std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec())
        }
    }
    #[cfg(not(windows))]
    {
        Ok(ciphertext.to_vec())
    }
}

pub fn validate_token(expected: &str, provided: Option<&str>) -> Result<(), WireSentinelError> {
    match provided {
        Some(t) if t == expected => Ok(()),
        _ => Err(WireSentinelError::Api("unauthorized".into())),
    }
}

pub fn rotate_token() -> Result<String, WireSentinelError> {
    let token = Uuid::new_v4().to_string();
    save_token(&token)?;
    Ok(token)
}

#[cfg(windows)]
pub fn restrict_token_acl() -> Result<(), WireSentinelError> {
    let path = token_path();
    if !path.exists() {
        return Ok(());
    }
    let path_str = path.to_string_lossy();
    let output = std::process::Command::new("icacls")
        .args([
            path_str.as_ref(),
            "/inheritance:r",
            "/grant:r",
            "Administrators:(F)",
            "SYSTEM:(F)",
        ])
        .output()
        .map_err(WireSentinelError::Io)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(WireSentinelError::Config(format!(
            "restrict_token_acl icacls failed: {stderr}"
        )));
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn restrict_token_acl() -> Result<(), WireSentinelError> {
    Ok(())
}
