-- Global split tunnel templates

CREATE TABLE IF NOT EXISTS split_tunnel_templates (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    default_route_json TEXT NOT NULL,
    app_rules_json TEXT NOT NULL DEFAULT '[]',
    domain_rules_json TEXT NOT NULL DEFAULT '[]',
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_split_templates_enabled ON split_tunnel_templates(enabled);
