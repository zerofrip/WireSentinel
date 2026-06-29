#![allow(non_camel_case_types)]

use std::ffi::c_void;
use std::path::Path;

pub type WinDivertHandle = isize;

pub const WINDIVERT_LAYER_NETWORK: i32 = 0;
pub const WINDIVERT_FLAG_SNIFF: u64 = 0x0001;
pub const WINDIVERT_ADDRESS_SIZE: usize = 80;

type WinDivertOpenFn = unsafe extern "system" fn(
    filter: *const u16,
    layer: i32,
    priority: i16,
    flags: u64,
) -> WinDivertHandle;

type WinDivertCloseFn = unsafe extern "system" fn(handle: WinDivertHandle) -> i32;

type WinDivertRecvFn = unsafe extern "system" fn(
    handle: WinDivertHandle,
    packet: *mut u8,
    packet_len: u32,
    recv_len: *mut u32,
    addr: *mut u8,
) -> i32;

type WinDivertSendFn = unsafe extern "system" fn(
    handle: WinDivertHandle,
    packet: *const u8,
    size: u32,
    addr: *const u8,
    addr_len: *mut u32,
) -> i32;

/// Parsed metadata from a WinDivert recv address buffer.
#[derive(Debug, Clone, Copy, Default)]
pub struct WinDivertAddressMeta {
    pub outbound: bool,
    pub ipv6: bool,
    pub process_id: u32,
}

impl WinDivertAddressMeta {
    pub fn from_bytes(addr: &[u8]) -> Self {
        if addr.len() < 24 {
            return Self::default();
        }
        let flags = u32::from_le_bytes(addr[8..12].try_into().unwrap_or([0; 4]));
        let outbound = (flags >> 17) & 1 == 1;
        let ipv6 = (flags >> 20) & 1 == 1;
        let process_id = u32::from_le_bytes(addr[20..24].try_into().unwrap_or([0; 4]));
        Self {
            outbound,
            ipv6,
            process_id,
        }
    }
}

pub struct WinDivertLib {
    _lib: libloading::Library,
    open: WinDivertOpenFn,
    close: WinDivertCloseFn,
    recv: WinDivertRecvFn,
    send: WinDivertSendFn,
}

impl WinDivertLib {
    pub fn load(path: &Path) -> Result<Self, String> {
        unsafe {
            let lib = libloading::Library::new(path).map_err(|e| e.to_string())?;
            let open: WinDivertOpenFn = *lib.get(b"WinDivertOpen\0").map_err(|e| e.to_string())?;
            let close: WinDivertCloseFn =
                *lib.get(b"WinDivertClose\0").map_err(|e| e.to_string())?;
            let recv: WinDivertRecvFn = *lib.get(b"WinDivertRecv\0").map_err(|e| e.to_string())?;
            let send: WinDivertSendFn = *lib.get(b"WinDivertSend\0").map_err(|e| e.to_string())?;
            Ok(Self {
                _lib: lib,
                open,
                close,
                recv,
                send,
            })
        }
    }

    pub fn open(
        &self,
        filter: &str,
        layer: i32,
        priority: i16,
        flags: u64,
    ) -> Result<WinDivertHandle, String> {
        let wide: Vec<u16> = filter.encode_utf16().chain(std::iter::once(0)).collect();
        let handle = unsafe { (self.open)(wide.as_ptr(), layer, priority, flags) };
        if handle == -1 {
            return Err("WinDivertOpen returned INVALID_HANDLE".into());
        }
        Ok(handle)
    }

    pub fn close(&self, handle: WinDivertHandle) {
        unsafe {
            (self.close)(handle);
        }
    }

    pub fn recv(
        &self,
        handle: WinDivertHandle,
        packet: &mut [u8],
        addr: &mut [u8],
    ) -> Result<u32, String> {
        let mut recv_len = 0u32;
        let ok = unsafe {
            (self.recv)(
                handle,
                packet.as_mut_ptr(),
                packet.len() as u32,
                &mut recv_len,
                addr.as_mut_ptr(),
            )
        };
        if ok == 0 || recv_len == 0 {
            return Err("WinDivertRecv returned no data".into());
        }
        Ok(recv_len)
    }

    pub fn send(&self, handle: &WinDivertHandle, packet: &[u8]) -> Result<(), String> {
        let sent = unsafe {
            (self.send)(
                *handle,
                packet.as_ptr(),
                packet.len() as u32,
                std::ptr::null(),
                std::ptr::null_mut(),
            )
        };
        if sent <= 0 {
            return Err("WinDivertSend failed".into());
        }
        Ok(())
    }
}

impl Drop for WinDivertLib {
    fn drop(&mut self) {
        let _ = self as *const _ as *const c_void;
    }
}
