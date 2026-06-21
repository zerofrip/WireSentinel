use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthStats {
    pub app_id: Uuid,
    pub interval_start: DateTime<Utc>,
    pub interval_end: DateTime<Utc>,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub peak_bps: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthSnapshot {
    pub app_id: Uuid,
    pub exe_name: String,
    pub bytes_in_per_sec: u64,
    pub bytes_out_per_sec: u64,
    pub total_bytes_in: u64,
    pub total_bytes_out: u64,
}
