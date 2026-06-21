//! Windows wireguard.dll FFI backend.

use crate::backend::VpnBackend;
use crate::conf::{read_conf_file, WireGuardConfig};
use async_trait::async_trait;
use libloading::Library;
use parking_lot::Mutex;
use shared_types::{Result, VPNProfile, VpnStats, VpnStatus, WireSentinelError};
use std::collections::HashMap;
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

pub const WIREGUARD_KEY_LENGTH: usize = 32;

type WireGuardAdapterHandleRaw = *mut c_void;

#[derive(Copy, Clone)]
struct WireGuardAdapterHandle(WireGuardAdapterHandleRaw);
// SAFETY: opaque WireGuard adapter handles are kernel-owned; used only under Mutex.
unsafe impl Send for WireGuardAdapterHandle {}
unsafe impl Sync for WireGuardAdapterHandle {}

impl WireGuardAdapterHandle {
    fn is_null(self) -> bool {
        self.0.is_null()
    }

    fn raw(self) -> WireGuardAdapterHandleRaw {
        self.0
    }
}

type CreateAdapterFn =
    unsafe extern "system" fn(*const u16, *const u16, *const u8) -> WireGuardAdapterHandleRaw;
type CloseAdapterFn = unsafe extern "system" fn(WireGuardAdapterHandleRaw);
type SetConfigurationFn =
    unsafe extern "system" fn(WireGuardAdapterHandleRaw, *const u8, u32) -> i32;
type SetAdapterStateFn = unsafe extern "system" fn(WireGuardAdapterHandleRaw, u32) -> i32;
type GetAdapterStateFn = unsafe extern "system" fn(WireGuardAdapterHandleRaw, *mut u32) -> i32;
type GetDriverVersionFn = unsafe extern "system" fn() -> u32;

const WIREGUARD_ADAPTER_STATE_UP: u32 = 1;

struct WireGuardDll {
    _lib: Library,
    create_adapter: CreateAdapterFn,
    close_adapter: CloseAdapterFn,
    set_configuration: SetConfigurationFn,
    set_adapter_state: SetAdapterStateFn,
    get_adapter_state: GetAdapterStateFn,
    get_driver_version: GetDriverVersionFn,
}

impl WireGuardDll {
    fn load(path: &Path) -> Result<Self> {
        let lib = unsafe { Library::new(path) }.map_err(|e| {
            WireSentinelError::Vpn(format!("load wireguard.dll from {}: {e}", path.display()))
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
    WireSentinelError::Vpn(format!("wireguard.dll symbol: {e}"))
}

struct AdapterState {
    handle: WireGuardAdapterHandle,
    name: String,
}

pub struct NativeWireGuardBackend {
    dll: Arc<WireGuardDll>,
    adapters: Mutex<HashMap<Uuid, AdapterState>>,
}

impl NativeWireGuardBackend {
    pub fn new(dll_path: PathBuf) -> Result<Self> {
        let path = resolve_dll_path(&dll_path);
        let dll = Arc::new(WireGuardDll::load(&path)?);
        let ver = dll.driver_version();
        if ver == 0 {
            warn!("wireguard driver not loaded (version 0)");
        } else {
            info!(version = ver, "wireguard-nt driver loaded");
        }
        Ok(Self {
            dll,
            adapters: Mutex::new(HashMap::new()),
        })
    }

    pub fn create_adapter(&self, name: &str) -> Result<WireGuardAdapterHandle> {
        let wide_name = str_to_wide(name);
        let tunnel_type = str_to_wide("WireGuard");
        let handle = unsafe {
            (self.dll.create_adapter)(wide_name.as_ptr(), tunnel_type.as_ptr(), std::ptr::null())
        };
        if handle.is_null() {
            return Err(wg_err("WireGuardCreateAdapter failed"));
        }
        Ok(WireGuardAdapterHandle(handle))
    }

    pub fn delete_adapter(&self, handle: WireGuardAdapterHandle) {
        if !handle.is_null() {
            unsafe { (self.dll.close_adapter)(handle.raw()) };
        }
    }

    pub fn set_config(
        &self,
        handle: WireGuardAdapterHandle,
        config: &WireGuardConfig,
    ) -> Result<()> {
        let blob = encode_config(config)?;
        let ok =
            unsafe { (self.dll.set_configuration)(handle.raw(), blob.as_ptr(), blob.len() as u32) };
        if ok == 0 {
            return Err(wg_err("WireGuardSetConfiguration failed"));
        }
        Ok(())
    }

    fn set_state(&self, handle: WireGuardAdapterHandle, up: bool) -> Result<()> {
        let state = if up { WIREGUARD_ADAPTER_STATE_UP } else { 0 };
        let ok = unsafe { (self.dll.set_adapter_state)(handle.raw(), state) };
        if ok == 0 {
            return Err(wg_err("WireGuardSetAdapterState failed"));
        }
        Ok(())
    }

    fn get_state(&self, handle: WireGuardAdapterHandle) -> VpnStatus {
        let mut state = 0u32;
        let ok = unsafe { (self.dll.get_adapter_state)(handle.raw(), &mut state) };
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
            return Err(wg_err("native backend requires config file path on disk"));
        }
        read_conf_file(&profile.config_path).map_err(|e| wg_err(format!("read config: {e}")))
    }
}

#[async_trait]
impl VpnBackend for NativeWireGuardBackend {
    async fn connect(&self, profile: &VPNProfile) -> Result<()> {
        let mut config = Self::load_profile_config(profile)?;
        if config.interface.private_key.is_none() {
            return Err(wg_err("missing Interface PrivateKey"));
        }
        if config.peers.is_empty() {
            return Err(wg_err("missing [Peer] section"));
        }

        let _handshake =
            crate::handshake_proxy::apply_handshake_proxy(profile, &mut config, None).await?;

        let adapter_name = format!("WS{}", &profile.id.to_string()[..8]);
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
}

fn resolve_dll_path(configured: &Path) -> PathBuf {
    if configured.exists() {
        return configured.to_path_buf();
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("wireguard.dll");
            if candidate.exists() {
                return candidate;
            }
        }
    }
    PathBuf::from("wireguard.dll")
}

fn str_to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn wg_err(msg: impl Into<String>) -> WireSentinelError {
    WireSentinelError::Vpn(msg.into())
}

fn encode_config(config: &WireGuardConfig) -> Result<Vec<u8>> {
    use std::mem;

    const FLAG_HAS_PRIVATE_KEY: u32 = 1 << 0;
    const FLAG_HAS_LISTEN_PORT: u32 = 1 << 1;
    const FLAG_HAS_PUBLIC_KEY: u32 = 1 << 0;
    const FLAG_HAS_ENDPOINT: u32 = 1 << 2;
    const FLAG_HAS_PERSISTENT_KEEPALIVE: u32 = 1 << 4;

    let private_key = decode_key(
        config
            .interface
            .private_key
            .as_deref()
            .ok_or_else(|| wg_err("missing private key"))?,
    )?;
    let peer = config.peers.first().ok_or_else(|| wg_err("missing peer"))?;
    let public_key = decode_key(
        peer.public_key
            .as_deref()
            .ok_or_else(|| wg_err("missing peer public key"))?,
    )?;

    let mut flags = FLAG_HAS_PRIVATE_KEY;
    let listen_port = config.interface.listen_port.unwrap_or(0);
    if listen_port > 0 {
        flags |= FLAG_HAS_LISTEN_PORT;
    }

    let mut peer_flags = FLAG_HAS_PUBLIC_KEY;
    let endpoint = peer.endpoint.clone().unwrap_or_default();
    let endpoint_wide = str_to_wide(&endpoint);
    if !endpoint.is_empty() {
        peer_flags |= FLAG_HAS_ENDPOINT;
    }
    let keepalive = peer.persistent_keepalive.unwrap_or(0);
    if keepalive > 0 {
        peer_flags |= FLAG_HAS_PERSISTENT_KEEPALIVE;
    }

    let peer_size = mem::size_of::<u32>() * 3
        + WIREGUARD_KEY_LENGTH
        + endpoint_wide.len() * 2
        + mem::size_of::<u16>();
    let total =
        mem::size_of::<u32>() * 4 + WIREGUARD_KEY_LENGTH + mem::size_of::<u32>() + peer_size;

    let mut blob = Vec::with_capacity(total);
    blob.extend_from_slice(&flags.to_le_bytes());
    blob.extend_from_slice(&2u16.to_le_bytes());
    blob.extend_from_slice(&1u16.to_le_bytes());
    blob.extend_from_slice(&private_key);
    blob.extend_from_slice(&listen_port.to_le_bytes());
    blob.extend_from_slice(&(1u32).to_le_bytes());
    blob.extend_from_slice(&peer_flags.to_le_bytes());
    blob.extend_from_slice(&public_key);
    if !endpoint.is_empty() {
        for w in &endpoint_wide {
            blob.extend_from_slice(&w.to_le_bytes());
        }
    }
    blob.extend_from_slice(&keepalive.to_le_bytes());
    Ok(blob)
}

fn decode_key(b64: &str) -> Result<[u8; WIREGUARD_KEY_LENGTH]> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|e| wg_err(format!("invalid base64 key: {e}")))?;
    if bytes.len() != WIREGUARD_KEY_LENGTH {
        return Err(wg_err(format!(
            "key length {} expected {}",
            bytes.len(),
            WIREGUARD_KEY_LENGTH
        )));
    }
    let mut out = [0u8; WIREGUARD_KEY_LENGTH];
    out.copy_from_slice(&bytes);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_key_validates_length() {
        assert!(decode_key("aaaa").is_err());
    }
}
