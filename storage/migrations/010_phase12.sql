-- Phase 12 schema: kernel / NDIS telemetry snapshots

CREATE TABLE IF NOT EXISTS kernel_telemetry_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    guardian_mode TEXT NOT NULL DEFAULT 'wfp',
    telemetry_json TEXT NOT NULL,
    statistics_json TEXT,
    captured_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_kernel_telemetry_captured ON kernel_telemetry_snapshots(captured_at);
