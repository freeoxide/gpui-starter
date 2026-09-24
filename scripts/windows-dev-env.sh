#!/usr/bin/env bash
#
# Windows (Git Bash / MSYS) MSVC build environment for gpui-starter.
#
# Two problems this solves on a machine that has VS Build Tools but no
# elevated shell:
#   1. /usr/bin/link.exe (coreutils) shadows MSVC's linker, so rustc links
#      against GNU `link` and fails.
#   2. The Windows SDK may not be installed. Without it link.exe cannot open
#      kernel32.lib / ucrt.lib. If no installed SDK is found, an xwin "splat"
#      (the official SDK + CRT MSIs, extracted without admin rights) is used.
#
# Source it, then build:
#   . scripts/windows-dev-env.sh
#   cargo build && cargo run
#
# One-time setup downloads the SDK via xwin (~700 MB) next to the repo:
#   scripts/windows-dev-env.sh --setup
#
# Optional: write .cargo/config.toml so plain `cargo build`/`run`/`test` work
# in EVERY shell (PowerShell, cmd, Git Bash) with no sourcing — the config
# carries the linker, INCLUDE/LIB, CC/CXX/AR and RC paths into every build:
#   scripts/windows-dev-env.sh --cargo-config
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
XWIN_SPLAT="${XWIN_SPLAT:-$REPO_ROOT/../.xwin/splat}"
VSWHERE="/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe"

fail() { echo "windows-dev-env: $*" >&2; exit 1; }

msvc_toolset_dir() {
  [ -x "$VSWHERE" ] || fail "vswhere not found; install VS 2022 Build Tools with the C++ workload"
  local root
  root="$("$VSWHERE" -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 \
    -property installationPath | tr -d '\r')" || true
  [ -n "$root" ] || fail "no MSVC toolset (VC.Tools.x86.x64) found by vswhere"
  local best="" d
  for d in "$root/VC/Tools/MSVC"/*/; do
    [ -d "$d" ] || continue
    best="$d" # glob sorts ascending; keep the last
  done
  [ -n "$best" ] || fail "no VC/Tools/MSVC/*/ directory in $root"
  printf '%s' "${best%/}"
}

# First installed Windows 10 SDK with libs, printed as a Windows-style root
# ("<root>/Lib/<ver>" exists), or empty.
installed_sdk() {
  local root="/c/Program Files (x86)/Windows Kits/10" lib
  [ -d "$root/Lib" ] || return 0
  for lib in "$root/Lib"/*/; do
    [ -d "${lib}um/x64" ] || continue
    cygpath -w "${lib}" | sed 's|\\Lib\\.*||'
    return 0
  done
}

# ---------------------------------------------------------------------------
# --setup: fetch the SDK via xwin if there is no usable local one.
# ---------------------------------------------------------------------------
if [ "${1:-}" = "--setup" ]; then
  if [ -n "$(installed_sdk)" ]; then
    echo "Installed Windows SDK found; nothing to do. Source the script to build:"
    echo "  . scripts/windows-dev-env.sh"
    exit 0
  fi
  command -v xwin >/dev/null 2>&1 || {
    echo "==> Downloading xwin (prebuilt) to ~/.local-tools"
    mkdir -p "$HOME/.local-tools"
    curl -sL -o "$HOME/.local-tools/xwin.tar.gz" \
      "https://github.com/Jake-Shadle/xwin/releases/download/0.10.0/xwin-0.10.0-x86_64-pc-windows-msvc.tar.gz" \
      || fail "xwin download failed; check network or install xwin via cargo"
    tar xzf "$HOME/.local-tools/xwin.tar.gz" -C "$HOME/.local-tools" --strip-components=1 \
      xwin-*-x86_64-pc-windows-msvc/xwin.exe
    rm "$HOME/.local-tools/xwin.tar.gz"
  }
  CACHE="$REPO_ROOT/../.xwin-cache"
  echo "==> Splatting Windows SDK (x86_64) into $XWIN_SPLAT (~700 MB download, one-time)"
  # Cache and output must share a drive: splat moves (renames) files.
  xwin --accept-license --cache-dir "$(cygpath -w "$CACHE")" \
    splat --output "$(cygpath -w "$XWIN_SPLAT")"

  # rc.exe is not part of the splat (headers/libs only); gpui's build script
  # needs it for its app manifest. The SDK BuildTools nupkg is a plain zip.
  RC_DIR="$XWIN_SPLAT/../sdktools"
  if [ ! -x "$RC_DIR/rc.exe" ]; then
    echo "==> Fetching rc.exe from the Windows SDK BuildTools package"
    mkdir -p "$RC_DIR"
    curl -sL -o "$RC_DIR/buildtools.nupkg" \
      "https://download.visualstudio.microsoft.com/download/pr/42786999-d45b-4428-b946-248bb9676505/db432106a12e44fc1055f520b147df183c86963ab79baf81d8a4a9dd867e6606/microsoft.windows.sdk.buildtools.10.0.26100.1742.nupkg" \
      || fail "SDK BuildTools download failed"
    unzip -o -q "$RC_DIR/buildtools.nupkg" "bin/*/x64/rc.exe" "bin/*/x64/rcdll.dll" -d "$RC_DIR"
    mv "$RC_DIR"/bin/*/x64/{rc.exe,rcdll.dll} "$RC_DIR/" && rm -r "$RC_DIR/bin" "$RC_DIR/buildtools.nupkg"
  fi
  echo "==> Done. Source the script and build:"
  echo "  . scripts/windows-dev-env.sh"
  exit 0
fi

# ---------------------------------------------------------------------------
# --cargo-config: generate .cargo/config.toml so no sourcing is ever needed.
# ---------------------------------------------------------------------------
if [ "${1:-}" = "--cargo-config" ]; then
  MSVC_DIR="$(msvc_toolset_dir)"
  MSVC_WIN="$(cygpath -w "$MSVC_DIR")"
  BIN_WIN="$MSVC_WIN\\bin\\Hostx64\\x64"
  SDK_ROOT="$(installed_sdk)"
  if [ -n "$SDK_ROOT" ]; then
    SDK_VER="$(ls "$SDK_ROOT/Lib" | tail -1)"
    INC="$(cygpath -w "$SDK_ROOT/Include/$SDK_VER")"
    LIBS="$(cygpath -w "$SDK_ROOT/Lib/$SDK_VER")"
    INCLUDE="$(cygpath -w "$MSVC_DIR/include");$INC\\um;$INC\\shared;$INC\\ucrt;$INC\\winrt;$INC\\cppwinrt"
    LIB="$(cygpath -w "$MSVC_DIR/lib/x64");$LIBS\\um\\x64;$LIBS\\ucrt\\x64"
    RC_WIN="$(cygpath -w "$SDK_ROOT/bin/$SDK_VER/x64/rc.exe")"
  else
    [ -d "$XWIN_SPLAT/sdk/lib" ] || fail "no installed SDK and no splat at $XWIN_SPLAT; run: scripts/windows-dev-env.sh --setup"
    SDK_INC="$(cygpath -w "$XWIN_SPLAT/sdk/include")"
    SDK_LIB="$(cygpath -w "$XWIN_SPLAT/sdk/lib")"
    CRT_INC="$(cygpath -w "$XWIN_SPLAT/crt/include")"
    CRT_LIB="$(cygpath -w "$XWIN_SPLAT/crt/lib/x86_64")"
    RC_DIR="$XWIN_SPLAT/../sdktools"
    [ -x "$RC_DIR/rc.exe" ] || fail "rc.exe missing at $RC_DIR; run: scripts/windows-dev-env.sh --setup"
    INCLUDE="$CRT_INC;$SDK_INC\\um;$SDK_INC\\shared;$SDK_INC\\ucrt;$SDK_INC\\winrt;$SDK_INC\\cppwinrt"
    LIB="$CRT_LIB;$SDK_LIB\\um\\x86_64;$SDK_LIB\\ucrt\\x86_64"
    RC_WIN="$(cygpath -w "$RC_DIR/rc.exe")"
  fi

  mkdir -p "$REPO_ROOT/.cargo"
  CONFIG="$REPO_ROOT/.cargo/config.toml"
  cat > "$CONFIG" <<EOF
# Generated by scripts/windows-dev-env.sh --cargo-config; not committed.
# Cargo injects these into every build (any shell), replacing the need to
# source the env script: linker beats Git Bash's coreutils link.exe shadow,
# INCLUDE/LIB point at the SDK, CC/CXX/AR at cl.exe/lib.exe, RC at rc.exe.

[target.x86_64-pc-windows-msvc]
linker = '$BIN_WIN\\link.exe'

[env]
INCLUDE = '$INCLUDE'
LIB = '$LIB'
CC_x86_64_pc_windows_msvc = '$BIN_WIN\\cl.exe'
CXX_x86_64_pc_windows_msvc = '$BIN_WIN\\cl.exe'
AR_x86_64_pc_windows_msvc = '$BIN_WIN\\lib.exe'
RC = '$RC_WIN'
EOF
  echo "==> Wrote $CONFIG"
  echo "    Plain cargo build/run/test now works in every shell; config is gitignored."
  exit 0
fi

# ---------------------------------------------------------------------------
# Exported environment (source mode).
# ---------------------------------------------------------------------------
MSVC_DIR="$(msvc_toolset_dir)"
MSVC_BIN="$(cygpath -u "$(cygpath -w "$MSVC_DIR")/bin/Hostx64/x64")"
[ -x "$MSVC_BIN/cl.exe" ] || fail "cl.exe missing under $MSVC_DIR (partial toolset install?)"

export PATH="$MSVC_BIN:$PATH"
INCLUDE="$(cygpath -w "$MSVC_DIR/include")"
LIB="$(cygpath -w "$MSVC_DIR/lib/x64")"

SDK_ROOT="$(installed_sdk)"
if [ -n "$SDK_ROOT" ]; then
  SDK_VER="$(ls "$SDK_ROOT/Lib" 2>/dev/null | tail -1)" # installed layout: Lib/<ver>/
  INC="$SDK_ROOT/Include/$SDK_VER"; LIBS="$SDK_ROOT/Lib/$SDK_VER"
  INCLUDE+=";$INC/um;$INC/shared;$INC/ucrt;$INC/winrt;$INC/cppwinrt"
  LIB+=";$LIBS/um/x64;$LIBS/ucrt/x64"
  export PATH="$SDK_ROOT/bin/$SDK_VER/x64:$PATH" # rc.exe, mt.exe
else
  [ -d "$XWIN_SPLAT/sdk/lib" ] || fail "no installed SDK and no splat at $XWIN_SPLAT; run: scripts/windows-dev-env.sh --setup"
  SDK_INC="$(cygpath -w "$XWIN_SPLAT/sdk/include")"
  SDK_LIB="$(cygpath -w "$XWIN_SPLAT/sdk/lib")"
  CRT_INC="$(cygpath -w "$XWIN_SPLAT/crt/include")"
  CRT_LIB="$(cygpath -w "$XWIN_SPLAT/crt/lib/x86_64")"
  # xwin CRT headers/libs (14.4x) replace the toolset's so cl.exe/link.exe and
  # the CRT stay on one version.
  INCLUDE="$CRT_INC;$SDK_INC/um;$SDK_INC/shared;$SDK_INC/ucrt;$SDK_INC/winrt;$SDK_INC/cppwinrt"
  LIB="$CRT_LIB;$SDK_LIB/um/x86_64;$SDK_LIB/ucrt/x86_64"
  RC_DIR="$XWIN_SPLAT/../sdktools"
  [ -x "$RC_DIR/rc.exe" ] || fail "rc.exe missing at $RC_DIR; run: scripts/windows-dev-env.sh --setup"
  export PATH="$RC_DIR:$PATH" # rc.exe for gpui's manifest build step
fi
export INCLUDE LIB

echo "windows-dev-env: MSVC $(basename "$MSVC_DIR") + $([ -n "$SDK_ROOT" ] && echo "installed SDK $SDK_VER" || echo "xwin splat")"
echo "windows-dev-env: PATH/INCLUDE/LIB exported; cargo build away"
