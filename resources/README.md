# Runtime Binaries (Signed Enforcement Stack)

WireSentinel bundles or invokes these Windows binaries. Place files under `resources/` before building the installer.

## VPN (WireGuard NT)

| File | Source |
|------|--------|
| `tunnel.dll` | [wireguard-windows/embeddable-dll-service](https://github.com/WireGuard/wireguard-windows/tree/master/embeddable-dll-service) |
| `wireguard.dll` | [wireguard-nt](https://github.com/WireGuard/wireguard-nt) |

Wintun is used **indirectly** via WireGuard NT. Do not ship a standalone `wintun.dll` unless you comply with the [Wintun prebuilt license](https://www.wintun.net/).

## WinDivert (signed stack)

| File | Source |
|------|--------|
| `WinDivert.dll` | [WinDivert release](https://reqrypt.org/windivert.html) (x64) |
| `WinDivert64.sys` | Same package (Microsoft-signed driver) |

License: **LGPLv3** (see `docs/third-party-licenses.md`).

```powershell
# Example: copy from WinDivert install or build output
copy WinDivert-x.x.x-A\x64\WinDivert.dll resources\
copy WinDivert-x.x.x-A\x64\WinDivert64.sys resources\
```

## sing-box (transport / TUN)

| File | Source |
|------|--------|
| `sing-box.exe` | [sing-box releases](https://github.com/SagerNet/sing-box/releases) (windows-amd64) |

License: **GPLv3** — used only as a **subprocess**, never linked into WireSentinel.

```powershell
copy sing-box-*.exe resources\sing-box.exe
```

## Tor (anonymity transport)

| File | Source |
|------|--------|
| `tor.exe` | [Tor Expert Bundle](https://www.torproject.org/download/tor/) (windows-x86_64) |

License: **BSD 3-Clause** — used only as a **subprocess** (spawned by sing-box `tor` outbound), never linked into WireSentinel.

```powershell
# Extract tor.exe from the Expert Bundle archive
copy tor-win64-0.4.8.14\tor\tor.exe resources\tor.exe
```

Record the exact version and SHA256 of the binary you ship in release notes.

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
