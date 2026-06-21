# WireSentinel Installer

Build artifacts and installer scripts for Windows deployment.

## Prerequisites

- WiX Toolset 4.x (for MSI)
- NSIS 3.x (for EXE installer)
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
│   └── Product.wxs          # WiX MSI skeleton
├── nsis/
│   └── installer.nsi        # NSIS EXE skeleton
└── README.md
resources/
├── README.md                # How to obtain tunnel.dll / wireguard.dll
└── .gitkeep
```

## Build (Windows)

```powershell
# 1. Build Rust service
cargo build -p core-service --release

# 2. Build Tauri UI
cd ui && npm install && npm run tauri build

# 3. Copy VPN DLLs into resources/
# See resources/README.md

# 4. Build MSI (WiX)
wix build installer/wix/Product.wxs -o dist/WireSentinel-0.1.0.msi

# 5. Build EXE (NSIS)
makensis installer/nsis/installer.nsi
```

## Guardian Kernel Driver (Phase 8)

Build [WireSentinel-Kernel](https://github.com/WireSentinel/WireSentinel-Kernel) and copy `Guardian.sys` + `guardian.inf` into the installer payload. Register with:

```powershell
pnputil /add-driver guardian.inf /install
sc.exe start WireSentinelGuardian
```

Set `wfp_engine_impl=kernel` in WireSentinel settings to use kernel enforcement.

## Service Registration

The installer registers `WireSentinel` Windows Service:

```
sc.exe create WireSentinel binPath= "\"C:\Program Files\WireSentinel\wire-sentinel-service.exe\""
sc.exe start WireSentinel
```

## Uninstall

```
sc.exe stop WireSentinel
sc.exe delete WireSentinel
```
