# WSL Portable Backend Debug

Cross-build `wire-sentinel-service.exe` on WSL and stage a portable bundle under `C:\dev\WireSentinel-portable\` for Windows console-mode testing and debugging.

This workflow does **not** include the Tauri UI or Windows service registration. Use the MSI/NSIS installer for full product deployment ([installer-guide.md](installer-guide.md)).

## Prerequisites (WSL)

Same toolchain as the Linux `windows-cross` CI job ([`.github/workflows/ci.yml`](../.github/workflows/ci.yml)):

```bash
sudo apt-get update
sudo apt-get install -y clang lld llvm
rustup target add x86_64-pc-windows-msvc
rustup component add llvm-tools
cargo install cargo-xwin --locked
export XWIN_ACCEPT_LICENSE=1
```

Runtime binaries (`tunnel.dll`, `wireguard.dll`, `sing-box.exe`, etc.) are copied from [`resources/`](../resources/). If files are missing, run [`scripts/fetch-vpn-resources.ps1`](../scripts/fetch-vpn-resources.ps1) on Windows.

## Build and stage

From the repository root in WSL:

```bash
./scripts/stage-windows-portable.sh
```

| Option | Effect |
|--------|--------|
| `--release` | Release build (default) |
| `--debug` | Debug build; copies `.pdb` when present |
| `--skip-build` | Re-stage only; reuse existing `target/` artifacts |
| `--out PATH` | Override output directory |

Default output: `/mnt/c/dev/WireSentinel-portable/` → `C:\dev\WireSentinel-portable\` on Windows.

Environment override: `WIRESENTINEL_PORTABLE_DIR=/mnt/c/other/path`.

## Run on Windows

Open **Administrator PowerShell** (WFP requires elevation):

```powershell
cd C:\dev\WireSentinel-portable
.\wire-sentinel-service.exe --console
```

Verify the API:

```powershell
Invoke-WebRequest http://127.0.0.1:8170/api/v1/auth/token
```

Service data (settings, token file, logs) still uses `%ProgramData%\WireSentinel\` — see [administrator-guide.md](administrator-guide.md).

## Debug from Cursor / Visual Studio (Windows)

1. Stage with `./scripts/stage-windows-portable.sh --debug` for symbols.
2. On Windows, open the repo or `C:\dev\WireSentinel-portable\`.
3. Configure a launch target:
   - **Program:** `C:\dev\WireSentinel-portable\wire-sentinel-service.exe`
   - **Args:** `--console`
   - **Working directory:** `C:\dev\WireSentinel-portable`

Use CodeLLDB or the MSVC debugger with the staged `.pdb` in debug builds.

## Limitations

| Included | Not included |
|----------|----------------|
| `wire-sentinel-service.exe` | `wire-sentinel.exe` (Tauri UI) |
| VPN/transport DLLs and helpers | MSI / NSIS installers |
| x64 cross-build from WSL | arm64 portable (future) |
| Console mode | SCM service install |

For UI development, build on Windows: `cd ui && npm run tauri dev`.

## Related scripts

- [`scripts/build-windows-cross.sh`](../scripts/build-windows-cross.sh) — low-level `cargo-xwin` wrapper
- [`scripts/stage-windows-portable.sh`](../scripts/stage-windows-portable.sh) — build + copy to portable folder
