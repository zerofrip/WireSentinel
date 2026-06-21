# API Reference

Base URL: `http://127.0.0.1:8170`

Interactive docs: [`/api/v1/docs`](http://127.0.0.1:8170/api/v1/docs) (Swagger UI)

## Authentication

All `/api/v1/*` endpoints except `/api/v1/docs` and `/api/v1/openapi.json` require:

```
Authorization: Bearer <token>
```

The token is stored at `%ProgramData%\WireSentinel\.api-token` (DPAPI-encrypted).

WebSocket connections accept the header or `?token=<token>` query parameter.

### Auth endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/auth/rotate` | Generate and persist a new token |

## Status and health

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/status` | Service status summary |
| GET | `/api/v1/diagnostics` | Subsystem health checks |
| POST | `/api/v1/diagnostics/export` | ZIP support bundle |
| GET | `/api/v1/metrics` | Metrics snapshot (JSON) |
| GET | `/api/v1/metrics?format=prometheus` | Prometheus text format |
| GET | `/api/v1/performance` | Performance benchmark snapshots |

## Applications and traffic

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/apps` | List discovered applications |
| POST | `/api/v1/apps` | Set per-app default route |
| GET | `/api/v1/traffic` | Bandwidth snapshots |
| GET | `/api/v1/traffic/logs` | Paginated traffic logs |
| GET | `/api/v1/traffic/export` | Export (JSON/CSV) |
| GET | `/api/v1/traffic/top-domains` | Top queried domains |
| GET | `/api/v1/correlations` | DNS/traffic correlation records |

## Rules and policy

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/rules` | List firewall rules |
| POST | `/api/v1/rules` | Create rule |
| PUT | `/api/v1/rules/{id}` | Update rule |
| DELETE | `/api/v1/rules/{id}` | Delete rule |
| POST | `/api/v1/rules/kill-switch` | Enable/disable kill switch |
| GET | `/api/v1/rules/mode` | Get policy mode |
| PUT | `/api/v1/rules/mode` | Set policy mode |

## Statistics and audit

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/statistics/routes` | Route statistics |
| GET | `/api/v1/statistics/blocked` | Blocked traffic summary |
| GET | `/api/v1/audit` | Security audit log |

## VPN

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/vpn` | List profiles with status |
| POST | `/api/v1/vpn` | Add profile (plaintext config body) |
| POST | `/api/v1/vpn/{id}/connect` | Connect tunnel |
| POST | `/api/v1/vpn/{id}/disconnect` | Disconnect tunnel |
| GET | `/api/v1/vpn/{id}/status` | Tunnel status |

## DNS

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/dns` | DNS settings |
| PUT | `/api/v1/dns` | Update DNS settings |
| GET | `/api/v1/dns/logs` | Query log |
| POST | `/api/v1/dns/resolve` | Resolve hostname |
| GET | `/api/v1/dns/providers` | DNS provider list |
| PUT | `/api/v1/dns/providers` | Upsert providers |

## Filter lists

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/filter-lists` | List subscriptions |
| POST | `/api/v1/filter-lists` | Add subscription |
| PUT | `/api/v1/filter-lists/{id}` | Update subscription |
| DELETE | `/api/v1/filter-lists/{id}` | Remove subscription |
| POST | `/api/v1/filter-lists/{id}/update` | Refresh from remote |

## Transports and chains

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/transports` | Transport profiles |
| GET | `/api/v1/transports/status` | Runtime status |
| GET | `/api/v1/chains` | Chain profiles |
| POST | `/api/v1/chains` | Create chain profile |

## Privacy and leaks

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/privacy` | Privacy score snapshot |
| GET | `/api/v1/leaks` | Recent leak incidents |

## Logs

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/logs` | Recent in-memory log entries |
| GET | `/api/v1/logs/download` | ZIP of log files |
| PUT | `/api/v1/settings/log-level` | Set log level |

## Backup

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/backup/export` | Export JSON backup |
| GET | `/api/v1/backup/export?format=encrypted` | DPAPI-encrypted export |
| POST | `/api/v1/backup/import` | Import backup (`format`: json/encrypted) |

## Enterprise and updates

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/enterprise/policy` | Active enterprise policy |
| PUT | `/api/v1/enterprise/policy` | Apply enterprise policy |
| GET | `/api/v1/update` | Current update info |
| POST | `/api/v1/update/check` | Check remote feed |

## WebSocket events

```
GET /api/v1/events
Upgrade: websocket
Authorization: Bearer <token>
```

Event payloads follow `ServiceEventInner` schema (see `shared-types/src/events.rs`).

## Rate limits

- REST: 100 requests per minute (HTTP 429 on exceed)
- WebSocket: maximum 5 concurrent connections

## Error responses

| Status | Meaning |
|--------|---------|
| 401 | Missing or invalid bearer token |
| 403 | Enterprise policy lock |
| 404 | Resource not found |
| 429 | Rate limit exceeded |
| 500 | Internal service error |
