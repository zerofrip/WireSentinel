//! Performance benchmark service — latency-focused snapshots.

use chrono::Utc;
use event_bus::drain_publish_count;
use shared_types::{BenchmarkSnapshot, Result};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use storage::Storage;
use tokio::sync::watch;
use tracing::warn;
use uuid::Uuid;

static WFP_LATENCY_MS: AtomicU64 = AtomicU64::new(0);
static ROUTE_LATENCY_MS: AtomicU64 = AtomicU64::new(0);
static DNS_LATENCY_MS: AtomicU64 = AtomicU64::new(0);
static TRANSPORT_STARTUP_MS: AtomicU64 = AtomicU64::new(0);
static UI_EVENT_THROUGHPUT: AtomicU64 = AtomicU64::new(0);

pub fn record_wfp_latency_ms(ms: f64) {
    WFP_LATENCY_MS.store(ms as u64, Ordering::Relaxed);
}

pub fn record_route_latency_ms(ms: f64) {
    ROUTE_LATENCY_MS.store(ms as u64, Ordering::Relaxed);
}

pub fn record_dns_latency_ms(ms: f64) {
    DNS_LATENCY_MS.store(ms as u64, Ordering::Relaxed);
}

pub fn record_transport_startup_ms(ms: f64) {
    TRANSPORT_STARTUP_MS.store(ms as u64, Ordering::Relaxed);
}

pub fn record_ui_event_published() {
    UI_EVENT_THROUGHPUT.fetch_add(1, Ordering::Relaxed);
}

pub struct BenchmarkService {
    storage: Arc<Storage>,
}

impl BenchmarkService {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    pub fn snapshot_now(&self) -> BenchmarkSnapshot {
        BenchmarkSnapshot {
            id: Uuid::new_v4(),
            wfp_latency_ms: WFP_LATENCY_MS.load(Ordering::Relaxed) as f64,
            route_latency_ms: ROUTE_LATENCY_MS.load(Ordering::Relaxed) as f64,
            dns_latency_ms: DNS_LATENCY_MS.load(Ordering::Relaxed) as f64,
            transport_startup_ms: TRANSPORT_STARTUP_MS.load(Ordering::Relaxed) as f64,
            ui_event_throughput: drain_publish_count() as f64
                + UI_EVENT_THROUGHPUT.swap(0, Ordering::Relaxed) as f64,
            timestamp: Utc::now(),
        }
    }

    pub async fn record(&self) -> Result<BenchmarkSnapshot> {
        let snapshot = self.snapshot_now();
        self.storage.benchmarks.insert(&snapshot).await?;
        Ok(snapshot)
    }

    pub async fn latest(&self) -> Result<Option<BenchmarkSnapshot>> {
        self.storage.benchmarks.latest().await
    }

    pub async fn list_recent(&self, limit: u32) -> Result<Vec<BenchmarkSnapshot>> {
        self.storage.benchmarks.list_recent(limit).await
    }

    pub fn start_periodic(
        service: Arc<Self>,
        interval_secs: u64,
        mut shutdown: watch::Receiver<bool>,
    ) {
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(interval_secs.max(10)));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = service.record().await {
                            warn!(error = %e, "benchmark snapshot failed");
                        }
                    }
                    changed = shutdown.changed() => {
                        if changed.is_ok() && *shutdown.borrow() {
                            break;
                        }
                    }
                }
            }
        });
    }
}
