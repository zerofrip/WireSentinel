mod service_manager;
mod ui_prefs;

use service_manager::{BackendServiceState, ServiceBootstrapResult};
use std::path::PathBuf;
use tauri::{Manager, RunEvent, WindowEvent};
use ui_prefs::{
    ensure_tray, get_ui_preferences, load_ui_preferences, set_ui_preferences, AppUiState,
    MAIN_WINDOW_LABEL,
};

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
fn ensure_backend_service(
    backend: tauri::State<'_, BackendServiceState>,
) -> Result<ServiceBootstrapResult, String> {
    service_manager::ensure_backend_service(backend.inner())
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

    let app = match tauri::Builder::default()
        .manage(BackendServiceState::new())
        .setup(|app| {
            let backend = app.state::<BackendServiceState>();
            match service_manager::ensure_backend_service(backend.inner()) {
                Ok(result) => tracing::info!(mode = %result.mode, "backend service ready"),
                Err(error) => {
                    tracing::warn!(error = %error, "backend service bootstrap failed during setup");
                }
            }

            let prefs = load_ui_preferences(app.handle());
            app.manage(AppUiState::new(prefs.clone()));
            if prefs.close_to_tray {
                ensure_tray(app.handle())?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            read_api_token,
            ensure_backend_service,
            get_ui_preferences,
            set_ui_preferences,
        ])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let state = app.state::<AppUiState>();
                let close_to_tray = state.prefs.lock().unwrap().close_to_tray;
                if close_to_tray && window.label() == MAIN_WINDOW_LABEL {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .build(tauri::generate_context!())
    {
        Ok(app) => app,
        Err(error) => {
            tracing::error!(error = %error, "WireSentinel UI failed to build");
            eprintln!("error while building WireSentinel UI: {error}");
            std::process::exit(1);
        }
    };

    app.run(|app_handle, event| {
        if let RunEvent::Exit = event {
            if let Some(backend) = app_handle.try_state::<BackendServiceState>() {
                backend.stop_console_child();
            }
        }
    });

    tracing::info!("WireSentinel UI shut down");
}
