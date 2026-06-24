-- Phase 13 schema: Katzenpost, Loopix, anonymous services

CREATE TABLE IF NOT EXISTS katzenpost_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
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

CREATE INDEX IF NOT EXISTS idx_katzenpost_profiles_enabled ON katzenpost_profiles(enabled);

CREATE TABLE IF NOT EXISTS katzenpost_gateways (
    id TEXT PRIMARY KEY NOT NULL,
    profile_id TEXT NOT NULL,
    gateway_id TEXT NOT NULL,
    address TEXT NOT NULL,
    identity_key TEXT,
    latency_ms INTEGER,
    last_seen TEXT,
    FOREIGN KEY (profile_id) REFERENCES katzenpost_profiles(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_katzenpost_gateways_profile ON katzenpost_gateways(profile_id);

CREATE TABLE IF NOT EXISTS loopix_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    provider_id TEXT,
    config_json TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    active INTEGER NOT NULL DEFAULT 0,
    latency_ms INTEGER,
    last_health_at TEXT,
    last_error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_loopix_profiles_enabled ON loopix_profiles(enabled);

CREATE TABLE IF NOT EXISTS loopix_providers (
    id TEXT PRIMARY KEY NOT NULL,
    profile_id TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    address TEXT NOT NULL,
    region TEXT,
    latency_ms INTEGER,
    FOREIGN KEY (profile_id) REFERENCES loopix_profiles(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_loopix_providers_profile ON loopix_providers(profile_id);

CREATE TABLE IF NOT EXISTS anonymous_services (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    provider TEXT NOT NULL,
    plugin_id TEXT,
    profile_id TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_anonymous_services_enabled ON anonymous_services(enabled);

CREATE TABLE IF NOT EXISTS anonymous_service_endpoints (
    id TEXT PRIMARY KEY NOT NULL,
    service_id TEXT NOT NULL,
    host TEXT NOT NULL,
    port INTEGER NOT NULL,
    protocol TEXT NOT NULL DEFAULT 'tcp',
    path TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (service_id) REFERENCES anonymous_services(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_anonymous_service_endpoints_service ON anonymous_service_endpoints(service_id);

ALTER TABLE privacy_analytics ADD COLUMN anonymity_set_estimate REAL;
ALTER TABLE privacy_analytics ADD COLUMN cover_traffic_efficiency REAL;
ALTER TABLE privacy_analytics ADD COLUMN mixnet_diversity REAL;
ALTER TABLE privacy_analytics ADD COLUMN federation_diversity REAL;
