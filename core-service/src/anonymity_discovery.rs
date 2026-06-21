//! Anonymous service discovery bridge.

use anonymity_discovery::AnonymousDiscoveryEngine;
use shared_types::Result;

pub struct AnonymityDiscoveryService {
    inner: AnonymousDiscoveryEngine,
}

impl Default for AnonymityDiscoveryService {
    fn default() -> Self {
        Self::new()
    }
}

impl AnonymityDiscoveryService {
    pub fn new() -> Self {
        Self {
            inner: AnonymousDiscoveryEngine::new(),
        }
    }

    pub fn list_services(&self) -> Result<Vec<String>> {
        Ok(self
            .inner
            .discover()
            .into_iter()
            .map(|s| s.endpoint)
            .collect())
    }
}
