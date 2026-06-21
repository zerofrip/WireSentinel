# WireSentinel

Production-grade Windows networking security application combining WireGuard VPN, application-level firewall, DNS security, and traffic monitoring.

## Architecture

- **`wire-sentinel-service`** — Windows Service (admin): WFP filtering, VPN, DNS, policy engine, REST/WS API
- **`wire-sentinel`** — Tauri GUI: dashboard, apps, traffic, VPN profiles, rules, DNS settings

## Crates

| Crate | Purpose |
|-------|---------|
| `shared-types` | Serde models + IPC DTOs |
| `policy-engine` | Deterministic rule evaluation |
| `wfp` | WFP abstraction (userspace MVP, kernel Phase 2) |
| `vpn-engine` | VPN backend (SCM + tunnel.dll MVP) |
| `traffic-monitor` | Connection polling + bandwidth |
| `dns` | DoH resolver + query logging |
| `core-service` | Service orchestrator + API |
| `ui/` | Tauri v2 + React frontend |

## Quick Start (Development)

### Service (console mode)

```bash
cargo run -p core-service -- --console
```

API listens on `http://127.0.0.1:8170`

### UI

```bash
cd ui
npm install
npm run tauri dev
```

### Tests

```bash
cargo test --workspace
```

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/auth/token` | Obtain API token |
| GET | `/api/v1/status` | Service status |
| GET/POST | `/api/v1/apps` | Application list / per-app rules |
| GET | `/api/v1/traffic` | Bandwidth snapshots |
| GET/POST | `/api/v1/rules` | Rule management |
| GET/POST | `/api/v1/vpn/profiles` | VPN profile CRUD |
| POST | `/api/v1/vpn/connect` | Connect VPN |
| GET/PUT | `/api/v1/dns` | DNS settings |
| WS | `/api/v1/events` | Real-time events |

## Configuration

Stored in `%ProgramData%\WireSentinel\` (DPAPI-encrypted tunnel configs under `tunnels\`, transport configs under `transports\`, service logs under `logs\`).

## Security

The REST/WebSocket API binds to **`127.0.0.1:8170` only** (loopback). Installers (MSI and NSIS) add a Windows Firewall inbound rule scoped to `127.0.0.1` on port 8170 so local clients can reach the API without exposing it to the LAN.

Authentication uses a bearer token stored at `%ProgramData%\WireSentinel\.api-token` (DPAPI-protected, `CRYPTPROTECT_LOCAL_MACHINE` scope).

- All `/api/v1/*` routes except Swagger docs (`/api/v1/docs`, `/api/v1/openapi.json`) require `Authorization: Bearer <token>`.
- **`POST /api/v1/auth/rotate`** generates a new token, persists it to the DPAPI file, updates the in-memory service token, and emits a security audit event. **Rotate the token after every fresh install** and whenever the token may have leaked. The response includes the new token once — update the UI/client configuration immediately.
- Rate limiting: REST endpoints are capped at **100 requests/minute**; WebSocket connections are limited to **5 concurrent** clients. WebSocket auth accepts the bearer header or `?token=` query parameter.
- VPN tunnel configs and transport profiles under `%ProgramData%\WireSentinel\` are DPAPI-encrypted at rest. Encrypted backup export is available via `GET /api/v1/backup/export?format=encrypted`.

See [docs/threat-model.md](docs/threat-model.md) and [docs/administrator-guide.md](docs/administrator-guide.md) for deployment hardening.

## Building for Windows

See [installer/README.md](installer/README.md) and [resources/README.md](resources/README.md).

## License

Apache-2.0
