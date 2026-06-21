use crate::backend::VpnBackend;
use crate::tailscale::TailscaleBackend;
use shared_types::VPNProfile;
use std::path::PathBuf;
use std::sync::Arc;

pub use shared_types::VpnBackendKind;

/// Selects VPN backend impl based on profile.
pub struct VpnBackendFactory {
    wireguard: Arc<dyn VpnBackend>,
    amnezia: Arc<dyn VpnBackend>,
    tailscale: Arc<TailscaleBackend>,
}

impl VpnBackendFactory {
    pub fn new(
        wireguard: Arc<dyn VpnBackend>,
        amnezia: Arc<dyn VpnBackend>,
        tailscale: Arc<TailscaleBackend>,
    ) -> Self {
        Self {
            wireguard,
            amnezia,
            tailscale,
        }
    }

    pub fn for_profile(&self, profile: &VPNProfile) -> Arc<dyn VpnBackend> {
        match profile.backend {
            VpnBackendKind::AmneziaWg => Arc::clone(&self.amnezia),
            VpnBackendKind::WireGuardNt => Arc::clone(&self.wireguard),
            VpnBackendKind::Tailscale => Arc::clone(&self.tailscale) as Arc<dyn VpnBackend>,
        }
    }

    pub fn wireguard_backend(&self) -> Arc<dyn VpnBackend> {
        Arc::clone(&self.wireguard)
    }

    pub fn amnezia_backend(&self) -> Arc<dyn VpnBackend> {
        Arc::clone(&self.amnezia)
    }

    pub fn tailscale_backend(&self) -> Arc<TailscaleBackend> {
        Arc::clone(&self.tailscale)
    }
}

pub fn default_factory(
    service_exe: PathBuf,
    wireguard_impl: &str,
    amnezia_impl: &str,
    dll_path: PathBuf,
) -> VpnBackendFactory {
    #[cfg(windows)]
    {
        use crate::native::{NativeAmneziaWgBackend, NativeWireGuardBackend};
        use crate::scm::ScmTunnelDllBackend;

        let wg: Arc<dyn VpnBackend> = if wireguard_impl == "native" {
            match NativeWireGuardBackend::new(dll_path.clone()) {
                Ok(b) => Arc::new(b),
                Err(e) => {
                    tracing::warn!(err = %e, "native wireguard init failed, falling back to SCM");
                    Arc::new(ScmTunnelDllBackend::new(
                        PathBuf::from("tunnel.dll"),
                        service_exe.clone(),
                    ))
                }
            }
        } else {
            Arc::new(ScmTunnelDllBackend::new(
                PathBuf::from("tunnel.dll"),
                service_exe.clone(),
            ))
        };

        let awg: Arc<dyn VpnBackend> = if amnezia_impl == "native" {
            match NativeAmneziaWgBackend::new(PathBuf::from("amnezia-tunnel.dll")) {
                Ok(b) => Arc::new(b),
                Err(e) => {
                    tracing::warn!(err = %e, "native amnezia init failed, falling back to SCM");
                    Arc::new(ScmTunnelDllBackend::new(
                        PathBuf::from("amnezia-tunnel.dll"),
                        service_exe.clone(),
                    ))
                }
            }
        } else {
            Arc::new(ScmTunnelDllBackend::new(
                PathBuf::from("amnezia-tunnel.dll"),
                service_exe.clone(),
            ))
        };

        let tailscale = Arc::new(TailscaleBackend::new());

        VpnBackendFactory::new(wg, awg, tailscale)
    }

    #[cfg(not(windows))]
    {
        let _ = (service_exe, wireguard_impl, amnezia_impl, dll_path);
        let wg = Arc::new(crate::stub::StubVpnBackend::new()) as Arc<dyn VpnBackend>;
        let awg = Arc::new(crate::stub::StubVpnBackend::new()) as Arc<dyn VpnBackend>;
        let tailscale = Arc::new(TailscaleBackend::new());
        VpnBackendFactory::new(wg, awg, tailscale)
    }
}

pub fn default_dll_path() -> PathBuf {
    PathBuf::from("wireguard.dll")
}
