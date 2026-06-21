use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{DnsBlockMode, DnsFilterMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DnsTransport {
    Plain,
    Doh,
    Dot,
    Doq,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsSettings {
    pub enabled: bool,
    pub transport: DnsTransport,
    pub provider: String,
    pub upstream_url: String,
    pub listen_addr: String,
    #[serde(default)]
    pub dot_enabled: bool,
    #[serde(default)]
    pub filter_mode: DnsFilterMode,
    #[serde(default)]
    pub dns_block_mode: DnsBlockMode,
}

impl Default for DnsSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            transport: DnsTransport::Doh,
            provider: "cloudflare".into(),
            upstream_url: "https://cloudflare-dns.com/dns-query".into(),
            listen_addr: "127.0.0.1:5353".into(),
            dot_enabled: false,
            filter_mode: DnsFilterMode::default(),
            dns_block_mode: DnsBlockMode::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DNSQueryLog {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub app_id: Option<Uuid>,
    pub pid: Option<u32>,
    pub qname: String,
    pub qtype: String,
    pub upstream: String,
    pub blocked: bool,
    pub latency_ms: u64,
    pub answers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<Uuid>,
}
