# Threat Model

WireSentinel protects Windows hosts from unwanted network egress, DNS leaks, and uncontrolled application connectivity. This document describes assets, trust boundaries, and mitigations for version 0.1.0.

## Assets

| Asset | Location | Sensitivity |
|-------|----------|-------------|
| API bearer token | `%ProgramData%\WireSentinel\.api-token` | High — full service control |
| VPN private keys | SQLite + `tunnels/` (DPAPI blobs) | Critical |
| Transport configs | SQLite + `transports/` | High |
| Traffic/DNS logs | SQLite | Medium — behavioral data |
| Filter list cache | Local filesystem | Low |
| Service logs | `logs/` | Medium |

## Trust boundaries

```
┌──────────────────────────────────────────────────┐
│  Untrusted: LAN / Internet                       │
└────────────────────┬─────────────────────────────┘
                     │ VPN tunnel (encrypted)
┌────────────────────▼─────────────────────────────┐
│  Semi-trusted: Other local processes / users     │
│  ── loopback API (127.0.0.1:8170) + bearer token │
└────────────────────┬─────────────────────────────┘
                     │
┌────────────────────▼─────────────────────────────┐
│  Trusted: WireSentinel Service (LocalSystem)     │
│  WFP · DNS · VPN · SQLite                        │
└──────────────────────────────────────────────────┘
```

## Threats and mitigations

### T1: Remote API exploitation

**Threat:** Attacker on the network accesses the REST API.

**Mitigations:**
- API binds to loopback only (`127.0.0.1:8170`)
- Windows Firewall rule restricts inbound TCP 8170 to `127.0.0.1`
- Bearer token required on all authenticated routes
- Rate limiting (100 req/min REST, 5 WS connections)

**Residual risk:** Local malware with user privileges can read the DPAPI token if running as Administrator or via token theft from the UI process.

### T2: Token leakage

**Threat:** API token exposed via logs, backups, or screenshots.

**Mitigations:**
- Token stored with DPAPI `LOCAL_MACHINE` scope
- `POST /api/v1/auth/rotate` for post-install rotation
- Audit event emitted on rotation
- Encrypted backup format for sensitive exports

**Recommendation:** Rotate after every install and restrict access to `%ProgramData%\WireSentinel\`.

### T3: VPN credential theft

**Threat:** Attacker extracts WireGuard private keys from disk.

**Mitigations:**
- Config blobs encrypted with DPAPI before persistence
- Service runs as LocalSystem — files inherit restrictive ACLs
- Materialized configs in `tunnels/` are transient

**Residual risk:** LocalSystem compromise exposes all secrets.

### T4: DNS leak

**Threat:** Applications bypass WireSentinel DNS and leak queries.

**Mitigations:**
- DNS layer intercepts and logs queries
- Leak detector records incidents (`/api/v1/leaks`)
- Kill switch blocks traffic on VPN drop
- Privacy score includes DNS encryption and route leakage metrics

### T5: Firewall bypass

**Threat:** Application circumvents WFP rules.

**Mitigations:**
- Userspace WFP engine (MVP) with kernel driver planned
- Policy engine evaluates all matched flows
- Split tunnel and route statistics for anomaly detection

**Residual risk:** Kernel-level malware or raw socket usage by privileged processes may bypass userspace filtering.

### T6: Supply chain / installer tampering

**Threat:** Modified installer or release artifact.

**Mitigations:**
- Release `manifest.json` includes SHA256 per artifact
- GitHub Release workflow publishes signed checksums
- Verify hashes before deployment

### T7: Denial of service

**Threat:** Flood API or exhaust resources.

**Mitigations:**
- REST rate limiter (governor crate)
- WebSocket connection cap
- Service recovery: automatic restart on failure (60 s delay)

## Out of scope (0.1.0)

- Multi-user token isolation (single machine token)
- Hardware security module (HSM) integration
- Remote administration over TLS (loopback only)
- Automatic security finding remediation (findings stored, manual review)

## Security findings storage

Phase 6 adds `security_findings` table for tracked vulnerabilities and misconfigurations. Findings include severity (`low`, `medium`, `high`, `critical`) and resolution status.

## Reporting vulnerabilities

Report security issues responsibly via the project repository's security advisory process. Do not disclose VPN keys, tokens, or customer logs in public issues.
