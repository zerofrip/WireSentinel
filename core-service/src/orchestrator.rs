//! Central orchestrator wiring all subsystems.

use crate::api::AppState;
use crate::auth;
use crate::benchmark::{self, BenchmarkService};
use crate::deps::ServiceDeps;
use crate::log_retention;
use crate::performance::PerformanceMonitor;
use crate::privacy::PrivacyScoreService;
use crate::privacy_analytics::PrivacyAnalyticsService;
use async_trait::async_trait;
use chrono::Utc;
use event_bus::EventBus;
use parking_lot::RwLock;
use policy_engine::ConnectionContext;
use shared_types::{
    ConnectionSnapshot, Direction, ServiceEvent, ServiceEventInner, ServiceStatus, TrafficEvent,
    ValidationStatus,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use storage::Storage;
use tokio::sync::watch;
use tracing::{info, warn};
use traffic_monitor::ConnectionHandler;

pub struct Orchestrator {
    deps: Arc<ServiceDeps>,
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
    api_handle: parking_lot::Mutex<Option<tokio::task::JoinHandle<()>>>,
    monitor_handle: parking_lot::Mutex<Option<tokio::task::JoinHandle<()>>>,
    event_bridge: parking_lot::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

struct OrchestratorHandler {
    deps: Arc<ServiceDeps>,
}

static CONN_FAIL_COUNT: AtomicU64 = AtomicU64::new(0);
static LAST_CONN_FAIL_LOG: std::sync::OnceLock<RwLock<Option<Instant>>> = std::sync::OnceLock::new();
// #region agent log
static CONN_OK_COUNT: AtomicU64 = AtomicU64::new(0);
static LAST_CONN_OK_LOG: std::sync::OnceLock<RwLock<Option<Instant>>> = std::sync::OnceLock::new();
// #endregion
static BANDWIDTH_LAST_PUBLISH: std::sync::OnceLock<RwLock<HashMap<uuid::Uuid, Instant>>> =
    std::sync::OnceLock::new();

#[async_trait]
impl ConnectionHandler for OrchestratorHandler {
    async fn on_connection(&self, conn: ConnectionSnapshot) {
        let started = Instant::now();
        if let Err(e) = self.deps.as_ref().process_connection(conn).await {
            let n = CONN_FAIL_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
            let log_gate = LAST_CONN_FAIL_LOG.get_or_init(|| RwLock::new(None));
            let mut last = log_gate.write();
            let now = Instant::now();
            if last
                .map(|t| now.duration_since(t) >= Duration::from_secs(60))
                .unwrap_or(true)
            {
                warn!(
                    error = %e,
                    suppressed_since_last = n.saturating_sub(1),
                    "connection processing failed"
                );
                // #region agent log
                shared_types::debug_log::emit_kv(
                    "core-service/src/orchestrator.rs:on_connection",
                    "process_connection error",
                    &[
                        ("hypothesisId", "H2_pool".to_string()),
                        ("error", e.to_string()),
                        ("suppressed_since_last", n.saturating_sub(1).to_string()),
                        ("elapsed_ms", started.elapsed().as_millis().to_string()),
                    ],
                );
                // #endregion
                CONN_FAIL_COUNT.store(0, Ordering::Relaxed);
                *last = Some(now);
            }
        } else {
            // #region agent log
            let ok = CONN_OK_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
            let gate = LAST_CONN_OK_LOG.get_or_init(|| RwLock::new(None));
            let now = Instant::now();
            let mut last = gate.write();
            if last
                .map(|t| now.duration_since(t) >= Duration::from_secs(60))
                .unwrap_or(true)
            {
                shared_types::debug_log::emit_kv(
                    "core-service/src/orchestrator.rs:on_connection",
                    "process_connection ok (sampled)",
                    &[
                        ("hypothesisId", "HEALTH".to_string()),
                        ("ok_total", ok.to_string()),
                        ("elapsed_ms", started.elapsed().as_millis().to_string()),
                    ],
                );
                *last = Some(now);
            }
            // #endregion
        }
    }
}

impl ServiceDeps {
    pub async fn process_connection(&self, conn: ConnectionSnapshot) -> shared_types::Result<()> {
        if conn.pid == 0 {
            return Ok(());
        }
        let (app, _discovered) = self.app_registry.resolve_or_register(conn.pid).await?;
        self.traffic.register_app(app.clone());

        let mut domain = conn.remote_domain.clone();
        let vpn_connected = self.vpn.any_connected().await;
        let active_vpn = self.vpn.active_profile().await;

        if domain.is_none() {
            let dest_ip = conn.remote_addr.ip().to_string();
            if let Ok(Some(resolved)) = self
                .domain_cache
                .resolve_ip_to_domain(Some(app.id()), &dest_ip)
                .await
            {
                domain = Some(resolved);
            }
        }

        let (exit_routes, active_exit_index, default_route, _) =
            self.exit_failover.resolve_active_route(&app.record);

        let ctx = ConnectionContext {
            app_id: app.id(),
            domain: domain.clone(),
            vpn_connected,
            active_vpn_profile: active_vpn,
            default_route,
            exit_routes,
            active_exit_index,
            ztna_subject: self.ztna.current_subject(),
        };

        let route_start = std::time::Instant::now();
        let decision = if let Some(ztna_decision) =
            crate::ztna_policy_hook::evaluate_ztna_gate(&*self.ztna, &ctx)
        {
            ztna_decision
        } else if let Some(sse_decision) =
            crate::sse_decision_hook::evaluate_sse_gate(&*self.sse, &ctx)
        {
            sse_decision
        } else {
            let resolved = self.split_templates.resolved_template();
            let (decision, trace) = self
                .policy
                .read()
                .decide_with_template(&ctx, resolved.as_ref());
            self.split_templates.store_trace(trace);
            decision
        };
        let wfp_start = std::time::Instant::now();
        let route = self.split_tunnel.enforce(&decision, &app).await?;
        benchmark::record_wfp_latency_ms(wfp_start.elapsed().as_secs_f64() * 1000.0);
        benchmark::record_route_latency_ms(route_start.elapsed().as_secs_f64() * 1000.0);
        let ts = Utc::now();

        if let Err(e) = self
            .leak_detector
            .check_connection(&conn, app.id(), &route, vpn_connected)
            .await
        {
            warn!(error = %e, "leak detection failed");
        }

        let mut event = TrafficEvent::new(
            app.clone(),
            Direction::Outbound,
            conn.protocol,
            conn.local_addr,
            conn.remote_addr,
            route.clone(),
            decision.verdict.clone(),
        );
        event.remote_domain = domain.clone();
        event.process_id = Some(conn.pid);
        event.source_ip = Some(conn.local_addr.ip().to_string());
        event.destination_ip = Some(conn.remote_addr.ip().to_string());
        event.source_port = Some(conn.local_addr.port());
        event.destination_port = Some(conn.remote_addr.port());
        event.bytes_out = conn.bytes_sent;
        event.bytes_in = conn.bytes_received;

        if let Some(dst) = &event.destination_ip {
            if event.remote_domain.is_none() {
                if let Ok(Some(resolved)) = self.correlator.on_traffic(Some(app.id()), dst).await {
                    event.remote_domain = Some(resolved);
                }
            }
        }

        let store_decisions = self.hot_settings.store_firewall_decisions().await;

        let firewall_record = self
            .split_tunnel
            .record_firewall_decision(
                Some(app.id()),
                event.remote_domain.clone(),
                event.destination_ip.clone(),
                &route,
                &decision.verdict,
            )
            .await?;

        if store_decisions {
            let _ = self
                .storage
                .firewall_decisions
                .insert(&firewall_record)
                .await;
        }

        self.events.publish(
            ServiceEventInner::FirewallDecision {
                decision: firewall_record,
            }
            .with_timestamp(ts),
        );

        let stats = self
            .split_tunnel
            .record_usage(
                Some(app.id()),
                route.profile_id(),
                event.remote_domain.clone(),
                &route,
                conn.bytes_received,
                conn.bytes_sent,
            )
            .await?;

        self.events
            .publish(ServiceEventInner::RouteUsageUpdated { stats }.with_timestamp(ts));

        if self.hot_settings.store_traffic_logs().await {
            let _ = self.storage.traffic_logs.insert(&event).await;
        }

        self.traffic.emit_traffic(event.clone());
        self.traffic.update_bandwidth(
            app.id(),
            app.display_name(),
            conn.bytes_received,
            conn.bytes_sent,
        );

        if self.events.has_subscribers() {
            let throttle = BANDWIDTH_LAST_PUBLISH.get_or_init(|| RwLock::new(HashMap::new()));
            let now = Instant::now();
            let publish = {
                let mut map = throttle.write();
                match map.get(&app.id()) {
                    Some(t) if now.duration_since(*t) < Duration::from_secs(1) => false,
                    _ => {
                        map.insert(app.id(), now);
                        true
                    }
                }
            };
            if publish {
                if let Some(snapshot) = self
                    .traffic
                    .bandwidth_snapshots()
                    .into_iter()
                    .find(|s| s.app_id == app.id())
                {
                    self.events
                        .publish(ServiceEventInner::BandwidthUpdated { snapshot }.with_timestamp(ts));
                }
            }
        }

        if route.is_blocked() {
            self.events
                .publish(ServiceEventInner::TrafficBlocked { event, route }.with_timestamp(ts));
        } else {
            self.events
                .publish(ServiceEventInner::TrafficAllowed { event }.with_timestamp(ts));
        }

        Ok(())
    }

    pub fn status(&self) -> ServiceStatus {
        ServiceStatus {
            running: true,
            kill_switch_active: self.policy.read().kill_switch_active(),
            policy_mode: self.policy.read().ruleset().mode,
            active_vpn_count: self.vpn.active_count(),
            monitored_app_count: self.traffic.apps().len() as u32,
            connection_count: self.traffic.connection_count(),
            api_port: self.api_listen_port,
        }
    }
}

impl Orchestrator {
    pub async fn new() -> shared_types::Result<Arc<Self>> {
        let storage = Arc::new(Storage::open().await?);
        let events = EventBus::new();
        let api_token = Arc::new(RwLock::new(auth::load_or_create_token()?));

        let deps = Arc::new(ServiceDeps::build(storage, events, api_token).await?);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        Ok(Arc::new(Self {
            deps,
            shutdown_tx,
            shutdown_rx,
            api_handle: parking_lot::Mutex::new(None),
            monitor_handle: parking_lot::Mutex::new(None),
            event_bridge: parking_lot::Mutex::new(None),
        }))
    }

    pub async fn start(self: &Arc<Self>) -> shared_types::Result<()> {
        info!("starting WireSentinel orchestrator");
        // #region agent log
        shared_types::debug_log::emit_kv(
            "core-service/src/orchestrator.rs:start",
            "orchestrator start begin",
            &[("hypothesisId", "H_START".to_string())],
        );
        // #endregion

        crate::exit_failover::install_exit_failover(Arc::clone(&self.deps));

        // #region agent log
        shared_types::debug_log::emit_kv(
            "core-service/src/orchestrator.rs:start",
            "before wfp.init",
            &[("hypothesisId", "H_START".to_string())],
        );
        // #endregion
        self.deps.wfp.init().await?;
        let _ = auth::restrict_token_acl();

        let validation_report = self.deps.validation.run_all().await?;
        let validation_ok = validation_report.overall_status != ValidationStatus::Fail;
        if !validation_ok {
            warn!("validation failed — privacy score timer disabled until checks pass");
        }

        if let Err(e) = crate::wfp_lifecycle::WfpLifecycleManager::reconcile(
            Arc::clone(&self.deps.wfp),
            Arc::clone(&self.deps.storage),
            self.deps.events.clone(),
        )
        .await
        {
            warn!(error = %e, "WFP filter reconcile failed");
        }

        self.deps.start_domain_cache_purge(self.shutdown_rx.clone());
        log_retention::start_retention_task(Arc::clone(&self.deps.storage), self.shutdown_rx.clone());
        let storage = Arc::clone(&self.deps.storage);
        tokio::spawn(async move {
            let days = storage.settings.log_retention_days().await.unwrap_or(7);
            let _ = log_retention::purge_older_than(&storage, days).await;
        });
        self.deps.start_filter_scheduler().await;

        if let Err(e) = self.deps.plugins.discover().await {
            warn!(error = %e, "plugin discovery failed");
        }

        if validation_ok {
            let privacy = Arc::clone(&self.deps.privacy);
            let privacy_calc = Arc::clone(&privacy);
            PrivacyScoreService::start_periodic(privacy, self.shutdown_rx.clone());
            tokio::spawn(async move {
                let _ = privacy_calc.calculate().await;
            });

            let analytics = Arc::clone(&self.deps.privacy_analytics);
            let analytics_calc = Arc::clone(&analytics);
            PrivacyAnalyticsService::start_periodic(analytics, self.shutdown_rx.clone());
            tokio::spawn(async move {
                let _ = analytics_calc.calculate().await;
            });
        }

        if let Err(e) = self.deps.cover_traffic.start_if_enabled().await {
            warn!(error = %e, "cover traffic auto-start failed");
        }

        let dns = &self.deps.dns;
        if dns.settings().enabled {
            if let Err(e) = dns.start() {
                warn!(error = %e, "DNS layer failed to start");
            }
        }

        let handler: Arc<dyn ConnectionHandler> = Arc::new(OrchestratorHandler {
            deps: Arc::clone(&self.deps),
        });
        let backend_name = self
            .deps
            .storage
            .settings
            .traffic_monitor_backend()
            .await
            .unwrap_or_else(|_| "packet".to_string());
        let cleaner_interval_ms = self
            .deps
            .storage
            .settings
            .traffic_poll_interval_ms()
            .await
            .unwrap_or(5000);
        // #region agent log
        shared_types::debug_log::emit_kv(
            "core-service/src/orchestrator.rs:start",
            "creating traffic backend",
            &[
                ("hypothesisId", "H_START".to_string()),
                ("backend_name", backend_name.clone()),
                ("cleaner_interval_ms", cleaner_interval_ms.to_string()),
            ],
        );
        // #endregion
        let backend = traffic_monitor::create_connection_backend(&backend_name);
        *self.monitor_handle.lock() = Some(traffic_monitor::spawn_monitor(
            Arc::clone(&self.deps.traffic),
            handler,
            self.shutdown_rx.clone(),
            backend,
            cleaner_interval_ms,
        ));

        let mut rx = self.deps.events.subscribe();
        let split_tunnel = Arc::clone(&self.deps.split_tunnel);
        let tcp_term = Arc::clone(&self.deps.tcp_termination);
        *self.event_bridge.lock() = Some(tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                match event {
                    ServiceEvent::VpnConnected { profile_id, .. } => {
                        if let Err(e) = split_tunnel.on_vpn_connected(profile_id).await {
                            warn!(error = %e, %profile_id, "split tunnel vpn connect sync failed");
                        }
                        if let Err(e) = tcp_term.on_vpn_connect(profile_id).await {
                            warn!(error = %e, %profile_id, "tcp termination on vpn connect failed");
                        }
                    }
                    ServiceEvent::VpnDisconnected { profile_id, .. } => {
                        if let Err(e) = tcp_term.on_vpn_disconnect(profile_id).await {
                            warn!(error = %e, %profile_id, "tcp termination on vpn disconnect failed");
                        }
                    }
                    ServiceEvent::RouteChanged {
                        app_id,
                        old_route,
                        new_route,
                        ..
                    } => {
                        if let Err(e) = tcp_term.on_route_change(app_id, old_route, new_route).await
                        {
                            warn!(error = %e, %app_id, "tcp termination on route change failed");
                        }
                    }
                    _ => {}
                }
            }
        }));

        let port = self.deps.api_listen_port;
        let state = AppState::from_deps(Arc::clone(&self.deps));
        let handle = tokio::spawn(async move {
            if let Err(e) = crate::api::serve(state, port).await {
                tracing::error!(error = %e, "API server error");
            }
        });
        *self.api_handle.lock() = Some(handle);

        info!(port, "WireSentinel service started");
        // #region agent log
        shared_types::debug_log::emit_kv(
            "core-service/src/orchestrator.rs:start",
            "service started (api task spawned)",
            &[
                ("hypothesisId", "H_START".to_string()),
                ("port", port.to_string()),
            ],
        );
        // #endregion

        let recovery_enabled = self
            .deps
            .storage
            .settings
            .recovery_enabled()
            .await
            .unwrap_or(true);
        let _ = self
            .deps
            .recovery
            .recover_all(
                &self.deps.vpn,
                &self.deps.transport,
                &self.deps.tor,
                recovery_enabled,
            )
            .await;

        let metrics_interval = self
            .deps
            .storage
            .settings
            .metrics_interval_secs()
            .await
            .unwrap_or(30);
        PerformanceMonitor::start_periodic(
            Arc::clone(&self.deps.performance),
            metrics_interval,
            self.shutdown_rx.clone(),
        );

        let bench_interval = self
            .deps
            .storage
            .settings
            .benchmark_interval_secs()
            .await
            .unwrap_or(60);
        BenchmarkService::start_periodic(
            Arc::clone(&self.deps.benchmark),
            bench_interval,
            self.shutdown_rx.clone(),
        );

        if let Ok(agent_cfg) =
            crate::controller_agent::ControllerAgent::load_config(&self.deps.storage).await
        {
            if agent_cfg.enabled {
                crate::controller_agent::ControllerAgent::spawn(
                    Arc::clone(&self.deps.storage),
                    self.deps.events.clone(),
                    Arc::clone(&self.deps.enterprise),
                    Arc::clone(&self.deps.metrics),
                    Arc::clone(&self.deps.audit),
                    Some(Arc::clone(&self.deps.ztna)),
                    Some(Arc::clone(&self.deps.sse)),
                    agent_cfg,
                    self.shutdown_rx.clone(),
                );
            }
        }

        if let Ok(ztna_cfg) = crate::ztna_agent::ZtnaAgent::load_config(&self.deps.storage).await {
            if ztna_cfg.enabled {
                crate::ztna_agent::ZtnaAgent::spawn(
                    Arc::clone(&self.deps.storage),
                    self.deps.events.clone(),
                    Arc::clone(&self.deps.ztna),
                    ztna_cfg,
                    self.shutdown_rx.clone(),
                );
            }
        }

        if let Ok(sse_cfg) = crate::sse_agent::SseAgent::load_config(&self.deps.storage).await {
            if sse_cfg.enabled {
                crate::sse_agent::SseAgent::spawn(
                    Arc::clone(&self.deps.storage),
                    self.deps.events.clone(),
                    Arc::clone(&self.deps.sse),
                    sse_cfg,
                    self.shutdown_rx.clone(),
                );
            }
        }

        if let Ok(xdr_cfg) = crate::xdr_agent::XdrAgent::load_config(&self.deps.storage).await {
            if xdr_cfg.enabled {
                crate::xdr_agent::XdrAgent::spawn(
                    Arc::clone(&self.deps.storage),
                    self.deps.events.clone(),
                    Arc::clone(&self.deps.xdr),
                    xdr_cfg,
                    self.shutdown_rx.clone(),
                );
            }
        }

        if let Ok(sync_cfg) =
            crate::cloud_sync::CloudSyncAgent::load_config(&self.deps.storage).await
        {
            if sync_cfg.enabled {
                crate::cloud_sync::CloudSyncAgent::spawn(
                    Arc::clone(&self.deps.storage),
                    Arc::clone(&self.deps.backup),
                    self.deps.events.clone(),
                    sync_cfg,
                    self.shutdown_rx.clone(),
                );
            }
        }

        if let Ok(cfg) =
            crate::cloud_usage_reporter::CloudUsageReporter::load_config(&self.deps.storage).await
        {
            if cfg.enabled {
                crate::cloud_usage_reporter::CloudUsageReporter::spawn(
                    Arc::clone(&self.deps.storage),
                    Arc::clone(&self.deps.metrics),
                    self.deps.events.clone(),
                    cfg,
                    self.shutdown_rx.clone(),
                );
            }
        }

        if let Ok(cfg) =
            crate::cloud_telemetry_reporter::CloudTelemetryReporter::load_config(&self.deps.storage)
                .await
        {
            if cfg.enabled {
                crate::cloud_telemetry_reporter::CloudTelemetryReporter::spawn(
                    Arc::clone(&self.deps.storage),
                    Arc::clone(&self.deps.metrics),
                    self.deps.events.clone(),
                    cfg,
                    self.shutdown_rx.clone(),
                );
            }
        }

        if let Ok(cfg) =
            crate::cloud_backup_reporter::CloudBackupReporter::load_config(&self.deps.storage).await
        {
            if cfg.enabled {
                crate::cloud_backup_reporter::CloudBackupReporter::spawn(
                    Arc::clone(&self.deps.storage),
                    Arc::clone(&self.deps.backup),
                    self.deps.events.clone(),
                    cfg,
                    self.shutdown_rx.clone(),
                );
            }
        }

        Ok(())
    }

    pub fn shutdown_sender(&self) -> watch::Sender<bool> {
        self.shutdown_tx.clone()
    }

    pub async fn stop(&self) -> shared_types::Result<()> {
        info!("stopping WireSentinel orchestrator");
        let _ = self.shutdown_tx.send(true);

        if let Err(e) = self
            .deps
            .recovery
            .flush_before_stop(&self.deps.vpn, &self.deps.transport, &self.deps.tor)
            .await
        {
            warn!(error = %e, "recovery flush before stop failed");
        }

        let scheduler = self.deps.filter_scheduler.lock().take();
        if let Some(mut scheduler) = scheduler {
            let _ = scheduler.shutdown().await;
        }

        for profile in self.deps.vpn.profiles() {
            let _ = self.deps.vpn.disconnect(profile.id).await;
        }

        self.deps.dns.stop();
        self.deps.wfp.shutdown().await?;

        if let Some(h) = self.api_handle.lock().take() {
            h.abort();
        }
        if let Some(h) = self.monitor_handle.lock().take() {
            h.abort();
        }
        if let Some(h) = self.event_bridge.lock().take() {
            h.abort();
        }
        Ok(())
    }

    pub async fn wait_for_shutdown(&self) {
        let mut rx = self.shutdown_rx.clone();
        while rx.changed().await.is_ok() {
            if *rx.borrow() {
                break;
            }
        }
    }

    pub fn deps(&self) -> Arc<ServiceDeps> {
        Arc::clone(&self.deps)
    }
}
