# Runtime Binaries (Signed Enforcement Stack)

WireSentinel bundles or invokes these Windows binaries under `resources/` before building the installer.

## CI / release (recommended)

`release.yml` and local release builds run:

```powershell
.\scripts\fetch-vpn-resources.ps1 -Arch x64   # or arm64
```

This downloads pinned versions from [`installer/third-party-versions.json`](../installer/third-party-versions.json) into `resources/`. Manual copying below is only needed for offline or custom builds.

| Architecture | Bundled in `resources/` |
|--------------|-------------------------|
| **x64** | `tunnel.dll`, `wireguard.dll`, `WinDivert.dll`, `WinDivert64.sys`, `sing-box.exe`, `tor.exe` |
| **arm64** | `tunnel.dll`, `wireguard.dll`, `sing-box.exe` (native arm64), `tor.exe` (x86_64 expert bundle; WoA x64 emulation). **No WinDivert** — official signed ARM64 driver unavailable; signed stack uses WFP-only on arm64. |

## VPN (WireGuard NT)

| File | Source |
|------|--------|
| `tunnel.dll` | [wireguard-windows/embeddable-dll-service](https://github.com/WireGuard/wireguard-windows/tree/master/embeddable-dll-service) |
| `wireguard.dll` | [wireguard-nt](https://github.com/WireGuard/wireguard-nt) |

Wintun is used **indirectly** via WireGuard NT. Do not ship a standalone `wintun.dll` unless you comply with the [Wintun prebuilt license](https://www.wintun.net/).

## WinDivert (signed stack, x64 only)

| File | Source |
|------|--------|
| `WinDivert.dll` | [WinDivert release](https://reqrypt.org/windivert.html) (x64) |
| `WinDivert64.sys` | Same package (Microsoft-signed driver) |

License: **LGPLv3** (see `docs/third-party-licenses.md`).

```powershell
# Manual fallback (x64 only)
copy WinDivert-x.x.x-A\x64\WinDivert.dll resources\
copy WinDivert-x.x.x-A\x64\WinDivert64.sys resources\
```

## sing-box (transport / TUN)

| File | Source |
|------|--------|
| `sing-box.exe` | [sing-box releases](https://github.com/SagerNet/sing-box/releases) (`windows-amd64` or `windows-arm64`) |

License: **GPLv3** — used only as a **subprocess**, never linked into WireSentinel.

```powershell
# Manual fallback
copy sing-box-*.exe resources\sing-box.exe
```

## Tor (anonymity transport)

| File | Source |
|------|--------|
| `tor.exe` | [Tor Expert Bundle](https://www.torproject.org/download/tor/) (`windows-x86_64`; same binary on arm64 via WoA emulation) |

License: **BSD 3-Clause** — used only as a **subprocess** (spawned by sing-box `tor` outbound), never linked into WireSentinel.

```powershell
# Manual fallback: extract tor.exe from the expert bundle .tar.gz
tar -xzf tor-expert-bundle-windows-x86_64-*.tar.gz tor/tor.exe
copy tor\tor.exe resources\tor.exe
```

Pinned versions and SHA256 are in `installer/third-party-versions.json`.

## Build WireGuard (Windows)

```powershell
cd wireguard-nt
msbuild wireguard-nt.props /p:Configuration=Release

cd wireguard-windows/embeddable-dll-service
.\build.bat

copy tunnel.dll ..\..\WireSentinel\resources\
copy wireguard.dll ..\..\WireSentinel\resources\
```

## AmneziaWG

For AmneziaWG profiles, use a separate `tunnel.dll` built from amneziawg-windows.
Do not mix with wireguard-NT tunnel.dll in the same process.

## Licenses

See [docs/third-party-licenses.md](../docs/third-party-licenses.md), [docs/LICENSE-AUDIT.md](../docs/LICENSE-AUDIT.md), and [installer/THIRD_PARTY_NOTICES.txt](../installer/THIRD_PARTY_NOTICES.txt) (full GPL/LGPL texts in `installer/licenses/`).
