use core_service::ztna_agent::{ZtnaAgent, ZtnaAgentConfig};
use core_service::ztna_policy_hook::evaluate_ztna_gate;
use policy_engine::{ConnectionContext, ZtnaPolicyLookup};
use shared_types::{
    Action, Subject, SubjectKind, ZtnaPolicyBundle, ZtnaSecurityPolicy,
};
use std::sync::Arc;
use storage::{init_pool_in_memory, Storage};
use uuid::Uuid;
use ztna_connectors::ApplicationConnector;

#[tokio::test]
async fn ztna_agent_config_round_trip() {
    let pool = init_pool_in_memory().await.expect("pool");
    let storage = Arc::new(Storage::new(pool));
    let cfg = ZtnaAgentConfig {
        enabled: true,
        controller_url: "https://controller.example.test".into(),
        enrollment_token: Some("token".into()),
        device_id: Some(Uuid::new_v4().to_string()),
        connector_name: "edge-1".into(),
        connector_endpoint: "https://edge.example.test:8443".into(),
        heartbeat_interval_secs: 45,
    };
    ZtnaAgent::save_config(&storage, &cfg)
        .await
        .expect("save");
    let loaded = ZtnaAgent::load_config(&storage).await.expect("load");
    assert_eq!(loaded, cfg);
}

#[test]
fn ztna_policy_gate_denies_when_trust_insufficient() {
    let agent = ZtnaAgent::new();
    agent.set_enabled(true);

    let subject_id = Uuid::new_v4();
    let mut policy = ZtnaSecurityPolicy::new("strict");
    policy.min_trust_score = 90;
    policy.default_action = Action::Deny;

    agent.apply_policy_bundle(ZtnaPolicyBundle {
        bundle_id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        policies: vec![policy],
        published_resources: vec![],
        segments: vec![],
        issued_at: chrono::Utc::now(),
    });

    agent.set_subject(Some(Subject {
        id: subject_id,
        kind: SubjectKind::Device,
        display_name: "test-device".into(),
        email: None,
        group_ids: vec![],
        role_ids: vec![],
        device_id: Some(subject_id),
    }));

    let ctx = ConnectionContext {
        app_id: Uuid::new_v4(),
        domain: Some("app.internal".into()),
        vpn_connected: false,
        active_vpn_profile: None,
        default_route: None,
        ztna_subject: agent.current_subject(),
    };

    let decision = evaluate_ztna_gate(&agent, &ctx).expect("ztna should deny");
    assert!(decision.is_blocked());
}

#[test]
fn ztna_policy_gate_allows_when_disabled() {
    let agent = ZtnaAgent::new();
    let ctx = ConnectionContext {
        app_id: Uuid::new_v4(),
        domain: Some("example.com".into()),
        vpn_connected: false,
        active_vpn_profile: None,
        default_route: None,
        ztna_subject: None,
    };
    assert!(evaluate_ztna_gate(&agent, &ctx).is_none());
}

#[test]
fn connector_registry_tracks_registration() {
    let registry = ApplicationConnector::new();
    let registration = registry.register(
        "connector-a",
        "https://connector.example.test",
        vec![Uuid::new_v4()],
    );
    assert_eq!(registry.list().len(), 1);
    assert_eq!(registration.name, "connector-a");
}

#[test]
fn ztna_lookup_returns_none_without_subject_when_inactive() {
    let agent = ZtnaAgent::new();
    let ctx = ConnectionContext {
        app_id: Uuid::new_v4(),
        domain: None,
        vpn_connected: false,
        active_vpn_profile: None,
        default_route: None,
        ztna_subject: None,
    };
    assert!(ZtnaPolicyLookup::evaluate(&agent, &ctx).is_none());
}

#[test]
fn ztna_inactive_policy_allows_connections() {
    let agent = ZtnaAgent::new();
    agent.set_enabled(true);
    agent.set_subject(Some(Subject {
        id: Uuid::new_v4(),
        kind: SubjectKind::User,
        display_name: "user".into(),
        email: Some("user@example.test".into()),
        group_ids: vec![],
        role_ids: vec![],
        device_id: None,
    }));

    let ctx = ConnectionContext {
        app_id: Uuid::new_v4(),
        domain: Some("resource.example.test".into()),
        vpn_connected: true,
        active_vpn_profile: None,
        default_route: None,
        ztna_subject: agent.current_subject(),
    };

    let lookup = ZtnaPolicyLookup::evaluate(&agent, &ctx).expect("lookup");
    assert!(lookup.allowed);
    assert!(evaluate_ztna_gate(&agent, &ctx).is_none());
}
