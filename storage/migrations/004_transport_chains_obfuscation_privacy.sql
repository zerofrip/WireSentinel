-- Transport, chains, obfuscation, DNS providers, leaks, privacy

CREATE TABLE IF NOT EXISTS transport_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    transport_kind TEXT NOT NULL,
    config_json TEXT,
    config_path TEXT,
    binary_path TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS obfuscation_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    preset TEXT NOT NULL DEFAULT 'disabled',
    modules_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS chain_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    hops_json TEXT NOT NULL,
    obfuscation_profile_id TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (obfuscation_profile_id) REFERENCES obfuscation_profiles(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS dns_providers (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    transport TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 100,
    enabled INTEGER NOT NULL DEFAULT 1,
    latency_ms INTEGER,
    last_check TEXT,
    failure_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_dns_providers_priority ON dns_providers(priority ASC);

CREATE TABLE IF NOT EXISTS leak_incidents (
    id TEXT PRIMARY KEY NOT NULL,
    leak_type TEXT NOT NULL,
    app_id TEXT,
    detail_json TEXT,
    severity TEXT NOT NULL DEFAULT 'warning',
    detected_at TEXT NOT NULL,
    resolved_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_leak_incidents_type ON leak_incidents(leak_type);
CREATE INDEX IF NOT EXISTS idx_leak_incidents_detected ON leak_incidents(detected_at DESC);

CREATE TABLE IF NOT EXISTS privacy_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    score INTEGER NOT NULL,
    components_json TEXT NOT NULL,
    timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_privacy_snapshots_ts ON privacy_snapshots(timestamp DESC);

-- Extend vpn_profiles (idempotent via pragma check pattern)
ALTER TABLE vpn_profiles ADD COLUMN transport_kind TEXT NOT NULL DEFAULT 'direct';
ALTER TABLE vpn_profiles ADD COLUMN chain_id TEXT;
ALTER TABLE vpn_profiles ADD COLUMN obfuscation_profile_id TEXT;
