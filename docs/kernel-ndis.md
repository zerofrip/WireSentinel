# Kernel / NDIS Integration (Phase 12)

Phase 12 adds an NDIS Lightweight Filter (LWF) datapath alongside the existing Guardian WFP driver.

## Guardian modes

| Mode | Behavior |
|------|----------|
| `wfp` | Guardian WFP only (default) |
| `ndis` | WFP policy + NDIS route sync for packet redirect |
| `hybrid` | Guardian kernel callouts + NDIS LWF with shared telemetry |

Configure via settings key `guardian_mode` (default `wfp`).

## API

- `GET /api/v1/kernel/telemetry` — combined Guardian + NDIS telemetry (`KernelTelemetryV2`)
- `GET /api/v1/kernel/statistics` — aggregate counters (`KernelStatistics`)

## Dependencies

- `WireSentinel-Kernel` — Guardian WFP driver (`guardian-controller`)
- `WireSentinel-Ndis` — NDIS LWF driver (`ndis-controller`, `ndis-sdk`)

## Persistence

Telemetry snapshots are stored in `kernel_telemetry_snapshots` (migration `010_kernel_ndis_telemetry.sql`).

See also [WireSentinel-Ndis architecture](../../WireSentinel-Ndis/docs/architecture.md).
