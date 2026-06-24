# Signed Driver Stack Feasibility Report

Verification date: 2026-06-24  
Scope: Full parity with Guardian.sys + NDIS LWF using WireGuard NT, Wintun (indirect), WinDivert, and sing-box.

## Executive summary

| Category | Verdict |
|----------|---------|
| Core enforcement (firewall, split tunnel, kill switch, VPN, DNS) | **Achievable** — userspace WFP + WireGuard NT already implemented |
| Packet redirect | **Achievable** — WinDivert NETWORK/FLOW divert (`windivert-engine`) |
| DPI / transform | **Achievable** — `dpi-transforms` + WinDivert modify + sing-box outbound |
| Cover traffic | **Achievable** — `CoverTrafficService` + WinDivert inject |
| Kernel per-CPU telemetry | **Approximate only** — ETW + WinDivert counters (not identical to Guardian ring buffers) |
| Hybrid WFP+NDIS coordination | **Achievable** — `SignedEnforcementCoordinator` (WFP policy + WinDivert datapath) |

**Gate result:** Redirect, transform, and cover traffic are implementable via WinDivert/sing-box. Default may switch to signed stack.

## Feature matrix

| Feature | Guardian.sys | NDIS LWF | Signed replacement | Status |
|---------|:------------:|:--------:|-------------------|--------|
| Per-app connection filter | Yes | — | Userspace WFP `ALE_APP_ID` | Existing |
| Split tunnel routing | Yes | Yes | WFP + sing-box TUN `strict_route` | PoC in `transport-engine` |
| Kill switch | Yes | — | Userspace WFP | Existing |
| Packet redirect | Permit | Yes | WinDivert divert | `windivert-engine` |
| DPI / LWO transform | hooks | Yes | dpi-transforms + WinDivert | `windivert-engine` |
| Cover traffic | — | Yes | CoverTrafficService + WinDivert | Integrated |
| Kernel telemetry | Yes | Yes | Approximate counters | Documented limitation |
| VPN tunnel | — | — | wireguard.dll (Wintun below) | Existing |
| Transport obfuscation | — | — | sing-box subprocess | Existing |

## PoC references

| PoC | Location | Notes |
|-----|----------|-------|
| WFP baseline | `core-service/tests/split_tunnel.rs` | Per-app routes via userspace WFP |
| WinDivert engine | `windivert-engine/` | DLL dynamic load, redirect/transform/telemetry |
| sing-box TUN | `transport-engine/src/singbox/config.rs` | `build_tun_config()` with strict_route |
| Enforcement mapping | `core-service/src/enforcement.rs` | signed ↔ guardian_mode/wfp_engine_impl |
| Backend tests | `core-service/tests/enforcement_backend.rs` | Mapping unit tests |

## NDIS IOCTL parity (signed stack)

| NDIS capability | Signed equivalent |
|-----------------|-------------------|
| `set_route` / `clear_route` | WinDivert flow tracking + WFP permit |
| redirect to SOCKS port | WinDivert reinject toward loopback proxy |
| transform profile IOCTL | WinDivert packet modify callback |
| cover traffic IOCTL | WinDivert inject + CoverTrafficService timing |
| telemetry summary | `windivert-engine::telemetry` counters |

## Switching backends

- Setting `enforcement_backend=signed` maps to `guardian_mode=wfp`, `wfp_engine_impl=userspace`, WinDivert NDIS shim.
- Setting `enforcement_backend=custom_kernel` maps to `guardian_mode=hybrid`, `wfp_engine_impl=kernel`.
- Service restart required after change.

## Known limitations

1. Kernel per-CPU classify latency histograms are **approximate** in signed mode.
2. sing-box must remain a **subprocess** (GPLv3).
3. WinDivert requires **administrator** privileges for driver install on first use.
4. WinDivert TLS spoof features are x64-only (ARM64 uses WFP-only signed path).
