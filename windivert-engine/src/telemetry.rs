use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub struct WinDivertTelemetry {
    pub packets_seen: AtomicU64,
    pub packets_modified: AtomicU64,
    pub packets_reinjected: AtomicU64,
    pub redirect_count: AtomicU64,
    pub transform_count: AtomicU64,
    pub cover_traffic_count: AtomicU64,
    pub error_count: AtomicU64,
}

impl WinDivertTelemetry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_seen(&self) {
        self.packets_seen.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_modified(&self) {
        self.packets_modified.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_reinject(&self) {
        self.packets_reinjected.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_redirect(&self) {
        self.redirect_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_transform(&self) {
        self.transform_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cover_traffic(&self) {
        self.cover_traffic_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> WinDivertTelemetrySnapshot {
        WinDivertTelemetrySnapshot {
            packets_seen: self.packets_seen.load(Ordering::Relaxed),
            packets_modified: self.packets_modified.load(Ordering::Relaxed),
            packets_reinjected: self.packets_reinjected.load(Ordering::Relaxed),
            redirect_count: self.redirect_count.load(Ordering::Relaxed),
            transform_count: self.transform_count.load(Ordering::Relaxed),
            cover_traffic_count: self.cover_traffic_count.load(Ordering::Relaxed),
            error_count: self.error_count.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WinDivertTelemetrySnapshot {
    pub packets_seen: u64,
    pub packets_modified: u64,
    pub packets_reinjected: u64,
    pub redirect_count: u64,
    pub transform_count: u64,
    pub cover_traffic_count: u64,
    pub error_count: u64,
}
