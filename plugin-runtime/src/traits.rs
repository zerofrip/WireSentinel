use shared_types::{PluginCapability, PluginManifest};
use std::collections::HashSet;
use uuid::Uuid;

/// Base trait implemented by every loaded plugin instance.
pub trait WireSentinelPlugin: Send + Sync {
    fn id(&self) -> Uuid;
    fn manifest(&self) -> &PluginManifest;
    fn capabilities(&self) -> &[PluginCapability];
}

pub trait FilterEnginePlugin: WireSentinelPlugin {
    fn parse_domains(&self, content: &str) -> HashSet<String>;
    fn is_blocked(&self, domain: &str, blocked: &HashSet<String>) -> bool;
}

pub trait DnsProviderPlugin: WireSentinelPlugin {
    fn resolve(&self, qname: &str) -> Option<String>;
}

pub trait TransportBackendPlugin: WireSentinelPlugin {
    fn backend_name(&self) -> &str;
}

pub trait TransformModulePlugin: WireSentinelPlugin {
    fn transform(&self, payload: &[u8]) -> Vec<u8>;
}

pub trait PolicyProviderPlugin: WireSentinelPlugin {
    fn policy_hint(&self, app_id: Uuid) -> Option<String>;
}

pub trait MetricsProviderPlugin: WireSentinelPlugin {
    fn collect_metrics(&self) -> serde_json::Value;
}

/// Third-party mixnet provider plugins (Nym, Katzenpost, etc.).
pub trait MixnetBackendPlugin: WireSentinelPlugin {
    fn provider_id(&self) -> &str;
}

/// Third-party anonymity backend plugins (Katzenpost, Loopix, federated mixnet).
#[allow(dead_code)]
pub trait AnonymityBackendPlugin: WireSentinelPlugin {
    fn provider_id(&self) -> &str;
}
