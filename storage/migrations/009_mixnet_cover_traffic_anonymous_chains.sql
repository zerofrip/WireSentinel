-- Mixnet, anonymous chains, cover traffic, privacy analytics

CREATE TABLE IF NOT EXISTS mixnet_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    provider TEXT NOT NULL,
    plugin_id TEXT,
    gateway_id TEXT,
    config_json TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    active INTEGER NOT NULL DEFAULT 0,
    latency_ms INTEGER,
    last_health_at TEXT,
    last_error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_mixnet_profiles_enabled ON mixnet_profiles(enabled);

CREATE TABLE IF NOT EXISTS mixnet_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    profile_id TEXT NOT NULL,
    route_json TEXT,
    state TEXT NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    rx_bytes INTEGER NOT NULL DEFAULT 0,
    tx_bytes INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_mixnet_sessions_profile ON mixnet_sessions(profile_id);
CREATE INDEX IF NOT EXISTS idx_mixnet_sessions_started ON mixnet_sessions(started_at);

CREATE TABLE IF NOT EXISTS anonymous_chains (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    hops_json TEXT NOT NULL DEFAULT '[]',
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS cover_traffic_settings (
    id TEXT PRIMARY KEY NOT NULL,
    mixnet_profile_id TEXT,
    cover_profile TEXT NOT NULL DEFAULT 'disabled',
    enabled INTEGER NOT NULL DEFAULT 0,
    rate_bps INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_cover_traffic_profile ON cover_traffic_settings(mixnet_profile_id);

CREATE TABLE IF NOT EXISTS privacy_analytics (
    id TEXT PRIMARY KEY NOT NULL,
    anonymity_score INTEGER NOT NULL,
    route_entropy REAL NOT NULL,
    path_diversity REAL NOT NULL,
    cover_traffic_effectiveness REAL NOT NULL,
    timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_privacy_analytics_timestamp ON privacy_analytics(timestamp);
