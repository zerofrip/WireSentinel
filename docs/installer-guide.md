# Installer Guide

WireSentinel ships Windows installers built with WiX (MSI) and NSIS (EXE). Release packaging adds ZIP archives and SHA256 manifests.

## Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| Rust stable | 1.75+ | Service binary |
| Node.js | 20+ | UI build |
| WiX Toolset | 4.x / 5.x | MSI |
| NSIS | 3.x | EXE setup |
| Visual Studio 2022 + WDK | — | Kernel drivers (Guardian + NDIS) |
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

# Skip driver test signing (unsigned placeholder .cat — not loadable)
.\scripts\build-installer.ps1 -SkipDriverSign

# Use pre-staged drivers (skip msbuild)
.\scripts\build-installer.ps1 -SkipDriverBuild
```

### Kernel driver build (standalone)

```powershell
# Build + stage to installer/staging/drivers/{arch}/
.\scripts\build-drivers.ps1 -Arch x64

# Test-sign staged drivers (auto-creates .cache/test-signing/wiresentinel-test.pfx)
.\scripts\sign-drivers.ps1 -Arch x64
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

Matches `ReleaseManifest` in `shared-types/src/validation_release.rs`.

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
| `drivers\guardian\*` | Optional — Guardian WFP driver payload |
| `drivers\ndis\*` | Optional — NDIS LWF driver payload |
| `scripts\install-kernel-drivers.ps1` | Driver install helper (MSI/NSIS) |

### Optional kernel drivers

Both installers expose an optional feature (default **on**):

| Installer | Feature / section | Level |
|-----------|-------------------|-------|
| MSI (WiX) | `KernelDriversFeature` | 2 (optional, recommended) |
| NSIS | `SecKernelDrivers` | Optional, pre-selected via `SelectSection` |

When selected, the installer registers:

- **Guardian** — `pnputil /add-driver guardian.inf /install`, then `sc start WireSentinelGuardian`
- **NDIS LWF** — `pnputil /add-driver guardian_lwf.inf /install`, then `netcfg -v -l guardian_lwf.inf -c s -i WireSentinelNdis`

NDIS binds to all adapters; uninstall runs `netcfg -u` to remove the binding.

**Signing:** Build scripts test-sign `.sys` / `.cat` with a self-signed code-signing certificate cached at `.cache/test-signing/wiresentinel-test.pfx`. No GitHub Secrets are required. WHQL is not implemented.

**Target PC requirement:** enable Windows test-signing mode before installing kernel drivers:

```powershell
bcdedit /set testsigning on
# Reboot required
```

Optional: import the test root certificate to `Cert:\LocalMachine\Root` if you copy the PFX from the build machine. Release artifacts are intended for development and validation, not production deployment without EV/WHQL signing.

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

Omit kernel drivers in silent mode:

```powershell
.\WireSentinel-0.1.0-x64-setup.exe /S /COMPONENTS="SecMain"
```

## Validation

Static validation (no install required):

```powershell
.\installer\tests\validate.ps1
.\installer\tests\installer-e2e.ps1   # Pester tests
```

CI uses `-SkipFileRefs -SkipDriverRefs` when build artifacts and staged drivers are absent.

## CI/CD

- **CI** ([`.github/workflows/ci.yml`](../.github/workflows/ci.yml)): fmt, clippy, test, audit, npm build, validate.ps1, Windows cross-compile checks
- **Release** ([`.github/workflows/release.yml`](../.github/workflows/release.yml)): builds x64 + arm64 installers and publishes a GitHub Release

### GitHub Actions release

Release workflow triggers:

1. **Tag push** — push a `v*` tag (for example `v0.1.0`)
2. **Manual** — Actions → **Release** → **Run workflow**, optional `ref` (default `main`)

Before releasing, bump version consistently in:

- `version.json` (build scripts and manifest)
- `Cargo.toml` workspace version
- `ui/package.json` and `ui/src-tauri/Cargo.toml`
- `ui/src-tauri/tauri.conf.json`
- `installer/wix/Product.wxs`
- NSIS default version (overridden at build time via `/DWIRESENTINEL_VERSION=`)

**Tag release example:**

```bash
git tag v0.1.0
git push origin v0.1.0
```

**Manual release example:**

1. Ensure `version.json` matches the version you want to ship
2. Run the **Release** workflow on `main` (or another `ref`)
3. The workflow creates tag `v{version}` from `version.json` and publishes the release

Manual runs fail if that tag already exists (prevents duplicate releases).

### Release artifacts

Each architecture (`x64`, `arm64`) produces:

| File | Description |
|------|-------------|
| `WireSentinel-{version}-{arch}.msi` | WiX per-machine installer |
| `WireSentinel-{version}-{arch}-setup.exe` | NSIS installer |
| `WireSentinel-{version}-{arch}.zip` | Portable bundle (exes + DLLs + `drivers/` + `version.json`) |
| `manifest.json` | SHA256 checksums for installers, ZIP, and individual driver files |

VPN DLLs are **not** committed to git. CI fetches them via [`scripts/fetch-vpn-resources.ps1`](../scripts/fetch-vpn-resources.ps1):

- `wireguard.dll` from [WireGuardNT SDK](https://download.wireguard.com/wireguard-nt/)
- `tunnel.dll` built from [wireguard-windows embeddable-dll-service](https://github.com/WireGuard/wireguard-windows/tree/master/embeddable-dll-service)

Kernel drivers are built from sibling repos (`WireSentinel-Kernel`, `WireSentinel-Ndis`) via [`scripts/build-drivers.ps1`](../scripts/build-drivers.ps1). WDK is restored from NuGet ([`installer/wdk/packages.config`](../installer/wdk/packages.config)) per [Microsoft's CI guidance](https://learn.microsoft.com/en-us/windows-hardware/drivers/install-the-wdk-using-nuget). Drivers are test-signed via [`scripts/sign-drivers.ps1`](../scripts/sign-drivers.ps1).

Local release (Windows, sibling repos checked out next to WireSentinel):

```powershell
.\scripts\fetch-vpn-resources.ps1 -Arch x64
.\scripts\release-builder.ps1 -Arch x64
```

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

Both stop the service, remove kernel drivers (if the optional feature was installed), remove the firewall rule, and delete program files. User data in `%ProgramData%\WireSentinel\` is retained.
