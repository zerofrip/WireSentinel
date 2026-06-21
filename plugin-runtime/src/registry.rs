use crate::traits::WireSentinelPlugin;
use shared_types::PluginRecord;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// In-memory registry of installed and loaded plugins.
#[derive(Default)]
pub struct PluginRegistry {
    records: HashMap<Uuid, PluginRecord>,
    loaded: HashMap<Uuid, Arc<dyn WireSentinelPlugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert_record(&mut self, record: PluginRecord) {
        self.records.insert(record.id, record);
    }

    pub fn record(&self, id: Uuid) -> Option<&PluginRecord> {
        self.records.get(&id)
    }

    pub fn records(&self) -> Vec<PluginRecord> {
        let mut records: Vec<_> = self.records.values().cloned().collect();
        records.sort_by(|a, b| b.installed_at.cmp(&a.installed_at));
        records
    }

    pub fn register_loaded(&mut self, plugin: Arc<dyn WireSentinelPlugin>, record: PluginRecord) {
        self.loaded.insert(plugin.id(), Arc::clone(&plugin));
        self.records.insert(record.id, record);
    }

    pub fn unregister_loaded(&mut self, id: Uuid) -> Option<Arc<dyn WireSentinelPlugin>> {
        self.loaded.remove(&id)
    }

    pub fn loaded(&self, id: Uuid) -> Option<Arc<dyn WireSentinelPlugin>> {
        self.loaded.get(&id).cloned()
    }

    pub fn is_loaded(&self, id: Uuid) -> bool {
        self.loaded.contains_key(&id)
    }
}
