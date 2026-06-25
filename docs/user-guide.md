# User Guide

WireSentinel helps you control which applications can reach the network, route traffic through VPN tunnels, and secure DNS queries — all from a single desktop app.

## Getting started

1. **Install** WireSentinel using the MSI or setup EXE (requires administrator approval).
2. **Open WireSentinel** from the Start Menu or desktop shortcut — the app starts the backend service automatically when needed.

The UI connects automatically to the local service at `http://127.0.0.1:8170`.

## Dashboard

The dashboard shows:

- Service status (running/stopped)
- Active VPN connections
- Recent traffic and blocked requests
- Privacy score snapshot
- Real-time events via WebSocket

## Applications

The **Apps** page lists discovered applications with their default routing (direct, VPN, block). Click an app to change its route or create per-app rules.

## Traffic

View bandwidth snapshots and detailed traffic logs. Export data as JSON or CSV from the API or UI traffic page.

## Rules

Create firewall rules to allow, block, or route traffic by application, domain, or IP. Policy modes:

- **Allow list** — block everything except matched rules
- **Block list** — allow everything except matched rules

Enable **kill switch** to block all traffic when VPN disconnects unexpectedly.

## VPN

Add WireGuard profiles by pasting a `.conf` file. Connect and disconnect from the VPN page. Profiles are stored encrypted under `%ProgramData%\WireSentinel\tunnels\`.

## DNS

Configure DNS-over-HTTPS (DoH), DNS-over-TLS (DoT), or plain DNS. View query logs and blocked domains. Add custom DNS providers for failover.

## Filter lists

Subscribe to remote block lists (similar to ad-block filter lists). Lists are cached locally and refreshed on demand.

## Settings

- Log level (debug/info/warn/error)
- Connection history and timeline preferences
- Backup export/import
- Update channel selection

## Backup and restore

Export your configuration from **Settings → Backup**:

- **JSON** — human-readable, unencrypted (use only on trusted systems)
- **Encrypted** — DPAPI-protected blob for same-machine restore

See [backup-recovery.md](backup-recovery.md) for details.

## Diagnostics

Run health checks from the Diagnostics page. Export a support bundle (ZIP) containing logs and subsystem status — useful when contacting support.

## Tips

- After a fresh install, rotate your API token in Settings → Security.
- Keep the WireSentinel service running for continuous protection.
- Use kill switch when you need strict VPN-only connectivity.
