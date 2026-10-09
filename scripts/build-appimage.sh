#!/usr/bin/env bash
#
# Build the gpui-starter AppImage: stages the tarball layout (binary at the
# AppDir root, bundled non-glibc libs in lib/, rpath $ORIGIN/lib) behind a
# custom packaging/appimage/AppRun, then linuxdeploy + plugin-appimage wrap
# it into GPUI-Starter-<version>-<arch>.AppImage (+ .zsync + .sha256).
# Decision record: docs/research/linux-packages.md.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

need() {
    echo "Error: '$1' not found — install it first:" >&2
    echo "  $2" >&2
    exit 1
}

# Opposite gate from build-deb.sh: AppImage is on the updater-compatible
# list, so the manifest URL must be baked in, not stripped.
if [ -z "${GPUI_UPDATE_MANIFEST_URL:-}" ]; then
    echo "Error: GPUI_UPDATE_MANIFEST_URL is unset; AppImage builds keep the in-app updater:" >&2
    echo "  GPUI_UPDATE_MANIFEST_URL=https://<owner>.github.io/<repo>/manifest.json ./scripts/build-appimage.sh" >&2
    exit 1
fi

command -v patchelf >/dev/null 2>&1 || need patchelf "apt install patchelf | brew install patchelf"
command -v curl >/dev/null 2>&1 || need curl "apt install curl"
# The plugin bundles appimagetool, which hard-fails without file and
# desktop-file-validate and silently skips the .zsync without zsyncmake.
command -v file >/dev/null 2>&1 || need file "apt install file"
command -v desktop-file-validate >/dev/null 2>&1 || need desktop-file-utils "apt install desktop-file-utils"
command -v zsyncmake >/dev/null 2>&1 || need zsync "apt install zsync"

# CI passes the workflow version; locally default to Cargo.toml's.
VERSION="${1:-}"
if [ -z "$VERSION" ]; then
    VERSION="$(cargo pkgid)"
    VERSION="${VERSION##*#}"
    VERSION="${VERSION##*@}"
fi

ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|aarch64) ;;
    *)
        echo "Error: unsupported architecture '$ARCH' (build on x86_64 or aarch64)" >&2
        exit 1
        ;;
esac

# Dated release tags, not `continuous`: their assets are immutable, so the
# sha256 pins below hold. linuxdeploy and the plugin are themselves AppImages.
LINUXDEPLOY_TAG="1-alpha-20251107-1"
PLUGIN_TAG="1-alpha-20250213-1"
case "$ARCH" in
    x86_64)
        LINUXDEPLOY_SHA256="c20cd71e3a4e3b80c3483cef793cda3f4e990aca14014d23c544ca3ce1270b4d"
        PLUGIN_SHA256="992d502a248e14ab185448ddf6f6e7d25558cb84d4623c354c3af350c25fccb3"
        ;;
    aarch64)
        LINUXDEPLOY_SHA256="620095110d693282b8ebeb244a95b5e911cf8f65f76c88b4b47d16ae6346fcff"
        PLUGIN_SHA256="83c292149274965a865dcd44c135cfca8ba28c6b7de3eb628d4b8b5f248af17c"
        ;;
esac

fetch_pinned() {
    local dest="$TOOLS/$1" url="$2" sha256="$3"
    if [ -f "$dest" ] && echo "$sha256  $dest" | sha256sum -c --status >/dev/null 2>&1; then
        return
    fi
    curl -fSL --retry 3 -o "$dest" "$url"
    echo "$sha256  $dest" | sha256sum -c --status || {
        echo "Error: sha256 mismatch for $1" >&2
        exit 1
    }
    chmod +x "$dest"
}

TOOLS="$ROOT_DIR/target/appimage-tools"
mkdir -p "$TOOLS"
# Only one arch's plugin may sit in $TOOLS: linuxdeploy resolves plugins
# by name there, and with two name-matching files the winner is unspecified.
fetch_pinned "linuxdeploy-$ARCH.AppImage" \
    "https://github.com/linuxdeploy/linuxdeploy/releases/download/$LINUXDEPLOY_TAG/linuxdeploy-$ARCH.AppImage" \
    "$LINUXDEPLOY_SHA256"
fetch_pinned "linuxdeploy-plugin-appimage-$ARCH.AppImage" \
    "https://github.com/linuxdeploy/linuxdeploy-plugin-appimage/releases/download/$PLUGIN_TAG/linuxdeploy-plugin-appimage-$ARCH.AppImage" \
    "$PLUGIN_SHA256"

cargo build --release

APPDIR="$ROOT_DIR/target/appimage-stage/AppDir"
rm -rf "$ROOT_DIR/target/appimage-stage"
mkdir -p "$APPDIR/lib"
cp target/release/gpui-starter "$APPDIR/gpui-starter"

# Same rpath as the tarball: binary at the AppDir root, libs in lib/ beside
# it; AppRun execs this binary directly (linuxdeploy's own AppRun is skipped).
patchelf --set-rpath '$ORIGIN/lib' "$APPDIR/gpui-starter"

# Same non-glibc bundling walk as build-deb.sh / the release workflow's
# Relink step: glibc/ld-linux must stay host-owned (bundling breaks across
# distros).
for lib in $(ldd "$APPDIR/gpui-starter" | awk '/=> \// {print $3}'); do
    base="$(basename "$lib")"
    case "$base" in
      libc.so*|libm.so*|libdl.so*|librt.so*|libpthread.so*|ld-linux*|libgcc_s.so*|libutil.so*|libresolv.so*)
        continue
        ;;
    esac
    [ -f "$APPDIR/lib/$base" ] || cp -L "$lib" "$APPDIR/lib/"
done

install -m 755 packaging/appimage/AppRun "$APPDIR/AppRun"

OUT_DIR="$ROOT_DIR/target/release"
OUT="$OUT_DIR/GPUI-Starter-${VERSION}-${ARCH}.AppImage"
UPDATE_INFORMATION="${APPIMAGE_UPDATE_INFORMATION:-gh-releases-zsync|hmziqagent|gpui-starter|latest|GPUI-Starter-*-${ARCH}.AppImage.zsync}"

# Tools are AppImages and CI runners lack /dev/fuse, so extract-and-run;
# zsyncmake writes <basename>.zsync into the CWD, hence the cd to OUT_DIR.
cd "$OUT_DIR"
APPIMAGE_EXTRACT_AND_RUN=1 \
LDAI_OUTPUT="$OUT" \
LINUXDEPLOY_OUTPUT_VERSION="$VERSION" \
LDAI_UPDATE_INFORMATION="$UPDATE_INFORMATION" \
"$TOOLS/linuxdeploy-$ARCH.AppImage" \
    --appdir "$APPDIR" \
    --desktop-file "$ROOT_DIR/resources/com.gpui-starter.app.desktop" \
    --icon-file "$ROOT_DIR/assets/app-icon/hicolor/512x512/apps/gpui-starter.png" \
    --output appimage

[ -f "$OUT" ] || { echo "Error: linuxdeploy produced no AppImage at $OUT" >&2; exit 1; }
[ -f "$OUT.zsync" ] || { echo "Error: no .zsync next to $OUT (zsyncmake missing?)" >&2; exit 1; }
sha256sum "$(basename "$OUT")" > "$OUT.sha256"

echo "==> Built $OUT (+ .zsync + .sha256)"
