//! WFP engine exports RouteEnforcer for split-tunnel Phase 2.

mod engine;
mod stub;

#[cfg(windows)]
mod debug_agent;

#[cfg(windows)]
mod kernel;

#[cfg(windows)]
mod userspace;

mod hybrid;
mod ndis;

#[cfg(all(windows, feature = "signed-stack"))]
mod windivert_ndis;

pub use engine::{RouteEnforcer, WfpEngine, WfpEvent, WfpEventKind};
pub use hybrid::HybridWfpEngine;
pub use ndis::{
    create_ndis_engine, ndis_driver_available, NdisCalloutEngine, NdisEngine, StubNdisEngine,
};

#[cfg(windows)]
pub use kernel::KernelCalloutEngine;

#[cfg(not(windows))]
pub use stub::KernelCalloutEngineStub as KernelCalloutEngine;

#[cfg(windows)]
pub use userspace::UserspaceWfpEngine;

use proxy_engine::ProxyListenPort;
use std::sync::Arc;

/// Probe whether Guardian.sys is loaded and reachable.
#[cfg(windows)]
pub fn kernel_driver_available() -> Result<String, String> {
    use guardian_controller::GuardianClient;
    let client = GuardianClient::connect().map_err(|e| e.to_string())?;
    let state = client.driver_state().map_err(|e| e.to_string())?;
    let major = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(state.version_major)) };
    let minor = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(state.version_minor)) };
    let patch = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(state.version_patch)) };
    let lifecycle = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(state.lifecycle_state)) };
    Ok(format!(
        "Guardian driver v{major}.{minor}.{patch} state={lifecycle}"
    ))
}

#[cfg(not(windows))]
pub fn kernel_driver_available() -> Result<String, String> {
    Err("kernel WFP requires Windows".into())
}

#[cfg(windows)]
pub type WindowsWfpEngine = UserspaceWfpEngine;

#[cfg(not(windows))]
pub use stub::StubWfpEngine as UserspaceWfpEngine;

fn build_base_wfp_engine(
    impl_name: &str,
    listen_ports: Arc<ProxyListenPort>,
) -> Arc<dyn WfpEngine> {
    match impl_name {
        "kernel" => {
            let engine = KernelCalloutEngine::new();
            engine.set_listen_ports(listen_ports);
            Arc::new(engine)
        }
        _ => Arc::new(UserspaceWfpEngine::new()),
    }
}

/// Create the active WFP engine from [`EnforcementBackend`] setting.
pub fn create_wfp_engine(
    enforcement_backend: &str,
    listen_ports: Arc<ProxyListenPort>,
) -> Arc<dyn WfpEngine> {
    let mapping = shared_types::EnforcementMapping::from_backend(
        shared_types::EnforcementBackend::parse(enforcement_backend),
    );
    create_wfp_engine_inner(
        mapping.guardian_mode.as_str(),
        mapping.wfp_engine_impl,
        mapping.use_windivert,
        listen_ports,
    )
}

fn create_wfp_engine_inner(
    guardian_mode: &str,
    impl_name: &str,
    use_windivert: bool,
    listen_ports: Arc<ProxyListenPort>,
) -> Arc<dyn WfpEngine> {
    let base = build_base_wfp_engine(impl_name, Arc::clone(&listen_ports));
    match guardian_mode {
        "hybrid" => {
            let ndis = create_ndis_engine(use_windivert);
            Arc::new(HybridWfpEngine::new(base, ndis))
        }
        "ndis" => {
            let ndis = create_ndis_engine(use_windivert);
            Arc::new(HybridWfpEngine::ndis_primary(base, ndis))
        }
        _ if use_windivert => {
            let ndis = create_ndis_engine(true);
            Arc::new(HybridWfpEngine::ndis_primary(base, ndis))
        }
        _ => base,
    }
}

/// Legacy entry point (guardian_mode + wfp_engine_impl only).
pub fn create_wfp_engine_legacy(
    guardian_mode: &str,
    impl_name: &str,
    listen_ports: Arc<ProxyListenPort>,
) -> Arc<dyn WfpEngine> {
    create_wfp_engine_inner(guardian_mode, impl_name, false, listen_ports)
}
