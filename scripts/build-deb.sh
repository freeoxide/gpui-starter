#!/usr/bin/env bash
#
# Build the gpui-starter .deb via cargo-deb: stages the bundled-libs layout
# (binary -> /usr/bin/gpui-starter, libs -> /usr/lib/gpui-starter/, rpath
# $ORIGIN/../lib/gpui-starter), then `cargo deb --no-build` over the
# [package.metadata.deb] asset map in Cargo.toml. Decision record:
# docs/research/linux-packages.md.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

need() {
    echo "Error: '$1' not found — install it first:" >&2
    echo "  $2" >&2
    exit 1
}

# The updater manifest URL is baked at compile time (option_env!); deb builds
# keep it unset so apt owns updates, not the in-app updater (linux-packages.md).
if [ -n "${GPUI_UPDATE_MANIFEST_URL:-}" ]; then
    echo "Error: GPUI_UPDATE_MANIFEST_URL is set; it must stay unset for deb builds:" >&2
    echo "  env -u GPUI_UPDATE_MANIFEST_URL ./scripts/build-deb.sh" >&2
    exit 1
fi

command -v patchelf >/dev/null 2>&1 || need patchelf "apt install patchelf | brew install patchelf"
cargo deb --version >/dev/null 2>&1 || need cargo-deb "cargo install cargo-deb --locked"

cargo build --release

STAGE="$ROOT_DIR/target/deb-stage"
rm -rf "$STAGE"
mkdir -p "$STAGE/lib"
cp target/release/gpui-starter "$STAGE/gpui-starter"

# The deb puts the binary in /usr/bin and libs in /usr/lib/gpui-starter, so
# the tarball's $ORIGIN/lib rpath would resolve to /usr/lib; go one deeper.
patchelf --set-rpath '$ORIGIN/../lib/gpui-starter' "$STAGE/gpui-starter"

# Same non-glibc bundling walk as the release workflow's Linux Relink step:
# glibc/ld-linux must stay host-owned (bundling them breaks across distros).
for lib in $(ldd "$STAGE/gpui-starter" | awk '/=> \// {print $3}'); do
    base="$(basename "$lib")"
    case "$base" in
      libc.so*|libm.so*|libdl.so*|librt.so*|libpthread.so*|ld-linux*|libgcc_s.so*|libutil.so*|libresolv.so*)
        continue
        ;;
    esac
    [ -f "$STAGE/lib/$base" ] || cp -L "$lib" "$STAGE/lib/"
done

rm -rf target/debian
cargo deb --no-build

DEB="$(find target/debian -maxdepth 1 -name '*.deb' -print -quit)"
[ -n "$DEB" ] || { echo "Error: cargo deb produced no .deb under target/debian" >&2; exit 1; }
echo "==> Built $DEB"

if command -v dpkg-deb >/dev/null 2>&1; then
    dpkg-deb --info "$DEB"
    dpkg-deb --contents "$DEB"
else
    echo "dpkg-deb not found; skipped package info/contents verification" >&2
fi
