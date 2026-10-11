#!/usr/bin/env bash
#
# Build the portable Windows artifacts: release exe for
# x86_64-pc-windows-msvc, the bare-exe copy the updater manifest points
# at, and GPUI-Starter-<version>-x86_64-windows.zip (exe + LICENSE +
# README), each with a .sha256 sidecar in target/release/. Git Bash on a
# Windows host; decision record: docs/research/icons-and-windows.md (B2-B4).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

need() {
    echo "Error: '$1' not found — install it first:" >&2
    echo "  $2" >&2
    exit 1
}

# Same gate side as build-appimage.sh: zip and installer builds keep the
# in-app updater, so the manifest URL must be baked in, not stripped.
if [ -z "${GPUI_UPDATE_MANIFEST_URL:-}" ]; then
    echo "Error: GPUI_UPDATE_MANIFEST_URL is unset; Windows builds keep the in-app updater:" >&2
    echo "  GPUI_UPDATE_MANIFEST_URL=https://<owner>.github.io/<repo>/manifest.json ./scripts/build-windows-portable.sh" >&2
    exit 1
fi

command -v cargo >/dev/null 2>&1 || need cargo "https://rustup.rs (GHA Windows images ship it)"
command -v sha256sum >/dev/null 2>&1 || need sha256sum "Git Bash ships it (coreutils)"
command -v cygpath >/dev/null 2>&1 || need cygpath "Git Bash / MSYS2"

# CI passes the workflow version; locally default to Cargo.toml's.
VERSION="${1:-}"
if [ -z "$VERSION" ]; then
    VERSION="$(cargo pkgid)"
    VERSION="${VERSION##*#}"
    VERSION="${VERSION##*@}"
fi

# aarch64-pc-windows-msvc is a later addition (icons-and-windows.md B2).
TARGET="x86_64-pc-windows-msvc"
cargo build --release --target "$TARGET"

EXE="target/$TARGET/release/gpui-starter.exe"
[ -f "$EXE" ] || { echo "Error: no exe at $EXE" >&2; exit 1; }

OUT_DIR="$ROOT_DIR/target/release"
BASE="GPUI-Starter-${VERSION}-x86_64-windows"
mkdir -p "$OUT_DIR"
rm -f "$OUT_DIR/$BASE.exe" "$OUT_DIR/$BASE.exe.sha256" "$OUT_DIR/$BASE.zip" "$OUT_DIR/$BASE.zip.sha256"

STAGE="$ROOT_DIR/target/windows-portable-stage"
rm -rf "$STAGE"
mkdir -p "$STAGE"
cp "$EXE" "$STAGE/gpui-starter.exe"
cp LICENSE "$STAGE/"
cp README.md "$STAGE/"
cp "$EXE" "$OUT_DIR/$BASE.exe"

# Git Bash's /usr/bin/tar is GNU tar (no zip writer); System32's tar is
# bsdtar, which writes zip via the -a extension sniff.
WINTAR="/c/Windows/System32/tar.exe"
if [ -x "$WINTAR" ]; then
    # All-relative invocation: MSYS never rewrites an argument that does
    # not start with a slash.
    cd "$OUT_DIR"
    "$WINTAR" -a -cf "$BASE.zip" -C ../windows-portable-stage gpui-starter.exe LICENSE README.md
    cd "$ROOT_DIR"
else
    command -v pwsh >/dev/null 2>&1 || need pwsh "winget install Microsoft.PowerShell (GHA Windows images ship it)"
    pwsh -NoProfile -Command "Compress-Archive -Path '$(cygpath -w "$STAGE")\*' -DestinationPath '$(cygpath -w "$OUT_DIR/$BASE.zip")' -Force"
fi

[ -f "$OUT_DIR/$BASE.zip" ] || { echo "Error: no zip at $OUT_DIR/$BASE.zip" >&2; exit 1; }

cd "$OUT_DIR"
sha256sum "$BASE.exe" > "$BASE.exe.sha256"
sha256sum "$BASE.zip" > "$BASE.zip.sha256"
cd "$ROOT_DIR"

echo "==> Built $OUT_DIR/$BASE.exe (+ .sha256) and $OUT_DIR/$BASE.zip (+ .sha256)"
