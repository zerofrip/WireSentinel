# Platform Support

WireSentinel targets modern Windows desktop and server platforms with x64 and ARM64 builds.

## Supported operating systems

| OS | Version | Status |
|----|---------|--------|
| Windows 11 | 22H2+ | Supported |
| Windows 10 | 1903+ (build 18362+) | Supported |
| Windows Server | 2019, 2022 | Supported |
| Windows on ARM | Snapdragon / ARM64 | Supported (arm64 build) |

## Architectures

| Architecture | Rust target | Installer suffix |
|--------------|-------------|------------------|
| x64 (AMD64) | native / `x86_64-pc-windows-msvc` | `-x64` |
| ARM64 | `aarch64-pc-windows-msvc` | `-arm64` |

Build for a specific architecture:

```powershell
.\scripts\build-installer.ps1 -Arch x64
.\scripts\build-installer.ps1 -Arch arm64
```

## Runtime dependencies

| Dependency | Notes |
|------------|-------|
| Windows Filtering Platform (WFP) | Built into Windows |
| WireGuard NT (`wireguard.dll`) | Bundled in installer |
| Embeddable tunnel service (`tunnel.dll`) | Bundled in installer |
| SQLite | Embedded via `sqlx` (no separate install) |
| WebView2 | Required for Tauri UI (preinstalled on Windows 11) |

### WebView2

Windows 11 includes WebView2 Runtime. On Windows 10, install the [Evergreen WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) if the UI fails to launch.

## Feature availability by platform

| Feature | x64 | ARM64 | Notes |
|---------|-----|-------|-------|
| WFP userspace engine | Yes | Yes | Kernel driver planned |
| WireGuard VPN | Yes | Yes | Requires matching DLL architecture |
| Traffic monitor | Yes | Yes | ETW-based on Windows |
| DNS (DoH/DoT) | Yes | Yes | |
| Tauri UI | Yes | Yes | |
| Windows Service | Yes | Yes | LocalSystem |
| DPAPI encryption | Yes | Yes | Machine scope |

## Unsupported platforms

- **Linux / macOS** — service and WFP integration are Windows-specific; UI may compile but is not supported
- **Windows 8.1 and earlier** — not tested
- **32-bit (x86) Windows** — not supported

## Cross-compilation (CI)

Linux CI validates Windows targets without running binaries:

```bash
cargo check --workspace --target x86_64-pc-windows-msvc
cargo check --workspace --target aarch64-pc-windows-msvc
```

Full installer builds require `windows-latest` GitHub Actions runners.

For a WSL → Windows portable backend bundle (console mode, no UI), see [wsl-portable-debug.md](wsl-portable-debug.md).

## VPN backend notes

| Backend | Source | Architecture |
|---------|--------|--------------|
| WireGuard NT | wireguard-nt | Must match host arch |
| AmneziaWG | amneziawg-windows | Separate tunnel.dll per backend |

Do not mix WireGuard NT and AmneziaWG `tunnel.dll` in the same install directory.

## Minimum hardware

| Resource | Minimum | Recommended |
|----------|---------|-------------|
| RAM | 4 GB | 8 GB+ |
| Disk | 500 MB free | 2 GB+ (logs, cache) |
| CPU | 2 cores | 4+ cores |

## Localization

UI strings are English-only in 0.1.0. Service logs and API responses use English.

## Future platform work

- Kernel-mode WFP driver (Phase 2)
- MSI per-user install option
- WebView2 offline bootstrapper in NSIS
