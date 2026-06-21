//! Split-tunnel enforcement via WFP and VPN tunnel awareness.

use chrono::{Timelike, Utc};
use event_bus::EventBus;
use parking_lot::RwLock;
use policy_engine::Decision;
use shared_types::{
    AppIdentity, FirewallDecisionRecord, Result, RouteStatisticsRecord, ServiceEventInner,
    TrafficRoute, TunnelIface, WfpFilterStateRecord,
};
use std::collections::HashMap;
use std::sync::Arc;
use storage::{RouteStatisticsRepository, WfpFilterStateRepository};
use tracing::{debug, info, warn};
use uuid::Uuid;
use vpn_engine::VpnManager;
use wfp::WfpEngine;

use crate::anonymity::AnonymityService;
use crate::anonymous_routing::AnonymousRoutingService;
use crate::domain_cache::DomainResolverCache;
use crate::kernel_route_bridge::KernelRouteBridge;
use crate::mixnet_redirect::MixnetRedirectEngine;
use crate::proxy::ProxyService;
use crate::proxy_redirect::ProxyRedirectEngine;

fn route_type_str(route: &TrafficRoute) -> &'static str {
    match route {
        TrafficRoute::Direct => "direct",
        TrafficRoute::WireGuard(_) => "wireguard",
        TrafficRoute::AmneziaWG(_) => "amneziawg",
        TrafficRoute::Tailnet(_) => "tailnet",
        TrafficRoute::Tor(_) => "tor",
        TrafficRoute::Anonymous(_) => "anonymous",
        TrafficRoute::Proxy(_) => "proxy",
        TrafficRoute::ProxyChain(_) => "proxy_chain",
        TrafficRoute::Chain(_) => "chain",
        TrafficRoute::Katzenpost(_) => "katzenpost",
        TrafficRoute::Loopix(_) => "loopix",
        TrafficRoute::FederatedMixnet(_) => "federated_mixnet",
        TrafficRoute::Blocked => "blocked",
    }
}

pub struct SplitTunnelEngine {
    wfp: Arc<dyn WfpEngine>,
    vpn: Arc<VpnManager>,
    domain_cache: Arc<DomainResolverCache>,
    route_stats: Arc<dyn RouteStatisticsRepository>,
    wfp_filter_state: Arc<dyn WfpFilterStateRepository>,
    proxy: Option<Arc<ProxyService>>,
    anonymous_routing: Option<Arc<AnonymousRoutingService>>,
    anonymity: Option<Arc<AnonymityService>>,
    kernel_route_bridge: Option<Arc<KernelRouteBridge>>,
    proxy_redirect: Option<Arc<ProxyRedirectEngine>>,
    mixnet_redirect: Option<Arc<MixnetRedirectEngine>>,
    events: Option<EventBus>,
    app_routes: RwLock<HashMap<Uuid, TrafficRoute>>,
}

impl SplitTunnelEngine {
    pub fn new(
        wfp: Arc<dyn WfpEngine>,
        vpn: Arc<VpnManager>,
        domain_cache: Arc<DomainResolverCache>,
        route_stats: Arc<dyn RouteStatisticsRepository>,
        wfp_filter_state: Arc<dyn WfpFilterStateRepository>,
    ) -> Self {
        Self {
            wfp,
            vpn,
            domain_cache,
            route_stats,
            wfp_filter_state,
            proxy: None,
            anonymous_routing: None,
            anonymity: None,
            kernel_route_bridge: None,
            proxy_redirect: None,
            mixnet_redirect: None,
            events: None,
            app_routes: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_events(mut self, events: EventBus) -> Self {
        self.events = Some(events);
        self
    }

    pub fn with_proxy(mut self, proxy: Arc<ProxyService>) -> Self {
        self.proxy = Some(proxy);
        self
    }

    pub fn with_anonymous_routing(mut self, routing: Arc<AnonymousRoutingService>) -> Self {
        self.anonymous_routing = Some(routing);
        self
    }

    pub fn with_anonymity(mut self, anonymity: Arc<AnonymityService>) -> Self {
        self.anonymity = Some(anonymity);
        self
    }

    pub fn with_kernel_route_bridge(mut self, bridge: Arc<KernelRouteBridge>) -> Self {
        self.kernel_route_bridge = Some(bridge);
        self
    }

    pub fn with_proxy_redirect(mut self, redirect: Arc<ProxyRedirectEngine>) -> Self {
        self.proxy_redirect = Some(redirect);
        self
    }

    pub fn with_mixnet_redirect(mut self, redirect: Arc<MixnetRedirectEngine>) -> Self {
        self.mixnet_redirect = Some(redirect);
        self
    }

    fn tunnel_for_route(&self, route: &TrafficRoute) -> Option<TunnelIface> {
        let profile_id = route.profile_id()?;
        let profile = self.vpn.get_profile(profile_id)?;
        Some(TunnelIface {
            profile_id,
            name: profile.name.clone(),
            luid: 0,
            socks_port: None,
        })
    }

    async fn tunnel_for_route_async(&self, route: &TrafficRoute) -> Option<TunnelIface> {
        let profile_id = route.profile_id()?;
        if let Some(iface) = self.vpn.tunnel_iface(profile_id).await {
            return Some(iface);
        }
        self.tunnel_for_route(route)
    }

    async fn persist_filter_state(
        &self,
        app: &AppIdentity,
        route: &TrafficRoute,
        rule_id: Option<Uuid>,
        filter_id: u64,
    ) -> Result<()> {
        let now = Utc::now();
        let record = WfpFilterStateRecord {
            id: Uuid::new_v4(),
            scope_type: "app".into(),
            scope_value: Some(app.id().to_string()),
            filter_id,
            route: route.clone(),
            rule_id,
            created_at: now,
            updated_at: now,
        };
        self.wfp_filter_state.upsert(&record).await
    }

    pub async fn enforce(&self, decision: &Decision, app: &AppIdentity) -> Result<TrafficRoute> {
        let route = decision.route.clone();
        let old_route = self.app_routes.read().get(&app.id()).cloned();
        let tunnel = self.tunnel_for_route_async(&route).await;

        match &route {
            TrafficRoute::Blocked => {
                self.wfp.block_connection(app).await?;
            }
            TrafficRoute::Direct => {
                self.wfp.allow_connection(app, &route).await?;
            }
            TrafficRoute::WireGuard(_) | TrafficRoute::AmneziaWG(_) | TrafficRoute::Tailnet(_) => {
                self.wfp
                    .route_connection(app, &route, tunnel.clone())
                    .await?;
            }
            TrafficRoute::Tor(_)
            | TrafficRoute::Chain(_)
            | TrafficRoute::Anonymous(_)
            | TrafficRoute::Katzenpost(_)
            | TrafficRoute::Loopix(_)
            | TrafficRoute::FederatedMixnet(_) => {
                let socks_port = if let Some(anonymity) = &self.anonymity {
                    anonymity.ensure_route_ready(&route).await.ok()
                } else if let Some(routing) = &self.anonymous_routing {
                    routing.ensure_ready(&route).await.ok()
                } else {
                    None
                };
                let anon_tunnel = route.profile_id().map(|profile_id| TunnelIface {
                    profile_id,
                    name: "anonymous".into(),
                    luid: 0,
                    socks_port,
                });
                self.wfp.route_connection(app, &route, anon_tunnel).await?;
            }
            TrafficRoute::Proxy(_) | TrafficRoute::ProxyChain(_) => {
                let socks_port = if let Some(proxy) = &self.proxy {
                    proxy.ensure_route_ready(&route).await.ok()
                } else {
                    None
                };
                let proxy_tunnel = route.profile_id().map(|profile_id| TunnelIface {
                    profile_id,
                    name: "proxy".into(),
                    luid: 0,
                    socks_port,
                });
                self.wfp.route_connection(app, &route, proxy_tunnel).await?;
            }
        }

        if let Some(bridge) = &self.kernel_route_bridge {
            if bridge.is_active() {
                bridge.sync_enforce(app, &route, tunnel.clone()).await?;
            }
        }
        if let Some(redirect) = &self.proxy_redirect {
            redirect.apply(app, &route, tunnel.clone()).await?;
        }
        if let Some(redirect) = &self.mixnet_redirect {
            redirect.apply(app, &route, tunnel.clone()).await?;
        }

        let filter_ids = self.wfp.filter_ids_for_app(app.id());
        let primary_filter_id = filter_ids.first().copied().unwrap_or(0);

        if let Err(e) = self
            .persist_filter_state(app, &route, decision.matched_rule_id, primary_filter_id)
            .await
        {
            warn!(error = %e, app = %app.display_name(), "wfp filter state persist failed");
        }

        self.app_routes.write().insert(app.id(), route.clone());

        if old_route.as_ref() != Some(&route) {
            if let Some(events) = &self.events {
                events.publish(
                    ServiceEventInner::RouteChanged {
                        app_id: app.id(),
                        old_route,
                        new_route: Some(route.clone()),
                    }
                    .with_timestamp(Utc::now()),
                );
            }
        }

        Ok(route)
    }

    pub async fn sync_app(&self, app: &AppIdentity, decision: &Decision) -> Result<()> {
        self.enforce(decision, app).await?;
        Ok(())
    }

    pub async fn sync_all(&self, apps: &[(AppIdentity, Decision)]) -> Result<()> {
        for (app, decision) in apps {
            if let Err(e) = self.enforce(decision, app).await {
                warn!(
                    error = %e,
                    app = %app.display_name(),
                    "split tunnel sync failed for app"
                );
            }
        }
        info!(count = apps.len(), "split tunnel sync_all completed");
        Ok(())
    }

    pub async fn on_vpn_connected(&self, profile_id: Uuid) -> Result<()> {
        let apps: Vec<(AppIdentity, Decision)> = self
            .app_routes
            .read()
            .iter()
            .filter_map(|(app_id, route)| {
                if route.profile_id() == Some(profile_id) {
                    let mut record =
                        shared_types::AppRecord::new(std::path::PathBuf::from("unknown.exe"));
                    record.app_id = *app_id;
                    let app = AppIdentity::new(0, record);
                    let decision = Decision {
                        route: route.clone(),
                        verdict: shared_types::Verdict::from_route(route, None, "vpn reconnected"),
                        matched_rule_id: None,
                    };
                    Some((app, decision))
                } else {
                    None
                }
            })
            .collect();

        self.sync_all(&apps).await?;
        let _ = profile_id;
        Ok(())
    }

    pub async fn on_vpn_disconnected(&self, profile_id: Uuid) -> Result<()> {
        let affected: Vec<Uuid> = self
            .app_routes
            .read()
            .iter()
            .filter_map(|(app_id, route)| {
                if route.profile_id() == Some(profile_id) {
                    Some(*app_id)
                } else {
                    None
                }
            })
            .collect();

        let count = affected.len();
        for app_id in affected {
            let mut record = shared_types::AppRecord::new(std::path::PathBuf::from("unknown.exe"));
            record.app_id = app_id;
            let app = AppIdentity::new(0, record);
            let blocked = Decision {
                route: TrafficRoute::Blocked,
                verdict: shared_types::Verdict::block("vpn disconnected"),
                matched_rule_id: None,
            };
            if let Err(e) = self.enforce(&blocked, &app).await {
                warn!(error = %e, %app_id, "failed to block app after vpn disconnect");
            }
        }
        debug!(%profile_id, count, "vpn disconnect handled");
        Ok(())
    }

    pub async fn record_usage(
        &self,
        app_id: Option<Uuid>,
        profile_id: Option<Uuid>,
        domain: Option<String>,
        route: &TrafficRoute,
        bytes_in: u64,
        bytes_out: u64,
    ) -> Result<RouteStatisticsRecord> {
        let now = Utc::now();
        let window_start = now
            .date_naive()
            .and_hms_opt(now.hour(), 0, 0)
            .map(|ndt| ndt.and_utc())
            .unwrap_or(now);
        let window_end = window_start + chrono::Duration::hours(1);

        let record = RouteStatisticsRecord {
            id: Uuid::new_v4(),
            app_id,
            profile_id,
            domain,
            route_type: route_type_str(route).to_string(),
            bytes_in,
            bytes_out,
            connection_count: 1,
            window_start,
            window_end,
            updated_at: now,
        };

        self.route_stats.upsert(&record).await?;
        Ok(record)
    }

    pub async fn record_firewall_decision(
        &self,
        app_id: Option<Uuid>,
        domain: Option<String>,
        dest_ip: Option<String>,
        route: &TrafficRoute,
        verdict: &shared_types::Verdict,
    ) -> Result<FirewallDecisionRecord> {
        let record = FirewallDecisionRecord {
            id: Uuid::new_v4(),
            app_id,
            domain,
            dest_ip,
            route: route.clone(),
            verdict: verdict.clone(),
            timestamp: Utc::now(),
        };
        Ok(record)
    }

    pub fn domain_cache(&self) -> Arc<DomainResolverCache> {
        Arc::clone(&self.domain_cache)
    }
}
