//! Userspace WFP engine using FWPM API with ALE_APP_ID conditions.
//!
//! On Windows this manages WFP filters programmatically. Filter application
//! requires elevation (WireSentinel service runs as admin).

use crate::engine::{RouteEnforcer, WfpEngine};
use async_trait::async_trait;
use parking_lot::RwLock;
use shared_types::{AppIdentity, Result, RuleAction, TrafficRoute, TunnelIface, WireSentinelError};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// WireSentinel WFP provider display name.
pub const PROVIDER_NAME: &str = "WireSentinel";
pub const SUBLAYER_NAME: &str = "WireSentinel/AppFilter";

pub struct UserspaceWfpEngine {
    initialized: AtomicBool,
    kill_switch: AtomicBool,
    provider_registered: AtomicBool,
    filter_ids: RwLock<HashMap<Uuid, Vec<u64>>>,
    kill_switch_filter_ids: RwLock<Vec<u64>>,
    engine_handle: RwLock<Option<u64>>,
}

impl UserspaceWfpEngine {
    pub fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
            kill_switch: AtomicBool::new(false),
            provider_registered: AtomicBool::new(false),
            filter_ids: RwLock::new(HashMap::new()),
            kill_switch_filter_ids: RwLock::new(Vec::new()),
            engine_handle: RwLock::new(None),
        }
    }

    fn open_engine(&self) -> Result<u64> {
        use windows::core::PCWSTR;
        use windows::Win32::NetworkManagement::WindowsFilteringPlatform::{
            FwpmEngineOpen0, FWPM_SESSION0,
        };

        let mut handle = std::ptr::null_mut();
        let session = FWPM_SESSION0::default();
        unsafe {
            FwpmEngineOpen0(
                PCWSTR::null(),
                0x00000002, // RPC_C_AUTHN_WINNT
                None,
                Some(&session as *const _ as *mut _),
                &mut handle,
            )
            .map_err(|e| WireSentinelError::Wfp(format!("FwpmEngineOpen0 failed: {e}")))?;
        }
        Ok(handle as u64)
    }

    fn close_engine(handle: u64) {
        use windows::Win32::NetworkManagement::WindowsFilteringPlatform::FwpmEngineClose0;
        if handle != 0 {
            unsafe {
                let _ = FwpmEngineClose0(handle as _);
            }
        }
    }

    fn app_id_from_path(
        path: &PathBuf,
    ) -> Result<*mut windows::Win32::NetworkManagement::WindowsFilteringPlatform::FWP_BYTE_BLOB>
    {
        use windows::core::PCWSTR;
        use windows::Win32::NetworkManagement::WindowsFilteringPlatform::FwpmGetAppIdFromFileName0;

        let wide = windows::core::HSTRING::from(path.as_os_str());
        let mut app_id = std::ptr::null_mut();
        unsafe {
            FwpmGetAppIdFromFileName0(PCWSTR(wide.as_ptr()), &mut app_id).map_err(|e| {
                WireSentinelError::Wfp(format!(
                    "FwpmGetAppIdFromFileName0 failed for {}: {e}",
                    path.display()
                ))
            })?;
        }
        Ok(app_id)
    }

    fn free_app_id(
        app_id: *mut windows::Win32::NetworkManagement::WindowsFilteringPlatform::FWP_BYTE_BLOB,
    ) {
        use windows::Win32::NetworkManagement::WindowsFilteringPlatform::FwpmFreeMemory0;
        if !app_id.is_null() {
            unsafe {
                let _ = FwpmFreeMemory0(&mut app_id as *mut _ as *mut _);
            }
        }
    }

    fn apply_filter_on_layer(
        &self,
        engine: u64,
        app: &AppIdentity,
        layer: windows::core::GUID,
        action: RuleAction,
        tunnel: Option<&TunnelIface>,
    ) -> Result<u64> {
        use windows::Win32::NetworkManagement::WindowsFilteringPlatform::{
            FwpmFilterAdd0, FWPM_CONDITION_ALE_APP_ID, FWPM_FILTER0, FWPM_FILTER_CONDITION0,
            FWP_ACTION_BLOCK, FWP_ACTION_PERMIT, FWP_MATCH_EQUAL,
        };

        let block = matches!(action, RuleAction::Block);
        let filter_action = if block {
            FWP_ACTION_BLOCK
        } else {
            FWP_ACTION_PERMIT
        };

        let app_id_blob = Self::app_id_from_path(&app.exe_path())?;
        let mut condition: FWPM_FILTER_CONDITION0 = unsafe { std::mem::zeroed() };
        condition.fieldKey = FWPM_CONDITION_ALE_APP_ID;
        condition.matchType = FWP_MATCH_EQUAL;
        condition.conditionValue.r#type =
            windows::Win32::NetworkManagement::WindowsFilteringPlatform::FWP_BYTE_BLOB_TYPE;
        condition.conditionValue.Anonymous.byteBlob = app_id_blob;

        let tunnel_note = tunnel
            .map(|t| format!(" via {}", t.name))
            .unwrap_or_default();
        let display_data = format!(
            "WireSentinel: {} ({}){tunnel_note}",
            app.display_name(),
            app.id()
        );
        debug!(filter = %display_data, block, "adding WFP ALE filter");

        let mut filter: FWPM_FILTER0 = unsafe { std::mem::zeroed() };
        filter.displayData.name =
            windows::core::PWSTR(windows::core::HSTRING::from(display_data.clone()).as_ptr() as _);
        filter.layerKey = layer;
        filter.action.r#type = filter_action;
        filter.weight.r#type =
            windows::Win32::NetworkManagement::WindowsFilteringPlatform::FWP_UINT8;
        filter.weight.Anonymous.uint8 = if block { 1 } else { 15 };
        filter.filterCondition = condition;
        filter.numFilterConditions = 1;

        // Interface LUID condition for VPN routes — stub until tunnel LUID is available.
        if !block && tunnel.is_some() {
            debug!(tunnel = ?tunnel, "VPN interface condition stub (permit only)");
        }

        let mut filter_id = 0u64;
        let add_result =
            unsafe { FwpmFilterAdd0(engine as _, &filter, None, Some(&mut filter_id)) };
        Self::free_app_id(app_id_blob);

        add_result.map_err(|e| {
            WireSentinelError::Wfp(format!(
                "FwpmFilterAdd0 failed for {}: {e}",
                app.exe_path().display()
            ))
        })?;

        Ok(filter_id)
    }

    fn apply_connection_filters(
        &self,
        engine: u64,
        app: &AppIdentity,
        route: &TrafficRoute,
        tunnel: Option<&TunnelIface>,
    ) -> Result<Vec<u64>> {
        use windows::Win32::NetworkManagement::WindowsFilteringPlatform::{
            FWPM_LAYER_ALE_AUTH_CONNECT_V4, FWPM_LAYER_ALE_AUTH_CONNECT_V6,
        };

        let action = match route {
            TrafficRoute::Blocked => RuleAction::Block,
            TrafficRoute::Direct => RuleAction::Allow,
            TrafficRoute::WireGuard(_) | TrafficRoute::AmneziaWG(_) => RuleAction::Allow,
        };

        let mut ids = Vec::with_capacity(2);
        ids.push(self.apply_filter_on_layer(
            engine,
            app,
            FWPM_LAYER_ALE_AUTH_CONNECT_V4,
            action,
            tunnel,
        )?);
        ids.push(self.apply_filter_on_layer(
            engine,
            app,
            FWPM_LAYER_ALE_AUTH_CONNECT_V6,
            action,
            tunnel,
        )?);
        Ok(ids)
    }

    fn remove_filters(&self, engine: u64, ids: &[u64]) {
        for id in ids {
            if let Err(e) = self.remove_filter(engine, *id) {
                warn!(filter_id = id, error = %e, "failed to remove WFP filter");
            }
        }
    }

    fn remove_filter(&self, engine: u64, filter_id: u64) -> Result<()> {
        use windows::Win32::NetworkManagement::WindowsFilteringPlatform::FwpmFilterDeleteById0;
        unsafe {
            FwpmFilterDeleteById0(engine as _, filter_id).map_err(|e| {
                WireSentinelError::Wfp(format!("FwpmFilterDeleteById0 failed: {e}"))
            })?;
        }
        Ok(())
    }

    fn replace_app_filters(
        &self,
        app: &AppIdentity,
        route: &TrafficRoute,
        tunnel: Option<&TunnelIface>,
    ) -> Result<Vec<u64>> {
        let handle = *self
            .engine_handle
            .read()
            .ok_or_else(|| WireSentinelError::Wfp("engine not initialized".into()))?;

        if let Some(old_ids) = self.filter_ids.write().remove(&app.id()) {
            self.remove_filters(handle, &old_ids);
        }

        let ids = self.apply_connection_filters(handle, app, route, tunnel)?;
        self.filter_ids.write().insert(app.id(), ids.clone());
        Ok(ids)
    }

    fn apply_kill_switch_filters(&self, engine: u64) -> Result<Vec<u64>> {
        use windows::Win32::NetworkManagement::WindowsFilteringPlatform::{
            FwpmFilterAdd0, FWPM_FILTER0, FWPM_LAYER_ALE_AUTH_CONNECT_V4,
            FWPM_LAYER_ALE_AUTH_CONNECT_V6, FWP_ACTION_BLOCK,
        };

        info!("applying kill switch block-all filters");
        let mut ids = Vec::new();
        for layer in [
            FWPM_LAYER_ALE_AUTH_CONNECT_V4,
            FWPM_LAYER_ALE_AUTH_CONNECT_V6,
        ] {
            let mut filter: FWPM_FILTER0 = unsafe { std::mem::zeroed() };
            filter.displayData.name = windows::core::PWSTR(
                windows::core::HSTRING::from("WireSentinel: KillSwitch").as_ptr() as _,
            );
            filter.layerKey = layer;
            filter.action.r#type = FWP_ACTION_BLOCK;
            filter.weight.r#type =
                windows::Win32::NetworkManagement::WindowsFilteringPlatform::FWP_UINT8;
            filter.weight.Anonymous.uint8 = 0;

            let mut filter_id = 0u64;
            unsafe {
                FwpmFilterAdd0(engine as _, &filter, None, Some(&mut filter_id)).map_err(|e| {
                    WireSentinelError::Wfp(format!("kill switch filter failed: {e}"))
                })?;
            }
            ids.push(filter_id);
        }
        Ok(ids)
    }

    fn ensure_provider(&self, engine: u64) -> Result<()> {
        use windows::core::GUID;
        use windows::Win32::NetworkManagement::WindowsFilteringPlatform::{
            FwpmProviderAdd0, FwpmSubLayerAdd0, FWPM_PROVIDER0, FWPM_SUBLAYER0,
        };

        let provider_key = GUID::from_u128(0xa1b2c3d4_e5f6_7890_abcd_ef1234567890);
        let sublayer_key = GUID::from_u128(0xb2c3d4e5_f6a7_8901_bcde_f12345678901);

        let mut provider: FWPM_PROVIDER0 = unsafe { std::mem::zeroed() };
        provider.providerKey = provider_key;
        provider.displayData.name =
            windows::core::PWSTR(windows::core::HSTRING::from(PROVIDER_NAME).as_ptr() as _);
        unsafe {
            let _ = FwpmProviderAdd0(engine as _, &provider, None);
        }

        let mut sublayer: FWPM_SUBLAYER0 = unsafe { std::mem::zeroed() };
        sublayer.subLayerKey = sublayer_key;
        sublayer.displayData.name =
            windows::core::PWSTR(windows::core::HSTRING::from(SUBLAYER_NAME).as_ptr() as _);
        sublayer.providerKey = provider_key;
        sublayer.weight = 0x100;
        unsafe {
            let _ = FwpmSubLayerAdd0(engine as _, &sublayer, None);
        }

        self.provider_registered.store(true, Ordering::SeqCst);
        Ok(())
    }
}

impl Default for UserspaceWfpEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WfpEngine for UserspaceWfpEngine {
    async fn init(&self) -> Result<()> {
        if self.initialized.load(Ordering::SeqCst) {
            return Ok(());
        }
        let handle = self.open_engine()?;
        if let Err(e) = self.ensure_provider(handle) {
            warn!(error = %e, "WFP provider registration failed (continuing)");
        }
        *self.engine_handle.write() = Some(handle);
        self.initialized.store(true, Ordering::SeqCst);
        info!("userspace WFP engine initialized");
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        if let Some(handle) = self.engine_handle.write().take() {
            for ids in self.filter_ids.write().drain().map(|(_, v)| v) {
                self.remove_filters(handle, &ids);
            }
            for id in self.kill_switch_filter_ids.write().drain(..) {
                let _ = self.remove_filter(handle, id);
            }
            Self::close_engine(handle);
        }
        self.initialized.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn apply_route(&self, app: &AppIdentity, route: &TrafficRoute) -> Result<()> {
        self.route_connection(app, route, None).await
    }

    async fn remove_app_rule(&self, app_id: Uuid) -> Result<()> {
        let handle = *self
            .engine_handle
            .read()
            .ok_or_else(|| WireSentinelError::Wfp("engine not initialized".into()))?;
        if let Some(ids) = self.filter_ids.write().remove(&app_id) {
            self.remove_filters(handle, &ids);
        }
        Ok(())
    }

    async fn apply_kill_switch(&self, active: bool) -> Result<()> {
        self.kill_switch.store(active, Ordering::SeqCst);
        let handle = *self
            .engine_handle
            .read()
            .ok_or_else(|| WireSentinelError::Wfp("engine not initialized".into()))?;

        if active {
            let ids = self.apply_kill_switch_filters(handle)?;
            *self.kill_switch_filter_ids.write() = ids;
        } else {
            for id in self.kill_switch_filter_ids.write().drain(..) {
                let _ = self.remove_filter(handle, id);
            }
        }
        Ok(())
    }

    async fn sync_rules(&self, rules: &[(Uuid, PathBuf, RuleAction)]) -> Result<()> {
        for (id, path, action) in rules {
            let mut record = shared_types::AppRecord::new(path.clone());
            record.app_id = *id;
            let app = AppIdentity::new(0, record);
            let route = match action {
                RuleAction::Block => TrafficRoute::Blocked,
                RuleAction::Allow | RuleAction::RouteDirect | RuleAction::LogOnly => {
                    TrafficRoute::Direct
                }
                RuleAction::RouteViaVpn(pid) => TrafficRoute::WireGuard(*pid),
            };
            self.apply_route(&app, &route).await?;
        }
        Ok(())
    }

    async fn allow_connection(&self, app: &AppIdentity, route: &TrafficRoute) -> Result<()> {
        self.route_connection(app, route, None).await
    }

    async fn block_connection(&self, app: &AppIdentity) -> Result<()> {
        self.route_connection(app, &TrafficRoute::Blocked, None)
            .await
    }

    async fn route_connection(
        &self,
        app: &AppIdentity,
        route: &TrafficRoute,
        tunnel: Option<TunnelIface>,
    ) -> Result<()> {
        let tunnel_ref = tunnel.as_ref();
        let _ = self.replace_app_filters(app, route, tunnel_ref)?;
        Ok(())
    }

    fn filter_ids_for_app(&self, app_id: Uuid) -> Vec<u64> {
        self.filter_ids
            .read()
            .get(&app_id)
            .cloned()
            .unwrap_or_default()
    }

    fn tracked_filter_count(&self) -> u32 {
        let app_filters: usize = self.filter_ids.read().values().map(|v| v.len()).sum();
        let ks = self.kill_switch_filter_ids.read().len();
        (app_filters + ks) as u32
    }

    async fn reconcile_filters(&self, known_ids: &[u64]) -> Result<u32> {
        let handle = *self
            .engine_handle
            .read()
            .ok_or_else(|| WireSentinelError::Wfp("engine not initialized".into()))?;
        let known: std::collections::HashSet<u64> = known_ids.iter().copied().collect();
        let mut removed = 0u32;

        for ids in self.filter_ids.write().values_mut() {
            ids.retain(|id| {
                if known.contains(id) {
                    true
                } else {
                    if self.remove_filter(handle, *id).is_ok() {
                        removed += 1;
                    }
                    false
                }
            });
        }

        Ok(removed)
    }

    async fn driver_state(&self) -> shared_types::DriverState {
        shared_types::DriverState {
            engine: "userspace".into(),
            state: if self.initialized.load(Ordering::SeqCst) {
                "running".into()
            } else {
                "stopped".into()
            },
            filter_count: self.tracked_filter_count(),
            provider_registered: self.provider_registered.load(Ordering::SeqCst),
            message: None,
        }
    }
}

#[async_trait]
impl RouteEnforcer for UserspaceWfpEngine {
    async fn apply_route(&self, app: &AppIdentity, route: &TrafficRoute) -> Result<()> {
        WfpEngine::apply_route(self, app, route).await
    }

    async fn apply_kill_switch(&self, active: bool) -> Result<()> {
        WfpEngine::apply_kill_switch(self, active).await
    }
}
