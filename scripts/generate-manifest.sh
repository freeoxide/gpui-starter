#!/usr/bin/env bash
set -euo pipefail

# Usage: generate-manifest.sh <VERSION> <GITHUB_REPO> [OUTPUT_FILE]
# Example: generate-manifest.sh 0.2.0 myorg/gpui-app manifest.json
# If OUTPUT_FILE is omitted, writes to stdout.
#
# Optional environment variables:
#   UPDATE_SIGNING_KEY  — base64-encoded Ed25519 private key for signing assets.
#                         If not set, the signature field is left empty.

if [ $# -lt 2 ]; then
    echo "Usage: $0 <VERSION> <GITHUB_REPO> [OUTPUT_FILE]" >&2
    exit 1
fi

VERSION="$1"
GITHUB_REPO="$2"
OUTPUT_FILE="${3:-}"

API_URL="https://api.github.com/repos/${GITHUB_REPO}/releases/tags/v${VERSION}"

# Authenticated curl: passes GH_TOKEN when present (private repos, and to dodge
# the shared-IP 60 req/hr anonymous limit). No-op for public repos w/o a token.
gh_curl() {
    if [ -n "${GH_TOKEN:-}" ]; then
        curl -H "Authorization: Bearer ${GH_TOKEN}" -H "Accept: application/vnd.github+json" "$@"
    else
        curl "$@"
    fi
}

# Fetch release metadata from GitHub.
RELEASE_JSON="$(gh_curl -sf "$API_URL")"

if [ -z "$RELEASE_JSON" ]; then
    echo "Error: Failed to fetch release for v${VERSION} from ${GITHUB_REPO}" >&2
    exit 1
fi

# Extract the release body as notes. Do NOT pre-escape here: the downstream
# `jq -n --arg notes` (below) is the single source of truth for JSON escaping.
# The old manual gsub chain double-escaped and turned "\n" into literal "\\n".
RELEASE_NOTES="$(echo "$RELEASE_JSON" | jq -r '.body // ""')"

# Build the platforms object. Keys mirror platform_key() in
# src/services/updater/types.rs (os-arch). The universal DMG serves both
# macOS rows; windows-x86_64 points at the BARE exe because the updater
# swaps single files (the zip and setup exe are for humans). deb/rpm/flatpak
# compile the URL out (the system package manager owns their updates) and
# the AppImage updates via its embedded zsync info, so no linux- entry.
DMG_NAME="GPUI-Starter-${VERSION}.dmg"
EXE_NAME="GPUI-Starter-${VERSION}-x86_64-windows.exe"

# Look up one release asset by exact name; a missing asset must fail the
# release instead of shipping a manifest without that platform.
find_asset() {
    echo "$RELEASE_JSON" | jq -r --arg name "$1" '.assets[] | select(.name == $name)'
}

DMG_ASSET="$(find_asset "$DMG_NAME")"
if [ -z "$DMG_ASSET" ]; then
    echo "Error: Asset '$DMG_NAME' not found in release v${VERSION}" >&2
    exit 1
fi

EXE_ASSET="$(find_asset "$EXE_NAME")"
if [ -z "$EXE_ASSET" ]; then
    echo "Error: Asset '$EXE_NAME' not found in release v${VERSION}" >&2
    exit 1
fi

DMG_URL="$(echo "$DMG_ASSET" | jq -r '.browser_download_url')"
DMG_SIZE="$(echo "$DMG_ASSET" | jq -r '.size')"
EXE_URL="$(echo "$EXE_ASSET" | jq -r '.browser_download_url')"
EXE_SIZE="$(echo "$EXE_ASSET" | jq -r '.size')"

# Compute the Ed25519 signature of each asset's SHA-256 hash. The signing key
# is a base64-encoded 64-byte Ed25519 private key (seed || public key).
DMG_SIGNATURE=""
EXE_SIGNATURE=""

# Signs a sha256 hex digest passed as $1 and prints the base64 signature;
# empty output means failure, which callers turn into a hard error. The hash
# travels via env (not shell interpolation) to avoid injection/quoting
# pitfalls. NOTE: `cryptography` is NOT on ubuntu images by default; the
# release workflow pip-installs it before calling this script.
sign_sha256() {
    FILE_HASH="$1" python3 -c '
import base64, os, sys

try:
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
    signing_key_b64 = os.environ.get("UPDATE_SIGNING_KEY", "")
    if not signing_key_b64:
        print("", end="")
        sys.exit(0)
    hexhash = os.environ.get("FILE_HASH", "")
    if len(hexhash) != 64:
        raise ValueError(f"bad sha256 length: {len(hexhash)}")
    key_bytes = base64.b64decode(signing_key_b64)
    private_key = Ed25519PrivateKey.from_private_bytes(key_bytes[:32])
    sig = private_key.sign(bytes.fromhex(hexhash))
    print(base64.b64encode(sig).decode(), end="")
except Exception as e:
    print(f"signing failed: {e}", file=sys.stderr)
    print("", end="")
'
}

# Downloads $1 (a browser_download_url), prints its signature. A failed
# download keeps the old warn-and-ship-unsigned behavior; a signing failure
# exits non-zero (the updater refuses unsigned assets, so hiding it helps
# nobody).
sign_release_asset() {
    FILE="$TMP_ASSETS/$(basename "$1")"
    if ! gh_curl -sfL -o "$FILE" "$1"; then
        echo "Warning: Failed to download asset for signing — signature will be empty" >&2
        return 0
    fi
    SIGNATURE="$(sign_sha256 "$(shasum -a 256 "$FILE" | awk '{print $1}')")"
    if [ -z "$SIGNATURE" ]; then
        echo "Error: UPDATE_SIGNING_KEY is set but Ed25519 signing produced an empty signature" >&2
        exit 1
    fi
    echo "Ed25519 signature generated: ${SIGNATURE:0:20}..." >&2
    printf '%s' "$SIGNATURE"
}

if [ -n "${UPDATE_SIGNING_KEY:-}" ]; then
    echo "Signing manifest with Ed25519..." >&2

    TMP_ASSETS="$(mktemp -d)"
    trap 'rm -rf "$TMP_ASSETS"' EXIT

    DMG_SIGNATURE="$(sign_release_asset "$DMG_URL")"
    EXE_SIGNATURE="$(sign_release_asset "$EXE_URL")"
fi

# Generate the manifest JSON matching the UpdateManifest struct in updater.rs:
#   { version, release_notes, platforms: { "<key>": { url, signature, size } } }
MANIFEST="$(jq -n \
    --arg version "$VERSION" \
    --arg notes "$RELEASE_NOTES" \
    --arg dmg_url "$DMG_URL" \
    --argjson dmg_size "$DMG_SIZE" \
    --arg dmg_signature "$DMG_SIGNATURE" \
    --arg exe_url "$EXE_URL" \
    --argjson exe_size "$EXE_SIZE" \
    --arg exe_signature "$EXE_SIGNATURE" \
    '{
        "version": $version,
        "release_notes": $notes,
        "platforms": {
            "macos-aarch64": {
                "url": $dmg_url,
                "signature": $dmg_signature,
                "size": $dmg_size
            },
            "macos-x86_64": {
                "url": $dmg_url,
                "signature": $dmg_signature,
                "size": $dmg_size
            },
            "windows-x86_64": {
                "url": $exe_url,
                "signature": $exe_signature,
                "size": $exe_size
            }
        }
    }')"

if [ -n "$OUTPUT_FILE" ]; then
    echo "$MANIFEST" > "$OUTPUT_FILE"
    echo "Manifest written to $OUTPUT_FILE" >&2
else
    echo "$MANIFEST"
fi
