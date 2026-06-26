-- Deterministic upsert support: unique buckets for stats, WFP state, domain cache

CREATE UNIQUE INDEX IF NOT EXISTS idx_route_stats_bucket
ON route_statistics(
    COALESCE(app_id, ''),
    COALESCE(profile_id, ''),
    COALESCE(domain, ''),
    route_type,
    window_start
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_wfp_filter_scope
ON wfp_filter_state(scope_type, COALESCE(scope_value, ''));

CREATE UNIQUE INDEX IF NOT EXISTS idx_domain_cache_app_ip
ON domain_resolver_cache(COALESCE(app_id, ''), ip_address);
