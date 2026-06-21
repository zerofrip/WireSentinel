//! Native AmneziaWG backend via amnezia-tunnel.dll (Windows) or stub (non-Windows).

#[cfg(windows)]
pub use imp::NativeAmneziaWgBackend;

#[cfg(not(windows))]
pub use stub::NativeAmneziaWgBackend;

#[cfg(windows)]
mod imp {
    use crate::backend::VpnBackend;
    use crate::conf::{encode_awg_config, parse_conf, read_conf_file, WireGuardConfig};
    use async_trait::async_trait;
    use libloading::Library;
    use parking_lot::Mutex;
    use shared_types::{Result, TunnelIface, VPNProfile, VpnStats, VpnStatus, WireSentinelError};
    use std::collections::HashMap;
    use std::ffi::c_void;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use tracing::{info, warn};
    use uuid::Uuid;

    type WireGuardAdapterHandle = *mut c_void;

    type CreateAdapterFn =
        unsafe extern "system" fn(*const u16, *const u16, *const u8) -> WireGuardAdapterHandle;
    type CloseAdapterFn = unsafe extern "system" fn(WireGuardAdapterHandle);
    type SetConfigurationFn =
        unsafe extern "system" fn(WireGuardAdapterHandle, *const u8, u32) -> i32;
    type SetAdapterStateFn = unsafe extern "system" fn(WireGuardAdapterHandle, u32) -> i32;
    type GetAdapterStateFn = unsafe extern "system" fn(WireGuardAdapterHandle, *mut u32) -> i32;
    type GetDriverVersionFn = unsafe extern "system" fn() -> u32;

    const WIREGUARD_ADAPTER_STATE_UP: u32 = 1;

    struct AmneziaTunnelDll {
        _lib: Library,
        create_adapter: CreateAdapterFn,
        close_adapter: CloseAdapterFn,
        set_configuration: SetConfigurationFn,
        set_adapter_state: SetAdapterStateFn,
        get_adapter_state: GetAdapterStateFn,
        get_driver_version: GetDriverVersionFn,
    }

    impl AmneziaTunnelDll {
        fn load(path: &Path) -> Result<Self> {
            let lib = unsafe { Library::new(path) }.map_err(|e| {
                WireSentinelError::Vpn(format!(
                    "load amnezia-tunnel.dll from {}: {e}",
                    path.display()
                ))
            })?;
            unsafe {
                Ok(Self {
                    create_adapter: *lib.get(b"WireGuardCreateAdapter\0").map_err(map_sym)?,
                    close_adapter: *lib.get(b"WireGuardCloseAdapter\0").map_err(map_sym)?,
                    set_configuration: *lib.get(b"WireGuardSetConfiguration\0").map_err(map_sym)?,
                    set_adapter_state: *lib.get(b"WireGuardSetAdapterState\0").map_err(map_sym)?,
                    get_adapter_state: *lib.get(b"WireGuardGetAdapterState\0").map_err(map_sym)?,
                    get_driver_version: *lib
                        .get(b"WireGuardGetRunningDriverVersion\0")
                        .map_err(map_sym)?,
                    _lib: lib,
                })
            }
        }

        fn driver_version(&self) -> u32 {
            unsafe { (self.get_driver_version)() }
        }
    }

    fn map_sym(e: libloading::Error) -> WireSentinelError {
        WireSentinelError::Vpn(format!("amnezia-tunnel.dll symbol: {e}"))
    }

    struct AdapterState {
        handle: WireGuardAdapterHandle,
        name: String,
    }

    pub struct NativeAmneziaWgBackend {
        dll: Arc<AmneziaTunnelDll>,
        adapters: Mutex<HashMap<Uuid, AdapterState>>,
    }

    impl NativeAmneziaWgBackend {
        pub fn new(dll_path: PathBuf) -> Result<Self> {
            let path = resolve_dll_path(&dll_path);
            let dll = Arc::new(AmneziaTunnelDll::load(&path)?);
            let ver = dll.driver_version();
            if ver == 0 {
                warn!("amnezia wireguard driver not loaded (version 0)");
            } else {
                info!(version = ver, "amnezia-tunnel driver loaded");
            }
            Ok(Self {
                dll,
                adapters: Mutex::new(HashMap::new()),
            })
        }

        fn create_adapter(&self, name: &str) -> Result<WireGuardAdapterHandle> {
            let wide_name = str_to_wide(name);
            let tunnel_type = str_to_wide("AmneziaWG");
            let handle = unsafe {
                (self.dll.create_adapter)(
                    wide_name.as_ptr(),
                    tunnel_type.as_ptr(),
                    std::ptr::null(),
                )
            };
            if handle.is_null() {
                return Err(awg_err("WireGuardCreateAdapter failed"));
            }
            Ok(handle)
        }

        fn delete_adapter(&self, handle: WireGuardAdapterHandle) {
            if !handle.is_null() {
                unsafe { (self.dll.close_adapter)(handle) };
            }
        }

        fn set_config(
            &self,
            handle: WireGuardAdapterHandle,
            config: &WireGuardConfig,
        ) -> Result<()> {
            let blob = encode_awg_config(config)?;
            let ok =
                unsafe { (self.dll.set_configuration)(handle, blob.as_ptr(), blob.len() as u32) };
            if ok == 0 {
                return Err(awg_err("WireGuardSetConfiguration failed"));
            }
            Ok(())
        }

        fn set_state(&self, handle: WireGuardAdapterHandle, up: bool) -> Result<()> {
            let state = if up { WIREGUARD_ADAPTER_STATE_UP } else { 0 };
            let ok = unsafe { (self.dll.set_adapter_state)(handle, state) };
            if ok == 0 {
                return Err(awg_err("WireGuardSetAdapterState failed"));
            }
            Ok(())
        }

        fn get_state(&self, handle: WireGuardAdapterHandle) -> VpnStatus {
            let mut state = 0u32;
            let ok = unsafe { (self.dll.get_adapter_state)(handle, &mut state) };
            if ok == 0 {
                return VpnStatus::Error;
            }
            if state == WIREGUARD_ADAPTER_STATE_UP {
                VpnStatus::Connected
            } else {
                VpnStatus::Disconnected
            }
        }

        fn load_profile_config(profile: &VPNProfile) -> Result<WireGuardConfig> {
            if profile.config_path.to_string_lossy().starts_with("db://") {
                return Err(awg_err("native backend requires config file path on disk"));
            }
            let content = read_conf_file(&profile.config_path)
                .map_err(|e| awg_err(format!("read config: {e}")))?;
            Ok(parse_conf(&content))
        }
    }

    #[async_trait]
    impl VpnBackend for NativeAmneziaWgBackend {
        async fn connect(&self, profile: &VPNProfile) -> Result<()> {
            let content = std::fs::read_to_string(&profile.config_path)
                .map_err(|e| awg_err(format!("read config: {e}")))?;
            crate::amnezia::AmneziaWgBackend::validate_config(&content)?;

            let mut config = Self::load_profile_config(profile)?;
            if config.interface.private_key.is_none() {
                return Err(awg_err("missing Interface PrivateKey"));
            }
            if config.peers.is_empty() {
                return Err(awg_err("missing [Peer] section"));
            }

            let _handshake =
                crate::handshake_proxy::apply_handshake_proxy(profile, &mut config, None).await?;

            let adapter_name = format!("WSA{}", &profile.id.to_string()[..8]);
            let handle = self.create_adapter(&adapter_name)?;
            if let Err(e) = self.set_config(handle, &config) {
                self.delete_adapter(handle);
                return Err(e);
            }
            if let Err(e) = self.set_state(handle, true) {
                self.delete_adapter(handle);
                return Err(e);
            }

            self.adapters.lock().insert(
                profile.id,
                AdapterState {
                    handle,
                    name: adapter_name,
                },
            );
            Ok(())
        }

        async fn disconnect(&self, profile_id: Uuid) -> Result<()> {
            if let Some(adapter) = self.adapters.lock().remove(&profile_id) {
                let _ = self.set_state(adapter.handle, false);
                self.delete_adapter(adapter.handle);
            }
            Ok(())
        }

        async fn status(&self, profile_id: Uuid) -> VpnStatus {
            self.adapters
                .lock()
                .get(&profile_id)
                .map(|a| self.get_state(a.handle))
                .unwrap_or(VpnStatus::Disconnected)
        }

        async fn stats(&self, _profile_id: Uuid) -> VpnStats {
            VpnStats::default()
        }

        async fn list_active(&self) -> Vec<Uuid> {
            self.adapters.lock().keys().copied().collect()
        }

        async fn tunnel_iface(&self, profile_id: Uuid) -> Option<TunnelIface> {
            self.adapters.lock().get(&profile_id).map(|a| TunnelIface {
                profile_id,
                name: a.name.clone(),
                luid: 0,
                socks_port: None,
            })
        }
    }

    fn resolve_dll_path(configured: &Path) -> PathBuf {
        if configured.exists() {
            return configured.to_path_buf();
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let candidate = dir.join("amnezia-tunnel.dll");
                if candidate.exists() {
                    return candidate;
                }
            }
        }
        PathBuf::from("amnezia-tunnel.dll")
    }

    fn str_to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn awg_err(msg: impl Into<String>) -> WireSentinelError {
        WireSentinelError::Vpn(msg.into())
    }
}

#[cfg(not(windows))]
mod stub {
    use crate::backend::VpnBackend;
    use async_trait::async_trait;
    use shared_types::{Result, VPNProfile, VpnStats, VpnStatus, WireSentinelError};
    use std::path::PathBuf;
    use uuid::Uuid;

    pub struct NativeAmneziaWgBackend {
        _dll_path: PathBuf,
    }

    impl NativeAmneziaWgBackend {
        pub fn new(dll_path: PathBuf) -> Result<Self> {
            Ok(Self {
                _dll_path: dll_path,
            })
        }
    }

    #[async_trait]
    impl VpnBackend for NativeAmneziaWgBackend {
        async fn connect(&self, _profile: &VPNProfile) -> Result<()> {
            Err(WireSentinelError::Vpn(
                "NativeAmneziaWgBackend requires Windows".into(),
            ))
        }

        async fn disconnect(&self, _profile_id: Uuid) -> Result<()> {
            Ok(())
        }

        async fn status(&self, _profile_id: Uuid) -> VpnStatus {
            VpnStatus::Disconnected
        }

        async fn stats(&self, _profile_id: Uuid) -> VpnStats {
            VpnStats::default()
        }

        async fn list_active(&self) -> Vec<Uuid> {
            Vec::new()
        }
    }
}
