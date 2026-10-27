#!/usr/bin/env bash
# Build the web (wasm32-unknown-unknown) artifact into wasm/pkg/.
#
#   wasm/build.sh            debug build (fast, huge, fine for development)
#   wasm/build.sh --release  release build (slow: 30-60 min for the gpui graph;
#                            run when a smaller artifact is needed)
#
# Steps: (1) cargo +nightly build of the lib cdylib (NIGHTLY is mandatory:
# wasm_thread uses `#![feature(stdarch_wasm_atomic_wait)]`), (2) wasm-bindgen
# --target web post-processing into wasm/pkg/ next to index.html.
#
# The wasm-bindgen CLI version MUST match the crate's wasm-bindgen pin
# (=0.2.129 in Cargo.toml) or the post-process fails with schema errors.
# Both profiles finish by writing wasm/pkg/version.json with a null
# version (200 for the harness boot probe, SW stays in dev no-op).
# Acquire the pinned CLI from GitHub release assets:
#   https://github.com/wasm-bindgen/wasm-bindgen/releases/download/0.2.129/ \
#     wasm-bindgen-0.2.129-x86_64-unknown-linux-musl.tar.gz
# (the release asset is named `wasm-bindgen-…`, not `wasm-bindgen-cli-…`).
#
# --release additionally runs wasm-opt (binaryen) over the bindgen output.
# Acquire the pinned binaryen from GitHub release assets (~seconds):
#   https://github.com/WebAssembly/binaryen/releases/download/version_123/ \
#     binaryen-version_123-x86_64-linux.tar.gz
#   -> extract bin/wasm-opt to ~/.local/bin. Missing binary = warning +
#   unoptimized artifact (the step is skippable; set WASM_OPT to override).
set -euo pipefail

cd "$(dirname "$0")/.."

PROFILE_DIR="debug"
PROFILE_FLAG=""
if [[ "${1:-}" == "--release" ]]; then
    PROFILE_DIR="release"
    PROFILE_FLAG="--release"
elif [[ -n "${1:-}" ]]; then
    echo "usage: $0 [--release]" >&2
    exit 2
fi

# Resolve cargo's target directory the way cargo itself does: the global
# ~/.cargo/config.toml redirects it to ~/.cache/cargo-target, so the artifact
# is NOT under ./target/. Honor an explicit CARGO_TARGET_DIR first.
if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
    TARGET_DIR="${CARGO_TARGET_DIR}"
else
    TARGET_DIR="$(python3 -c '
import json, subprocess
meta = json.loads(subprocess.check_output(
    ["cargo", "+nightly", "metadata", "--format-version", "1", "--no-deps"]))
print(meta["target_directory"])
')"
fi

echo "==> cargo +nightly build --target wasm32-unknown-unknown --lib ${PROFILE_FLAG}"
cargo +nightly build --target wasm32-unknown-unknown --lib ${PROFILE_FLAG}

WASM="${TARGET_DIR}/wasm32-unknown-unknown/${PROFILE_DIR}/gpui_starter.wasm"
if [[ ! -f "${WASM}" ]]; then
    echo "error: cdylib artifact not found at ${WASM}" >&2
    exit 1
fi

BINDGEN="${WASM_BINDGEN:-wasm-bindgen}"
if ! command -v "${BINDGEN}" >/dev/null 2>&1; then
    echo "error: wasm-bindgen CLI not found (need exactly 0.2.129)" >&2
    echo "  install: see the URL in this script's header comment" >&2
    exit 1
fi
VERSION="$("${BINDGEN}" --version | awk '{print $NF}')"
if [[ "${VERSION}" != "0.2.129" ]]; then
    echo "error: wasm-bindgen ${VERSION} found, but Cargo.toml pins =0.2.129" >&2
    echo "  (set WASM_BINDGEN=/path/to/wasm-bindgen to override)" >&2
    exit 1
fi

echo "==> wasm-bindgen ${VERSION} --target web -> wasm/pkg/"
rm -rf wasm/pkg
mkdir -p wasm/pkg
"${BINDGEN}" "${WASM}" \
    --target web \
    --out-dir wasm/pkg \
    --out-name gpui_starter

# Release-only wasm-opt pass (binaryen) over the bindgen output. Skippable:
# a missing binary logs a warning and keeps the unoptimized artifact.
if [[ "${PROFILE_DIR}" == "release" ]]; then
    OPT="${WASM_OPT:-wasm-opt}"
    if command -v "${OPT}" >/dev/null 2>&1; then
        OUT="wasm/pkg/gpui_starter_bg.opt.wasm"
        BEFORE="$(stat -c%s wasm/pkg/gpui_starter_bg.wasm)"
        echo "==> ${OPT} -O3 (this can take a few minutes)"
        "${OPT}" -O3 \
            --enable-bulk-memory --enable-mutable-globals \
            --strip-debug \
            -o "${OUT}" wasm/pkg/gpui_starter_bg.wasm
        mv "${OUT}" wasm/pkg/gpui_starter_bg.wasm
        AFTER="$(stat -c%s wasm/pkg/gpui_starter_bg.wasm)"
        echo "    wasm-opt: ${BEFORE} -> ${AFTER} bytes"
    else
        echo "==> WARNING: wasm-opt not found — keeping unoptimized artifact" >&2
        echo "    install: see the binaryen URL in this script's header" >&2
    fi
fi

# version.json: the harness boot probe (index.html) and sw.js's update
# check both GET /pkg/version.json — write it on EVERY build so plain
# debug runs no longer log a 404 to the console. `version` stays null
# here ON PURPOSE: serve_http.py injects any non-null version into
# sw.js's BUILD_VERSION, flipping the SW out of its deliberate dev
# no-op (see the sw.js header — cache-first /pkg/* on a ~95 MB debug
# artifact would shadow live rebuilds). `just wasm-release` /
# precompress.mjs overwrites this file with the real content hash once
# the release artifacts + compressed siblings are final.
printf '{"version":null,"generated":"%s"}\n' \
    "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > wasm/pkg/version.json
echo "==> wrote wasm/pkg/version.json (version: null — dev no-op SW)"
echo "    precompress (release flow) replaces it with the real content hash"

echo "==> done: wasm/pkg/gpui_starter.js + wasm/pkg/gpui_starter_bg.wasm"
if [[ "${PROFILE_DIR}" == "release" ]]; then
    echo "    precompress for serving (gzip/brotli + version.json):"
    echo "      node wasm/precompress.mjs   (or: just wasm-release)"
fi
echo "    serve with: just wasm-serve  (or bash wasm/serve.sh)"
