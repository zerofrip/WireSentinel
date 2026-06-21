use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterListType {
    Hosts,
    Easylist,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DnsFilterMode {
    #[default]
    Blacklist,
    Whitelist,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterListRecord {
    pub id: Uuid,
    pub name: String,
    pub url: Option<String>,
    pub list_type: FilterListType,
    pub enabled: bool,
    pub update_interval_secs: Option<u32>,
    pub last_updated: Option<DateTime<Utc>>,
    pub cache_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainCorrelation {
    pub id: Uuid,
    pub app_id: Option<Uuid>,
    pub domain: String,
    pub ip_address: Option<String>,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub query_count: u32,
    pub traffic_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopDomainEntry {
    pub domain: String,
    pub query_count: u64,
    pub blocked_count: u64,
}
