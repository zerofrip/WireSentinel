# VPN Backend Binaries

WireSentinel MVP uses the embeddable tunnel service from WireGuard for Windows.

## Required files

| File | Source |
|------|--------|
| `tunnel.dll` | Build from [wireguard-windows/embeddable-dll-service](https://github.com/WireGuard/wireguard-windows/tree/master/embeddable-dll-service) |
| `wireguard.dll` | Build from [wireguard-nt](https://github.com/WireGuard/wireguard-nt) or copy from wireguard-windows release |

## Build (Windows)

```powershell
# wireguard-nt
cd wireguard-nt
msbuild wireguard-nt.props /p:Configuration=Release

# wireguard-windows embeddable DLL
cd wireguard-windows/embeddable-dll-service
.\build.bat

# Copy to WireSentinel
copy tunnel.dll ..\..\WireSentinel\resources\
copy wireguard.dll ..\..\WireSentinel\resources\
```

## AmneziaWG (Phase 2)

For AmneziaWG profiles, use a separate `tunnel.dll` built from amneziawg-windows.
Do not mix with wireguard-NT tunnel.dll in the same process.

## License

WireGuard components are licensed under the MIT License. See upstream repositories.
