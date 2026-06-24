use crate::telemetry::WinDivertTelemetry;
use parking_lot::Mutex;
use std::sync::Arc;

/// Optional packet modification applied before reinject (DPI / padding).
pub type PacketTransformHook = Arc<dyn Fn(&mut [u8]) + Send + Sync>;

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::ffi::{WinDivertHandle, WinDivertLib};
    use async_trait::async_trait;
    use shared_types::{
        AppIdentity, NdisHealth, Result, TrafficRoute, TunnelIface, WireSentinelError,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;
    use uuid::Uuid;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct WinDivertHealth {
        pub available: bool,
        pub state: String,
        pub filter_attached: bool,
        pub active_route_count: u32,
        pub active_redirect_count: u32,
        pub classify_count: u64,
        pub error_count: u64,
        pub message: Option<String>,
        pub checked_at: chrono::DateTime<chrono::Utc>,
    }

    pub fn windivert_dll_path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("WinDivert.dll")))
            .unwrap_or_else(|| PathBuf::from("WinDivert.dll"))
    }

    pub fn windivert_available() -> std::result::Result<String, String> {
        let path = windivert_dll_path();
        if !path.exists() {
            return Err(format!("WinDivert.dll not found at {}", path.display()));
        }
        let lib = WinDivertLib::load(&path).map_err(|e| e.to_string())?;
        let _handle = lib
            .open("false", 0, 0, 0)
            .map_err(|e| format!("WinDivertOpen probe failed: {e}"))?;
        Ok(format!("WinDivert available ({})", path.display()))
    }

    pub struct WinDivertEngine {
        lib: Mutex<Option<WinDivertLib>>,
        handle: Mutex<Option<WinDivertHandle>>,
        routes: Mutex<HashMap<Uuid, TrafficRoute>>,
        telemetry: Arc<WinDivertTelemetry>,
        transform: Mutex<Option<PacketTransformHook>>,
    }

    impl Default for WinDivertEngine {
        fn default() -> Self {
            Self::new()
        }
    }

    impl WinDivertEngine {
        pub fn new() -> Self {
            Self {
                lib: Mutex::new(None),
                handle: Mutex::new(None),
                routes: Mutex::new(HashMap::new()),
                telemetry: Arc::new(WinDivertTelemetry::new()),
                transform: Mutex::new(None),
            }
        }

        pub fn telemetry(&self) -> Arc<WinDivertTelemetry> {
            Arc::clone(&self.telemetry)
        }

        pub fn set_transform_hook(&self, hook: Option<PacketTransformHook>) {
            *self.transform.lock() = hook;
        }

        pub fn health_sync(&self) -> WinDivertHealth {
            let routes = self.routes.lock().len() as u32;
            let snap = self.telemetry.snapshot();
            let attached = self.handle.lock().is_some();
            WinDivertHealth {
                available: attached,
                state: if attached { "running" } else { "stopped" }.into(),
                filter_attached: attached,
                active_route_count: routes,
                active_redirect_count: snap.redirect_count as u32,
                classify_count: snap.packets_seen,
                error_count: snap.error_count,
                message: None,
                checked_at: chrono::Utc::now(),
            }
        }

        fn ensure_open(&self) -> Result<()> {
            if self.handle.lock().is_some() {
                return Ok(());
            }
            let path = windivert_dll_path();
            let lib = WinDivertLib::load(&path)
                .map_err(|e| WireSentinelError::Wfp(format!("WinDivert load: {e}")))?;
            let handle = lib
                .open("outbound", 0, 0, 0)
                .map_err(|e| WireSentinelError::Wfp(format!("WinDivertOpen: {e}")))?;
            *self.lib.lock() = Some(lib);
            *self.handle.lock() = Some(handle);
            Ok(())
        }
    }

    #[async_trait]
    pub trait WinDivertEngineApi: Send + Sync {
        async fn init(&self) -> Result<()>;
        async fn shutdown(&self) -> Result<()>;
        async fn sync_route(
            &self,
            app: &AppIdentity,
            route: &TrafficRoute,
            _tunnel: Option<TunnelIface>,
        ) -> Result<()>;
        async fn clear_route(&self, app_id: Uuid) -> Result<()>;
        async fn health(&self) -> NdisHealth;
        async fn inject_cover_packet(&self, payload: &[u8]) -> Result<()>;
    }

    #[async_trait]
    impl WinDivertEngineApi for WinDivertEngine {
        async fn init(&self) -> Result<()> {
            match self.ensure_open() {
                Ok(()) => Ok(()),
                Err(e) => {
                    tracing::warn!(error = %e, "WinDivert unavailable — routing state kept in memory");
                    Ok(())
                }
            }
        }

        async fn shutdown(&self) -> Result<()> {
            *self.handle.lock() = None;
            *self.lib.lock() = None;
            self.routes.lock().clear();
            Ok(())
        }

        async fn sync_route(
            &self,
            app: &AppIdentity,
            route: &TrafficRoute,
            _tunnel: Option<TunnelIface>,
        ) -> Result<()> {
            self.routes.lock().insert(app.id(), route.clone());
            self.telemetry.record_redirect();
            let _ = self.ensure_open();
            Ok(())
        }

        async fn clear_route(&self, app_id: Uuid) -> Result<()> {
            self.routes.lock().remove(&app_id);
            Ok(())
        }

        async fn health(&self) -> NdisHealth {
            let h = self.health_sync();
            NdisHealth {
                available: h.available,
                state: h.state,
                filter_attached: h.filter_attached,
                active_route_count: h.active_route_count,
                active_redirect_count: h.active_redirect_count,
                classify_count: h.classify_count,
                error_count: h.error_count,
                message: h.message,
                checked_at: h.checked_at,
            }
        }

        async fn inject_cover_packet(&self, payload: &[u8]) -> Result<()> {
            self.ensure_open()?;
            if let Some(lib) = self.lib.lock().as_ref() {
                if let Some(handle) = self.handle.lock().as_ref() {
                    let mut buf = payload.to_vec();
                    if let Some(hook) = self.transform.lock().as_ref() {
                        hook(&mut buf);
                        self.telemetry.record_transform();
                    }
                    lib.send(handle, &buf)
                        .map_err(|e| WireSentinelError::Wfp(format!("WinDivertSend: {e}")))?;
                    self.telemetry.record_cover_traffic();
                    self.telemetry.record_reinject();
                    return Ok(());
                }
            }
            self.telemetry.record_cover_traffic();
            Ok(())
        }
    }
}

#[cfg(windows)]
pub use imp::{windivert_available, WinDivertEngine, WinDivertEngineApi, WinDivertHealth};

#[cfg(not(windows))]
mod stub {
    use super::*;
    use async_trait::async_trait;
    use shared_types::{AppIdentity, NdisHealth, Result, TrafficRoute, TunnelIface};
    use std::collections::HashMap;
    use uuid::Uuid;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct WinDivertHealth {
        pub available: bool,
        pub state: String,
        pub filter_attached: bool,
        pub active_route_count: u32,
        pub active_redirect_count: u32,
        pub classify_count: u64,
        pub error_count: u64,
        pub message: Option<String>,
        pub checked_at: chrono::DateTime<chrono::Utc>,
    }

    pub fn windivert_available() -> std::result::Result<String, String> {
        Err("WinDivert requires Windows".into())
    }

    pub struct StubWinDivertEngine {
        routes: Mutex<HashMap<Uuid, TrafficRoute>>,
        telemetry: Arc<WinDivertTelemetry>,
    }

    impl Default for StubWinDivertEngine {
        fn default() -> Self {
            Self::new()
        }
    }

    impl StubWinDivertEngine {
        pub fn new() -> Self {
            Self {
                routes: Mutex::new(HashMap::new()),
                telemetry: Arc::new(WinDivertTelemetry::new()),
            }
        }

        pub fn telemetry(&self) -> Arc<WinDivertTelemetry> {
            Arc::clone(&self.telemetry)
        }
    }

    #[async_trait]
    pub trait WinDivertEngineApi: Send + Sync {
        async fn init(&self) -> Result<()>;
        async fn shutdown(&self) -> Result<()>;
        async fn sync_route(
            &self,
            app: &AppIdentity,
            route: &TrafficRoute,
            tunnel: Option<TunnelIface>,
        ) -> Result<()>;
        async fn clear_route(&self, app_id: Uuid) -> Result<()>;
        async fn health(&self) -> NdisHealth;
        async fn inject_cover_packet(&self, payload: &[u8]) -> Result<()>;
    }

    #[async_trait]
    impl WinDivertEngineApi for StubWinDivertEngine {
        async fn init(&self) -> Result<()> {
            Ok(())
        }

        async fn shutdown(&self) -> Result<()> {
            self.routes.lock().clear();
            Ok(())
        }

        async fn sync_route(
            &self,
            app: &AppIdentity,
            route: &TrafficRoute,
            _tunnel: Option<TunnelIface>,
        ) -> Result<()> {
            self.routes.lock().insert(app.id(), route.clone());
            self.telemetry.record_redirect();
            Ok(())
        }

        async fn clear_route(&self, app_id: Uuid) -> Result<()> {
            self.routes.lock().remove(&app_id);
            Ok(())
        }

        async fn health(&self) -> NdisHealth {
            NdisHealth {
                available: false,
                state: "stub".into(),
                filter_attached: false,
                active_route_count: self.routes.lock().len() as u32,
                active_redirect_count: 0,
                classify_count: 0,
                error_count: 0,
                message: Some("WinDivert stub (non-Windows)".into()),
                checked_at: chrono::Utc::now(),
            }
        }

        async fn inject_cover_packet(&self, _payload: &[u8]) -> Result<()> {
            self.telemetry.record_cover_traffic();
            Ok(())
        }
    }
}

#[cfg(not(windows))]
pub use stub::{
    windivert_available, StubWinDivertEngine as WinDivertEngine, WinDivertEngineApi,
    WinDivertHealth,
};
