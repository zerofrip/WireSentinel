# Troubleshooting

Common WireSentinel issues and resolution steps.

## Service will not start

**Symptoms:** UI shows "service stopped"; `sc.exe query WireSentinel` reports STOPPED.

**Checks:**

1. Verify binaries exist: `C:\Program Files\WireSentinel\wire-sentinel-service.exe`
2. Check event log and service logs: `%ProgramData%\WireSentinel\logs\`
3. Run in console mode for detailed output:
   ```powershell
   & "C:\Program Files\WireSentinel\wire-sentinel-service.exe" --console
   ```
4. Confirm `tunnel.dll` and `wireguard.dll` are present in the install directory
5. Ensure `%ProgramData%\WireSentinel\` is writable by LocalSystem

**Common causes:**
- Missing WireGuard DLLs
- Corrupt SQLite database (restore from backup)
- Port 8170 already in use by another process

## UI cannot connect to API

**Symptoms:** Dashboard shows connection error; HTTP requests fail.

**Checks:**

1. Confirm service is running: `sc.exe query WireSentinel`
2. Test loopback: `curl http://127.0.0.1:8170/api/v1/status -H "Authorization: Bearer <token>"`
3. Verify firewall rule:
   ```powershell
   netsh advfirewall firewall show rule name="WireSentinel API (loopback)"
   ```
4. Check token file exists: `%ProgramData%\WireSentinel\.api-token`
5. Rotate token if stale: `POST /api/v1/auth/rotate`

## 401 Unauthorized

- Token mismatch between UI and service (restart service after manual token deletion)
- Missing `Authorization` header
- Token rotated but UI not updated

## 429 Too Many Requests

REST rate limit is 100 requests/minute. WebSocket limit is 5 connections. Reduce polling frequency or close unused WebSocket clients.

## VPN will not connect

1. Check profile config is valid WireGuard format
2. Review service logs for SCM/tunnel.dll errors
3. `GET /api/v1/vpn/{id}/status` for backend error details
4. Ensure no conflicting WireGuard adapter from other VPN software
5. Run diagnostics: `GET /api/v1/diagnostics`

## DNS not blocking / leaking

1. Confirm DNS enabled: `GET /api/v1/dns`
2. Check leak incidents: `GET /api/v1/leaks`
3. Verify DoH provider reachable
4. Review DNS logs: `GET /api/v1/dns/logs?blocked=true`

## Kill switch stuck active

Disable via API:

```powershell
curl -X POST http://127.0.0.1:8170/api/v1/rules/kill-switch `
  -H "Authorization: Bearer <token>" `
  -H "Content-Type: application/json" `
  -d '{"active": false}'
```

Or restart service after disconnecting VPN.

## Installer validation fails

Run static validation:

```powershell
.\installer\tests\validate.ps1
.\installer\tests\installer-e2e.ps1
```

Use `-SkipFileRefs -SkipDriverRefs` on CI/Linux when binaries and staged drivers are not built:

```powershell
.\installer\tests\validate.ps1 -SkipFileRefs -SkipDriverRefs
```

## Kernel driver will not load

**Symptoms:** `pnputil` or `sc start WireSentinelGuardian` fails; Device Manager shows driver blocked; Event Log reports signature policy error.

**Checks:**

1. Confirm test-signing is enabled (required for test-signed drivers):
   ```powershell
   bcdedit /enum {current} | findstr testsigning
   ```
   Expected: `testsigning             Yes`

2. If disabled, enable and reboot:
   ```powershell
   bcdedit /set testsigning on
   shutdown /r /t 0
   ```

3. Verify driver files are signed (on build machine or from ZIP `drivers/` folder):
   ```powershell
   signtool verify /pa "C:\Program Files\WireSentinel\drivers\guardian\Guardian.sys"
   ```

4. If you used `-SkipDriverSign` during build, drivers are unsigned and will not load — rebuild with default test signing.

5. Check `pnputil /enum-drivers` for `guardian.inf` / `guardian_lwf.inf` entries after install.

## Database corruption

1. Stop service
2. Back up `%ProgramData%\WireSentinel\wiresentinel.db`
3. Delete database file (service recreates schema on start)
4. Import backup: `POST /api/v1/backup/import`

## Collecting support data

```powershell
curl -X POST http://127.0.0.1:8170/api/v1/diagnostics/export `
  -H "Authorization: Bearer <token>" `
  -o wiresentinel-diagnostics.zip
```

Or download logs: `GET /api/v1/logs/download`

## Uninstall leftovers

If uninstall fails to remove the service:

```powershell
sc.exe stop WireSentinel
sc.exe delete WireSentinel
netsh advfirewall firewall delete rule name="WireSentinel API (loopback)"
```

Remove `%ProgramData%\WireSentinel\` manually if a full wipe is required.
