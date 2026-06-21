-- Phase 7 schema: plugins, tailnet, tor, bridges, proxy profiles

CREATE TABLE IF NOT EXISTS plugins (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    format TEXT NOT NULL,
    manifest_json TEXT NOT NULL,
    state TEXT NOT NULL,
    permissions_json TEXT NOT NULL DEFAULT '[]',
    wasm_path TEXT,
    sha256 TEXT,
    error_message TEXT,
    installed_at TEXT NOT NULL,
    loaded_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_plugins_state ON plugins(state);

CREATE TABLE IF NOT EXISTS plugin_security_events (
    id TEXT PRIMARY KEY NOT NULL,
    plugin_id TEXT,
    violation_type TEXT NOT NULL,
    detail_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tailnet_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    auth_key TEXT,
    exit_node TEXT,
    subnet_router INTEGER NOT NULL DEFAULT 0,
    magic_dns INTEGER NOT NULL DEFAULT 1,
    hostname TEXT,
    tailnet_ip TEXT,
    connected INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tor_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    control_port INTEGER NOT NULL DEFAULT 9051,
    socks_port INTEGER NOT NULL DEFAULT 9050,
    data_dir TEXT NOT NULL,
    bridge_ids_json TEXT NOT NULL DEFAULT '[]',
    enabled INTEGER NOT NULL DEFAULT 1,
    bootstrap_progress INTEGER NOT NULL DEFAULT 0,
    circuit_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS bridge_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    bridge_type TEXT NOT NULL,
    config_json TEXT NOT NULL DEFAULT '{}',
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS proxy_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    host TEXT NOT NULL,
    port INTEGER NOT NULL,
    username TEXT,
    password_encrypted TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    latency_ms INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

ALTER TABLE filter_lists ADD COLUMN plugin_id TEXT;
