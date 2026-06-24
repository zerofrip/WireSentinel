-- TCP termination + handshake proxy on VPN profiles

ALTER TABLE vpn_profiles ADD COLUMN handshake_proxy_json TEXT;

ALTER TABLE obfuscation_profiles ADD COLUMN handshake_proxy_json TEXT;

CREATE TABLE IF NOT EXISTS tcp_termination_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    mode TEXT NOT NULL DEFAULT 'disabled',
    updated_at TEXT NOT NULL
);

INSERT OR IGNORE INTO tcp_termination_settings (id, mode, updated_at)
VALUES (1, 'disabled', datetime('now'));

CREATE TABLE IF NOT EXISTS tcp_termination_rules (
    id TEXT PRIMARY KEY NOT NULL,
    process_path TEXT,
    process_name TEXT,
    profile_id TEXT,
    route_json TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tcp_term_rules_enabled ON tcp_termination_rules(enabled);
