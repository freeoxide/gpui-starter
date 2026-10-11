#!/usr/bin/env bash
#
# Regenerate every app-icon artifact from assets/app-icon/gpui-starter.svg:
# the Linux hicolor PNG set, the macOS .icns, the Windows .ico, and the
# legacy resources/gpui-starter.png. Run after changing the SVG; the
# generated files are committed (see docs/research/icons-and-windows.md A4).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ICON_DIR="$ROOT_DIR/assets/app-icon"
SVG="$ICON_DIR/gpui-starter.svg"
HICOLOR_DIR="$ICON_DIR/hicolor"

HICOLOR_SIZES=(16 22 24 32 48 64 128 256 512)
ICNS_SIZES=(16 32 64 128 256 512 1024)
ICO_SIZES="256,128,64,48,32,24,16"

[ -f "$SVG" ] || { echo "Error: SVG master not found: $SVG" >&2; exit 1; }

need() {
    echo "Error: '$1' not found — install it first:" >&2
    echo "  Debian/Ubuntu: apt install $2   |   macOS: brew install $3" >&2
    exit 1
}

# rsvg-convert is the preferred renderer; ImageMagick SVG output depends on
# its librsvg delegate, so it is only a fallback when rsvg-convert is absent.
RASTERIZER=""
if command -v rsvg-convert >/dev/null 2>&1; then
    RASTERIZER=rsvg-convert
elif command -v magick >/dev/null 2>&1; then
    RASTERIZER=magick
elif command -v convert >/dev/null 2>&1; then
    RASTERIZER=convert
else
    echo "Error: no SVG rasterizer found (tried rsvg-convert, magick, convert)" >&2
    need rsvg-convert librsvg2-bin librsvg
fi

ICO_TOOL=""
if command -v magick >/dev/null 2>&1; then
    ICO_TOOL=magick
elif command -v convert >/dev/null 2>&1; then
    ICO_TOOL=convert
else
    need magick imagemagick imagemagick
fi

command -v png2icns >/dev/null 2>&1 || need png2icns icnsutils icnsutils
command -v icns2png >/dev/null 2>&1 || need icns2png icnsutils icnsutils

render_png() {
    local size="$1" out="$2"
    if [ "$RASTERIZER" = rsvg-convert ]; then
        rsvg-convert -w "$size" -h "$size" "$SVG" > "$out"
    else
        "$RASTERIZER" "$SVG" -resize "${size}x${size}" "$out"
    fi
}

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

for size in "${HICOLOR_SIZES[@]}"; do
    out="$HICOLOR_DIR/${size}x${size}/apps/gpui-starter.png"
    mkdir -p "$(dirname "$out")"
    render_png "$size" "$out"
done
mkdir -p "$HICOLOR_DIR/scalable/apps"
cp "$SVG" "$HICOLOR_DIR/scalable/apps/gpui-starter.svg"

render_png 512 "$ROOT_DIR/resources/gpui-starter.png"

ICNS_INPUTS=()
for size in "${ICNS_SIZES[@]}"; do
    render_png "$size" "$TMP_DIR/icon-${size}.png"
    ICNS_INPUTS+=("$TMP_DIR/icon-${size}.png")
done
png2icns "$ICON_DIR/gpui-starter.icns" "${ICNS_INPUTS[@]}"
# macOS-only fallback if png2icns output proves incomplete in CI:
# iconutil -c icns gpui-starter.iconset from the same PNGs (research A1).
echo "icns entries:"
icns2png -l "$ICON_DIR/gpui-starter.icns" || true

"$ICO_TOOL" "$TMP_DIR/icon-1024.png" -resize 256x256 \
    -define "icon:auto-resize=$ICO_SIZES" "$ICON_DIR/gpui-starter.ico"

echo "Regenerated from $SVG:"
echo "  $HICOLOR_DIR (${HICOLOR_SIZES[*]} + scalable svg)"
echo "  $ICON_DIR/gpui-starter.icns"
echo "  $ICON_DIR/gpui-starter.ico"
echo "  $ROOT_DIR/resources/gpui-starter.png"
