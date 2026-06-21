use policy_engine::{ConnectionContext, PolicyEngine};
use shared_types::{AnonymousRoute, Rule, RuleAction, Ruleset, TrafficRoute};
use std::sync::Arc;
use uuid::Uuid;

#[test]
fn route_via_anonymous_maps_to_traffic_route() {
    let profile_id = Uuid::from_u128(100);
    let app_id = Uuid::new_v4();
    let mut ruleset = Ruleset::default();
    ruleset.rules.push(Rule::for_app(
        app_id,
        10,
        RuleAction::RouteViaAnonymous(AnonymousRoute::Tor(profile_id)),
    ));

    let engine = engine_with_rules(ruleset);
    let decision = engine.decide(&ConnectionContext {
        app_id,
        domain: None,
        vpn_connected: false,
        active_vpn_profile: None,
        default_route: None,
        ztna_subject: None,
    });

    assert_eq!(
        decision.route,
        TrafficRoute::Anonymous(AnonymousRoute::Tor(profile_id))
    );
}

#[test]
fn route_via_mixnet_maps_to_future_mixnet_anonymous() {
    let profile_id = Uuid::from_u128(101);
    let app_id = Uuid::new_v4();
    let mut ruleset = Ruleset::default();
    ruleset.rules.push(Rule::for_app(
        app_id,
        10,
        RuleAction::RouteViaMixnet(profile_id),
    ));

    let engine = engine_with_rules(ruleset);
    let decision = engine.decide(&ConnectionContext {
        app_id,
        domain: None,
        vpn_connected: false,
        active_vpn_profile: None,
        default_route: None,
        ztna_subject: None,
    });

    assert_eq!(
        decision.route,
        TrafficRoute::Anonymous(AnonymousRoute::FutureMixnet(profile_id))
    );
}

fn engine_with_rules(ruleset: Ruleset) -> PolicyEngine {
    PolicyEngine::new(ruleset, Arc::new(TestLookup))
}

struct TestLookup;

impl policy_engine::ProfileLookup for TestLookup {
    fn backend_for(&self, _profile_id: Uuid) -> Option<shared_types::VpnBackendKind> {
        None
    }
}
