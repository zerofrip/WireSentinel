# WireSentinel Installer

Build artifacts and installer scripts for Windows deployment.

## Prerequisites

- WiX Toolset 4.x / 5.x (for MSI)
- NSIS 3.x (for EXE installer)
- Visual Studio 2022 + Windows Driver Kit (for kernel drivers)
- Sibling repositories (next to WireSentinel):
  - `WireSentinel-Kernel` — Guardian WFP driver
  - `WireSentinel-Ndis` — NDIS LWF filter
- Built binaries:
  - `target/release/wire-sentinel-service.exe`
  - `ui/src-tauri/target/release/wire-sentinel.exe`
- VPN resources:
  - `resources/tunnel.dll` (from wireguard-windows embeddable-dll-service)
  - `resources/wireguard.dll` (from wireguard-nt)

## Directory Layout

```
installer/
├── wix/
│   └── Product.wxs          # WiX MSI (optional KernelDriversFeature)
├── nsis/
│   └── installer.nsi        # NSIS EXE (optional SecKernelDrivers)
├── staging/
│   └── drivers/             # Built by scripts/build-drivers.ps1 (gitignored)
└── tests/
    ├── validate.ps1
    └── installer-e2e.ps1
scripts/
├── build-installer.ps1      # Unified MSI + NSIS build
├── build-drivers.ps1        # Guardian + NDIS msbuild + staging
├── sign-drivers.ps1         # inf2cat + test signtool (self-signed PFX)
├── install-kernel-drivers.ps1
└── uninstall-kernel-drivers.ps1
.cache/
└── test-signing/            # Auto-generated test PFX (gitignored)
```

## Build (Windows)

```powershell
# Full installer build (app + drivers + MSI + NSIS)
.\scripts\fetch-vpn-resources.ps1 -Arch x64
.\scripts\build-installer.ps1

# ARM64
.\scripts\build-installer.ps1 -Arch arm64

# Skip test signing (unsigned placeholder .cat — not loadable)
.\scripts\build-installer.ps1 -SkipDriverSign

# Package only (binaries + staged drivers must already exist)
.\scripts\build-installer.ps1 -SkipBuild
```

## Kernel Drivers (optional feature)

Both MSI and NSIS expose an **optional** kernel driver feature (default **on**, recommended):

| Driver | Service | Purpose |
|--------|---------|---------|
| Guardian (`Guardian.sys`) | `WireSentinelGuardian` | WFP kernel enforcement |
| NDIS LWF (`guardian_lwf.sys`) | `WireSentinelNdis` | NDIS lightweight filter |

When selected, the installer:

1. Copies driver payload to `%ProgramFiles%\WireSentinel\drivers\`
2. Runs `scripts/install-kernel-drivers.ps1` (`pnputil`, `sc`, `netcfg`)

Build scripts test-sign `.sys` / `.cat` with a self-signed certificate (`.cache/test-signing/wiresentinel-test.pfx`). **Target PCs must enable testsigning:**

```powershell
bcdedit /set testsigning on
```

Reboot after enabling. WHQL / EV signing is not implemented.

Set `wfp_engine_impl=kernel` in WireSentinel settings to use kernel enforcement.

## Service Registration

The installer registers `WireSentinel` Windows Service:

```
sc.exe create WireSentinel binPath= "\"C:\Program Files\WireSentinel\wire-sentinel-service.exe\""
sc.exe start WireSentinel
```

## Uninstall

MSI and NSIS stop the WireSentinel service, remove kernel drivers (if installed), delete the loopback firewall rule, and remove program files. User data in `%ProgramData%\WireSentinel\` is retained.

See [docs/installer-guide.md](../docs/installer-guide.md) for validation, CI, and release packaging.
