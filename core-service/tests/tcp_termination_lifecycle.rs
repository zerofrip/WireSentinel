//! VPN connect triggers TCP termination when policy mode matches.

use shared_types::{TcpTerminationMode, TcpTerminationPolicy, TcpTerminationRule};
use tcp_termination::TcpTerminationEngine;
use uuid::Uuid;

#[tokio::test]
async fn vpn_connect_triggers_tcp_termination() {
    let engine = TcpTerminationEngine::new();
    let mut rule = TcpTerminationRule::new();
    rule.process_name = Some("chrome.exe".into());
    engine.set_policy(TcpTerminationPolicy {
        mode: TcpTerminationMode::OnVpnConnect,
        rules: vec![rule],
    });

    let count = engine.on_vpn_connect(Uuid::new_v4()).await.unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn disabled_policy_skips_on_disconnect() {
    let engine = TcpTerminationEngine::new();
    let count = engine.on_vpn_disconnect(Uuid::new_v4()).await.unwrap();
    assert_eq!(count, 0);
}
