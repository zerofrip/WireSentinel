//! WireSentinel core service library.

pub const SERVICE_NAME: &str = "WireSentinel";

pub mod anonymity;
pub mod anonymity_decoy;
pub mod anonymity_discovery;
pub mod anonymity_entropy;
pub mod anonymity_security;
pub mod anonymous_routing;
pub mod api;
pub mod audit;
pub mod auth;
pub mod backup;
pub mod benchmark;
pub mod binary_paths;
pub mod cloud_backup_reporter;
pub mod cloud_sync;
pub mod cloud_telemetry_reporter;
pub mod cloud_usage_reporter;
pub mod controller_agent;
pub mod correlation;
pub mod cover_traffic;
pub mod deps;
pub mod deterministic_id;
pub mod diagnostics;
pub mod domain_cache;
pub mod enforcement;
pub mod enterprise;
pub mod exit_failover;
pub mod fault_injection;
pub mod guardian_hybrid;
pub mod hot_settings;
pub mod kernel_route_bridge;
pub mod kernel_telemetry;
pub mod leak_detector;
pub mod log_retention;
pub mod logging;
pub mod metrics;
pub mod mixnet;
pub mod mixnet_redirect;
pub mod mixnet_security;
pub mod orchestrator;
pub mod performance;
pub mod plugins;
pub mod privacy;
pub mod privacy_analytics;
pub mod proxy;
pub mod proxy_redirect;
pub mod recovery;
pub mod route_stats;
pub mod security_audit;
pub mod service;
pub mod split_templates;
pub mod split_tunnel;
pub mod sse_agent;
pub mod sse_decision_hook;
pub mod tailscale;
pub mod tcp_termination;
pub mod tor;
pub mod tor_control;
pub mod transport;
pub mod update;
pub mod validation;
pub mod wfp_lifecycle;
pub mod xdr_agent;
pub mod xdr_response_executor;
pub mod ztna_agent;
pub mod ztna_policy_hook;

use orchestrator::Orchestrator;
use storage::Storage;
use tokio::sync::watch;

pub async fn init_logging() -> anyhow::Result<()> {
    let storage = Storage::open().await?;
    // #region agent log
    shared_types::debug_log::emit_kv(
        "core-service/src/lib.rs:init_logging",
        "il: storage opened",
        &[("hypothesisId", "H_LOGINIT".to_string())],
    );
    // #endregion
    let level = storage.settings.log_level().await.unwrap_or_default();
    let json = storage.settings.log_json_enabled().await.unwrap_or(false);
    let max_files = storage.settings.log_max_files().await.unwrap_or(5);
    // #region agent log
    shared_types::debug_log::emit_kv(
        "core-service/src/lib.rs:init_logging",
        "il: settings read",
        &[
            ("hypothesisId", "H_LOGINIT".to_string()),
            ("json", json.to_string()),
            ("max_files", max_files.to_string()),
        ],
    );
    // #endregion
    let _ = logging::LoggingService::init(level, json, max_files);
    // #region agent log
    shared_types::debug_log::emit_kv(
        "core-service/src/lib.rs:init_logging",
        "il: LoggingService::init returned",
        &[("hypothesisId", "H_LOGINIT".to_string())],
    );
    // #endregion
    Ok(())
}

pub async fn run_service(external_shutdown: Option<watch::Receiver<bool>>) -> anyhow::Result<()> {
    // #region agent log
    shared_types::debug_log::emit_kv(
        "core-service/src/lib.rs:run_service",
        "run_service entry",
        &[("hypothesisId", "H_EARLY".to_string())],
    );
    // #endregion
    init_logging().await?;
    // #region agent log
    shared_types::debug_log::emit_kv(
        "core-service/src/lib.rs:run_service",
        "logging initialized",
        &[("hypothesisId", "H_EARLY".to_string())],
    );
    // #endregion

    let orchestrator = Orchestrator::new().await?;
    // #region agent log
    shared_types::debug_log::emit_kv(
        "core-service/src/lib.rs:run_service",
        "orchestrator constructed",
        &[("hypothesisId", "H_EARLY".to_string())],
    );
    // #endregion

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
