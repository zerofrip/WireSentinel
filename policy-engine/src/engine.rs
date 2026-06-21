use crate::lookup::ProfileLookup;
use shared_types::{
    PolicyMode, ResolvedTemplate, Rule, RuleAction, RuleScope, Ruleset, Subject, TemplateMode,
    TemplateResolutionTrace, TemplateTraceStep, TrafficRoute, Verdict,
};
use std::sync::Arc;
use uuid::Uuid;

/// Input context for a connection decision.
#[derive(Debug, Clone)]
pub struct ConnectionContext {
    pub app_id: Uuid,
    pub domain: Option<String>,
    pub vpn_connected: bool,
    pub active_vpn_profile: Option<Uuid>,
    pub default_route: Option<TrafficRoute>,
    /// Optional ZTNA subject for zero-trust pre-policy evaluation.
    pub ztna_subject: Option<Subject>,
}

/// Result of policy evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub route: TrafficRoute,
    pub verdict: Verdict,
    pub matched_rule_id: Option<Uuid>,
}

/// Deterministic rule engine with conflict-safe evaluation order.
pub struct PolicyEngine {
    ruleset: Ruleset,
    vpn_disconnected_route: TrafficRoute,
    profiles: Arc<dyn ProfileLookup>,
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new(Ruleset::default(), Arc::new(crate::lookup::EmptyProfileLookup))
    }
}

impl PolicyEngine {
    pub fn new(ruleset: Ruleset, profiles: Arc<dyn ProfileLookup>) -> Self {
        Self {
            ruleset,
            vpn_disconnected_route: TrafficRoute::Blocked,
            profiles,
        }
    }

    pub fn with_vpn_disconnected_route(mut self, route: TrafficRoute) -> Self {
        self.vpn_disconnected_route = route;
        self
    }

    pub fn ruleset(&self) -> &Ruleset {
        &self.ruleset
    }

    pub fn ruleset_mut(&mut self) -> &mut Ruleset {
        &mut self.ruleset
    }

    pub fn set_kill_switch(&mut self, active: bool) {
        self.ruleset.kill_switch_active = active;
    }

    pub fn kill_switch_active(&self) -> bool {
        self.ruleset.kill_switch_active
    }

    pub fn set_mode(&mut self, mode: PolicyMode) {
        self.ruleset.mode = mode;
    }

    pub fn add_rule(&mut self, rule: Rule) {
        self.ruleset.rules.push(rule);
    }

    pub fn remove_rule(&mut self, rule_id: Uuid) -> bool {
        let before = self.ruleset.rules.len();
        self.ruleset.rules.retain(|r| r.id != rule_id);
        self.ruleset.rules.len() < before
    }

    pub fn update_rule(&mut self, rule: Rule) -> bool {
        if let Some(existing) = self.ruleset.rules.iter_mut().find(|r| r.id == rule.id) {
            *existing = rule;
            true
        } else {
            false
        }
    }

    pub fn decide(&self, ctx: &ConnectionContext) -> Decision {
        if self.ruleset.kill_switch_active {
            let route = TrafficRoute::Blocked;
            return Decision {
                route: route.clone(),
                verdict: Verdict::from_route(&route, None, "kill switch active"),
                matched_rule_id: None,
            };
        }

        if let Some(decision) = self.evaluate_scope(RuleScopeFilter::App(ctx.app_id), ctx) {
            return decision;
        }

        if let Some(domain) = &ctx.domain {
            if let Some(decision) =
                self.evaluate_scope(RuleScopeFilter::Domain(domain.clone()), ctx)
            {
                return decision;
            }
        }

        if let Some(default_route) = &ctx.default_route {
            return Decision {
                route: default_route.clone(),
                verdict: Verdict::from_route(default_route, None, "app default route"),
                matched_rule_id: None,
            };
        }

        if let Some(decision) = self.evaluate_scope(RuleScopeFilter::Global, ctx) {
            return decision;
        }

        let route = match self.ruleset.mode {
            PolicyMode::Blacklist => TrafficRoute::Direct,
            PolicyMode::Whitelist => TrafficRoute::Blocked,
        };
        let reason = match self.ruleset.mode {
            PolicyMode::Blacklist => "default direct (blacklist mode)",
            PolicyMode::Whitelist => "default blocked (whitelist mode)",
        };
        Decision {
            route: route.clone(),
            verdict: Verdict::from_route(&route, None, reason),
            matched_rule_id: None,
        }
    }

    /// Evaluate with optional global split-tunnel template (Phase 18.5).
    pub fn decide_with_template(
        &self,
        ctx: &ConnectionContext,
        template: Option<&ResolvedTemplate>,
    ) -> (Decision, TemplateResolutionTrace) {
        let mut trace = TemplateResolutionTrace {
            template_id: template.map(|t| t.template_id),
            mode: template.map(|t| t.mode).unwrap_or(TemplateMode::Disabled),
            steps: Vec::new(),
            final_route: None,
        };

        if let Some(tmpl) = template {
            trace.steps.push(TemplateTraceStep {
                stage: "global_template".into(),
                detail: format!("{:?}", tmpl.mode),
                route: Some(tmpl.default_route.clone()),
            });

            if tmpl.mode == TemplateMode::Override {
                if let Some(route) = Self::match_template_rules(ctx, tmpl) {
                    trace.final_route = Some(route.clone());
                    return (
                        Decision {
                            route: route.clone(),
                            verdict: Verdict::from_route(&route, None, "template override app/domain rule"),
                            matched_rule_id: None,
                        },
                        trace,
                    );
                }
                let route = tmpl.default_route.clone();
                trace.final_route = Some(route.clone());
                return (
                    Decision {
                        route: route.clone(),
                        verdict: Verdict::from_route(&route, None, "template override default"),
                        matched_rule_id: None,
                    },
                    trace,
                );
            }

            if let Some(route) = Self::match_template_rules(ctx, tmpl) {
                trace.steps.push(TemplateTraceStep {
                    stage: "template_rule".into(),
                    detail: "matched template rule".into(),
                    route: Some(route.clone()),
                });
                trace.final_route = Some(route.clone());
                return (
                    Decision {
                        route: route.clone(),
                        verdict: Verdict::from_route(&route, None, "template merge rule"),
                        matched_rule_id: None,
                    },
                    trace,
                );
            }
        }

        let decision = self.decide(ctx);
        trace.final_route = Some(decision.route.clone());
        trace.steps.push(TemplateTraceStep {
            stage: "policy_engine".into(),
            detail: "profile and policy rules".into(),
            route: Some(decision.route.clone()),
        });
        (decision, trace)
    }

    fn match_template_rules(ctx: &ConnectionContext, tmpl: &ResolvedTemplate) -> Option<TrafficRoute> {
        for rule in &tmpl.app_rules {
            if rule.enabled && rule.app_id == ctx.app_id {
                return Some(rule.route.clone());
            }
        }
        if let Some(domain) = &ctx.domain {
            for rule in &tmpl.domain_rules {
                if rule.enabled && domain_matches(domain, &rule.pattern) {
                    return Some(rule.route.clone());
                }
            }
        }
        None
    }

    fn evaluate_scope(&self, filter: RuleScopeFilter, ctx: &ConnectionContext) -> Option<Decision> {
        let mut candidates: Vec<&Rule> = self
            .ruleset
            .rules
            .iter()
            .filter(|r| r.enabled && filter.matches(&r.scope))
            .collect();

        candidates.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.id.cmp(&b.id))
        });

        candidates.first().map(|rule| {
            let route = self.action_to_route(&rule.action, ctx);
            let reason = format!("matched rule {}", rule.id);
            Decision {
                route: route.clone(),
                verdict: Verdict::from_route(&route, Some(rule.id), reason),
                matched_rule_id: Some(rule.id),
            }
        })
    }

    fn action_to_route(&self, action: &RuleAction, ctx: &ConnectionContext) -> TrafficRoute {
        match action {
            RuleAction::Allow => TrafficRoute::Direct,
            RuleAction::Block => TrafficRoute::Blocked,
            RuleAction::RouteDirect => TrafficRoute::Direct,
            RuleAction::LogOnly => TrafficRoute::Direct,
            RuleAction::RouteViaVpn(profile_id) => {
                if ctx.vpn_connected && ctx.active_vpn_profile == Some(*profile_id) {
                    self.profile_to_route(*profile_id)
                } else if !ctx.vpn_connected {
                    self.vpn_disconnected_route.clone()
                } else {
                    TrafficRoute::Blocked
                }
            }
            RuleAction::RouteViaTailnet(id) => TrafficRoute::Tailnet(*id),
            RuleAction::RouteViaTor(id) => TrafficRoute::Tor(*id),
            RuleAction::RouteViaProxy(id) => TrafficRoute::Proxy(*id),
            RuleAction::RouteViaProxyChain(id) => TrafficRoute::ProxyChain(*id),
            RuleAction::RouteViaChain(id) => TrafficRoute::Chain(*id),
            RuleAction::RouteViaAnonymous(route) => TrafficRoute::Anonymous(route.clone()),
            RuleAction::RouteViaMixnet(id) => {
                TrafficRoute::Anonymous(shared_types::AnonymousRoute::FutureMixnet(*id))
            }
            RuleAction::SegmentDeny(_) => TrafficRoute::Blocked,
        }
    }

    fn profile_to_route(&self, profile_id: Uuid) -> TrafficRoute {
        match self.profiles.backend_for(profile_id) {
            Some(shared_types::VpnBackendKind::AmneziaWg) => TrafficRoute::AmneziaWG(profile_id),
            Some(shared_types::VpnBackendKind::Tailscale) => TrafficRoute::Tailnet(profile_id),
            Some(shared_types::VpnBackendKind::WireGuardNt) | None => {
                TrafficRoute::WireGuard(profile_id)
            }
        }
    }
}

enum RuleScopeFilter {
    Global,
    App(Uuid),
    Domain(String),
}

impl RuleScopeFilter {
    fn matches(&self, scope: &RuleScope) -> bool {
        match (self, scope) {
            (Self::Global, RuleScope::Global) => true,
            (Self::App(id), RuleScope::App(rule_id)) => id == rule_id,
            (Self::Domain(d), RuleScope::Domain(rule_domain)) => domain_matches(d, rule_domain),
            _ => false,
        }
    }
}

fn domain_matches(query: &str, pattern: &str) -> bool {
    let query = query.trim_end_matches('.').to_ascii_lowercase();
    let pattern = pattern.trim_start_matches("*.").to_ascii_lowercase();
    if query == pattern {
        return true;
    }
    query.ends_with(&format!(".{pattern}"))
}

impl Decision {
    pub fn is_blocked(&self) -> bool {
        self.route.is_blocked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lookup::{EmptyProfileLookup, HashMapProfileLookup};
    use shared_types::VpnBackendKind;
    use std::collections::HashMap;

    fn ctx(app_id: Uuid) -> ConnectionContext {
        ConnectionContext {
            app_id,
            domain: Some("example.com".into()),
            vpn_connected: true,
            active_vpn_profile: Some(Uuid::new_v4()),
            default_route: None,
            ztna_subject: None,
        }
    }

    fn engine_with_profiles(ruleset: Ruleset, profiles: HashMap<Uuid, VpnBackendKind>) -> PolicyEngine {
        PolicyEngine::new(ruleset, Arc::new(HashMapProfileLookup::new(profiles)))
    }

    #[test]
    fn kill_switch_blocks_all() {
        let mut engine = PolicyEngine::default();
        engine.set_kill_switch(true);
        let decision = engine.decide(&ctx(Uuid::new_v4()));
        assert!(decision.is_blocked());
    }

    #[test]
    fn app_rule_overrides_global() {
        let app_a = Uuid::new_v4();
        let app_b = Uuid::new_v4();
        let mut ruleset = Ruleset::default();
        ruleset.rules.push(Rule::global(10, RuleAction::Block));
        ruleset.rules.push(Rule::for_app(app_a, 20, RuleAction::Allow));

        let engine = PolicyEngine::new(ruleset, Arc::new(EmptyProfileLookup));
        let allow = engine.decide(&ConnectionContext {
            app_id: app_a,
            domain: Some("example.com".into()),
            vpn_connected: false,
            active_vpn_profile: None,
            default_route: None,
            ztna_subject: None,
        });
        assert!(!allow.is_blocked());

        let block = engine.decide(&ConnectionContext {
            app_id: app_b,
            domain: Some("example.com".into()),
            vpn_connected: false,
            active_vpn_profile: None,
            default_route: None,
            ztna_subject: None,
        });
        assert!(block.is_blocked());
    }

    #[test]
    fn route_via_vpn_maps_wireguard() {
        let profile = Uuid::from_u128(42);
        let app_id = Uuid::new_v4();
        let mut ruleset = Ruleset::default();
        ruleset
            .rules
            .push(Rule::for_app(app_id, 10, RuleAction::RouteViaVpn(profile)));

        let mut map = HashMap::new();
        map.insert(profile, VpnBackendKind::WireGuardNt);
        let engine = engine_with_profiles(ruleset, map);

        let decision = engine.decide(&ConnectionContext {
            app_id,
            domain: None,
            vpn_connected: true,
            active_vpn_profile: Some(profile),
            default_route: None,
            ztna_subject: None,
        });
        assert_eq!(decision.route, TrafficRoute::WireGuard(profile));
    }

    #[test]
    fn route_via_vpn_maps_amneziawg() {
        let profile = Uuid::from_u128(43);
        let app_id = Uuid::new_v4();
        let mut ruleset = Ruleset::default();
        ruleset
            .rules
            .push(Rule::for_app(app_id, 10, RuleAction::RouteViaVpn(profile)));

        let mut map = HashMap::new();
        map.insert(profile, VpnBackendKind::AmneziaWg);
        let engine = engine_with_profiles(ruleset, map);

        let decision = engine.decide(&ConnectionContext {
            app_id,
            domain: None,
            vpn_connected: true,
            active_vpn_profile: Some(profile),
            default_route: None,
            ztna_subject: None,
        });
        assert_eq!(decision.route, TrafficRoute::AmneziaWG(profile));
    }

    #[test]
    fn whitelist_default_blocks() {
        let mut ruleset = Ruleset::default();
        ruleset.mode = PolicyMode::Whitelist;
        let engine = PolicyEngine::new(ruleset, Arc::new(EmptyProfileLookup));
        let decision = engine.decide(&ConnectionContext {
            app_id: Uuid::new_v4(),
            domain: None,
            vpn_connected: false,
            active_vpn_profile: None,
            default_route: None,
            ztna_subject: None,
        });
        assert!(decision.is_blocked());
    }
}
