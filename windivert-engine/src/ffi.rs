#![allow(non_camel_case_types)]

use std::ffi::c_void;
use std::path::Path;

pub type WinDivertHandle = isize;

type WinDivertOpenFn = unsafe extern "system" fn(
    filter: *const u16,
    layer: i32,
    priority: i16,
    flags: u64,
) -> WinDivertHandle;

type WinDivertCloseFn = unsafe extern "system" fn(handle: WinDivertHandle) -> i32;
type WinDivertSendFn = unsafe extern "system" fn(
    handle: WinDivertHandle,
    packet: *const u8,
    size: u32,
    addr: *const u8,
    addr_len: *mut u32,
) -> i32;

pub struct WinDivertLib {
    _lib: libloading::Library,
    open: WinDivertOpenFn,
    close: WinDivertCloseFn,
    send: WinDivertSendFn,
}

impl WinDivertLib {
    pub fn load(path: &Path) -> Result<Self, String> {
        unsafe {
            let lib = libloading::Library::new(path).map_err(|e| e.to_string())?;
            let open: WinDivertOpenFn = *lib.get(b"WinDivertOpen\0").map_err(|e| e.to_string())?;
            let close: WinDivertCloseFn =
                *lib.get(b"WinDivertClose\0").map_err(|e| e.to_string())?;
            let send: WinDivertSendFn = *lib.get(b"WinDivertSend\0").map_err(|e| e.to_string())?;
            Ok(Self {
                _lib: lib,
                open,
                close,
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
        // handles closed by WinDivertEngine
        let _ = self as *const _ as *const c_void;
    }
}
