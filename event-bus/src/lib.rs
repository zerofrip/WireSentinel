//! Typed event bus for service-wide pub/sub.

use shared_types::ServiceEvent;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::broadcast;
use tracing::debug;

const DEFAULT_CAPACITY: usize = 1024;

static PUBLISH_COUNT: AtomicU64 = AtomicU64::new(0);

/// Drain and reset the event publish counter (benchmark throughput).
pub fn drain_publish_count() -> u64 {
    PUBLISH_COUNT.swap(0, Ordering::Relaxed)
}

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<ServiceEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(DEFAULT_CAPACITY);
        Self { tx }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn publish(&self, event: ServiceEvent) {
        PUBLISH_COUNT.fetch_add(1, Ordering::Relaxed);
        debug!(event = ?std::mem::discriminant(&event), "publishing event");
        let _ = self.tx.send(event);
    }

    pub fn has_subscribers(&self) -> bool {
        self.tx.receiver_count() > 0
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServiceEvent> {
        self.tx.subscribe()
    }

    pub fn sender(&self) -> broadcast::Sender<ServiceEvent> {
        self.tx.clone()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use shared_types::ServiceEventInner;

    #[tokio::test]
    async fn publish_subscribe() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        bus.publish(
            ServiceEventInner::SystemWarning {
                message: "test".into(),
            }
            .with_timestamp(Utc::now()),
        );
        let event = rx.recv().await.unwrap();
        assert!(matches!(event, ServiceEvent::SystemWarning { .. }));
    }
}
