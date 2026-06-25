use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

#[cfg_attr(not(windows), allow(dead_code))]
pub const SERVICE_NAME: &str = "WireSentinel";
const DEFAULT_API_PORT: u16 = 8170;
const API_READY_TIMEOUT: Duration = Duration::from_secs(30);
const API_POLL_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Serialize)]
pub struct ServiceBootstrapResult {
    pub mode: String,
    pub message: String,
}

pub struct BackendServiceState {
    pub console_child: Mutex<Option<std::process::Child>>,
}

impl BackendServiceState {
    pub fn new() -> Self {
        Self {
            console_child: Mutex::new(None),
        }
    }

    pub fn stop_console_child(&self) {
        if let Ok(mut guard) = self.console_child.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

fn api_port() -> u16 {
    std::env::var("WIRESENTINEL_API_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_API_PORT)
}

pub fn api_token_url() -> String {
    format!("http://127.0.0.1:{}/api/v1/auth/token", api_port())
}

pub fn wait_for_api_ready() -> Result<(), String> {
    let url = api_token_url();
    let deadline = std::time::Instant::now() + API_READY_TIMEOUT;
    while std::time::Instant::now() < deadline {
        match ureq::get(&url).call() {
            Ok(response) if response.status() == 200 => return Ok(()),
            Ok(response) => {
                tracing::debug!(status = response.status(), "API not ready yet");
            }
            Err(error) => {
                tracing::debug!(error = %error, "API poll failed");
            }
        }
        std::thread::sleep(API_POLL_INTERVAL);
    }
    Err(format!(
        "Timed out waiting for WireSentinel API at {}",
        url
    ))
}

#[cfg_attr(not(windows), allow(dead_code))]
pub fn resolve_service_exe() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("WIRESENTINEL_SERVICE_EXE") {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    let current = std::env::current_exe().ok()?;
    let mut candidates = Vec::new();
    if let Some(dir) = current.parent() {
        candidates.push(dir.join("wire-sentinel-service.exe"));
        candidates.push(
            dir.join("../../../target/debug/wire-sentinel-service.exe"),
        );
        candidates.push(
            dir.join("../../../target/release/wire-sentinel-service.exe"),
        );
        candidates.push(
            dir.join("../../../../target/debug/wire-sentinel-service.exe"),
        );
        candidates.push(
            dir.join("../../../../target/release/wire-sentinel-service.exe"),
        );
        candidates.push(
            dir.join("../../../../../target/debug/wire-sentinel-service.exe"),
        );
        candidates.push(
            dir.join("../../../../../target/release/wire-sentinel-service.exe"),
        );
    }

    for candidate in candidates {
        if let Ok(path) = candidate.canonicalize() {
            if path.is_file() {
                return Some(path);
            }
        }
    }

    None
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{GetLastError, WIN32_ERROR};
    use windows::Win32::System::Services::{
        CloseServiceHandle, OpenSCManagerW, OpenServiceW, QueryServiceStatus, StartServiceW,
        SC_MANAGER_CONNECT, SERVICE_ALL_ACCESS, SERVICE_RUNNING, SERVICE_STATUS,
        SERVICE_STATUS_CURRENT_STATE,
    };
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const ERROR_ACCESS_DENIED: u32 = 5;
    const ERROR_SERVICE_DOES_NOT_EXIST: u32 = 1060;

    fn wide(value: &str) -> Vec<u16> {
        OsStr::new(value)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    fn last_win32_error() -> WIN32_ERROR {
        unsafe { GetLastError() }
    }

    pub fn scm_registered() -> bool {
        query_service_state().is_some()
    }

    pub fn query_service_state() -> Option<SERVICE_STATUS_CURRENT_STATE> {
        let name = wide(SERVICE_NAME);
        unsafe {
            let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT).ok()?;
            let service = match OpenServiceW(scm, PCWSTR(name.as_ptr()), SERVICE_ALL_ACCESS) {
                Ok(handle) => handle,
                Err(_) => {
                    let _ = CloseServiceHandle(scm);
                    return None;
                }
            };
            let mut status = SERVICE_STATUS::default();
            let result = QueryServiceStatus(service, &mut status)
                .ok()
                .map(|_| status.dwCurrentState);
            let _ = CloseServiceHandle(service);
            let _ = CloseServiceHandle(scm);
            result
        }
    }

    pub fn start_scm_service() -> Result<(), String> {
        let state = query_service_state();
        if state == Some(SERVICE_RUNNING) {
            return Ok(());
        }
        if state.is_none() {
            return Err("WireSentinel Windows service is not installed".into());
        }

        let name = wide(SERVICE_NAME);
        unsafe {
            let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT)
                .map_err(|error| format!("OpenSCManagerW: {error}"))?;
            let service = OpenServiceW(scm, PCWSTR(name.as_ptr()), SERVICE_ALL_ACCESS)
                .map_err(|error| format!("OpenServiceW: {error}"))?;
            if let Err(error) = StartServiceW(service, None) {
                let code = last_win32_error().0;
                let _ = CloseServiceHandle(service);
                let _ = CloseServiceHandle(scm);
                return Err(format!("StartServiceW: {error} (win32={code})"));
            }
            let _ = CloseServiceHandle(service);
            let _ = CloseServiceHandle(scm);
        }
        Ok(())
    }

    pub fn start_scm_service_elevated() -> Result<(), String> {
        let params = wide("-NoProfile -WindowStyle Hidden -Command \"Start-Service -Name WireSentinel\"");
        let file = wide("powershell.exe");
        let verb = wide("runas");
        unsafe {
            let result = ShellExecuteW(
                None,
                PCWSTR(verb.as_ptr()),
                PCWSTR(file.as_ptr()),
                PCWSTR(params.as_ptr()),
                PCWSTR::null(),
                SW_HIDE,
            );
            if result.0 as isize <= 32 {
                return Err(format!(
                    "elevated service start declined or failed (ShellExecute={:?})",
                    result.0
                ));
            }
        }

        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        while std::time::Instant::now() < deadline {
            if query_service_state() == Some(SERVICE_RUNNING) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        Err("Timed out waiting for elevated WireSentinel service start".into())
    }

    pub fn spawn_console_service(
        state: &BackendServiceState,
        exe: &PathBuf,
    ) -> Result<(), String> {
        let mut guard = state
            .console_child
            .lock()
            .map_err(|_| "console child lock poisoned".to_string())?;
        if let Some(child) = guard.as_mut() {
            if child.try_wait().ok().flatten().is_none() {
                return Ok(());
            }
            guard.take();
        }

        let child = Command::new(exe)
            .arg("--console")
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("failed to spawn {}: {error}", exe.display()))?;
        tracing::info!(pid = child.id(), path = %exe.display(), "spawned console backend service");
        *guard = Some(child);
        Ok(())
    }

    pub fn ensure_backend_service(state: &BackendServiceState) -> Result<ServiceBootstrapResult, String> {
        if wait_for_api_ready().is_ok() {
            return Ok(ServiceBootstrapResult {
                mode: "already_running".into(),
                message: "WireSentinel API is already available".into(),
            });
        }

        match query_service_state() {
            Some(state) if state == SERVICE_RUNNING => {
                wait_for_api_ready()?;
                return Ok(ServiceBootstrapResult {
                    mode: "scm".into(),
                    message: "WireSentinel service already running".into(),
                });
            }
            Some(_) => match start_scm_service() {
                Ok(()) => {
                    wait_for_api_ready()?;
                    return Ok(ServiceBootstrapResult {
                        mode: "scm".into(),
                        message: "Started WireSentinel Windows service".into(),
                    });
                }
                Err(error) if error.contains(&format!("win32={ERROR_ACCESS_DENIED}")) => {
                    tracing::warn!(error = %error, "SCM start denied; requesting elevation");
                    start_scm_service_elevated()?;
                    wait_for_api_ready()?;
                    return Ok(ServiceBootstrapResult {
                        mode: "elevated".into(),
                        message: "Started WireSentinel service with administrator approval".into(),
                    });
                }
                Err(error) => return Err(error),
            },
            None => {
                let exe = resolve_service_exe().ok_or_else(|| {
                    "WireSentinel service is not installed and wire-sentinel-service.exe was not found"
                        .to_string()
                })?;
                spawn_console_service(state, &exe)?;
                wait_for_api_ready()?;
                return Ok(ServiceBootstrapResult {
                    mode: "console".into(),
                    message: "Started WireSentinel backend in console mode".into(),
                });
            }
        }
    }

    pub fn is_access_denied(error: &str) -> bool {
        error.contains(&format!("win32={ERROR_ACCESS_DENIED}"))
            || error.contains(&format!("{ERROR_SERVICE_DOES_NOT_EXIST}"))
    }
}

#[cfg(not(windows))]
mod windows_impl {
    use super::*;

    pub fn ensure_backend_service(_state: &BackendServiceState) -> Result<ServiceBootstrapResult, String> {
        if wait_for_api_ready().is_ok() {
            return Ok(ServiceBootstrapResult {
                mode: "already_running".into(),
                message: "WireSentinel API is already available".into(),
            });
        }
        Err("WireSentinel backend auto-start is only supported on Windows".into())
    }
}

pub use windows_impl::ensure_backend_service;
