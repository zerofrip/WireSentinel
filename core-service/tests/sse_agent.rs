use core_service::sse_agent::{SseAgent, SseAgentConfig};
use core_service::sse_decision_hook::evaluate_sse_gate;
use policy_engine::{ConnectionContext, SsePolicyLookup};
use shared_types::{
    DlpAction, DlpPatternKind, DlpPolicy, RiskLevel, SsePolicyBundle, ThreatIndicator, UrlCategory,
    WebAccessAction, WebPolicy,
};
use std::sync::Arc;
use storage::{init_pool_in_memory, Storage};
use uuid::Uuid;

#[tokio::test]
async fn sse_agent_config_round_trip() {
    let pool = init_pool_in_memory().await.expect("pool");
    let storage = Arc::new(Storage::new(pool));
    let cfg = SseAgentConfig {
        enabled: true,
        controller_url: "https://controller.example.test".into(),
        enrollment_token: Some("token".into()),
        device_id: Some(Uuid::new_v4().to_string()),
        telemetry_interval_secs: 45,
    };
    SseAgent::save_config(&storage, &cfg).await.expect("save");
    let loaded = SseAgent::load_config(&storage).await.expect("load");
    assert_eq!(loaded, cfg);
}

#[test]
fn sse_policy_gate_denies_threat_domain() {
    let agent = SseAgent::new();
    agent.set_enabled(true);

    let indicator = ThreatIndicator {
        id: Uuid::new_v4(),
        indicator_type: "domain".into(),
        value: "phish.evil.test".into(),
        severity: RiskLevel::Critical,
        source_feed_id: Uuid::new_v4(),
        expires_at: None,
    };

    agent.apply_policy_bundle(SsePolicyBundle {
        bundle_id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        security_policy: None,
        web_policies: vec![],
        dlp_policies: vec![],
        threat_indicators: vec![indicator],
        issued_at: chrono::Utc::now(),
    });

    let ctx = ConnectionContext {
        app_id: Uuid::new_v4(),
        domain: Some("phish.evil.test".into()),
        vpn_connected: false,
        active_vpn_profile: None,
        default_route: None,
        exit_routes: Vec::new(),
        active_exit_index: 0,
        ztna_subject: None,
    };

    let decision = evaluate_sse_gate(&agent, &ctx).expect("sse should deny");
    assert!(decision.is_blocked());
}

#[test]
fn sse_policy_gate_allows_when_disabled() {
    let agent = SseAgent::new();
    let ctx = ConnectionContext {
        app_id: Uuid::new_v4(),
        domain: Some("example.com".into()),
        vpn_connected: false,
        active_vpn_profile: None,
        default_route: None,
        exit_routes: Vec::new(),
        active_exit_index: 0,
        ztna_subject: None,
    };
    assert!(evaluate_sse_gate(&agent, &ctx).is_none());
}

#[test]
fn sse_lookup_returns_none_without_domain() {
    let agent = SseAgent::new();
    agent.set_enabled(true);
    let ctx = ConnectionContext {
        app_id: Uuid::new_v4(),
        domain: None,
        vpn_connected: false,
        active_vpn_profile: None,
        default_route: None,
        exit_routes: Vec::new(),
        active_exit_index: 0,
        ztna_subject: None,
    };
    assert!(SsePolicyLookup::evaluate(&agent, &ctx).is_none());
}

#[test]
fn sse_dlp_blocks_sensitive_content() {
    let agent = SseAgent::new();
    agent.set_enabled(true);

    let mut dlp = DlpPolicy::new("pii");
    dlp.patterns = vec![DlpPatternKind::Ssn];
    dlp.action = DlpAction::Block;

    agent.apply_policy_bundle(SsePolicyBundle {
        bundle_id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        security_policy: None,
        web_policies: vec![],
        dlp_policies: vec![dlp],
        threat_indicators: vec![],
        issued_at: chrono::Utc::now(),
    });

    let ctx = ConnectionContext {
        app_id: Uuid::new_v4(),
        domain: Some("upload.example.test/SSN: 123-45-6789".into()),
        vpn_connected: true,
        active_vpn_profile: None,
        default_route: None,
        exit_routes: Vec::new(),
        active_exit_index: 0,
        ztna_subject: None,
    };

    let decision = evaluate_sse_gate(&agent, &ctx).expect("dlp deny");
    assert!(decision.is_blocked());
}

#[test]
fn sse_swg_allows_clean_domain() {
    let agent = SseAgent::new();
    agent.set_enabled(true);

    let mut swg = WebPolicy::new("corp");
    swg.blocked_domains = vec!["blocked.test".into()];
    swg.default_action = WebAccessAction::Block;

    agent.apply_policy_bundle(SsePolicyBundle {
        bundle_id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        security_policy: None,
        web_policies: vec![swg],
        dlp_policies: vec![],
        threat_indicators: vec![],
        issued_at: chrono::Utc::now(),
    });

    let ctx = ConnectionContext {
        app_id: Uuid::new_v4(),
        domain: Some("safe.example.test".into()),
        vpn_connected: false,
        active_vpn_profile: None,
        default_route: None,
        exit_routes: Vec::new(),
        active_exit_index: 0,
        ztna_subject: None,
    };

    let lookup = SsePolicyLookup::evaluate(&agent, &ctx).expect("lookup");
    assert!(lookup.allowed);
    assert!(evaluate_sse_gate(&agent, &ctx).is_none());
}

#[test]
fn sse_swg_blocks_category() {
    let agent = SseAgent::new();
    agent.set_enabled(true);

    let mut swg = WebPolicy::new("social");
    swg.blocked_categories = vec![UrlCategory::SocialMedia];
    swg.default_action = WebAccessAction::Block;

    agent.apply_policy_bundle(SsePolicyBundle {
        bundle_id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        security_policy: None,
        web_policies: vec![swg],
        dlp_policies: vec![],
        threat_indicators: vec![],
        issued_at: chrono::Utc::now(),
    });

    let ctx = ConnectionContext {
        app_id: Uuid::new_v4(),
        domain: Some("www.facebook.com".into()),
        vpn_connected: false,
        active_vpn_profile: None,
        default_route: None,
        exit_routes: Vec::new(),
        active_exit_index: 0,
        ztna_subject: None,
    };

    let decision = evaluate_sse_gate(&agent, &ctx).expect("swg deny");
    assert!(decision.is_blocked());
}
