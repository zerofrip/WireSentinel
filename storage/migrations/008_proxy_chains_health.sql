-- Phase 9 schema: proxy health columns and proxy chains

ALTER TABLE proxy_profiles ADD COLUMN active INTEGER NOT NULL DEFAULT 0;
ALTER TABLE proxy_profiles ADD COLUMN last_health_at TEXT;
ALTER TABLE proxy_profiles ADD COLUMN last_error TEXT;

CREATE TABLE IF NOT EXISTS proxy_chains (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    hops_json TEXT NOT NULL DEFAULT '[]',
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
