//! WireSentinel core service library.

pub const SERVICE_NAME: &str = "WireSentinel";

pub mod api;
pub mod audit;
pub mod auth;
pub mod backup;
pub mod benchmark;
pub mod cloud_backup_reporter;
pub mod cloud_telemetry_reporter;
pub mod cloud_usage_reporter;
pub mod controller_agent;
pub mod ztna_agent;
pub mod ztna_policy_hook;
pub mod sse_agent;
pub mod sse_decision_hook;
pub mod xdr_agent;
pub mod xdr_response_executor;
pub mod cloud_sync;
pub mod correlation;
pub mod deps;
pub mod diagnostics;
pub mod domain_cache;
pub mod enterprise;
pub mod fault_injection;
pub mod leak_detector;
pub mod logging;
pub mod metrics;
pub mod orchestrator;
pub mod performance;
pub mod plugins;
pub mod mixnet;
pub mod privacy;
pub mod mixnet_security;
pub mod cover_traffic;
pub mod anonymous_routing;
pub mod privacy_analytics;
pub mod proxy;
pub mod guardian_hybrid;
pub mod kernel_route_bridge;
pub mod kernel_telemetry;
pub mod mixnet_redirect;
pub mod anonymity;
pub mod anonymity_security;
pub mod anonymity_entropy;
pub mod anonymity_discovery;
pub mod anonymity_decoy;
pub mod proxy_redirect;
pub mod recovery;
pub mod route_stats;
pub mod service;
pub mod split_tunnel;
pub mod split_templates;
pub mod tcp_termination;
pub mod tailscale;
pub mod tor;
pub mod transport;
pub mod update;
pub mod validation;
pub mod security_audit;
pub mod wfp_lifecycle;

use orchestrator::Orchestrator;
use storage::Storage;
use tokio::sync::watch;

pub async fn init_logging() -> anyhow::Result<()> {
    let storage = Storage::open().await?;
    let level = storage.settings.log_level().await.unwrap_or_default();
    let json = storage.settings.log_json_enabled().await.unwrap_or(false);
    let max_files = storage.settings.log_max_files().await.unwrap_or(5);
    let _ = logging::LoggingService::init(level, json, max_files);
    Ok(())
}

pub async fn run_service(external_shutdown: Option<watch::Receiver<bool>>) -> anyhow::Result<()> {
    init_logging().await?;

    let orchestrator = Orchestrator::new().await?;

    if let Some(mut ext) = external_shutdown {
        let tx = orchestrator.shutdown_sender();
        tokio::spawn(async move {
            while ext.changed().await.is_ok() {
                if *ext.borrow() {
                    let _ = tx.send(true);
                    break;
                }
            }
        });
    }

    orchestrator.start().await?;
    orchestrator.wait_for_shutdown().await;
    orchestrator.stop().await?;
    Ok(())
}
