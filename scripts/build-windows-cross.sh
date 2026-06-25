#!/usr/bin/env bash
# Cross-compile WireSentinel Windows binaries from Linux/WSL using cargo-xwin.
# Plain `cargo build --target x86_64-pc-windows-msvc` fails on aws-lc-sys because
# the Linux host compiler is used without the MSVC Windows SDK toolchain.
#
# Usage:
#   ./scripts/build-windows-cross.sh
#   ./scripts/build-windows-cross.sh core-service
#   ./scripts/build-windows-cross.sh core-service --release
#   ./scripts/build-windows-cross.sh core-service   # debug (default profile)

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TARGET="${CARGO_BUILD_TARGET:-x86_64-pc-windows-msvc}"
PACKAGE="${1:-core-service}"
shift || true

ensure_clang_cl() {
    if command -v clang-cl >/dev/null 2>&1; then
        return 0
    fi
    if ! command -v clang >/dev/null 2>&1; then
        echo "error: clang-cl or clang is required (sudo apt-get install -y clang lld)" >&2
        exit 1
    fi
    # Debian/Ubuntu ship `clang` but not always a `clang-cl` shim on PATH.
    local bindir="$ROOT/.cache/ws-cross/bin"
    mkdir -p "$bindir"
    cat > "$bindir/clang-cl" << 'EOF'
#!/usr/bin/env bash
exec clang --driver-mode=cl "$@"
EOF
    chmod +x "$bindir/clang-cl"
    export PATH="$bindir:$PATH"
    echo "Using clang --driver-mode=cl wrapper at $bindir/clang-cl"
}

ensure_llvm_tools() {
    if ! rustup component list --installed | grep -qx 'llvm-tools'; then
        echo "Installing rustup llvm-tools..."
        rustup component add llvm-tools
    fi
    if command -v llvm-lib >/dev/null 2>&1; then
        return 0
    fi
    local llvm_bindir=""
    if command -v llvm-config >/dev/null 2>&1; then
        llvm_bindir="$(llvm-config --bindir)"
    elif [[ -d /usr/lib/llvm-18/bin ]]; then
        llvm_bindir="/usr/lib/llvm-18/bin"
    fi
    if [[ -n "$llvm_bindir" && -x "$llvm_bindir/llvm-lib" ]]; then
        export PATH="$llvm_bindir:$PATH"
        echo "Added LLVM tools to PATH: $llvm_bindir"
        return 0
    fi
    echo "error: llvm-lib not found (install: sudo apt-get install -y clang lld llvm)" >&2
    exit 1
}

ensure_clang_cl
ensure_llvm_tools

if ! command -v cargo-xwin >/dev/null 2>&1; then
    echo "Installing cargo-xwin..."
    cargo install cargo-xwin --locked
fi

if ! rustup target list --installed | grep -qx "$TARGET"; then
    rustup target add "$TARGET"
fi

export XWIN_ACCEPT_LICENSE="${XWIN_ACCEPT_LICENSE:-1}"

echo "Building package=$PACKAGE target=$TARGET (via cargo-xwin)..."
cargo xwin build -p "$PACKAGE" --target "$TARGET" "$@"

PROFILE="debug"
for arg in "$@"; do
    if [[ "$arg" == "--release" ]]; then
        PROFILE="release"
        break
    fi
done

BIN="wire-sentinel-service.exe"
candidate="$ROOT/target/$TARGET/$PROFILE/$BIN"
if [[ -f "$candidate" ]]; then
    echo "Output: $candidate"
else
    echo "error: expected binary not found: $candidate" >&2
    exit 1
fi
