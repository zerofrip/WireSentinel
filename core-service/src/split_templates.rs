//! Phase 18.5 split tunnel template service.

use parking_lot::RwLock;
use shared_types::{ResolvedTemplate, Result, SplitTemplateModeSettings, SplitTunnelTemplate};
use split_tunnel::{SplitTunnelTemplateManager, TemplateResolver};
use std::sync::Arc;
use storage::Storage;
use uuid::Uuid;

pub struct SplitTemplateService {
    manager: Arc<SplitTunnelTemplateManager>,
    storage: Arc<Storage>,
    last_trace: RwLock<Option<shared_types::TemplateResolutionTrace>>,
}

impl SplitTemplateService {
    pub fn new(storage: Arc<Storage>, manager: Arc<SplitTunnelTemplateManager>) -> Self {
        Self {
            manager,
            storage,
            last_trace: RwLock::new(None),
        }
    }

    pub fn manager(&self) -> Arc<SplitTunnelTemplateManager> {
        Arc::clone(&self.manager)
    }

    pub async fn reload(&self) -> Result<()> {
        let templates = self.storage.split_templates.list().await?;
        let mode = self.storage.split_templates.get_mode().await?;
        self.manager.load(templates, mode);
        Ok(())
    }

    pub fn resolved_template(&self) -> Option<ResolvedTemplate> {
        let mode = self.manager.mode();
        TemplateResolver::build_resolved(mode.mode, self.manager.active_template())
    }

    pub fn store_trace(&self, trace: shared_types::TemplateResolutionTrace) {
        *self.last_trace.write() = Some(trace);
    }

    pub fn last_trace(&self) -> Option<shared_types::TemplateResolutionTrace> {
        self.last_trace.read().clone()
    }

    pub async fn list(&self) -> Result<Vec<SplitTunnelTemplate>> {
        self.storage.split_templates.list().await
    }

    pub async fn upsert(&self, template: SplitTunnelTemplate) -> Result<()> {
        if self.storage.split_templates.get(template.id).await?.is_some() {
            self.storage.split_templates.update(&template).await?;
        } else {
            self.storage.split_templates.insert(&template).await?;
        }
        self.reload().await
    }

    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let ok = self.storage.split_templates.delete(id).await?;
        self.reload().await?;
        Ok(ok)
    }

    pub async fn get_mode(&self) -> Result<SplitTemplateModeSettings> {
        self.storage.split_templates.get_mode().await
    }

    pub async fn set_mode(&self, settings: SplitTemplateModeSettings) -> Result<()> {
        self.storage.split_templates.set_mode(&settings).await?;
        self.reload().await
    }
}
