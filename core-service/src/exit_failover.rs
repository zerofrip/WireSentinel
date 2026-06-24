//! Per-app ordered exit route failover state and handling.

use crate::deps::ServiceDeps;
use parking_lot::RwLock;
use policy_engine::Decision;
use shared_types::{
    AppExitConfig, AppIdentity, AppRecord, ExitOnExhaustion, ServiceEvent, ServiceEventInner,
    TrafficRoute, Verdict,
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

pub struct ExitFailoverService {
    active_index: RwLock<HashMap<Uuid, usize>>,
}

impl Default for ExitFailoverService {
    fn default() -> Self {
        Self::new()
    }
}

impl ExitFailoverService {
    pub fn new() -> Self {
        Self {
            active_index: RwLock::new(HashMap::new()),
        }
    }

    pub fn active_index(&self, app_id: Uuid) -> usize {
        *self.active_index.read().get(&app_id).unwrap_or(&0)
    }

    pub fn reset_index(&self, app_id: Uuid) {
        self.active_index.write().insert(app_id, 0);
    }

    pub fn resolve_active_route(
        &self,
        record: &AppRecord,
    ) -> (
        Vec<TrafficRoute>,
        usize,
        Option<TrafficRoute>,
        ExitOnExhaustion,
    ) {
        let config = record.effective_exit_config();
        let routes = config
            .as_ref()
            .map(|c| c.routes.clone())
            .unwrap_or_default();
        let on_exhaustion = config
            .as_ref()
            .map(|c| c.on_exhaustion)
            .unwrap_or(ExitOnExhaustion::Blocked);
        let index = self
            .active_index(record.app_id)
            .min(routes.len().saturating_sub(1));
        let route = routes.get(index).cloned();
        (routes, index, route, on_exhaustion)
    }

    fn route_matches_profile(route: &TrafficRoute, profile_id: Uuid) -> bool {
        route.profile_id() == Some(profile_id)
    }

    pub async fn on_vpn_disconnected(
        &self,
        deps: &ServiceDeps,
        profile_id: Uuid,
    ) -> shared_types::Result<()> {
        let apps = deps.app_registry.list(None, Some(5000)).await?;
        for record in apps {
            let Some(config) = record.effective_exit_config() else {
                continue;
            };
            if config.routes.is_empty() {
                continue;
            }
            let current_index = self.active_index(record.app_id);
            let current_route = match config.routes.get(current_index) {
                Some(r) => r.clone(),
                None => continue,
            };
            if !Self::route_matches_profile(&current_route, profile_id) {
                continue;
            }
            self.advance_or_exhaust(deps, &record, config, current_index, current_route)
                .await?;
        }
        Ok(())
    }

    async fn advance_or_exhaust(
        &self,
        deps: &ServiceDeps,
        record: &AppRecord,
        config: AppExitConfig,
        from_index: usize,
        failed_route: TrafficRoute,
    ) -> shared_types::Result<()> {
        let app_id = record.app_id;
        let next_index = from_index + 1;
        if let Some(next_route) = config.routes.get(next_index).cloned() {
            self.active_index.write().insert(app_id, next_index);
            info!(%app_id, from_index, next_index, "exit route failover");
            deps.events
                .publish(ServiceEvent::now(ServiceEventInner::ExitFailover {
                    app_id,
                    from_index,
                    to_index: next_index,
                    route: next_route.clone(),
                }));
            let app = AppIdentity::new(0, record.clone());
            let decision = Decision {
                route: next_route.clone(),
                verdict: Verdict::from_route(&next_route, None, "exit failover"),
                matched_rule_id: None,
            };
            if let Err(e) = deps.split_tunnel.enforce(&decision, &app).await {
                warn!(error = %e, %app_id, "failover enforce failed");
            }
            let _ = deps
                .audit
                .record_route_changed(app_id, Some(failed_route), Some(next_route), None)
                .await;
            return Ok(());
        }

        self.handle_exhaustion(deps, record, config.on_exhaustion, app_id)
            .await
    }

    async fn handle_exhaustion(
        &self,
        deps: &ServiceDeps,
        record: &AppRecord,
        on_exhaustion: ExitOnExhaustion,
        app_id: Uuid,
    ) -> shared_types::Result<()> {
        info!(%app_id, ?on_exhaustion, "exit routes exhausted");
        deps.events
            .publish(ServiceEvent::now(ServiceEventInner::ExitExhausted {
                app_id,
                action: on_exhaustion,
            }));

        let route = match on_exhaustion {
            ExitOnExhaustion::KillSwitch => {
                deps.policy.write().set_kill_switch(true);
                let _ = deps.wfp.apply_kill_switch(true).await;
                TrafficRoute::Blocked
            }
            ExitOnExhaustion::Blocked => TrafficRoute::Blocked,
            ExitOnExhaustion::Direct => TrafficRoute::Direct,
        };

        let app = AppIdentity::new(0, record.clone());
        let decision = Decision {
            route: route.clone(),
            verdict: Verdict::from_route(&route, None, "exit routes exhausted"),
            matched_rule_id: None,
        };
        if let Err(e) = deps.split_tunnel.enforce(&decision, &app).await {
            warn!(error = %e, %app_id, "exhaustion enforce failed");
        }
        Ok(())
    }
}

pub fn install_exit_failover(deps: Arc<ServiceDeps>) {
    let failover = Arc::clone(&deps.exit_failover);
    let deps_for_task = deps;
    tokio::spawn(async move {
        let mut rx = deps_for_task.events.subscribe();
        while let Ok(event) = rx.recv().await {
            if let ServiceEvent::VpnDisconnected { profile_id, .. } = event {
                if let Err(e) = failover
                    .on_vpn_disconnected(deps_for_task.as_ref(), profile_id)
                    .await
                {
                    warn!(error = %e, %profile_id, "exit failover on vpn disconnect failed");
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn resolve_active_route_uses_index_and_exhaustion_default() {
        let svc = ExitFailoverService::new();
        let profile_a = Uuid::new_v4();
        let profile_b = Uuid::new_v4();
        let mut record = AppRecord::new(PathBuf::from("app.exe"));
        record.exit_config = Some(AppExitConfig {
            routes: vec![
                TrafficRoute::WireGuard(profile_a),
                TrafficRoute::WireGuard(profile_b),
            ],
            on_exhaustion: ExitOnExhaustion::KillSwitch,
        });
        svc.active_index.write().insert(record.app_id, 1);
        let (routes, index, route, on_exhaustion) = svc.resolve_active_route(&record);
        assert_eq!(routes.len(), 2);
        assert_eq!(index, 1);
        assert_eq!(route, Some(TrafficRoute::WireGuard(profile_b)));
        assert_eq!(on_exhaustion, ExitOnExhaustion::KillSwitch);
    }

    #[test]
    fn reset_index_returns_to_primary_route() {
        let svc = ExitFailoverService::new();
        let app_id = Uuid::new_v4();
        svc.active_index.write().insert(app_id, 2);
        svc.reset_index(app_id);
        assert_eq!(svc.active_index(app_id), 0);
    }
}
