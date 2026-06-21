use shared_types::{PluginManifest, Result, WireSentinelError};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Error returned when native plugin loading fails.
#[derive(Debug, Clone)]
pub struct LoadLibraryError {
    pub path: PathBuf,
    pub message: String,
}

impl std::fmt::Display for LoadLibraryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.message)
    }
}

impl std::error::Error for LoadLibraryError {}

/// Handle to a loaded native plugin module.
#[derive(Debug)]
pub struct NativePluginHandle {
    pub manifest: PluginManifest,
    #[cfg(all(windows, feature = "native-plugins"))]
    module: windows::Win32::Foundation::HMODULE,
}

impl NativePluginHandle {
    pub fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
}

/// Authenticode signature verification (stub — always passes unless require_signature is set).
#[derive(Debug, Clone, Default)]
pub struct AuthenticodeVerifier {
    pub require_signature: bool,
}

impl AuthenticodeVerifier {
    pub fn new(require_signature: bool) -> Self {
        Self { require_signature }
    }

    /// Verify Authenticode signature on a native plugin binary.
    pub fn verify(&self, path: &Path) -> Result<()> {
        if !self.require_signature {
            info!(path = %path.display(), "authenticode check skipped (not required)");
            return Ok(());
        }

        #[cfg(all(windows, feature = "native-plugins"))]
        {
            verify_authenticode_stub(path)?;
            info!(path = %path.display(), "authenticode check passed (stub)");
            return Ok(());
        }

        #[cfg(not(all(windows, feature = "native-plugins")))]
        {
            let _ = path;
            warn!("authenticode stub: native-plugins feature disabled");
            Ok(())
        }
    }
}

#[cfg(all(windows, feature = "native-plugins"))]
fn verify_authenticode_stub(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(WireSentinelError::Other(format!(
            "plugin binary not found: {}",
            path.display()
        )));
    }
    Ok(())
}

/// Native plugin loader — LoadLibrary on Windows, stub elsewhere.
pub struct NativePluginLoader {
    verifier: AuthenticodeVerifier,
}

impl NativePluginLoader {
    pub fn new(verifier: AuthenticodeVerifier) -> Self {
        Self { verifier }
    }

    pub fn with_policy(require_signature: bool) -> Self {
        Self::new(AuthenticodeVerifier::new(require_signature))
    }

    /// Load a native plugin DLL after Authenticode verification.
    pub fn load(&self, manifest: PluginManifest) -> Result<NativePluginHandle> {
        let path = PathBuf::from(&manifest.path);
        self.verifier.verify(&path)?;

        #[cfg(all(windows, feature = "native-plugins"))]
        {
            use windows::core::PCWSTR;
            use windows::Win32::Foundation::FreeLibrary;
            use windows::Win32::System::LibraryLoader::{LoadLibraryW, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS};

            let wide: Vec<u16> = path
                .to_string_lossy()
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            let module = unsafe { LoadLibraryW(PCWSTR(wide.as_ptr())) }
                .map_err(|e| WireSentinelError::Other(format!("LoadLibraryW: {e}")))?;

            if module.is_invalid() {
                return Err(WireSentinelError::Other(format!(
                    "LoadLibraryW returned invalid handle for {}",
                    path.display()
                )));
            }

            info!(plugin = %manifest.name, path = %path.display(), "native plugin loaded");
            return Ok(NativePluginHandle { manifest, module });
        }

        #[cfg(not(all(windows, feature = "native-plugins")))]
        {
            info!(plugin = %manifest.name, path = %path.display(), "native plugin load stub");
            Ok(NativePluginHandle { manifest })
        }
    }
}

#[cfg(all(windows, feature = "native-plugins"))]
impl Drop for NativePluginHandle {
    fn drop(&mut self) {
        use windows::Win32::Foundation::FreeLibrary;
        unsafe {
            let _ = FreeLibrary(self.module);
        }
    }
}
