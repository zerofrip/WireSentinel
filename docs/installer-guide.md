# Installer Guide

WireSentinel ships Windows installers built with WiX (MSI) and NSIS (EXE). Release packaging adds ZIP archives and SHA256 manifests.

## Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| Rust stable | 1.75+ | Service binary |
| Node.js | 20+ | UI build |
| WiX Toolset | 4.x / 5.x | MSI |
| NSIS | 3.x | EXE setup |
| WireGuard DLLs | — | See [resources/README.md](../resources/README.md) |

## Quick build

```powershell
# Full build: Rust + Tauri + MSI + NSIS (x64)
.\scripts\build-installer.ps1

# ARM64
.\scripts\build-installer.ps1 -Arch arm64

# Package only (binaries already built)
.\scripts\build-installer.ps1 -SkipBuild

# MSI or NSIS only
.\scripts\build-installer.ps1 -MsiOnly
.\scripts\build-installer.ps1 -NsisOnly
```

## Release packaging

```powershell
# ZIP + MSI + NSIS + manifest.json with SHA256 checksums
.\scripts\release-builder.ps1

# ARM64 release
.\scripts\release-builder.ps1 -Arch arm64
```

Output layout:

```
dist/
├── WireSentinel-0.1.0-x64.msi
├── WireSentinel-0.1.0-x64-setup.exe
└── release/
    └── x64/
        ├── WireSentinel-0.1.0-x64.msi
        ├── WireSentinel-0.1.0-x64-setup.exe
        ├── WireSentinel-0.1.0-x64.zip
        └── manifest.json
```

### manifest.json format

```json
{
  "version": "0.1.0",
  "channel": "stable",
  "build_date": "2026-06-21T12:00:00.0000000Z",
  "arch": "x64",
  "artifacts": [
    {
      "path": "WireSentinel-0.1.0-x64.msi",
      "sha256": "<hex>",
      "size_bytes": 12345678
    }
  ]
}
```

Matches `ReleaseManifest` in `shared-types/src/phase6.rs`.

## Version management

Version is defined in:

- `version.json` (repo root) — used by build scripts
- `Cargo.toml` workspace version
- `ui/src-tauri/tauri.conf.json`
- `installer/wix/Product.wxs` Package Version attribute

Bump all locations together when releasing.

## What installers do

### Files installed

| File | Destination |
|------|-------------|
| `wire-sentinel-service.exe` | `C:\Program Files\WireSentinel\` |
| `wire-sentinel.exe` | `C:\Program Files\WireSentinel\` |
| `tunnel.dll` | `C:\Program Files\WireSentinel\` |
| `wireguard.dll` | `C:\Program Files\WireSentinel\` |

### Directories created

| Path | Purpose |
|------|---------|
| `%ProgramData%\WireSentinel\` | Data root |
| `%ProgramData%\WireSentinel\logs\` | Service logs |
| `%ProgramData%\WireSentinel\tunnels\` | VPN configs |
| `%ProgramData%\WireSentinel\transports\` | Transport configs |

### Service registration

- Name: `WireSentinel`
- Start: Manual (demand)
- Account: LocalSystem
- Dependencies: Tcpip, Dnscache
- Recovery: Restart × 3 with 60 s delay

### Firewall rule

Both MSI and NSIS add:

```
WireSentinel API (loopback) — TCP 8170 inbound, remote 127.0.0.1
```

MSI includes a **rollback custom action** (`CA_RollbackFirewallRule`) that removes the rule if installation fails after the forward action runs.

### NSIS silent install

```powershell
.\WireSentinel-0.1.0-x64-setup.exe /S
```

## Validation

Static validation (no install required):

```powershell
.\installer\tests\validate.ps1
.\installer\tests\installer-e2e.ps1   # Pester tests
```

CI uses `-SkipFileRefs` when build artifacts are absent.

## CI/CD

- **CI** (`.github/workflows/ci.yml`): fmt, clippy, test, audit, npm build, validate.ps1, Windows cross-compile checks
- **Release** (`.github/workflows/release.yml`): triggered on `v*` tags, builds x64 + arm64, publishes GitHub Release with artifacts

## Manual WiX / NSIS

```powershell
wix build -ext WixToolset.Util.wixext installer/wix/Product.wxs -o dist/WireSentinel-0.1.0.msi
makensis /DWIRESENTINEL_VERSION=0.1.0 /DWIRESENTINEL_ARCH=x64 installer/nsis/installer.nsi
```

## Upgrade behavior

WiX `MajorUpgrade` replaces previous versions with the same UpgradeCode. Downgrade attempts show an error message.

## Uninstall

- MSI: Add/Remove Programs or `msiexec /x {ProductCode}`
- NSIS: `Uninstall.exe` in install directory or Add/Remove Programs

Both stop the service, remove the firewall rule, and delete program files. User data in `%ProgramData%\WireSentinel\` is retained.
