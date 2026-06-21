# Telemetry and Observability

WireSentinel provides local observability without sending data to third parties by default. All metrics, logs, and diagnostics remain on the host unless explicitly exported.

## Design principles

- **Local-first** — no phone-home telemetry in 0.1.0
- **Opt-in export** — administrators trigger diagnostics/backup downloads
- **Audit trail** — security-sensitive actions are logged locally

## Metrics

### JSON snapshot

```http
GET /api/v1/metrics
Authorization: Bearer <token>
```

Returns `MetricsSnapshot` including:

- Blocked firewall decisions count
- DNS queries processed
- Open leak incidents
- VPN connection count
- Transport process states
- Uptime

### Prometheus format

```http
GET /api/v1/metrics?format=prometheus
```

Exports counters compatible with Prometheus scraping. Example metrics:

| Metric | Description |
|--------|-------------|
| `wiresentinel_blocked_requests` | Blocked firewall decisions |
| `wiresentinel_dns_queries_total` | DNS queries observed |
| `wiresentinel_leak_incidents_open` | Unresolved leak incidents |
| `wiresentinel_vpn_active` | Active VPN tunnels |

Bind Prometheus to loopback only — the API is not TLS-terminated.

## Performance benchmarks

Phase 6 stores benchmark snapshots in SQLite:

```http
GET /api/v1/performance?limit=20
```

Fields: WFP latency, route latency, DNS latency, transport startup time, UI event throughput.

## Logging

### In-memory ring buffer

```http
GET /api/v1/logs?limit=100&level=warn
```

### File logs

Rotating files under `%ProgramData%\WireSentinel\logs\`.

```http
GET /api/v1/logs/download
```

Returns a ZIP archive of log files.

### Log level

```http
PUT /api/v1/settings/log-level
{"level": "info"}
```

Enterprise policy may lock this setting.

## Diagnostics health

```http
GET /api/v1/diagnostics
```

Returns subsystem health for:

| Subsystem | Checks |
|-----------|--------|
| WFP | Engine loaded |
| VPN | Active tunnel count |
| DNS | Enabled/disabled state |
| Transport | Profile count and status |
| Database | SQLite connectivity |
| Disk | Free space under ProgramData |

### Support bundle

```http
POST /api/v1/diagnostics/export
```

ZIP containing logs, health snapshot, and sanitized configuration metadata.

## Real-time events (WebSocket)

```http
GET /api/v1/events
Upgrade: websocket
```

Event types (`ServiceEventInner`):

- Traffic blocked/allowed
- DNS query observed/blocked
- VPN state changes
- Rule created/updated/deleted
- Filter list updates
- Security audit entries
- Token rotation

Maximum 5 concurrent WebSocket connections per service instance.

## Audit log

Security-relevant actions are persisted:

```http
GET /api/v1/audit?limit=100&event_type=token_rotation
```

Recorded actions include:

- Token rotation
- Policy changes (kill switch, policy mode)
- Route changes
- Enterprise policy updates
- Backup import/export (via backup manifest)

## Privacy score

```http
GET /api/v1/privacy
```

Composite score from:

- Encrypted DNS usage
- Active filter lists
- VPN coverage
- Route leakage (direct vs tunneled bytes)
- Recent leak incidents

Snapshots stored in `privacy_snapshots` table.

## Validation results (Phase 6)

Installer and runtime validation checks are stored in `validation_results`:

- Check name and status (`pass`, `fail`, `warn`)
- Timestamp and optional message

Used by release QA and future in-app validation dashboard.

## Security findings (Phase 6)

Tracked in `security_findings` table with severity levels. No automatic external reporting — review via database or future admin UI.

## Update check (optional network)

When `WIRESENTINEL_UPDATE_FEED` is set:

```http
POST /api/v1/update/check
```

Contacts the configured HTTPS feed. Only version metadata is exchanged — no usage telemetry.

## Third-party data flows

| Destination | Data | Trigger |
|-------------|------|---------|
| DNS providers (DoH/DoT) | DNS queries | User-enabled DNS |
| Filter list URLs | HTTP download | User-configured subscriptions |
| Update feed | Version check | Admin-configured env var |
| VPN server | Encrypted tunnel traffic | User VPN connect |

No analytics, crash reporting, or license phone-home is included in 0.1.0.

## Hardening recommendations

- Scrape metrics via localhost-only Prometheus agent
- Forward logs to SIEM by copying `logs/` or using diagnostics export on schedule
- Disable update feed if air-gapped
- Review audit log regularly for token rotation and policy changes
