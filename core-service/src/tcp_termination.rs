//! VPN gateway compatibility TCP termination service wrapper.

use shared_types::Result;
use std::sync::Arc;
use storage::Storage;
use tcp_termination::TcpTerminationEngine;
use uuid::Uuid;

pub struct TcpTerminationService {
    engine: Arc<TcpTerminationEngine>,
    storage: Arc<Storage>,
}

impl TcpTerminationService {
    pub fn new(storage: Arc<Storage>, engine: Arc<TcpTerminationEngine>) -> Self {
        Self { storage, engine }
    }

    pub fn engine(&self) -> Arc<TcpTerminationEngine> {
        Arc::clone(&self.engine)
    }

    pub async fn reload_policy(&self) -> Result<()> {
        let policy = self.storage.tcp_termination.load_policy().await?;
        self.engine.set_policy(policy);
        Ok(())
    }

    pub async fn on_vpn_connect(&self, profile_id: Uuid) -> Result<u32> {
        self.engine.on_vpn_connect(profile_id).await
    }

    pub async fn on_vpn_disconnect(&self, profile_id: Uuid) -> Result<u32> {
        self.engine.on_vpn_disconnect(profile_id).await
    }

    pub async fn on_route_change(
        &self,
        app_id: Uuid,
        old_route: Option<shared_types::TrafficRoute>,
        new_route: Option<shared_types::TrafficRoute>,
    ) -> Result<u32> {
        self.engine
            .on_route_change(Some(app_id), old_route, new_route)
            .await
    }
}
