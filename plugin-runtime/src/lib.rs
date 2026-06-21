//! Plugin runtime — WASM (wasmtime) manager, native loader, and capability traits.

mod loader;
mod manager;
mod registry;
mod traits;
mod wasm_loader;

pub use loader::{AuthenticodeVerifier, LoadLibraryError, NativePluginHandle, NativePluginLoader};
pub use manager::{plugins_data_dir, PluginManager};
pub use registry::PluginRegistry;
pub use traits::{
    DnsProviderPlugin, FilterEnginePlugin, MetricsProviderPlugin, MixnetBackendPlugin,
    PolicyProviderPlugin, TransformModulePlugin, TransportBackendPlugin, WireSentinelPlugin,
};
pub use wasm_loader::{hex_sha256, PluginLoader, PluginSecurityEnforcer};

pub use shared_types::PluginSecurityPolicy;
