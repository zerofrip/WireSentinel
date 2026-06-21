//! Phase 18.5 WireSock compatibility shared DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{ConnectionSnapshot, Protocol, TrafficRoute};

/// Handshake proxy protocol (WireGuard / AmneziaWG obfuscation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProxyType {
    #[default]
    Socks5,
    Http,
    Https,
}

/// SOCKS5 / HTTP(S) proxy settings for VPN handshake obfuscation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct HandshakeProxySettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub proxy_type: ProxyType,
    pub host: String,
    pub port: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

impl Default for HandshakeProxySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            proxy_type: ProxyType::Socks5,
            host: String::new(),
            port: 1080,
            username: None,
            password: None,
        }
    }
}

/// When to terminate existing TCP sessions for route reconnect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum TcpTerminationMode {
    #[default]
    Disabled,
    OnVpnConnect,
    OnVpnDisconnect,
    OnRouteChange,
    Always,
}

/// Process-aware TCP reconnect rule.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TcpTerminationRule {
    pub id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<TrafficRoute>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_true() -> bool {
    true
}

impl TcpTerminationRule {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            process_path: None,
            process_name: None,
            profile_id: None,
            route: None,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }
}

impl Default for TcpTerminationRule {
    fn default() -> Self {
        Self::new()
    }
}

/// Global TCP termination policy.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TcpTerminationPolicy {
    #[serde(default)]
    pub mode: TcpTerminationMode,
    #[serde(default)]
    pub rules: Vec<TcpTerminationRule>,
}

impl Default for TcpTerminationPolicy {
    fn default() -> Self {
        Self {
            mode: TcpTerminationMode::Disabled,
            rules: Vec::new(),
        }
    }
}

/// Persisted TCP termination settings.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TcpTerminationSettings {
    #[serde(default)]
    pub mode: TcpTerminationMode,
    pub updated_at: DateTime<Utc>,
}

impl Default for TcpTerminationSettings {
    fn default() -> Self {
        Self {
            mode: TcpTerminationMode::Disabled,
            updated_at: Utc::now(),
        }
    }
}

/// TCP session snapshot for termination engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpConnectionSnapshot {
    pub pid: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_id: Option<Uuid>,
    pub exe_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exe_path: Option<String>,
    pub protocol: Protocol,
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_domain: Option<String>,
}

impl From<ConnectionSnapshot> for TcpConnectionSnapshot {
    fn from(conn: ConnectionSnapshot) -> Self {
        Self {
            pid: conn.pid,
            app_id: conn.app_id,
            exe_name: conn.exe_name,
            exe_path: None,
            protocol: conn.protocol,
            local_addr: conn.local_addr,
            remote_addr: conn.remote_addr,
            state: conn.state,
            remote_domain: conn.remote_domain,
        }
    }
}

/// Global split-tunnel template application mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum TemplateMode {
    #[default]
    Disabled,
    Merge,
    Override,
}

/// Per-application split-tunnel rule within a template.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AppRule {
    pub id: Uuid,
    pub app_id: Uuid,
    pub route: TrafficRoute,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Per-domain split-tunnel rule within a template.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DomainRule {
    pub id: Uuid,
    pub pattern: String,
    pub route: TrafficRoute,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Reusable global split-tunnel policy template.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SplitTunnelTemplate {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub default_route: TrafficRoute,
    #[serde(default)]
    pub app_rules: Vec<AppRule>,
    #[serde(default)]
    pub domain_rules: Vec<DomainRule>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SplitTunnelTemplate {
    pub fn new(name: String, default_route: TrafficRoute) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description: String::new(),
            default_route,
            app_rules: Vec::new(),
            domain_rules: Vec::new(),
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }
}

/// One step in template resolution diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TemplateTraceStep {
    pub stage: String,
    pub detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<TrafficRoute>,
}

/// Diagnostics trace for template + policy resolution.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct TemplateResolutionTrace {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_id: Option<Uuid>,
    #[serde(default)]
    pub mode: TemplateMode,
    #[serde(default)]
    pub steps: Vec<TemplateTraceStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_route: Option<TrafficRoute>,
}

/// Active template mode configuration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SplitTemplateModeSettings {
    #[serde(default)]
    pub mode: TemplateMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_template_id: Option<Uuid>,
    pub updated_at: DateTime<Utc>,
}

impl Default for SplitTemplateModeSettings {
    fn default() -> Self {
        Self {
            mode: TemplateMode::Disabled,
            active_template_id: None,
            updated_at: Utc::now(),
        }
    }
}

/// Active resolved template for policy evaluation.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ResolvedTemplate {
    pub mode: TemplateMode,
    pub default_route: TrafficRoute,
    pub app_rules: Vec<AppRule>,
    pub domain_rules: Vec<DomainRule>,
    pub template_id: Uuid,
}
