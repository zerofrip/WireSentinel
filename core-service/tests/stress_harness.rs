use chrono::Utc;
use core_service::deps::ServiceDeps;
use event_bus::EventBus;
use parking_lot::RwLock;
use shared_types::{ConnectionSnapshot, Protocol, StressTestReport};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Instant;
use storage::{init_pool_in_memory, Storage};

async fn build_deps() -> Arc<ServiceDeps> {
    let pool = init_pool_in_memory().await.expect("pool");
    let storage = Arc::new(Storage::new(pool));
    let events = EventBus::new();
    let token = Arc::new(RwLock::new("stress".into()));
    Arc::new(
        ServiceDeps::build(storage, events, token)
            .await
            .expect("deps"),
    )
}

#[tokio::test]
#[ignore = "stress harness — run in CI nightly"]
async fn connection_flood_stress() {
    let deps = build_deps().await;
    let start = Instant::now();
    let mut errors = 0u32;
    let count = 1000u64;

    for i in 0..count {
        let conn = ConnectionSnapshot {
            pid: (i % 65535) as u32 + 1,
            app_id: None,
            exe_name: format!("stress-{i}.exe"),
            protocol: Protocol::Tcp,
            local_addr: SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                40000 + (i % 1000) as u16,
            ),
            remote_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)), 443),
            state: "ESTABLISHED".into(),
            remote_domain: Some(format!("example-{i}.com")),
            bytes_sent: 100,
            bytes_received: 200,
        };
        if deps.process_connection(conn).await.is_err() {
            errors += 1;
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    let throughput = event_bus::drain_publish_count() as f64
        / (duration_ms.max(1) as f64 / 1000.0);
    let report = StressTestReport {
        connections_processed: count,
        duration_ms,
        memory_bytes_peak: 0,
        event_throughput: throughput,
        errors,
        timestamp: Utc::now(),
    };
    eprintln!("{}", serde_json::to_string_pretty(&report).unwrap());
    assert!(u64::from(errors) < count / 2);
}
