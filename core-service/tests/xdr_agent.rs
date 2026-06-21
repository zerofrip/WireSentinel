use core_service::xdr_agent::{XdrAgent, XdrAgentConfig};
use core_service::xdr_response_executor::{response_request_from_command, CoreXdrPolicyLookup};
use policy_engine::XdrPolicyLookup;
use shared_types::{ProcessEvent, ResponseActionKind, XdrSecurityPolicy};
use uuid::Uuid;

#[test]
fn xdr_agent_config_defaults_disabled() {
    let cfg = XdrAgentConfig::default();
    assert!(!cfg.enabled);
    assert_eq!(cfg.telemetry_interval_secs, 60);
}

#[tokio::test]
async fn xdr_agent_ingest_process_increments_telemetry() {
    let events = event_bus::EventBus::with_capacity(16);
    let agent = XdrAgent::new(events);
    let device_id = Uuid::new_v4();

    agent
        .ingest_process(ProcessEvent {
            id: Uuid::new_v4(),
            device_id,
            pid: 1234,
            parent_pid: Some(1),
            process_name: "cmd.exe".into(),
            command_line: Some("cmd /c echo".into()),
            user: Some("SYSTEM".into()),
            observed_at: chrono::Utc::now(),
        })
        .unwrap();

    let payload = agent.telemetry_payload();
    assert_eq!(payload.process_events, 1);
}

#[test]
fn xdr_policy_lookup_blocks_disallowed_actions() {
    let lookup = CoreXdrPolicyLookup::new(XdrSecurityPolicy::default());
    assert!(lookup.is_action_allowed(ResponseActionKind::BlockDomain));
    assert!(!lookup.is_action_allowed(ResponseActionKind::KillProcess));
}

#[test]
fn response_request_builder_sets_fields() {
    let tenant = Uuid::new_v4();
    let request = response_request_from_command(
        tenant,
        ResponseActionKind::BlockDomain,
        "evil.example",
        "test",
    );
    assert_eq!(request.tenant_id, tenant);
    assert_eq!(request.target, "evil.example");
}
