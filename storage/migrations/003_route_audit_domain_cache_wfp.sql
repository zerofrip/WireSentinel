-- Route statistics, audit, domain cache, WFP state, firewall decisions

CREATE TABLE IF NOT EXISTS route_statistics (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT,
    profile_id TEXT,
    domain TEXT,
    route_type TEXT NOT NULL,
    bytes_in INTEGER NOT NULL DEFAULT 0,
    bytes_out INTEGER NOT NULL DEFAULT 0,
    connection_count INTEGER NOT NULL DEFAULT 0,
    window_start TEXT NOT NULL,
    window_end TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_route_stats_app_route ON route_statistics(app_id, route_type);
CREATE INDEX IF NOT EXISTS idx_route_stats_domain ON route_statistics(domain);
CREATE INDEX IF NOT EXISTS idx_route_stats_window ON route_statistics(window_end DESC);

CREATE TABLE IF NOT EXISTS audit_log (
    id TEXT PRIMARY KEY NOT NULL,
    event_type TEXT NOT NULL,
    actor TEXT,
    target_type TEXT,
    target_id TEXT,
    detail_json TEXT,
    timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_log_type ON audit_log(event_type);
CREATE INDEX IF NOT EXISTS idx_audit_log_timestamp ON audit_log(timestamp DESC);

CREATE TABLE IF NOT EXISTS domain_resolver_cache (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT,
    domain TEXT NOT NULL,
    ip_address TEXT NOT NULL,
    wildcard INTEGER NOT NULL DEFAULT 0,
    expires_at TEXT NOT NULL,
    first_seen TEXT NOT NULL,
    last_seen TEXT NOT NULL,
    hit_count INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_domain_cache_ip ON domain_resolver_cache(ip_address);
CREATE INDEX IF NOT EXISTS idx_domain_cache_domain ON domain_resolver_cache(domain);
CREATE INDEX IF NOT EXISTS idx_domain_cache_expires ON domain_resolver_cache(expires_at);

CREATE TABLE IF NOT EXISTS wfp_filter_state (
    id TEXT PRIMARY KEY NOT NULL,
    scope_type TEXT NOT NULL,
    scope_value TEXT,
    filter_id INTEGER NOT NULL,
    route_json TEXT NOT NULL,
    rule_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_wfp_filter_scope ON wfp_filter_state(scope_type, scope_value);

CREATE TABLE IF NOT EXISTS firewall_decisions (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT,
    domain TEXT,
    dest_ip TEXT,
    route_json TEXT NOT NULL,
    verdict_json TEXT,
    timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_firewall_decisions_app ON firewall_decisions(app_id);
CREATE INDEX IF NOT EXISTS idx_firewall_decisions_timestamp ON firewall_decisions(timestamp DESC);

CREATE TABLE IF NOT EXISTS vpn_config_files (
    profile_id TEXT PRIMARY KEY NOT NULL,
    disk_path TEXT NOT NULL,
    materialized_at TEXT NOT NULL,
    FOREIGN KEY (profile_id) REFERENCES vpn_profiles(id) ON DELETE CASCADE
);
