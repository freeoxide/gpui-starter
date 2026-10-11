#!/usr/bin/env bash
#
# Compile packaging/windows/gpui-starter.iss with ISCC into target/release/
# GPUI-Starter-<version>-x86_64-windows-setup.exe (+ .sha256). Inno Setup 6
# is preinstalled on GHA windows-2025 images; run
# scripts/build-windows-portable.sh first (this script does not build).
# Decision record: docs/research/icons-and-windows.md (B1, B3).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

need() {
    echo "Error: '$1' not found — install it first:" >&2
    echo "  $2" >&2
    exit 1
}

command -v sha256sum >/dev/null 2>&1 || need sha256sum "Git Bash ships it (coreutils)"
command -v cygpath >/dev/null 2>&1 || need cygpath "Git Bash / MSYS2"

# CI passes the workflow version; locally default to Cargo.toml's.
VERSION="${1:-}"
if [ -z "$VERSION" ]; then
    VERSION="$(cargo pkgid)"
    VERSION="${VERSION##*#}"
    VERSION="${VERSION##*@}"
fi

EXE="target/x86_64-pc-windows-msvc/release/gpui-starter.exe"
[ -f "$EXE" ] || { echo "Error: $EXE missing; run ./scripts/build-windows-portable.sh first" >&2; exit 1; }
[ -f assets/app-icon/gpui-starter.ico ] || { echo "Error: assets/app-icon/gpui-starter.ico missing (committed; scripts/gen-icons.sh regenerates)" >&2; exit 1; }

ISCC=""
if command -v ISCC.exe >/dev/null 2>&1; then
    ISCC="ISCC.exe"
elif [ -x "/c/Program Files (x86)/Inno Setup 6/ISCC.exe" ]; then
    ISCC="/c/Program Files (x86)/Inno Setup 6/ISCC.exe"
else
    need "ISCC (Inno Setup 6)" "winget install JRSoftware.InnoSetup (preinstalled on GHA windows-2025)"
fi

OUT_DIR="$ROOT_DIR/target/release"
mkdir -p "$OUT_DIR"

# ISCC flags start with '/', which MSYS rewrites into Windows paths;
# MSYS2_ARG_CONV_EXCL disables that, so paths below are already Windows ones.
MSYS2_ARG_CONV_EXCL='*' "$ISCC" \
    "/DVersion=$VERSION" \
    "/DRepoRoot=$(cygpath -w "$ROOT_DIR")" \
    "/O$(cygpath -w "$OUT_DIR")" \
    "$(cygpath -w "$ROOT_DIR/packaging/windows/gpui-starter.iss")"

SETUP="$OUT_DIR/GPUI-Starter-${VERSION}-x86_64-windows-setup.exe"
[ -f "$SETUP" ] || { echo "Error: ISCC produced no setup exe at $SETUP" >&2; exit 1; }

cd "$OUT_DIR"
sha256sum "$(basename "$SETUP")" > "$SETUP.sha256"
cd "$ROOT_DIR"

echo "==> Built $SETUP (+ .sha256)"
