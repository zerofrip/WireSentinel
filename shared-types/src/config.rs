use crate::{DnsSettings, PolicyMode, Rule, VPNProfile};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Top-level persisted configuration (legacy JSON shape for migration).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub version: u32,
    pub policy_mode: PolicyMode,
    pub rules: Vec<Rule>,
    pub vpn_profiles: Vec<VPNProfile>,
    pub dns: DnsSettings,
    pub api_port: u16,
    pub store_traffic_logs: bool,
    pub store_dns_logs: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            policy_mode: PolicyMode::Blacklist,
            rules: Vec::new(),
            vpn_profiles: Vec::new(),
            dns: DnsSettings::default(),
            api_port: 8170,
            store_traffic_logs: true,
            store_dns_logs: true,
        }
    }
}

/// Service status DTO for API.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ServiceStatus {
    pub running: bool,
    pub kill_switch_active: bool,
    pub policy_mode: PolicyMode,
    pub active_vpn_count: u32,
    pub monitored_app_count: u32,
    pub connection_count: u32,
    pub api_port: u16,
}
