//! Template merge/override tests.

use policy_engine::{ConnectionContext, PolicyEngine, ProfileLookup};
use shared_types::{
    AppRule, ResolvedTemplate, Ruleset, TemplateMode, TrafficRoute, VerdictKind, VpnBackendKind,
};
use split_tunnel::TemplateResolver;
use std::sync::Arc;
use uuid::Uuid;

struct EmptyLookup;
impl ProfileLookup for EmptyLookup {
    fn backend_for(&self, _profile_id: Uuid) -> Option<VpnBackendKind> {
        None
    }
}

#[test]
fn override_uses_template_app_rule() {
    let app_id = Uuid::new_v4();
    let vpn_id = Uuid::new_v4();
    let tmpl = ResolvedTemplate {
        mode: TemplateMode::Override,
        default_route: TrafficRoute::WireGuard(vpn_id),
        app_rules: vec![AppRule {
            id: Uuid::new_v4(),
            app_id,
            route: TrafficRoute::Direct,
            enabled: true,
            description: None,
        }],
        domain_rules: vec![],
        template_id: Uuid::new_v4(),
    };

    let engine = PolicyEngine::new(Ruleset::default(), Arc::new(EmptyLookup));
    let ctx = ConnectionContext {
        app_id,
        domain: None,
        vpn_connected: true,
        active_vpn_profile: Some(vpn_id),
        default_route: None,
        ztna_subject: None,
    };
    let (decision, _) = engine.decide_with_template(&ctx, Some(&tmpl));
    assert_eq!(decision.route, TrafficRoute::Direct);
}

#[test]
fn merge_falls_through_to_policy_block() {
    let app_id = Uuid::new_v4();
    let other_app = Uuid::new_v4();
    let tmpl = ResolvedTemplate {
        mode: TemplateMode::Merge,
        default_route: TrafficRoute::Direct,
        app_rules: vec![AppRule {
            id: Uuid::new_v4(),
            app_id,
            route: TrafficRoute::WireGuard(Uuid::new_v4()),
            enabled: true,
            description: None,
        }],
        domain_rules: vec![],
        template_id: Uuid::new_v4(),
    };

    let mut ruleset = Ruleset::default();
    ruleset.rules.push(shared_types::Rule::for_app(
        other_app,
        10,
        shared_types::RuleAction::Block,
    ));
    let engine = PolicyEngine::new(ruleset, Arc::new(EmptyLookup));
    let ctx = ConnectionContext {
        app_id: other_app,
        domain: None,
        vpn_connected: true,
        active_vpn_profile: None,
        default_route: None,
        ztna_subject: None,
    };
    let (decision, _) = engine.decide_with_template(&ctx, Some(&tmpl));
    assert_eq!(decision.verdict.kind, VerdictKind::Block);
}

#[test]
fn resolver_builds_from_template() {
    let t = shared_types::SplitTunnelTemplate::new("work".into(), TrafficRoute::Direct);
    let resolved = TemplateResolver::build_resolved(TemplateMode::Merge, Some(t));
    assert!(resolved.is_some());
}
