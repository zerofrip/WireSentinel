-- WireSentinel initial schema

CREATE TABLE IF NOT EXISTS apps (
    app_id TEXT PRIMARY KEY NOT NULL,
    display_name TEXT NOT NULL,
    exe_path TEXT NOT NULL UNIQUE,
    publisher TEXT,
    sha256 TEXT,
    icon_path TEXT,
    first_seen TEXT NOT NULL,
    last_seen TEXT NOT NULL,
    default_route_json TEXT
);

CREATE INDEX IF NOT EXISTS idx_apps_exe_path ON apps(exe_path);
CREATE INDEX IF NOT EXISTS idx_apps_sha256 ON apps(sha256);

CREATE TABLE IF NOT EXISTS rules (
    id TEXT PRIMARY KEY NOT NULL,
    priority INTEGER NOT NULL,
    scope_type TEXT NOT NULL,
    scope_value TEXT,
    action_json TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    description TEXT
);

CREATE INDEX IF NOT EXISTS idx_rules_priority ON rules(priority DESC);

CREATE TABLE IF NOT EXISTS vpn_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    backend TEXT NOT NULL,
    config_blob BLOB NOT NULL,
    auto_connect INTEGER NOT NULL DEFAULT 0,
    group_name TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS traffic_logs (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT,
    timestamp TEXT NOT NULL,
    protocol TEXT NOT NULL,
    local_addr TEXT NOT NULL,
    remote_addr TEXT NOT NULL,
    domain TEXT,
    route_json TEXT NOT NULL,
    bytes_in INTEGER NOT NULL DEFAULT 0,
    bytes_out INTEGER NOT NULL DEFAULT 0,
    verdict_json TEXT
);

CREATE INDEX IF NOT EXISTS idx_traffic_logs_timestamp ON traffic_logs(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_traffic_logs_app_id ON traffic_logs(app_id);

CREATE TABLE IF NOT EXISTS dns_logs (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT,
    pid INTEGER,
    timestamp TEXT NOT NULL,
    qname TEXT NOT NULL,
    qtype TEXT NOT NULL,
    upstream TEXT NOT NULL,
    blocked INTEGER NOT NULL DEFAULT 0,
    latency_ms INTEGER NOT NULL DEFAULT 0,
    answers_json TEXT NOT NULL DEFAULT '[]'
);

CREATE INDEX IF NOT EXISTS idx_dns_logs_timestamp ON dns_logs(timestamp DESC);

CREATE TABLE IF NOT EXISTS bandwidth_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    app_id TEXT NOT NULL,
    interval_start TEXT NOT NULL,
    interval_end TEXT NOT NULL,
    bytes_in INTEGER NOT NULL DEFAULT 0,
    bytes_out INTEGER NOT NULL DEFAULT 0,
    peak_bps INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_bandwidth_app_id ON bandwidth_stats(app_id);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY NOT NULL,
    value_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);
