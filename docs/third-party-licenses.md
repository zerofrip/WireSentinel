# Third-Party Licenses

WireSentinel (Apache-2.0) bundles or invokes several third-party components.
This document summarizes license obligations for the **signed enforcement stack**.

Installer bundles include:

| Path | Content |
|------|---------|
| `THIRD_PARTY_NOTICES.txt` | Attribution summary (install root) |
| `licenses/LGPL-3.0.txt` | GNU LGPL v3 full text (WinDivert) |
| `licenses/GPL-3.0.txt` | GNU GPL v3 full text (sing-box) |
| `licenses/MIT.txt` | MIT full text (WireGuard) |

Pinned third-party versions: [`installer/third-party-versions.json`](../installer/third-party-versions.json).

## WireSentinel ecosystem (Apache-2.0)

These sibling repositories are path-linked at build time. Each ships its own `LICENSE` file:

WireSentinel-Mixnet, WireSentinel-Anonymity, WireSentinel-ZTNA, WireSentinel-SSE,
WireSentinel-XDR, WireSentinel-Kernel, WireSentinel-Ndis, WireSentinel-Cloud,
WireSentinel-Controller, WireSentinel-CNAPP, WireSentinel-AI, WireSentinel-Plugin-Sdk.

See [LICENSE-AUDIT.md](LICENSE-AUDIT.md) for the full audit matrix.

## WireSentinel

- **License:** Apache License 2.0
- **Source:** This repository

## WireGuard NT (`wireguard.dll`, `tunnel.dll`)

- **License:** MIT License
- **Upstream:** [wireguard-nt](https://github.com/WireGuard/wireguard-nt), [wireguard-windows](https://github.com/WireGuard/wireguard-windows)
- **Distribution:** Bundled in installer `resources/`
- **Obligation:** Include copyright and permission notice (`licenses/MIT.txt`, `THIRD_PARTY_NOTICES.txt`)

## Wintun (via WireGuard NT)

- **License:** Prebuilt `wintun.dll` — WireGuard LLC prebuilt binaries license; source — GPLv2
- **Upstream:** [wintun](https://www.wintun.net/)
- **Distribution:** WireSentinel uses Wintun **indirectly** through `wireguard.dll` / WireGuard NT. We do **not** ship a standalone `wintun.dll`.
- **Obligation:** If direct Wintun API use is added later, comply with prebuilt license (API-only bundling, no reverse engineering).

## WinDivert (`WinDivert.dll`, `WinDivert64.sys`)

- **License:** **GNU LGPL v3** (selected alternative to GPLv2)
- **Version:** See `installer/third-party-versions.json` (default 2.2.2-A)
- **Upstream:** [WinDivert](https://reqrypt.org/windivert.html)
- **Distribution:** Dynamic link to `WinDivert.dll`; kernel driver `WinDivert64.sys` (Microsoft-signed) bundled alongside
- **Obligation:**
  - Include LGPL v3 license text (`licenses/LGPL-3.0.txt`)
  - Provide notice that WinDivert is used
  - Users may replace `WinDivert.dll` with a modified version (LGPL)
  - Do **not** statically link WinDivert source into WireSentinel binaries

## sing-box (`sing-box.exe`)

- **License:** **GNU GPL v3** with additional terms (no derivative may use the name or imply association without prior consent)
- **Version:** See `installer/third-party-versions.json` (default 1.11.8)
- **Upstream:** [sing-box](https://github.com/SagerNet/sing-box)
- **Source (exact version):** `https://github.com/SagerNet/sing-box/archive/refs/tags/v{version}.tar.gz`
- **Distribution:** **Separate subprocess only** — never linked into `wire-sentinel-service.exe`
- **Obligation:**
  - Include GPL v3 license text (`licenses/GPL-3.0.txt`)
  - Offer corresponding source for the exact sing-box version shipped (tarball URL in `THIRD_PARTY_NOTICES.txt`)
  - Do not imply sing-box project endorsement of WireSentinel
  - WireSentinel communicates via JSON config and process IPC only

## Tor (`tor.exe`)

- **License:** **BSD 3-Clause**
- **Version:** See `installer/third-party-versions.json` (default 0.4.8.14)
- **Upstream:** [The Tor Project](https://www.torproject.org/)
- **Expert bundle:** https://www.torproject.org/download/tor/
- **Distribution:** **Separate subprocess only** — spawned by sing-box `tor` outbound, never linked into WireSentinel binaries
- **Obligation:**
  - Include BSD license text (`licenses/BSD-3-Clause-Tor.txt`)
  - Do not imply Tor Project endorsement of WireSentinel

## Custom kernel drivers (optional, `custom_kernel` mode)

- **Guardian.sys** / **guardian_lwf.sys** — WireSentinel project drivers (Apache-2.0 source); test-signed for development
- Not part of the default signed stack

## GPL / LGPL contamination policy

| Rule | Detail |
|------|--------|
| No GPL Rust crates in core | `core-service`, `wfp`, `storage` must not depend on GPL libraries |
| sing-box | Subprocess only |
| WinDivert | DLL dynamic load via `windivert-engine` FFI wrapper; no WinDivert source in tree |
| Attribution | `THIRD_PARTY_NOTICES.txt` + `licenses/*.txt` shipped with every release |
| UI npm deps | `ui/src/generated/npm-licenses.json`; Legal page in app |

## Building third-party binaries

See [resources/README.md](../resources/README.md) for acquisition and build instructions.
