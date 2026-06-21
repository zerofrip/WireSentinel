use shared_types::WireSentinelError;
use std::path::PathBuf;

/// Resolve executable path for a process ID.
pub fn exe_path_for_pid(pid: u32) -> Result<PathBuf, WireSentinelError> {
    #[cfg(windows)]
    {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use windows::Win32::Foundation::{CloseHandle, MAX_PATH};
        use windows::Win32::System::ProcessStatus::K32GetModuleFileNameExW;
        use windows::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
        };

        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid)
                .map_err(|e| WireSentinelError::Other(format!("OpenProcess: {e}")))?;
            let mut buf = vec![0u16; MAX_PATH as usize];
            let len = K32GetModuleFileNameExW(handle, None, &mut buf);
            let _ = CloseHandle(handle);
            if len == 0 {
                return Err(WireSentinelError::Other(format!(
                    "no exe path for pid {pid}"
                )));
            }
            buf.truncate(len as usize);
            return Ok(PathBuf::from(OsString::from_wide(&buf)));
        }
    }

    #[cfg(not(windows))]
    {
        let path = format!("/proc/{pid}/exe");
        std::fs::read_link(&path)
            .map_err(|e| WireSentinelError::Io(e))
            .or_else(|_| Ok(PathBuf::from(format!("unknown-{pid}.exe"))))
    }
}
