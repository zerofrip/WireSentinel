#!/usr/bin/env bash
# Cross-build core-service on WSL and stage a portable Windows backend bundle.
#
# Usage:
#   ./scripts/stage-windows-portable.sh
#   ./scripts/stage-windows-portable.sh --debug
#   ./scripts/stage-windows-portable.sh --skip-build
#   ./scripts/stage-windows-portable.sh --out /mnt/c/dev/WireSentinel-portable

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TARGET="${CARGO_BUILD_TARGET:-x86_64-pc-windows-msvc}"
OUT_DIR="${WIRESENTINEL_PORTABLE_DIR:-/mnt/c/dev/WireSentinel-portable}"
PROFILE="release"
SKIP_BUILD=0

usage() {
    cat <<'EOF'
Usage: ./scripts/stage-windows-portable.sh [OPTIONS]

Cross-build wire-sentinel-service.exe and copy runtime dependencies into a
portable folder accessible from Windows (default: C:\dev\WireSentinel-portable).

Options:
  --release      Release build (default)
  --debug        Debug build (includes .pdb when present)
  --skip-build   Reuse existing target/ artifacts; only copy/stage
  --out PATH     Output directory (default: /mnt/c/dev/WireSentinel-portable)
  -h, --help     Show this help
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --release)
            PROFILE="release"
            shift
            ;;
        --debug)
            PROFILE="debug"
            shift
            ;;
        --skip-build)
            SKIP_BUILD=1
            shift
            ;;
        --out)
            if [[ $# -lt 2 ]]; then
                echo "error: --out requires a path" >&2
                exit 1
            fi
            OUT_DIR="$2"
            shift 2
            ;;
        -h | --help)
            usage
            exit 0
            ;;
        *)
            echo "error: unknown option: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

if [[ "$OUT_DIR" == /mnt/c/* ]] && [[ ! -d /mnt/c ]]; then
    echo "error: /mnt/c is not mounted — start WSL with Windows drive access or use --out" >&2
    exit 1
fi

RESOURCES=(
    tunnel.dll
    wireguard.dll
    sing-box.exe
    tor.exe
    WinDivert.dll
    WinDivert64.sys
)

if [[ "$SKIP_BUILD" -eq 0 ]]; then
    echo "Building core-service ($PROFILE, $TARGET)..."
    build_args=(core-service)
    if [[ "$PROFILE" == "release" ]]; then
        build_args+=(--release)
    fi
    "$ROOT/scripts/build-windows-cross.sh" "${build_args[@]}"
fi

BIN_DIR="$ROOT/target/$TARGET/$PROFILE"
SERVICE_EXE="$BIN_DIR/wire-sentinel-service.exe"

if [[ ! -f "$SERVICE_EXE" ]]; then
    echo "error: missing $SERVICE_EXE — run without --skip-build or fix the cross build" >&2
    exit 1
fi

missing=()
for name in "${RESOURCES[@]}"; do
    if [[ ! -f "$ROOT/resources/$name" ]]; then
        missing+=("$name")
    fi
done
if [[ ${#missing[@]} -gt 0 ]]; then
    echo "error: missing files in $ROOT/resources/:" >&2
    printf '  - %s\n' "${missing[@]}" >&2
    echo "hint: run scripts/fetch-vpn-resources.ps1 on Windows if binaries are absent" >&2
    exit 1
fi

mkdir -p "$OUT_DIR"

cp -f "$SERVICE_EXE" "$OUT_DIR/"
for name in "${RESOURCES[@]}"; do
    cp -f "$ROOT/resources/$name" "$OUT_DIR/"
done

PDB="$BIN_DIR/wire_sentinel_service.pdb"
if [[ ! -f "$PDB" ]]; then
    PDB="$BIN_DIR/wire-sentinel-service.pdb"
fi
if [[ "$PROFILE" == "debug" && -f "$PDB" ]]; then
    cp -f "$PDB" "$OUT_DIR/"
fi

for name in "${RESOURCES[@]}"; do
    if [[ ! -s "$ROOT/resources/$name" ]]; then
        echo "warning: resources/$name is missing or empty — run scripts/fetch-vpn-resources.ps1 on Windows" >&2
    fi
done

echo ""
echo "Staged portable backend ($PROFILE) -> $OUT_DIR"
echo "Files:"
ls -1 "$OUT_DIR"
echo ""
echo "Run on Windows (Administrator PowerShell recommended for WFP):"
win_path="${OUT_DIR#/mnt/c/}"
win_path="C:\\${win_path//\//\\}"
echo "  ${win_path}\\wire-sentinel-service.exe --console"
