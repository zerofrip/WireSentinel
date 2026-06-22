use crate::platform::{default_terminator, TcpSessionTerminator};
use chrono::Utc;
use event_bus::EventBus;
use parking_lot::RwLock;
use shared_types::{
    Result, ServiceEventInner, TcpConnectionSnapshot, TcpTerminationMode, TcpTerminationPolicy,
    TcpTerminationRule, TrafficRoute,
};
use tracing::{debug, warn};
use uuid::Uuid;

pub struct TcpTerminationEngine {
    terminator: Box<dyn TcpSessionTerminator>,
    policy: RwLock<TcpTerminationPolicy>,
    events: Option<EventBus>,
}

impl TcpTerminationEngine {
    pub fn new() -> Self {
        Self {
            terminator: default_terminator(),
            policy: RwLock::new(TcpTerminationPolicy::default()),
            events: None,
        }
    }

    pub fn with_terminator(mut self, terminator: Box<dyn TcpSessionTerminator>) -> Self {
        self.terminator = terminator;
        self
    }

    pub fn with_events(mut self, events: EventBus) -> Self {
        self.events = Some(events);
        self
    }

    pub fn set_policy(&self, policy: TcpTerminationPolicy) {
        *self.policy.write() = policy;
    }

    pub fn policy(&self) -> TcpTerminationPolicy {
        self.policy.read().clone()
    }

    pub fn enumerate(&self) -> Vec<TcpConnectionSnapshot> {
        self.terminator.enumerate()
    }

    fn should_run(&self, trigger: TcpTerminationMode) -> bool {
        let mode = self.policy.read().mode;
        match mode {
            TcpTerminationMode::Disabled => false,
            TcpTerminationMode::Always => true,
            _ => mode == trigger,
        }
    }

    pub async fn on_vpn_connect(&self, profile_id: Uuid) -> Result<u32> {
        self.run_termination(TcpTerminationMode::OnVpnConnect, Some(profile_id))
            .await
    }

    pub async fn on_vpn_disconnect(&self, profile_id: Uuid) -> Result<u32> {
        self.run_termination(TcpTerminationMode::OnVpnDisconnect, Some(profile_id))
            .await
    }

    pub async fn on_route_change(
        &self,
        _app_id: Option<Uuid>,
        _old_route: Option<TrafficRoute>,
        _new_route: Option<TrafficRoute>,
    ) -> Result<u32> {
        self.run_termination(TcpTerminationMode::OnRouteChange, None)
            .await
    }

    async fn run_termination(
        &self,
        trigger: TcpTerminationMode,
        profile_id: Option<Uuid>,
    ) -> Result<u32> {
        if !self.should_run(trigger) {
            return Ok(0);
        }

        let policy = self.policy.read().clone();
        let connections = self.terminator.enumerate();
        let mut terminated = 0u32;

        for conn in connections {
            if !Self::conn_matches_rules(&conn, &policy.rules, profile_id) {
                continue;
            }
            match self.terminator.terminate(&conn) {
                Ok(()) => {
                    terminated += 1;
                    debug!(
                        pid = conn.pid,
                        exe = %conn.exe_name,
                        "tcp session terminated"
                    );
                }
                Err(e) => {
                    warn!(error = %e, pid = conn.pid, "tcp termination failed");
                    if let Some(events) = &self.events {
                        events.publish(
                            ServiceEventInner::TcpTerminationFailed {
                                error: e.to_string(),
                                profile_id,
                            }
                            .with_timestamp(Utc::now()),
                        );
                    }
                }
            }
        }

        if terminated > 0 {
            if let Some(events) = &self.events {
                events.publish(
                    ServiceEventInner::TcpConnectionsTerminated {
                        count: terminated,
                        profile_id,
                        mode: policy.mode,
                    }
                    .with_timestamp(Utc::now()),
                );
            }
        }

        Ok(terminated)
    }

    fn conn_matches_rules(
        conn: &TcpConnectionSnapshot,
        rules: &[TcpTerminationRule],
        profile_id: Option<Uuid>,
    ) -> bool {
        if rules.is_empty() {
            return true;
        }

        rules.iter().any(|rule| {
            if !rule.enabled {
                return false;
            }
            if let Some(pid) = rule.profile_id {
                if profile_id != Some(pid) {
                    return false;
                }
            }
            if let Some(ref name) = rule.process_name {
                if !conn.exe_name.eq_ignore_ascii_case(name) {
                    return false;
                }
            }
            if let Some(ref path) = rule.process_path {
                if let Some(ref exe_path) = conn.exe_path {
                    if !exe_path.eq_ignore_ascii_case(path) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            true
        })
    }
}

impl Default for TcpTerminationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::TcpSessionTerminator;
    use shared_types::Protocol;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    struct MockTerminator {
        connections: Vec<TcpConnectionSnapshot>,
    }

    impl TcpSessionTerminator for MockTerminator {
        fn enumerate(&self) -> Vec<TcpConnectionSnapshot> {
            self.connections.clone()
        }

        fn terminate(&self, _conn: &TcpConnectionSnapshot) -> Result<()> {
            Ok(())
        }
    }

    fn sample_conn(name: &str) -> TcpConnectionSnapshot {
        TcpConnectionSnapshot {
            pid: 100,
            app_id: None,
            exe_name: name.into(),
            exe_path: None,
            protocol: Protocol::Tcp,
            local_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5000),
            remote_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 443),
            state: "established".into(),
            remote_domain: None,
        }
    }

    #[tokio::test]
    async fn terminates_matching_process_on_connect() {
        let engine = TcpTerminationEngine::new().with_terminator(Box::new(MockTerminator {
            connections: vec![sample_conn("chrome.exe"), sample_conn("outlook.exe")],
        }));

        let mut rule = TcpTerminationRule::new();
        rule.process_name = Some("chrome.exe".into());
        engine.set_policy(TcpTerminationPolicy {
            mode: TcpTerminationMode::OnVpnConnect,
            rules: vec![rule],
        });

        let count = engine.on_vpn_connect(Uuid::new_v4()).await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn disabled_mode_skips() {
        let engine = TcpTerminationEngine::new().with_terminator(Box::new(MockTerminator {
            connections: vec![sample_conn("chrome.exe")],
        }));
        engine.set_policy(TcpTerminationPolicy::default());
        let count = engine.on_vpn_connect(Uuid::new_v4()).await.unwrap();
        assert_eq!(count, 0);
    }
}
