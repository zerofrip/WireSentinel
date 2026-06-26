//! Performance monitoring — CPU, memory, latency snapshots.

use chrono::Utc;
use event_bus::EventBus;
use parking_lot::Mutex;
use shared_types::{PerformanceSnapshot, Result, ServiceEventInner};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use storage::Storage;
use sysinfo::{Pid, ProcessesToUpdate, System};
use tokio::sync::watch;
use tracing::warn;
use uuid::Uuid;

static EVENT_COUNTER: AtomicU64 = AtomicU64::new(0);
static WFP_LATENCY_MS: AtomicU64 = AtomicU64::new(0);
static API_LATENCY_MS: AtomicU64 = AtomicU64::new(0);

pub fn record_event_published() {
    EVENT_COUNTER.fetch_add(1, Ordering::Relaxed);
}

pub fn record_wfp_latency_ms(ms: f64) {
    WFP_LATENCY_MS.store(ms as u64, Ordering::Relaxed);
}

pub fn record_api_latency_ms(ms: f64) {
    API_LATENCY_MS.store(ms as u64, Ordering::Relaxed);
}

pub struct PerformanceMonitor {
    storage: Arc<Storage>,
    events: EventBus,
    system: Mutex<System>,
}

impl PerformanceMonitor {
    pub fn new(storage: Arc<Storage>, events: EventBus) -> Self {
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::All, true);
        Self {
            storage,
            events,
            system: Mutex::new(system),
        }
    }

    pub fn snapshot_now(&self) -> PerformanceSnapshot {
        let mut sys = self.system.lock();
        sys.refresh_processes(ProcessesToUpdate::All, true);
        let pid = Pid::from_u32(std::process::id());
        let cpu = sys
            .process(pid)
            .map(|p| p.cpu_usage() as f64)
            .unwrap_or(0.0);
        let memory = sys.process(pid).map(|p| p.memory()).unwrap_or(0);

        PerformanceSnapshot {
            id: Uuid::new_v4(),
            cpu_percent: cpu,
            memory_bytes: memory,
            api_latency_ms: API_LATENCY_MS.load(Ordering::Relaxed) as f64,
            wfp_latency_ms: WFP_LATENCY_MS.load(Ordering::Relaxed) as f64,
            event_throughput: EVENT_COUNTER.swap(0, Ordering::Relaxed) as f64,
            timestamp: Utc::now(),
        }
    }

    pub async fn record(&self) -> Result<PerformanceSnapshot> {
        let snapshot = self.snapshot_now();
        self.storage.performance.insert(&snapshot).await?;
        self.events.publish(
            ServiceEventInner::PerformanceSnapshot {
                snapshot: snapshot.clone(),
            }
            .with_timestamp(snapshot.timestamp),
        );
        Ok(snapshot)
    }

    pub async fn latest(&self) -> Result<Option<PerformanceSnapshot>> {
        self.storage.performance.latest().await
    }

    pub async fn list_recent(&self, limit: u32) -> Result<Vec<PerformanceSnapshot>> {
        self.storage.performance.list_recent(limit).await
    }

    pub fn start_periodic(
        monitor: Arc<PerformanceMonitor>,
        interval_secs: u64,
        mut shutdown: watch::Receiver<bool>,
    ) {
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(interval_secs.max(10)));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = monitor.record().await {
                            warn!(error = %e, "performance snapshot failed");
                        }
                    }
                    _ = shutdown.changed() => {
                        if *shutdown.borrow() {
                            break;
                        }
                    }
                }
            }
        });
    }
}
