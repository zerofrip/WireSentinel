use crate::providers::{easylist, hosts};
use plugin_runtime::FilterEnginePlugin;
use shared_types::FilterListType;
use std::collections::HashSet;
use std::sync::Arc;

/// Native or WASM-backed filter list parser plugin.
pub trait FilterPlugin: Send + Sync {
    fn list_type(&self) -> FilterListType;
    fn parse(&self, content: &str) -> HashSet<String>;
}

pub struct HostsFilterPlugin;

impl FilterPlugin for HostsFilterPlugin {
    fn list_type(&self) -> FilterListType {
        FilterListType::Hosts
    }

    fn parse(&self, content: &str) -> HashSet<String> {
        hosts::parse(content)
    }
}

pub struct EasyListFilterPlugin;

impl FilterPlugin for EasyListFilterPlugin {
    fn list_type(&self) -> FilterListType {
        FilterListType::Easylist
    }

    fn parse(&self, content: &str) -> HashSet<String> {
        easylist::parse(content)
    }
}

/// WASM adapter stub — delegates parsing to a loaded `FilterEnginePlugin`.
pub struct WasmFilterPlugin {
    inner: Arc<dyn FilterEnginePlugin>,
    kind: FilterListType,
}

impl WasmFilterPlugin {
    pub fn new(inner: Arc<dyn FilterEnginePlugin>, kind: FilterListType) -> Self {
        Self { inner, kind }
    }

    pub fn plugin_id(&self) -> uuid::Uuid {
        self.inner.id()
    }
}

impl FilterPlugin for WasmFilterPlugin {
    fn list_type(&self) -> FilterListType {
        self.kind
    }

    fn parse(&self, content: &str) -> HashSet<String> {
        self.inner.parse_domains(content)
    }
}

/// Default built-in filter plugins (hosts + easylist).
pub fn default_plugins() -> Vec<Arc<dyn FilterPlugin>> {
    vec![Arc::new(HostsFilterPlugin), Arc::new(EasyListFilterPlugin)]
}
