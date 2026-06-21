#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

fn data_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var("PROGRAMDATA")
            .map(|p| PathBuf::from(p).join("WireSentinel"))
            .unwrap_or_else(|_| PathBuf::from(r"C:\ProgramData\WireSentinel"))
    } else {
        PathBuf::from("/tmp/WireSentinel")
    }
}

#[cfg(windows)]
fn decrypt_token(ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    use windows::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};
    let mut input = CRYPT_INTEGER_BLOB {
        cbData: ciphertext.len() as u32,
        pbData: ciphertext.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptUnprotectData(&mut input, None, None, None, None, 0, &mut output)
            .map_err(|e| format!("CryptUnprotectData: {e}"))?;
        Ok(std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec())
    }
}

#[cfg(not(windows))]
fn decrypt_token(ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    Ok(ciphertext.to_vec())
}

#[tauri::command]
fn read_api_token() -> Result<String, String> {
    let path = data_dir().join(".api-token");
    let bytes = std::fs::read(&path).map_err(|e| format!("read token: {e}"))?;
    let plain = decrypt_token(&bytes)?;
    String::from_utf8(plain).map_err(|e| format!("token utf8: {e}"))
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! WireSentinel UI ready.")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet, read_api_token])
        .run(tauri::generate_context!())
        .expect("error while running WireSentinel UI");
}
