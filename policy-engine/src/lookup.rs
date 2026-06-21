use shared_types::VpnBackendKind;
use uuid::Uuid;

/// Resolves VPN profile backend type without coupling to vpn-engine.
pub trait ProfileLookup: Send + Sync {
    fn backend_for(&self, profile_id: Uuid) -> Option<VpnBackendKind>;
}

/// Default lookup when no VPN profiles are loaded.
pub struct EmptyProfileLookup;

impl ProfileLookup for EmptyProfileLookup {
    fn backend_for(&self, _profile_id: Uuid) -> Option<VpnBackendKind> {
        None
    }
}

/// In-memory profile lookup for tests and simple deployments.
pub struct HashMapProfileLookup {
    map: std::collections::HashMap<Uuid, VpnBackendKind>,
}

impl HashMapProfileLookup {
    pub fn new(map: std::collections::HashMap<Uuid, VpnBackendKind>) -> Self {
        Self { map }
    }
}

impl ProfileLookup for HashMapProfileLookup {
    fn backend_for(&self, profile_id: Uuid) -> Option<VpnBackendKind> {
        self.map.get(&profile_id).copied()
    }
}
