//! HTTP REST + WebSocket API server.

mod anonymity_routes;
mod app_routes;
mod kernel_routes;
mod middleware;
mod mixnet_routes;
mod openapi;
mod proxy_routes;
pub mod routes;
mod settings_routes;
mod tailnet_routes;
mod tor_routes;
pub mod vpn_gateway_compat;
mod ws;

use crate::deps::ServiceDeps;
use shared_types::{Result, WireSentinelError};
use std::sync::Arc;

pub struct AppState {
    pub deps: Arc<ServiceDeps>,
}

impl AppState {
    pub fn from_deps(deps: Arc<ServiceDeps>) -> Self {
        Self { deps }
    }
}

pub async fn serve(state: AppState, port: u16) -> Result<()> {
    let app = routes::router(state);
    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| WireSentinelError::Api(format!("bind {addr}: {e}")))?;
    tracing::info!(addr = %addr, "API server listening");
    axum::serve(listener, app)
        .await
        .map_err(|e| WireSentinelError::Api(format!("serve: {e}")))?;
    Ok(())
}
