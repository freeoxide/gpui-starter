#!/usr/bin/env bash
#
# Build the gpui-starter .rpm via cargo-generate-rpm: stages the bundled-libs
# layout (binary -> /usr/bin/gpui-starter, libs -> /usr/lib/gpui-starter/,
# rpath $ORIGIN/../lib/gpui-starter), then `cargo generate-rpm` over the
# [package.metadata.generate-rpm] asset map in Cargo.toml. Decision record:
# docs/research/linux-packages.md.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

need() {
    echo "Error: '$1' not found — install it first:" >&2
    echo "  $2" >&2
    exit 1
}

# The updater manifest URL is baked at compile time (option_env!); rpm builds
# keep it unset so dnf owns updates, not the in-app updater (linux-packages.md).
if [ -n "${GPUI_UPDATE_MANIFEST_URL:-}" ]; then
    echo "Error: GPUI_UPDATE_MANIFEST_URL is set; it must stay unset for rpm builds:" >&2
    echo "  env -u GPUI_UPDATE_MANIFEST_URL ./scripts/build-rpm.sh" >&2
    exit 1
fi

command -v patchelf >/dev/null 2>&1 || need patchelf "apt install patchelf | brew install patchelf"
cargo generate-rpm --version >/dev/null 2>&1 || need cargo-generate-rpm "cargo install cargo-generate-rpm --locked"

cargo build --release

STAGE="$ROOT_DIR/target/rpm-stage"
rm -rf "$STAGE"
mkdir -p "$STAGE/lib"
cp target/release/gpui-starter "$STAGE/gpui-starter"

# The rpm puts the binary in /usr/bin and libs in /usr/lib/gpui-starter, so
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

rm -rf target/generate-rpm
cargo generate-rpm

RPM="$(find target/generate-rpm -maxdepth 1 -name '*.rpm' -print -quit)"
[ -n "$RPM" ] || { echo "Error: cargo generate-rpm produced no .rpm under target/generate-rpm" >&2; exit 1; }
echo "==> Built $RPM"

if command -v rpm >/dev/null 2>&1; then
    rpm -qpl "$RPM"
    rpm -qp --requires "$RPM"
else
    echo "rpm not found; skipped package contents/requires verification" >&2
fi

# Snap is deliberately not built (docs/research/linux-packages.md): the store
# needs an account, a manually reviewed name, and per-release review, and a
# Vulkan snap must be tested against live driver stacks CI cannot provide;
# the gpu-2404 content snap is the base if it is ever revived.
