use chrono::Utc;
use event_bus::EventBus;
use parking_lot::RwLock;
use shared_types::{
    AppRule, DomainRule, ServiceEventInner, SplitTemplateModeSettings, SplitTunnelTemplate,
    TemplateMode, TemplateResolutionTrace, TemplateTraceStep,
};
use std::collections::HashMap;
use uuid::Uuid;

/// Manages global split-tunnel templates and active mode.
pub struct SplitTunnelTemplateManager {
    templates: RwLock<HashMap<Uuid, SplitTunnelTemplate>>,
    mode: RwLock<SplitTemplateModeSettings>,
    events: Option<EventBus>,
}

impl SplitTunnelTemplateManager {
    pub fn new() -> Self {
        Self {
            templates: RwLock::new(HashMap::new()),
            mode: RwLock::new(SplitTemplateModeSettings::default()),
            events: None,
        }
    }

    pub fn with_events(mut self, events: EventBus) -> Self {
        self.events = Some(events);
        self
    }

    pub fn load(&self, templates: Vec<SplitTunnelTemplate>, mode: SplitTemplateModeSettings) {
        let mut map = self.templates.write();
        map.clear();
        for t in templates {
            map.insert(t.id, t);
        }
        *self.mode.write() = mode;
    }

    pub fn mode(&self) -> SplitTemplateModeSettings {
        self.mode.read().clone()
    }

    pub fn set_mode(&self, settings: SplitTemplateModeSettings) {
        *self.mode.write() = settings;
    }

    pub fn list(&self) -> Vec<SplitTunnelTemplate> {
        self.templates.read().values().cloned().collect()
    }

    pub fn get(&self, id: Uuid) -> Option<SplitTunnelTemplate> {
        self.templates.read().get(&id).cloned()
    }

    pub fn upsert(&self, template: SplitTunnelTemplate) {
        self.templates.write().insert(template.id, template);
    }

    pub fn remove(&self, id: Uuid) -> bool {
        self.templates.write().remove(&id).is_some()
    }

    pub fn active_template(&self) -> Option<SplitTunnelTemplate> {
        let mode = self.mode.read();
        if mode.mode == TemplateMode::Disabled {
            return None;
        }
        let id = mode.active_template_id?;
        self.templates.read().get(&id).cloned()
    }

    pub fn resolve_trace(&self) -> TemplateResolutionTrace {
        let mode = self.mode.read().clone();
        let template = self.active_template();
        let mut trace = TemplateResolutionTrace {
            template_id: template.as_ref().map(|t| t.id),
            mode: mode.mode,
            steps: Vec::new(),
            final_route: None,
        };

        if mode.mode == TemplateMode::Disabled {
            trace.steps.push(TemplateTraceStep {
                stage: "template".into(),
                detail: "disabled".into(),
                route: None,
            });
            return trace;
        }

        if let Some(t) = &template {
            trace.steps.push(TemplateTraceStep {
                stage: "template".into(),
                detail: format!("{} ({:?})", t.name, mode.mode),
                route: Some(t.default_route.clone()),
            });
            if let Some(events) = &self.events {
                events.publish(
                    ServiceEventInner::SplitTemplateApplied {
                        template_id: t.id,
                        mode: mode.mode,
                    }
                    .with_timestamp(Utc::now()),
                );
            }
        } else {
            trace.steps.push(TemplateTraceStep {
                stage: "template".into(),
                detail: "no active template".into(),
                route: None,
            });
        }

        trace
    }

    pub fn app_rules(&self) -> Vec<AppRule> {
        self.active_template()
            .map(|t| t.app_rules.iter().filter(|r| r.enabled).cloned().collect())
            .unwrap_or_default()
    }

    pub fn domain_rules(&self) -> Vec<DomainRule> {
        self.active_template()
            .map(|t| {
                t.domain_rules
                    .iter()
                    .filter(|r| r.enabled)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Default for SplitTunnelTemplateManager {
    fn default() -> Self {
        Self::new()
    }
}
