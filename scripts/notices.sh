#!/usr/bin/env bash
#
# Regenerate THIRD-PARTY-NOTICES: crate licenses from Cargo.lock via
# cargo-about, plus the shipped assets/ license texts, each input stamped
# with its SHA-256 so a review shows exactly which licenses the build carries.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

need() {
    echo "Error: '$1' not found — install it first:" >&2
    echo "  $2" >&2
    exit 1
}

INPUTS=(
    Cargo.lock
    Cargo.toml
    LICENSE
    about.toml
    assets/fonts/OFL.txt
    assets/icons/LICENSE-LUCIDE
)

# Shipped third-party assets: name|upstream|license file.
COMPONENTS=(
    "Noto Sans|https://github.com/googlei18n/noto-fonts|assets/fonts/OFL.txt"
    "Lucide icons|https://lucide.dev|assets/icons/LICENSE-LUCIDE"
)

about="${CARGO_ABOUT:-cargo-about}"
command -v "$about" >/dev/null 2>&1 || need "$about" "cargo install cargo-about --version 0.9.3 --locked"
# Pinned: cargo-about output is not stable across releases, so neither is
# this file if the version drifts.
if [[ "$("$about" --version)" != "cargo-about 0.9.3" ]]; then
    echo "Error: need cargo-about 0.9.3, got '$("$about" --version)'" >&2
    exit 1
fi

# --fail makes an unaccepted license (about.toml) exit non-zero, so a new
# dep with an unvetted license cannot slip through regeneration.
crates="$("$about" generate --locked --fail scripts/notices.hbs)"

# Strip CR so hashes match across checkouts with different line endings.
sha() { tr -d '\r' < "$1" | sha256sum | cut -d' ' -f1; }

{
    printf 'gpui-starter third-party notices\n'
    printf '=================================\n'
    printf 'Regenerate with just notices.\n\n'
    for path in "${INPUTS[@]}"; do
        printf 'Input-SHA256: %s %s\n' "$(sha "$path")" "$path"
    done
    printf '\n%s\n' "$crates"
    for component in "${COMPONENTS[@]}"; do
        IFS='|' read -r name source file <<<"$component"
        printf '\n----------------------------------------------------------------------\n'
        printf '%s\n%s\n\n' "$name" "$source"
        printf -- '--- %s ---\n%s\n\n' "$file" "$(tr -d '\r' < "$file")"
    done
} > THIRD-PARTY-NOTICES

echo "==> Wrote THIRD-PARTY-NOTICES"
