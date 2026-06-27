//! TTL-cached boolean settings consulted on the per-connection hot path.
//!
//! `store_traffic_logs` / `store_firewall_decisions` / `store_dns_logs` were
//! previously read from SQLite on every connection, adding several `SELECT`s per
//! event and contending on the connection pool. These flags change rarely, so we
//! cache them in atomics and refresh at most once per [`TTL_MS`].

use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use storage::Storage;

const TTL_MS: i64 = 30_000;

pub struct HotSettings {
    storage: Arc<Storage>,
    traffic_logs: AtomicBool,
    firewall_decisions: AtomicBool,
    dns_logs: AtomicBool,
    last_refresh_ms: AtomicI64,
}

impl HotSettings {
    pub async fn new(storage: Arc<Storage>) -> Arc<Self> {
        let hot = Arc::new(Self {
            storage,
            traffic_logs: AtomicBool::new(true),
            firewall_decisions: AtomicBool::new(true),
            dns_logs: AtomicBool::new(true),
            last_refresh_ms: AtomicI64::new(0),
        });
        hot.refresh().await;
        hot
    }

    fn now_ms() -> i64 {
        chrono::Utc::now().timestamp_millis()
    }

    async fn refresh(&self) {
        let traffic = self
            .storage
            .settings
            .store_traffic_logs()
            .await
            .unwrap_or(true);
        let firewall = self
            .storage
            .settings
            .store_firewall_decisions()
            .await
            .unwrap_or(true);
        let dns = self.storage.settings.store_dns_logs().await.unwrap_or(true);
        self.traffic_logs.store(traffic, Ordering::Relaxed);
        self.firewall_decisions.store(firewall, Ordering::Relaxed);
        self.dns_logs.store(dns, Ordering::Relaxed);
        self.last_refresh_ms.store(Self::now_ms(), Ordering::Relaxed);
    }

    async fn maybe_refresh(&self) {
        if Self::now_ms() - self.last_refresh_ms.load(Ordering::Relaxed) >= TTL_MS {
            self.refresh().await;
        }
    }

    pub async fn store_traffic_logs(&self) -> bool {
        self.maybe_refresh().await;
        self.traffic_logs.load(Ordering::Relaxed)
    }

    pub async fn store_firewall_decisions(&self) -> bool {
        self.maybe_refresh().await;
        self.firewall_decisions.load(Ordering::Relaxed)
    }

    pub async fn store_dns_logs(&self) -> bool {
        self.maybe_refresh().await;
        self.dns_logs.load(Ordering::Relaxed)
    }

    /// Force the next read to reload from the database; call after a settings write.
    pub fn invalidate(&self) {
        self.last_refresh_ms.store(0, Ordering::Relaxed);
    }
}
