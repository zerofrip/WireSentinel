# Backup and Recovery

WireSentinel stores configuration in SQLite and encrypted files under `%ProgramData%\WireSentinel\`. This guide covers export, import, and disaster recovery.

## What is backed up

The backup bundle (`BackupBundle`) includes:

| Category | Contents |
|----------|----------|
| Rules | Firewall rules and policy mode |
| Apps | Application registry entries |
| DNS | Settings and provider records |
| VPN | Profile metadata (configs stored as encrypted blobs) |
| Filter lists | Subscriptions and cache paths |
| Transports | Transport and chain profiles |
| Obfuscation | Obfuscation profiles |
| Enterprise | Active enterprise policy |
| Settings | Service settings JSON |

Traffic logs, DNS query logs, and audit history are **not** included in standard backup exports.

## Export via API

### JSON (portable, unencrypted)

```http
GET /api/v1/backup/export
Authorization: Bearer <token>
```

Returns a JSON document suitable for review or import on another machine (VPN secrets remain encrypted in blob form).

### Encrypted (same-machine restore)

```http
GET /api/v1/backup/export?format=encrypted
Authorization: Bearer <token>
```

Returns a DPAPI-protected binary blob. Restore only works on the **same Windows machine** that created the export.

## Import via API

```http
POST /api/v1/backup/import
Authorization: Bearer <token>
Content-Type: application/json

{
  "format": "json",
  "data": "<json string or base64 for encrypted>"
}
```

For encrypted imports, set `"format": "encrypted"` and provide base64-encoded data.

Import triggers an audit event and records a manifest entry with checksum.

## Manual file backup

Stop the service before copying files to ensure consistency:

```powershell
sc.exe stop WireSentinel
Copy-Item -Recurse "$env:ProgramData\WireSentinel" "D:\backups\WireSentinel-$(Get-Date -Format yyyyMMdd)"
sc.exe start WireSentinel
```

Critical files:

- `wiresentinel.db`
- `.api-token`
- `tunnels\`
- `transports\`

## Recovery after crash

WireSentinel persists runtime state for VPN, transport, chain, and DNS scopes in the `runtime_state` table. On service restart, `RecoveryService::recover_all` attempts to restore:

- Previously connected VPN tunnels
- Running transport processes
- Active chain configurations
- DNS provider selection

Check recovery status in service logs after restart.

## Restore procedure

1. Stop WireSentinel service
2. Back up current `%ProgramData%\WireSentinel\` (if accessible)
3. Replace `wiresentinel.db` with backup copy **or** import via API on a fresh install
4. Restore `tunnels\` and `transports\` directories if copied manually
5. Start service
6. **Rotate API token** — restored `.api-token` may be stale or compromised
7. Verify VPN profiles and rules in UI

## Migration to new hardware

1. Export JSON backup from old machine
2. Install WireSentinel on new machine
3. Import JSON backup
4. Re-add VPN configs if encrypted blobs cannot be decrypted cross-machine
5. Rotate token and reconfigure UI

DPAPI-encrypted exports and tunnel blobs are **machine-bound** and cannot be moved directly.

## Backup manifest audit

Each export/import records an entry in the `backup_manifest` table with operation type, format, checksum, and timestamp. Query via SQLite or future admin API.

## Best practices

- Schedule weekly JSON exports to secure storage
- Rotate tokens after any restore
- Test restore on a VM before relying on backups in production
- Keep WireGuard private keys in a separate password manager as ultimate fallback
