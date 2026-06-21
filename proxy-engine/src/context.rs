use parking_lot::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

/// Tracks local SOCKS listen ports keyed by proxy or chain profile id.
#[derive(Debug, Default)]
pub struct ProxyListenPort {
    ports: RwLock<HashMap<Uuid, u16>>,
}

impl ProxyListenPort {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&self, id: Uuid, port: u16) {
        self.ports.write().insert(id, port);
    }

    pub fn get(&self, id: Uuid) -> Option<u16> {
        self.ports.read().get(&id).copied()
    }

    pub fn remove(&self, id: Uuid) {
        self.ports.write().remove(&id);
    }

    pub fn snapshot(&self) -> HashMap<Uuid, u16> {
        self.ports.read().clone()
    }
}
