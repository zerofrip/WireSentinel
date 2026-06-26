//! WinDivert sniff-mode packet capture for connection detection.

#[cfg(windows)]
mod imp {
    use crate::ffi::{
        WinDivertAddressMeta, WinDivertHandle, WinDivertLib, WINDIVERT_ADDRESS_SIZE,
        WINDIVERT_FLAG_SNIFF, WINDIVERT_LAYER_NETWORK,
    };
    use crate::redirect::windivert_dll_path;
    use std::path::PathBuf;

    pub struct CapturedPacket {
        pub data: Vec<u8>,
        pub meta: WinDivertAddressMeta,
    }

    pub struct WinDivertCapture {
        lib: WinDivertLib,
        handle: WinDivertHandle,
    }

    impl WinDivertCapture {
        pub fn open_sniff(filter: &str) -> Result<Self, String> {
            let path = windivert_dll_path();
            let lib = WinDivertLib::load(&path)?;
            let handle = lib.open(
                filter,
                WINDIVERT_LAYER_NETWORK,
                0,
                WINDIVERT_FLAG_SNIFF,
            )?;
            Ok(Self { lib, handle })
        }

        pub fn recv_blocking(&self) -> Result<CapturedPacket, String> {
            let mut packet = vec![0u8; 0xFFFF];
            let mut addr = vec![0u8; WINDIVERT_ADDRESS_SIZE];
            let len = self.lib.recv(self.handle, &mut packet, &mut addr)?;
            packet.truncate(len as usize);
            Ok(CapturedPacket {
                data: packet,
                meta: WinDivertAddressMeta::from_bytes(&addr),
            })
        }
    }

    impl Drop for WinDivertCapture {
        fn drop(&mut self) {
            self.lib.close(self.handle);
        }
    }

    pub fn capture_available() -> Result<PathBuf, String> {
        let path = windivert_dll_path();
        if !path.exists() {
            return Err(format!("WinDivert.dll not found at {}", path.display()));
        }
        let _probe = WinDivertCapture::open_sniff("false")?;
        Ok(path)
    }
}

#[cfg(windows)]
pub use imp::{capture_available, CapturedPacket, WinDivertCapture};

#[cfg(not(windows))]
mod stub {
    use std::path::PathBuf;

    pub struct CapturedPacket {
        pub data: Vec<u8>,
    }

    pub struct WinDivertCapture;

    impl WinDivertCapture {
        pub fn open_sniff(_filter: &str) -> Result<Self, String> {
            Err("WinDivert capture is only available on Windows".into())
        }

        pub fn recv_blocking(&self) -> Result<CapturedPacket, String> {
            Err("WinDivert capture is only available on Windows".into())
        }
    }

    pub fn capture_available() -> Result<PathBuf, String> {
        Err("WinDivert capture is only available on Windows".into())
    }
}

#[cfg(not(windows))]
pub use stub::{capture_available, CapturedPacket, WinDivertCapture};
