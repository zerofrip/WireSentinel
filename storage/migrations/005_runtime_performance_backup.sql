-- Runtime state, performance, enterprise policy, backup manifest

CREATE TABLE IF NOT EXISTS runtime_state (
    id TEXT PRIMARY KEY NOT NULL,
    scope TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    state_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_runtime_state_scope ON runtime_state(scope);
CREATE UNIQUE INDEX IF NOT EXISTS idx_runtime_state_scope_entity ON runtime_state(scope, entity_id);

CREATE TABLE IF NOT EXISTS performance_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    cpu_percent REAL NOT NULL,
    memory_bytes INTEGER NOT NULL,
    api_latency_ms REAL NOT NULL,
    wfp_latency_ms REAL NOT NULL,
    event_throughput REAL NOT NULL,
    timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_performance_snapshots_ts ON performance_snapshots(timestamp DESC);

CREATE TABLE IF NOT EXISTS enterprise_policy (
    id TEXT PRIMARY KEY NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    policy_json TEXT NOT NULL,
    locked_keys_json TEXT NOT NULL DEFAULT '[]',
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS backup_manifest (
    id TEXT PRIMARY KEY NOT NULL,
    operation TEXT NOT NULL,
    format TEXT NOT NULL,
    checksum TEXT NOT NULL,
    created_at TEXT NOT NULL,
    detail_json TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_backup_manifest_created ON backup_manifest(created_at DESC);
