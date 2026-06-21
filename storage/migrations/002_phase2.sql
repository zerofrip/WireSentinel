-- Phase 2 schema extensions

ALTER TABLE traffic_logs ADD COLUMN process_id INTEGER;
ALTER TABLE traffic_logs ADD COLUMN source_ip TEXT;
ALTER TABLE traffic_logs ADD COLUMN destination_ip TEXT;
ALTER TABLE traffic_logs ADD COLUMN source_port INTEGER;
ALTER TABLE traffic_logs ADD COLUMN destination_port INTEGER;
ALTER TABLE traffic_logs ADD COLUMN bytes_sent INTEGER NOT NULL DEFAULT 0;
ALTER TABLE traffic_logs ADD COLUMN bytes_received INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_traffic_logs_dest_ip ON traffic_logs(destination_ip);

ALTER TABLE dns_logs ADD COLUMN response TEXT;
ALTER TABLE dns_logs ADD COLUMN correlation_id TEXT;

CREATE TABLE IF NOT EXISTS filter_lists (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    url TEXT,
    type TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    update_interval_secs INTEGER,
    last_updated TEXT,
    cache_path TEXT
);

CREATE INDEX IF NOT EXISTS idx_filter_lists_enabled ON filter_lists(enabled);

CREATE TABLE IF NOT EXISTS domain_correlations (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT,
    domain TEXT NOT NULL,
    ip_address TEXT,
    first_seen TEXT NOT NULL,
    last_seen TEXT NOT NULL,
    query_count INTEGER NOT NULL DEFAULT 0,
    traffic_count INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_correlations_app_domain ON domain_correlations(app_id, domain);
CREATE INDEX IF NOT EXISTS idx_correlations_ip ON domain_correlations(ip_address);
