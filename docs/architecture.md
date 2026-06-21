# WireSentinel Architecture

WireSentinel is a Windows networking security platform that combines application-level firewall, VPN, DNS security, traffic monitoring, and policy enforcement behind a single privileged Windows Service and a Tauri desktop UI.

## Process model

| Component | Binary | Privilege | Role |
|-----------|--------|-----------|------|
| Core service | `wire-sentinel-service.exe` | LocalSystem (Windows Service) | WFP filtering, VPN orchestration, DNS layer, REST/WebSocket API |
| Desktop UI | `wire-sentinel.exe` | Standard user (elevated only when needed) | Dashboard, configuration, real-time events |

The UI communicates with the service exclusively over the loopback REST API (`127.0.0.1:8170`) using a DPAPI-protected bearer token.

## Crate layout

```
shared-types/     Serde models, IPC DTOs, phase schemas
event-bus/        In-process pub/sub for service events
storage/          SQLite persistence + repository traits
app-registry/     Per-application identity and routing defaults
policy-engine/    Deterministic rule evaluation (allow/block/route)
wfp/              Windows Filtering Platform abstraction
vpn-engine/       WireGuard SCM + tunnel.dll backend
traffic-monitor/  Connection polling, bandwidth snapshots
dns/              DoH/DoT resolver, query logging, provider failover
filter-lists/     Remote filter list subscriptions
dpi-transforms/   Traffic obfuscation pipeline
transport-engine/ sing-box / xray / amnezia transport backends
core-service/     Orchestrator, API, diagnostics, backup, recovery
ui/               Tauri v2 + React frontend
```

## Data flow

```
Application traffic
       │
       ▼
  WFP engine ──► policy-engine ──► verdict (allow / block / route)
       │                                    │
       ▼                                    ▼
 traffic-monitor                      vpn-engine / transport-engine
       │                                    │
       ▼                                    ▼
   SQLite storage ◄──── event-bus ────► REST/WS API ──► UI
```

## Persistence

All service state lives under `%ProgramData%\WireSentinel\`:

| Path | Contents |
|------|----------|
| `wiresentinel.db` | SQLite database (rules, apps, logs, settings) |
| `.api-token` | DPAPI-encrypted API bearer token |
| `tunnels/` | Materialized VPN configs (DPAPI-encrypted blobs) |
| `transports/` | Transport profile configs |
| `logs/` | Rotating service log files |

Schema migrations are applied automatically on startup (`storage/migrations/`).

## API layer

Built on **Axum 0.7** with:

- Bearer auth middleware on all `/api/v1/*` routes (except Swagger)
- Rate limiting: 100 REST requests/minute, max 5 WebSocket connections
- OpenAPI docs at `/api/v1/docs`
- Real-time events via WebSocket at `/api/v1/events`

## Orchestrator

`core-service/src/orchestrator.rs` coordinates:

1. Traffic events from WFP → policy evaluation → firewall decision logging
2. DNS queries through the DNS layer with optional blocking/filtering
3. VPN connect/disconnect with crash-recovery state persistence
4. Transport chain management and leak detection
5. Privacy score calculation and metrics aggregation

## Phase 6 additions

Phase 6 introduces release packaging, installer hardening, validation/benchmark storage, security findings, and operational documentation. See `shared-types/src/phase6.rs` and migration `006_phase6.sql`.

## Deployment topology

```
┌─────────────────────────────────────────┐
│  Windows host                           │
│  ┌─────────────┐    loopback:8170       │
│  │ wire-sentinel│◄──────────────────┐  │
│  │ (Tauri UI)  │                     │  │
│  └─────────────┘                     │  │
│                                      │  │
│  ┌──────────────────────────────────┴┐ │
│  │ WireSentinel Service (LocalSystem)  │ │
│  │  WFP · VPN · DNS · Policy · API     │ │
│  └─────────────────────────────────────┘ │
│         │              │                 │
│    tunnel.dll     Windows Firewall       │
│    wireguard.dll  (loopback rule)        │
└─────────────────────────────────────────┘
```
