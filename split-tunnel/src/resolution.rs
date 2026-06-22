//! Template resolution helpers for split-tunnel merge/override modes.

use shared_types::{TemplateMode, TemplateTraceStep, TrafficRoute};

pub use shared_types::ResolvedTemplate;

pub struct TemplateResolver;

impl TemplateResolver {
    pub fn build_resolved(
        mode: TemplateMode,
        template: Option<shared_types::SplitTunnelTemplate>,
    ) -> Option<ResolvedTemplate> {
        let t = template?;
        if mode == TemplateMode::Disabled || !t.enabled {
            return None;
        }
        Some(ResolvedTemplate {
            mode,
            default_route: t.default_route.clone(),
            app_rules: t.app_rules.clone(),
            domain_rules: t.domain_rules.clone(),
            template_id: t.id,
        })
    }

    pub fn trace_step_for_template(resolved: &ResolvedTemplate) -> TemplateTraceStep {
        TemplateTraceStep {
            stage: "global_template".into(),
            detail: format!("template {:?} mode", resolved.mode),
            route: Some(resolved.default_route.clone()),
        }
    }

    pub fn match_template_rules(
        app_id: uuid::Uuid,
        domain: Option<&str>,
        tmpl: &ResolvedTemplate,
    ) -> Option<TrafficRoute> {
        for rule in &tmpl.app_rules {
            if rule.enabled && rule.app_id == app_id {
                return Some(rule.route.clone());
            }
        }
        if let Some(domain) = domain {
            for rule in &tmpl.domain_rules {
                if rule.enabled && domain_matches(domain, &rule.pattern) {
                    return Some(rule.route.clone());
                }
            }
        }
        None
    }
}

fn domain_matches(query: &str, pattern: &str) -> bool {
    let q = query.to_ascii_lowercase();
    let p = pattern.to_ascii_lowercase();
    if p.starts_with("*.") {
        let suffix = &p[1..];
        q.ends_with(suffix) || q == p.trim_start_matches('*').trim_start_matches('.')
    } else {
        q == p || q.ends_with(&format!(".{p}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::{AppRule, SplitTunnelTemplate};
    use uuid::Uuid;

    #[test]
    fn build_resolved_disabled_returns_none() {
        let t = SplitTunnelTemplate::new("t".into(), TrafficRoute::Direct);
        assert!(TemplateResolver::build_resolved(TemplateMode::Disabled, Some(t)).is_none());
    }

    #[test]
    fn match_app_rule() {
        let app_id = Uuid::new_v4();
        let tmpl = ResolvedTemplate {
            mode: TemplateMode::Override,
            default_route: TrafficRoute::Direct,
            app_rules: vec![AppRule {
                id: Uuid::new_v4(),
                app_id,
                route: TrafficRoute::Blocked,
                enabled: true,
                description: None,
            }],
            domain_rules: vec![],
            template_id: Uuid::new_v4(),
        };
        let route = TemplateResolver::match_template_rules(app_id, None, &tmpl);
        assert_eq!(route, Some(TrafficRoute::Blocked));
    }
}
