#!/usr/bin/env bash
#
# Build a local com.gpui-starter.app .flatpak bundle: regenerates the cargo
# vendor sources from Cargo.lock, runs flatpak-builder over
# packaging/flatpak/com.gpui-starter.app.yml (freedesktop 25.08 + rust-stable
# SDK extension, --features notifications-portal), then packs the repo into
# target/release/GPUI-Starter-<version>-<arch>.flatpak. Decision record:
# docs/research/linux-packages.md.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

need() {
    echo "Error: '$1' not found — install it first:" >&2
    echo "  $2" >&2
    exit 1
}

# Same gate as build-deb.sh: flatpak update owns upgrades, so the in-app
# updater URL must not be compiled into the sandboxed binary.
if [ -n "${GPUI_UPDATE_MANIFEST_URL:-}" ]; then
    echo "Error: GPUI_UPDATE_MANIFEST_URL is set; it must stay unset for flatpak builds:" >&2
    echo "  env -u GPUI_UPDATE_MANIFEST_URL ./scripts/build-flatpak.sh" >&2
    exit 1
fi

command -v flatpak >/dev/null 2>&1 || need flatpak "apt install flatpak"
command -v flatpak-builder >/dev/null 2>&1 || need flatpak-builder "apt install flatpak-builder"
command -v flatpak-cargo-generator >/dev/null 2>&1 || need flatpak-cargo-generator "pipx install flatpak-cargo-generator"

VERSION="${1:-}"
if [ -z "$VERSION" ]; then
    VERSION="$(cargo pkgid)"
    VERSION="${VERSION##*#}"
    VERSION="${VERSION##*@}"
fi

APP_ID="com.gpui-starter.app"
BRANCH="stable"
RUNTIME_VERSION="25.08"
STAGE="$ROOT_DIR/target/flatpak"
OUT="$ROOT_DIR/target/release/GPUI-Starter-${VERSION}-$(uname -m).flatpak"

flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
for ref in "org.freedesktop.Sdk//${RUNTIME_VERSION}" \
           "org.freedesktop.Sdk.Extension.rust-stable//${RUNTIME_VERSION}"; do
    flatpak info "$ref" >/dev/null 2>&1 ||
        flatpak install --user --noninteractive flathub "$ref"
done

flatpak-cargo-generator -o packaging/flatpak/cargo-sources.json Cargo.lock

flatpak-builder \
    --user \
    --state-dir="$STAGE/state" \
    --repo="$STAGE/repo" \
    --force-clean \
    "$STAGE/build" \
    packaging/flatpak/com.gpui-starter.app.yml

mkdir -p "$(dirname "$OUT")"
# --runtime-repo lets `flatpak install ./<bundle>` pull the freedesktop
# runtime on user machines instead of shipping the 25.08 Platform inside.
flatpak build-bundle \
    --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo \
    "$STAGE/repo" \
    "$OUT" \
    "$APP_ID" \
    "$BRANCH"

[ -f "$OUT" ] || { echo "Error: no bundle at $OUT" >&2; exit 1; }
echo "==> Built $OUT"
