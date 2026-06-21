use crate::{AppIdentity, TrafficRoute, Verdict};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Inbound,
    Outbound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Tcp,
    Udp,
    Icmp,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub app: AppIdentity,
    pub direction: Direction,
    pub protocol: Protocol,
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
    pub remote_domain: Option<String>,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub route: TrafficRoute,
    pub verdict: Verdict,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ip: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination_ip: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination_port: Option<u16>,
}

impl TrafficEvent {
    pub fn new(
        app: AppIdentity,
        direction: Direction,
        protocol: Protocol,
        local_addr: SocketAddr,
        remote_addr: SocketAddr,
        route: TrafficRoute,
        verdict: Verdict,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            app,
            direction,
            protocol,
            local_addr,
            remote_addr,
            remote_domain: None,
            bytes_in: 0,
            bytes_out: 0,
            route,
            verdict,
            process_id: None,
            source_ip: None,
            destination_ip: None,
            source_port: None,
            destination_port: None,
        }
    }
}

/// Lightweight connection snapshot for live monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSnapshot {
    pub pid: u32,
    pub app_id: Option<Uuid>,
    pub exe_name: String,
    pub protocol: Protocol,
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
    pub state: String,
    pub remote_domain: Option<String>,
    #[serde(default)]
    pub bytes_sent: u64,
    #[serde(default)]
    pub bytes_received: u64,
}
