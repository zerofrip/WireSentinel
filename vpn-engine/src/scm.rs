//! SCM + tunnel.dll backend (MasselGUARD / wireguard-windows pattern).

use crate::backend::VpnBackend;
use async_trait::async_trait;
use parking_lot::RwLock;
use shared_types::{Result, VPNProfile, VpnStats, VpnStatus, WireSentinelError};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{info, warn};
use uuid::Uuid;

const SERVICE_PREFIX: &str = "WireGuardTunnel$";

pub struct ScmTunnelDllBackend {
    tunnel_dll_path: PathBuf,
    service_exe_path: PathBuf,
    states: RwLock<HashMap<Uuid, VpnStatus>>,
    service_names: RwLock<HashMap<Uuid, String>>,
}

impl ScmTunnelDllBackend {
    pub fn new(tunnel_dll_path: PathBuf, service_exe_path: PathBuf) -> Self {
        Self {
            tunnel_dll_path,
            service_exe_path,
            states: RwLock::new(HashMap::new()),
            service_names: RwLock::new(HashMap::new()),
        }
    }

    fn service_name(tunnel_name: &str) -> String {
        format!("{SERVICE_PREFIX}{tunnel_name}")
    }

    #[cfg(windows)]
    fn install_and_start_service(&self, service_name: &str, config_path: &PathBuf) -> Result<()> {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows::core::PCWSTR;
        use windows::Win32::System::Services::{
            CloseServiceHandle, CreateServiceW, OpenSCManagerW, OpenServiceW, StartServiceW,
            SC_MANAGER_CREATE_SERVICE, SERVICE_ALL_ACCESS, SERVICE_DEMAND_START,
            SERVICE_ERROR_NORMAL, SERVICE_WIN32_OWN_PROCESS,
        };

        let bin_path = format!(
            "\"{}\" /service \"{}\"",
            self.service_exe_path.display(),
            config_path.display()
        );
        let bin_wide: Vec<u16> = OsStr::new(&bin_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let name_wide: Vec<u16> = OsStr::new(service_name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CREATE_SERVICE)
                .map_err(|e| WireSentinelError::Vpn(format!("OpenSCManagerW: {e}")))?;

            let service = CreateServiceW(
                scm,
                PCWSTR(name_wide.as_ptr()),
                PCWSTR(name_wide.as_ptr()),
                SERVICE_ALL_ACCESS,
                SERVICE_WIN32_OWN_PROCESS,
                SERVICE_DEMAND_START,
                SERVICE_ERROR_NORMAL,
                PCWSTR(bin_wide.as_ptr()),
                PCWSTR::null(),
                None,
                PCWSTR::null(),
                PCWSTR::null(),
                PCWSTR::null(),
            );

            match service {
                Ok(handle) => {
                    StartServiceW(handle, None)
                        .map_err(|e| WireSentinelError::Vpn(format!("StartServiceW: {e}")))?;
                    let _ = CloseServiceHandle(handle);
                }
                Err(_) => {
                    // Service may already exist — try to start it
                    let existing =
                        OpenServiceW(scm, PCWSTR(name_wide.as_ptr()), SERVICE_ALL_ACCESS)
                            .map_err(|e| WireSentinelError::Vpn(format!("OpenServiceW: {e}")))?;
                    StartServiceW(existing, None).map_err(|e| {
                        WireSentinelError::Vpn(format!("StartServiceW existing: {e}"))
                    })?;
                    let _ = CloseServiceHandle(existing);
                }
            }
            let _ = CloseServiceHandle(scm);
        }
        Ok(())
    }

    #[cfg(windows)]
    fn stop_service(&self, service_name: &str) -> Result<()> {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows::core::PCWSTR;
        use windows::Win32::System::Services::{
            CloseServiceHandle, ControlService, OpenSCManagerW, OpenServiceW, SC_MANAGER_CONNECT,
            SERVICE_ALL_ACCESS, SERVICE_CONTROL_STOP,
        };

        let name_wide: Vec<u16> = OsStr::new(service_name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT)
                .map_err(|e| WireSentinelError::Vpn(format!("OpenSCManagerW: {e}")))?;
            let service = OpenServiceW(scm, PCWSTR(name_wide.as_ptr()), SERVICE_ALL_ACCESS)
                .map_err(|e| WireSentinelError::Vpn(format!("OpenServiceW: {e}")))?;
            let mut status = std::mem::zeroed();
            ControlService(service, SERVICE_CONTROL_STOP, &mut status).ok();
            let _ = CloseServiceHandle(service);
            let _ = CloseServiceHandle(scm);
        }
        Ok(())
    }

    #[cfg(windows)]
    fn query_service_running(&self, service_name: &str) -> bool {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows::core::PCWSTR;
        use windows::Win32::System::Services::{
            CloseServiceHandle, OpenSCManagerW, OpenServiceW, QueryServiceStatus,
            SC_MANAGER_CONNECT, SERVICE_ALL_ACCESS, SERVICE_RUNNING,
        };

        let name_wide: Vec<u16> = OsStr::new(service_name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let Ok(scm) = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT) else {
                return false;
            };
            let Ok(service) = OpenServiceW(scm, PCWSTR(name_wide.as_ptr()), SERVICE_ALL_ACCESS)
            else {
                let _ = CloseServiceHandle(scm);
                return false;
            };
            let mut status = std::mem::zeroed();
            let running = QueryServiceStatus(service, &mut status)
                .ok()
                .map(|_| status.dwCurrentState == SERVICE_RUNNING)
                .unwrap_or(false);
            let _ = CloseServiceHandle(service);
            let _ = CloseServiceHandle(scm);
            running
        }
    }

    async fn config_path_with_handshake_proxy(&self, profile: &VPNProfile) -> Result<PathBuf> {
        if profile
            .handshake_proxy
            .as_ref()
            .map(|s| s.enabled)
            .unwrap_or(false)
        {
            let content = std::fs::read_to_string(&profile.config_path)
                .map_err(|e| WireSentinelError::Vpn(format!("read config: {e}")))?;
            let mut config = crate::conf::parse_conf(&content);
            let _session =
                crate::handshake_proxy::apply_handshake_proxy(profile, &mut config, None).await?;
            let temp =
                crate::materialize::vpn_config_dir().join(format!("handshake-{}.conf", profile.id));
            let rendered = crate::conf::write_conf(&config);
            std::fs::write(&temp, rendered)
                .map_err(|e| WireSentinelError::Vpn(format!("write temp config: {e}")))?;
            return Ok(temp);
        }
        Ok(profile.config_path.clone())
    }
}

#[async_trait]
impl VpnBackend for ScmTunnelDllBackend {
    async fn connect(&self, profile: &VPNProfile) -> Result<()> {
        let service_name = Self::service_name(&profile.name);
        info!(name = %profile.name, service = %service_name, "connecting VPN via SCM");

        self.states
            .write()
            .insert(profile.id, VpnStatus::Connecting);

        let config_path = self.config_path_with_handshake_proxy(profile).await?;

        #[cfg(windows)]
        {
            self.install_and_start_service(&service_name, &config_path)?;
        }
        #[cfg(not(windows))]
        {
            warn!("SCM VPN connect is only supported on Windows");
        }

        self.service_names.write().insert(profile.id, service_name);
        self.states.write().insert(profile.id, VpnStatus::Connected);
        Ok(())
    }

    async fn disconnect(&self, profile_id: Uuid) -> Result<()> {
        self.states
            .write()
            .insert(profile_id, VpnStatus::Disconnecting);

        if let Some(name) = self.service_names.write().remove(&profile_id) {
            #[cfg(windows)]
            self.stop_service(&name)?;
        }

        self.states
            .write()
            .insert(profile_id, VpnStatus::Disconnected);
        Ok(())
    }

    async fn status(&self, profile_id: Uuid) -> VpnStatus {
        if let Some(name) = self.service_names.read().get(&profile_id) {
            #[cfg(windows)]
            {
                if self.query_service_running(name) {
                    return VpnStatus::Connected;
                }
                return VpnStatus::Disconnected;
            }
        }
        self.states
            .read()
            .get(&profile_id)
            .copied()
            .unwrap_or(VpnStatus::Disconnected)
    }

    async fn stats(&self, _profile_id: Uuid) -> VpnStats {
        VpnStats::default()
    }

    async fn list_active(&self) -> Vec<Uuid> {
        self.states
            .read()
            .iter()
            .filter(|(_, s)| **s == VpnStatus::Connected)
            .map(|(id, _)| *id)
            .collect()
    }
}
