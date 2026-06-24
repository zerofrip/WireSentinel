-- Validation, benchmark, security findings

CREATE TABLE IF NOT EXISTS validation_results (
    id TEXT PRIMARY KEY NOT NULL,
    check_name TEXT NOT NULL,
    status TEXT NOT NULL,
    message TEXT,
    checked_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_validation_results_name ON validation_results(check_name);
CREATE INDEX IF NOT EXISTS idx_validation_results_checked ON validation_results(checked_at DESC);

CREATE TABLE IF NOT EXISTS benchmark_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    wfp_latency_ms REAL NOT NULL,
    route_latency_ms REAL NOT NULL,
    dns_latency_ms REAL NOT NULL,
    transport_startup_ms REAL NOT NULL,
    ui_event_throughput REAL NOT NULL,
    timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_benchmark_snapshots_ts ON benchmark_snapshots(timestamp DESC);

CREATE TABLE IF NOT EXISTS security_findings (
    id TEXT PRIMARY KEY NOT NULL,
    severity TEXT NOT NULL,
    category TEXT NOT NULL,
    title TEXT NOT NULL,
    detail_json TEXT NOT NULL DEFAULT '{}',
    resolved INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    resolved_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_security_findings_created ON security_findings(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_security_findings_resolved ON security_findings(resolved);
