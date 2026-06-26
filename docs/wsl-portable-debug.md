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

Startup log should include `traffic backend: packet` when `WinDivert.dll` is present. If the DLL is missing, the service falls back to `iphlpapi` polling automatically.

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
|----------|--------------|
| `wire-sentinel-service.exe` | `wire-sentinel.exe` (Tauri UI) |
| VPN/transport DLLs and helpers | MSI / NSIS installers |
| `WinDivert.dll` + `WinDivert64.sys` (traffic `packet` backend) | — |
| x64 cross-build from WSL | arm64 portable (future) |
| Console mode | SCM service install |

For UI development, build on Windows from a **native NTFS path** (not `\\wsl$\`). See [Frontend on Windows](#frontend-on-windows-with-wsl-backend) below.

## Frontend on Windows (with WSL backend)

Tauri (`npm run tauri dev`) must run on **Windows**, on a normal drive path such as `C:\dev\WireSentinel`. Do **not** run `npm install` from PowerShell against `\\wsl.localhost\...\WireSentinel\ui`.

### Why `EISDIR` on `node_modules\.bin\json5`?

WSL `npm install` creates Unix symlinks under `node_modules/.bin/`. Windows npm over `\\wsl$\` cannot `lstat` those entries correctly and fails with:

```text
EISDIR: illegal operation on a directory, lstat '...\node_modules\.bin\json5'
```

Mixing WSL and Windows npm on the same `node_modules` tree is unsupported.

### Recommended layout

| Component | Where |
|-----------|-------|
| Rust backend cross-build | WSL: `~/github/WireSentinel` → `stage-windows-portable.sh` |
| Tauri UI + `npm install` | Windows: `C:\dev\WireSentinel` (git clone or copy) |
| Running backend | `C:\dev\WireSentinel-portable\wire-sentinel-service.exe --console` |

### Setup (one-time)

**1. WSL — clean mixed `node_modules` if present:**

```bash
rm -rf ~/github/WireSentinel/ui/node_modules
```

**2. Copy repo to native Windows path** (pick one):

**Option A — WSL `rsync` (recommended; avoids Git `safe.directory` on UNC):**

```bash
mkdir -p /mnt/c/dev
rsync -a --delete \
  --exclude node_modules \
  --exclude target \
  --exclude ui/src-tauri/target \
  --exclude .git \
  ~/github/WireSentinel/ /mnt/c/dev/WireSentinel/
```

**Option B — clone from GitHub** (run in **Windows** PowerShell from `C:\dev`, not from `\\wsl$\`):

```powershell
cd C:\dev
git clone https://github.com/zerofrip/WireSentinel.git WireSentinel
```

**Option C — `robocopy`** (no `.git` history):

```powershell
robocopy \\wsl.localhost\Ubuntu\home\zero\github\WireSentinel C:\dev\WireSentinel /E /XD node_modules target ui\src-tauri\target .git
```

Do **not** use `git clone \\wsl.localhost\...` from Windows — Git 2.35+ reports `dubious ownership` on the WSL `.git` directory.

**3. Windows — install UI deps:**

Prerequisites for `npm run tauri dev` on Windows (in addition to Node.js):

1. **Rust (MSVC)** — [https://rustup.rs](https://rustup.rs) → run `rustup-init.exe`, default *Visual Studio* toolchain
2. **Visual Studio Build Tools 2022** — workload *Desktop development with C++* (MSVC linker for `cargo build`)
3. **WebView2** — included on Windows 11; install [Evergreen WebView2](https://developer.microsoft.com/microsoft-edge/webview2/) on Windows 10 if the window fails to open

After install, open a **new** PowerShell and verify:

```powershell
cargo --version
rustc --version
```

If `cargo` is not found, restart the terminal or log out/in so `%USERPROFILE%\.cargo\bin` is on `PATH`.

```powershell
cd C:\dev\WireSentinel\ui
npm install
```

If npm 11 blocks `esbuild` postinstall: `npm approve-scripts esbuild`

If Vite crashes with `EBUSY` on `src-tauri\target\...\build_script_build-*.exe`, ensure `ui/vite.config.ts` sets `server.watch.ignored: ["**/src-tauri/**"]` (already in repo) and re-copy or pull the latest `ui/vite.config.ts`.

**4. Windows — run UI** (backend portable or service must be up):

```powershell
cd C:\dev\WireSentinel\ui
npm run tauri dev
```

Optional: `RUST_LOG=debug npm run tauri dev`

### Vite-only (no Tauri) from WSL

If you only need the React dev server without the desktop shell:

```bash
cd ~/github/WireSentinel/ui
npm install   # WSL only; do not run Windows npm on this tree afterward
npm run dev
```

## UI / Service connection verification

After `wire-sentinel-service.exe` is running (portable `--console` or Windows Service):

```powershell
powershell -ExecutionPolicy Bypass -File C:\dev\WireSentinel\scripts\verify-ui-service-connection.ps1
```

Checks: `GET /api/v1/auth/token` → `GET /api/v1/status` (Bearer) → `GET /api/v1/diagnostics` → WebSocket `/api/v1/events`.

With the Tauri UI (`wire-sentinel.exe`), the Dashboard should show **connected** when `EventContext` successfully hydrates via REST and WebSocket. Both executables should live in the same directory (or set `WIRESENTINEL_SERVICE_EXE`).

Traffic monitor design comparison: [traffic-monitor-comparison.md](traffic-monitor-comparison.md).

## Related scripts

- [`scripts/build-windows-cross.sh`](../scripts/build-windows-cross.sh) — low-level `cargo-xwin` wrapper
- [`scripts/stage-windows-portable.sh`](../scripts/stage-windows-portable.sh) — build + copy to portable folder
- [`scripts/verify-ui-service-connection.ps1`](../scripts/verify-ui-service-connection.ps1) — API/WebSocket health check
