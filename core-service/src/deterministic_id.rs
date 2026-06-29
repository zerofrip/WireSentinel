//! Deterministic UUID v5 keys for SQLite upsert buckets.

use chrono::{DateTime, Utc};
use uuid::{uuid, Uuid};

const ROUTE_STATS_NS: Uuid = uuid!("a1b2c3d4-e5f6-7890-abcd-ef1234567001");
const WFP_STATE_NS: Uuid = uuid!("a1b2c3d4-e5f6-7890-abcd-ef1234567002");
const DOMAIN_CACHE_NS: Uuid = uuid!("a1b2c3d4-e5f6-7890-abcd-ef1234567003");

pub fn route_statistics_id(
    app_id: Option<Uuid>,
    profile_id: Option<Uuid>,
    domain: Option<&str>,
    route_type: &str,
    window_start: DateTime<Utc>,
) -> Uuid {
    let key = format!(
        "{}|{}|{}|{}|{}",
        app_id.map(|u| u.to_string()).unwrap_or_default(),
        profile_id.map(|u| u.to_string()).unwrap_or_default(),
        domain.unwrap_or(""),
        route_type,
        window_start.to_rfc3339(),
    );
    Uuid::new_v5(&ROUTE_STATS_NS, key.as_bytes())
}

pub fn wfp_filter_state_id(scope_type: &str, scope_value: Option<&str>) -> Uuid {
    let key = format!("{}|{}", scope_type, scope_value.unwrap_or(""));
    Uuid::new_v5(&WFP_STATE_NS, key.as_bytes())
}

pub fn domain_cache_id(app_id: Option<Uuid>, ip_address: &str) -> Uuid {
    let key = format!(
        "{}|{}",
        app_id.map(|u| u.to_string()).unwrap_or_default(),
        ip_address,
    );
    Uuid::new_v5(&DOMAIN_CACHE_NS, key.as_bytes())
}
