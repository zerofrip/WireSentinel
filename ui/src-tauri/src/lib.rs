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

#[cfg(debug_assertions)]
fn init_debug_logging() {
    use tracing_subscriber::{fmt, EnvFilter};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(filter)
        .try_init();

    std::panic::set_hook(Box::new(|info| {
        tracing::error!(panic = %info, "WireSentinel UI panicked");
    }));
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
    match std::fs::read(&path) {
        Ok(bytes) => match decrypt_token(&bytes) {
            Ok(plain) => match String::from_utf8(plain) {
                Ok(token) => {
                    tracing::debug!(path = %path.display(), "API token loaded");
                    Ok(token)
                }
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "API token UTF-8 decode failed");
                    Err(format!("token utf8: {e}"))
                }
            },
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "API token decrypt failed");
                Err(e)
            }
        },
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "API token file not readable");
            Err(format!("read token: {e}"))
        }
    }
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! WireSentinel UI ready.")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(debug_assertions)]
    init_debug_logging();

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "WireSentinel UI starting");

    if let Err(e) = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet, read_api_token])
        .run(tauri::generate_context!())
    {
        tracing::error!(error = %e, "WireSentinel UI exited with error");
        eprintln!("error while running WireSentinel UI: {e}");
        std::process::exit(1);
    }

    tracing::info!("WireSentinel UI shut down");
}
