use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent, TrayIconId};
use tauri::{AppHandle, Manager, State};

pub const MAIN_WINDOW_LABEL: &str = "main";
pub const TRAY_ID: &str = "wire-sentinel-tray";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiPreferences {
    #[serde(default = "default_close_to_tray")]
    pub close_to_tray: bool,
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            close_to_tray: default_close_to_tray(),
        }
    }
}

fn default_close_to_tray() -> bool {
    true
}

pub struct AppUiState {
    pub prefs: Mutex<UiPreferences>,
    pub tray_id: Mutex<Option<TrayIconId>>,
}

impl AppUiState {
    pub fn new(prefs: UiPreferences) -> Self {
        Self {
            prefs: Mutex::new(prefs),
            tray_id: Mutex::new(None),
        }
    }
}

fn prefs_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("app data dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("create app data dir: {e}"))?;
    Ok(dir.join("ui-preferences.json"))
}

pub fn load_ui_preferences(app: &AppHandle) -> UiPreferences {
    let path = match prefs_path(app) {
        Ok(path) => path,
        Err(e) => {
            tracing::warn!(error = %e, "using default UI preferences");
            return UiPreferences::default();
        }
    };

    match fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_else(|e| {
            tracing::warn!(error = %e, path = %path.display(), "invalid UI preferences file");
            UiPreferences::default()
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => UiPreferences::default(),
        Err(e) => {
            tracing::warn!(error = %e, path = %path.display(), "failed to read UI preferences");
            UiPreferences::default()
        }
    }
}

pub fn save_ui_preferences(app: &AppHandle, prefs: &UiPreferences) -> Result<(), String> {
    let path = prefs_path(app)?;
    let raw = serde_json::to_string_pretty(prefs).map_err(|e| e.to_string())?;
    fs::write(&path, raw).map_err(|e| format!("write UI preferences: {e}"))?;
    Ok(())
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn ensure_tray(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppUiState>();
    if state.tray_id.lock().unwrap().is_some() {
        return Ok(());
    }

    let open_item =
        MenuItem::with_id(app, "tray-open", "Open WireSentinel", true, None::<&str>)
            .map_err(|e| e.to_string())?;
    let quit_item = MenuItem::with_id(app, "tray-quit", "Exit", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let menu = Menu::with_items(app, &[&open_item, &quit_item]).map_err(|e| e.to_string())?;

    let icon = app
        .default_window_icon()
        .ok_or_else(|| "default window icon missing".to_string())?
        .clone();

    let tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("WireSentinel")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "tray-open" => show_main_window(app),
            "tray-quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)
        .map_err(|e| e.to_string())?;

    *state.tray_id.lock().unwrap() = Some(tray.id().clone());
    Ok(())
}

pub fn remove_tray(app: &AppHandle) {
    let state = app.state::<AppUiState>();
    if let Some(id) = state.tray_id.lock().unwrap().take() {
        let _ = app.remove_tray_by_id(id);
    }
}

pub fn sync_tray(app: &AppHandle, close_to_tray: bool) -> Result<(), String> {
    if close_to_tray {
        ensure_tray(app)
    } else {
        remove_tray(app);
        Ok(())
    }
}

#[tauri::command]
pub fn get_ui_preferences(state: State<'_, AppUiState>) -> UiPreferences {
    state.prefs.lock().unwrap().clone()
}

#[tauri::command]
pub fn set_ui_preferences(
    app: AppHandle,
    prefs: UiPreferences,
    state: State<'_, AppUiState>,
) -> Result<(), String> {
    save_ui_preferences(&app, &prefs)?;
    let close_to_tray = prefs.close_to_tray;
    *state.prefs.lock().unwrap() = prefs;
    sync_tray(&app, close_to_tray)?;
    tracing::info!(close_to_tray, "UI preferences updated");
    Ok(())
}
