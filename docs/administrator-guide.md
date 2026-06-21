# Administrator Guide

This guide covers enterprise deployment, service management, and security hardening for WireSentinel on Windows.

## Requirements

- Windows 10/11 or Windows Server 2019+ (x64 or ARM64)
- Administrator privileges for installation
- WireGuard runtime DLLs (`tunnel.dll`, `wireguard.dll`) bundled with the installer — see [resources/README.md](../resources/README.md)

## Installation

Use either MSI (WiX) or NSIS EXE. Both register the `WireSentinel` Windows Service, create `%ProgramData%\WireSentinel\` data directories, and add a loopback firewall rule for port 8170.

```powershell
# Silent NSIS install
.\WireSentinel-0.1.0-x64-setup.exe /S

# MSI install
msiexec /i WireSentinel-0.1.0-x64.msi /qn
```

After install:

1. Start the service: `sc.exe start WireSentinel`
2. Rotate the API token: `POST http://127.0.0.1:8170/api/v1/auth/rotate` (requires initial token from `.api-token`)
3. Launch the UI from Start Menu or desktop shortcut

See [installer-guide.md](installer-guide.md) for build and packaging details.

## Service configuration

| Property | Value |
|----------|-------|
| Service name | `WireSentinel` |
| Display name | WireSentinel Network Security Service |
| Account | LocalSystem |
| Start type | Manual (demand) |
| Dependencies | `Tcpip`, `Dnscache` |
| Recovery | Restart after 60 s (3 attempts, reset daily) |

### Manual service control

```powershell
sc.exe query WireSentinel
sc.exe start WireSentinel
sc.exe stop WireSentinel
```

Console mode (development/troubleshooting):

```powershell
wire-sentinel-service.exe --console
```

## Data directories

| Directory | Purpose | Backup priority |
|-----------|---------|-----------------|
| `%ProgramData%\WireSentinel\wiresentinel.db` | All configuration and logs | **Critical** |
| `%ProgramData%\WireSentinel\.api-token` | API authentication | **Critical** (rotate after restore) |
| `%ProgramData%\WireSentinel\tunnels\` | Encrypted VPN configs | High |
| `%ProgramData%\WireSentinel\transports\` | Transport profiles | High |
| `%ProgramData%\WireSentinel\logs\` | Service logs | Medium |

## Enterprise policy

Apply locked settings via `PUT /api/v1/enterprise/policy`. Locked fields (e.g. `log_level`) cannot be changed from the UI when enterprise policy is active.

Audit events for policy changes are emitted on the event bus and stored in the audit log.

## API token management

The token file uses DPAPI with `CRYPTPROTECT_LOCAL_MACHINE`. Only LocalSystem and Administrators can decrypt it on the installed machine.

**Post-install checklist:**

- [ ] Rotate token via API or delete `.api-token` and restart service (generates new token)
- [ ] Confirm UI connects with new token
- [ ] Restrict physical/logical access to `%ProgramData%\WireSentinel\`

## Firewall

Installers add an inbound rule:

```
Name:     WireSentinel API (loopback)
Port:     8170/TCP
Remote:   127.0.0.1 only
```

Verify:

```powershell
netsh advfirewall firewall show rule name="WireSentinel API (loopback)"
```

The service API does not bind to `0.0.0.0`; the firewall rule is defense-in-depth for local policy compliance.

## Monitoring

- **Health**: `GET /api/v1/diagnostics`
- **Metrics**: `GET /api/v1/metrics` (JSON) or `?format=prometheus`
- **Audit log**: `GET /api/v1/audit`
- **Log download**: `GET /api/v1/logs/download`

## Updates

WireSentinel 0.1.0 ships a check-only update framework. Configure the feed URL:

```
set WIRESENTINEL_UPDATE_FEED=https://updates.example.com
```

Then call `POST /api/v1/update/check`. Full auto-update staging is planned for a future release.

## Uninstall

MSI/NSIS uninstallers stop and remove the service, delete the firewall rule, and remove program files. **User data under `%ProgramData%\WireSentinel\` is preserved** unless manually deleted.
