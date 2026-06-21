use async_trait::async_trait;
use shared_types::ConnectionSnapshot;

/// Callback for new/updated connections from the traffic monitor.
#[async_trait]
pub trait ConnectionHandler: Send + Sync {
    async fn on_connection(&self, conn: ConnectionSnapshot);
}
