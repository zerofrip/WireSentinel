//! WFP engine exports RouteEnforcer for split-tunnel Phase 2.

mod engine;
mod stub;

#[cfg(windows)]
mod kernel;

#[cfg(windows)]
mod userspace;

mod hybrid;
mod ndis;

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
    Ok(format!(
        "Guardian driver v{}.{}.{} state={}",
        state.version_major, state.version_minor, state.version_patch, state.lifecycle_state
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

/// Create the active WFP engine based on guardian mode and implementation setting.
pub fn create_wfp_engine(
    guardian_mode: &str,
    impl_name: &str,
    listen_ports: Arc<ProxyListenPort>,
) -> Arc<dyn WfpEngine> {
    let base = build_base_wfp_engine(impl_name, Arc::clone(&listen_ports));
    match guardian_mode {
        "hybrid" => {
            let ndis = create_ndis_engine();
            Arc::new(HybridWfpEngine::new(base, ndis))
        }
        "ndis" => {
            let ndis = create_ndis_engine();
            Arc::new(HybridWfpEngine::ndis_primary(base, ndis))
        }
        _ => base,
    }
}
