use core_service::deps::ServiceDeps;
use event_bus::EventBus;
use parking_lot::RwLock;
use shared_types::{ConnectionSnapshot, Protocol, ServiceEvent, TrafficRoute};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use storage::{init_pool_in_memory, Storage};
use uuid::Uuid;

async fn test_deps() -> Arc<ServiceDeps> {
    let pool = init_pool_in_memory().await.expect("pool");
    let storage = Arc::new(Storage::new(pool));
    let events = EventBus::new();
    let token = Arc::new(RwLock::new("test-token".to_string()));
    Arc::new(
        ServiceDeps::build(storage, events, token)
            .await
            .expect("deps"),
    )
}

fn sample_conn(port: u16) -> ConnectionSnapshot {
    ConnectionSnapshot {
        pid: 1234,
        app_id: None,
        exe_name: "test.exe".into(),
        protocol: Protocol::Udp,
        local_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)), 53000),
        remote_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), port),
        state: "ESTABLISHED".into(),
        remote_domain: None,
        bytes_sent: 100,
        bytes_received: 50,
    }
}

#[tokio::test]
async fn detects_dns_leak_on_port_53() {
    let deps = test_deps().await;
    let mut rx = deps.events.subscribe();
    let app_id = Uuid::new_v4();

    deps.leak_detector
        .check_connection(&sample_conn(53), app_id, &TrafficRoute::Direct, true)
        .await
        .expect("check");

    let incidents = deps
        .storage
        .leak_incidents
        .list_recent(10)
        .await
        .expect("list");
    assert_eq!(incidents.len(), 1);
    assert_eq!(incidents[0].leak_type, shared_types::LeakType::Dns);

    let event = rx.try_recv().expect("event");
    assert!(matches!(event, ServiceEvent::LeakDetected { .. }));
}

#[tokio::test]
async fn detects_route_leak_with_kill_switch() {
    let deps = test_deps().await;
    deps.policy.write().set_kill_switch(true);
    let app_id = Uuid::new_v4();

    deps.leak_detector
        .check_connection(&sample_conn(443), app_id, &TrafficRoute::Direct, true)
        .await
        .expect("check");

    let incidents = deps
        .storage
        .leak_incidents
        .list_recent(10)
        .await
        .expect("list");
    assert!(incidents
        .iter()
        .any(|i| i.leak_type == shared_types::LeakType::Route));
}

#[tokio::test]
async fn no_leak_when_detection_disabled() {
    let deps = test_deps().await;
    deps.storage
        .settings
        .set("leak_detection_enabled", "false")
        .await
        .expect("set");
    let app_id = Uuid::new_v4();

    deps.leak_detector
        .check_connection(&sample_conn(53), app_id, &TrafficRoute::Direct, true)
        .await
        .expect("check");

    let incidents = deps
        .storage
        .leak_incidents
        .list_recent(10)
        .await
        .expect("list");
    assert!(incidents.is_empty());
}
