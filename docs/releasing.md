# Releasing

What a gpui-starter release contains, how to cut one, per-format notes, and
the decisions the release work left open. The underlying research lives in
[research/release-matrix.md](research/release-matrix.md) (the full format and
channel landscape), [research/linux-packages.md](research/linux-packages.md)
(deb, rpm, AppImage, Flatpak, snap, AUR, nix decisions), and
[research/icons-and-windows.md](research/icons-and-windows.md) (icon pipeline
and Windows packaging).

## The counted answer

There are 15 desktop artifact formats and 14 distribution channels in
practice, 29 delivery paths total ([research/release-matrix.md](research/release-matrix.md)).
No project ships all of them; Zed, the largest GPUI app, builds only 3 itself.

This repo ships 10 of the 15 formats as 18 release assets per release:

- Linux (6): tarball, deb, rpm, PKGBUILD (the pacman path, generated inside
  the tarball rather than shipped as a built package), Flatpak bundle,
  AppImage.
- macOS (1): DMG, universal plus per-arch.
- Windows (3): portable zip, bare exe, Inno Setup exe.

Skipped: snap, macOS `.app`-in-zip, macOS `.pkg`, Windows `.msi`, `.msix`.
Rationale in [research/release-matrix.md](research/release-matrix.md).

Of the 14 channels the project hosts none itself: GitHub Releases carries the
artifacts, a gh-pages manifest feeds the in-app updater, the nix flake builds
from any commit, and AUR and winget entries are documented follow-ups.

## The 18 release assets

`<ver>` is the release version, for example `0.2.1`. macOS, AppImage, Flatpak
and Windows names use the `GPUI-Starter-` prefix; tarball, deb and rpm use
lowercase `gpui-starter` (deb with underscores, rpm with dashes and a `-1`
release tag).

| Asset | Platform | Installs via | Install |
|---|---|---|---|
| `GPUI-Starter-<ver>.dmg` | macOS, universal (aarch64 + x86_64); the default download | drag from the mounted DMG to Applications | `open GPUI-Starter-<ver>.dmg` |
| `GPUI-Starter-<ver>-aarch64.dmg` | macOS Apple Silicon | same | `open GPUI-Starter-<ver>-aarch64.dmg` |
| `GPUI-Starter-<ver>-x86_64.dmg` | macOS Intel | same | `open GPUI-Starter-<ver>-x86_64.dmg` |
| `gpui-starter-<ver>-x86_64-linux.tar.gz` | Linux x86_64, PKGBUILD inside | run in place, no root | `tar -xzf <file> && cd gpui-starter && ./install-linux.sh` |
| `gpui-starter-<ver>-aarch64-linux.tar.gz` | Linux aarch64, PKGBUILD inside | run in place, no root | `tar -xzf <file> && cd gpui-starter && ./install-linux.sh` |
| `gpui-starter_<ver>-1_amd64.deb` | Linux amd64 | apt / dpkg | `sudo apt install ./gpui-starter_<ver>-1_amd64.deb` |
| `gpui-starter_<ver>-1_arm64.deb` | Linux arm64 | apt / dpkg | `sudo apt install ./gpui-starter_<ver>-1_arm64.deb` |
| `gpui-starter-<ver>-1.x86_64.rpm` | Linux x86_64 | dnf / rpm / zypper | `sudo dnf install ./gpui-starter-<ver>-1.x86_64.rpm` |
| `gpui-starter-<ver>-1.aarch64.rpm` | Linux aarch64 | dnf / rpm / zypper | `sudo dnf install ./gpui-starter-<ver>-1.aarch64.rpm` |
| `GPUI-Starter-<ver>-x86_64.AppImage` | Linux x86_64 | self-contained | `chmod +x <file> && ./<file>` |
| `GPUI-Starter-<ver>-x86_64.AppImage.zsync` | delta index for the AppImage above | AppImageUpdate | not installed manually |
| `GPUI-Starter-<ver>-aarch64.AppImage` | Linux aarch64 | self-contained | `chmod +x <file> && ./<file>` |
| `GPUI-Starter-<ver>-aarch64.AppImage.zsync` | delta index | AppImageUpdate | not installed manually |
| `GPUI-Starter-<ver>-x86_64.flatpak` | Linux x86_64 | flatpak (the bundle embeds the runtime repo) | `flatpak install --user ./GPUI-Starter-<ver>-x86_64.flatpak` |
| `GPUI-Starter-<ver>-x86_64-windows.exe` | Windows x86_64 | nothing; run directly | download and run (also the updater's swap target) |
| `GPUI-Starter-<ver>-x86_64-windows.zip` | Windows x86_64 portable | unzip | `Expand-Archive <file>` then run `gpui-starter.exe` |
| `GPUI-Starter-<ver>-x86_64-windows-setup.exe` | Windows x86_64 | Inno Setup, per-user, no admin | run the installer |
| `SHA256SUMS.txt` | all | checksums for the 17 files above | `sha256sum -c SHA256SUMS.txt` |

`install-linux.sh` registers the desktop entry and the app icon per-user
(XDG dirs). The tarball ships the single 512px png, so that is the icon the
script installs (hicolor/512x512/apps); the binary itself runs from wherever
the tarball was extracted, so symlink it into `~/.local/bin` yourself. CI
also produces `.sha256` sidecars, but the release ships only
`SHA256SUMS.txt`.

## Cutting a release

### Trigger

`.github/workflows/release.yml` fires on a pushed `v*` tag or on
`workflow_dispatch` with a `version` input. The justfile release ops target
the fork `hmziqagent/gpui-starter` (git remote `fork`); the update URL baked
into binaries derives from whichever repository actually runs the workflow.

Version discipline: `scripts/normalize-version.sh` strips one leading `v` and
validates `X.Y.Z` in every job. The deb and rpm filenames embed the
Cargo.toml version (neither cargo-deb nor cargo-generate-rpm can override
it), so a dispatch version that differs from Cargo.toml fails the SHA256SUMS
step loudly instead of publishing a partial release.

### justfile release ops

| Recipe | What it does |
|---|---|
| `just force-release [version]` | Moves the tag (default: latest `v*` tag) to HEAD, deletes and re-pushes it to the fork, the reliable way to fire a fresh tag-push event. Commit first: the tag marks a commit, not your working tree. |
| `just dispatch-release [version] [ref]` | Runs the workflow via `gh workflow run` on a ref (default `feat/linux-infra`) without moving any tag. For rebuilding from a branch tip. |
| `just watch-release [runid]` | `gh run watch` on the latest (or the given) Release run; exits non-zero if the run fails. |
| `just release-runs [limit]` | Lists recent Release runs on the fork. |

Re-publishing is an upsert: `softprops/action-gh-release` replaces assets and
regenerates notes on the existing release, so no manual release deletion is
needed.

### Pipeline

build-macos, build-linux and build-windows build with the nightly toolchain
and share kache plus rust-cache caching. build-flatpak compiles inside the
sandbox with the runtime's rust-stable extension and caches only its flatpak
user installation (actions/cache); publish-release builds nothing.

| Job | Runner | Produces |
|---|---|---|
| build-macos | macos-latest, both arches | universal + aarch64 + x86_64 `.app`, three DMGs |
| build-linux | matrix: ubuntu-24.04 (x86_64), ubuntu-24.04-arm (aarch64) | tarball + generated PKGBUILD, AppImage + `.zsync`, deb, rpm |
| build-windows | windows-2025 | bare exe, portable zip, Inno setup exe |
| build-flatpak | ubuntu-24.04 inside `ghcr.io/flathub-infra/flatpak-github-actions:freedesktop-25.08` (`--privileged` container) | x86_64 `.flatpak` bundle |
| publish-release | ubuntu-24.04, needs all four | `SHA256SUMS.txt`, the GitHub Release with the 18 files, `manifest.json` pushed to gh-pages |

Two Windows constraints are documented in the workflow itself. The cargo
build runs in a pwsh step because Git Bash puts its own `link.exe` (coreutils)
ahead of the MSVC linker; the bash packaging scripts then only hit the build
cache. Job-level `CARGO_HOME: C:\cargo` keeps the longest checkout path under
MAX_PATH (the default cargo home would exceed it by 6 characters).

Update URL policy: `GPUI_UPDATE_MANIFEST_URL` is baked in at compile time
(`option_env!` in `src/services/updater/types.rs`). It is set for tarball,
AppImage and Windows builds, and unset (`env -u`) for deb, rpm and flatpak
builds where the system package manager owns updates. Every build script
hard-fails if the policy is violated in its direction. Inside build-linux the
AppImage step runs before deb and rpm so the URL flip costs one relink
instead of three (`option_env!` is dep-info tracked).

### Secrets

| Secret | Consumed by | Behavior when absent |
|---|---|---|
| `APPLE_DEV_CERT_P12` + `APPLE_CERT_PASSWORD` | certificate import, codesign | DMG apps get ad-hoc signatures (`codesign --sign -`) |
| `APPLE_SIGNING_IDENTITY` | codesign identity name | falls back to `Developer ID Application` |
| `APPLE_ID` + `APPLE_APP_PASSWORD` + `APPLE_TEAM_ID` | notarytool + stapler | notarization step is skipped unless the cert and `APPLE_ID` are both present |
| `UPDATE_SIGNING_KEY` | Ed25519 signing in `scripts/generate-manifest.sh` | manifest ships with empty signature fields; the release still publishes. A set-but-broken key hard-fails |
| `GITHUB_TOKEN` | release API, gh-pages push | automatic, never configured by hand |

One-time manual step: enable GitHub Pages on the `gh-pages` branch, otherwise
the baked manifest URL never resolves.

### Auto-update coverage

`manifest.json` on gh-pages carries three platform rows, keyed like
`platform_key()` in `src/services/updater/types.rs`:

- `macos-aarch64` and `macos-x86_64` both point at the universal DMG (one
  file, one shared signature).
- `windows-x86_64` points at the bare exe, because the updater swaps single
  files, not archives.
- No Linux row, by policy. deb, rpm and flatpak binaries compile the URL out;
  the AppImage carries embedded `gh-releases-zsync` information instead. The
  tarball and AppImage binaries do query the manifest and then report "no
  asset for platform linux-x86_64"; that error is expected and pre-existing.

### First CI run watch list

The pipeline has never executed end to end; everything was validated against
tool sources and stand-in runs on a Linux box. Watch these on the first run
(a `workflow_dispatch` on the fork):

1. build-linux apt step: zsync, desktop-file-utils and appstream install, and
   both validators pass on the real desktop + metainfo pair.
2. cargo-deb 3.8.0 and cargo-generate-rpm 0.21.0 install from source (~1-3
   minutes each), then the first real `cargo deb --no-build` and
   `cargo generate-rpm` over the release binary; check the emitted filenames
   against the asset table.
3. AppImage: first real-binary linuxdeploy run and zsyncmake `.zsync` output.
4. build-flatpak: container pull, `--privileged` bwrap, pip availability
   (`--break-system-packages` fallback), `flatpak --user` as root in the
   container, flatpak-builder resolving the system-preinstalled 25.08 SDK
   against the `--user` rust-stable extension download, cache hit on
   `/root/.local/share/flatpak` from run 2 onward.
5. build-windows: first kache-wrapped Windows build, winresource rc.exe icon
   embed, ISCC probe (PATH then `C:\Program Files (x86)\Inno Setup 6`) and
   `MSYS2_ARG_CONV_EXCL` flag passing, runner disk headroom, the System32
   bsdtar zip path, and the claim that the bash cargo build is a pure cache
   hit (a fingerprint divergence would rerun the link under bash and fail at
   the documented spot).
6. publish-release: the 18-file list matches the downloads exactly (the
   SHA256SUMS step hard-fails on any name drift, for example a dispatch
   version that differs from the Cargo.toml version), and manifest generation
   finds the DMG and bare exe on the just-created release.
7. gen-icons.sh is not exercised by this workflow at all (releases consume
   committed artifacts); its real-tool path stays unverified until someone
   runs it (see open decisions).

## Per-format notes

### Linux tarball

Binary plus a `lib/` directory of bundled non-glibc shared libraries
(Wayland, XCB, the Vulkan loader and friends), rpath `$ORIGIN/lib`, glibc
left to the host. The glibc floor is the build host: ubuntu-24.04, so glibc
2.39. `install-linux.sh` ships inside and registers the desktop entry and
the single 512px png icon per-user (the script installs a full hicolor set
when one sits next to it, but the workflow stages only
`resources/gpui-starter.png` into the tarball); it never needs root. The
deb, rpm and flatpak carry the full hicolor set; the AppImage carries the
512px png as its icon.

### AUR / pacman

Every tarball contains a generated PKGBUILD for manual `makepkg` use. Its
`package()` step re-rpaths the binary to `$ORIGIN/../lib/gpui-starter` to
match the `/usr/bin` + `/usr/lib/gpui-starter` install layout, so makepkg
needs `patchelf` (listed in makedepends). An AUR `gpui-starter-bin` entry is
a later community step: it needs the real sha256 (SKIP is not allowed for
tarball sources) plus a `.SRCINFO`.

### deb

Built by cargo-deb 3.8.0 from `[package.metadata.deb]` in Cargo.toml:
`/usr/bin/gpui-starter` plus bundled libs under `/usr/lib/gpui-starter`,
rpath `$ORIGIN/../lib/gpui-starter`. `Depends: $auto` with
`Suggests: mesa-vulkan-drivers`: every non-glibc NEEDED library is bundled
and resolved via rpath, and the Suggests documents the one piece that cannot
be bundled, the Vulkan installable client driver. No maintainer scripts; dpkg
triggers refresh the desktop and icon caches on their own. lintian reports
custom-library-search-path and embedded-library, the accepted cost of the
bundling strategy for a third-party deb. The updater URL is compiled out.

### rpm

Built by cargo-generate-rpm 0.21.0 from `[package.metadata.generate-rpm]`,
same file layout as the deb. Requires stay auto-detected (rpm auto-req over
the binary's NEEDED entries and GLIBC symbols). Unsigned; rpmsign only
matters for gpgcheck-enabled repos.

### AppImage

Built by `scripts/build-appimage.sh` with sha256-pinned linuxdeploy and
plugin-appimage over the tarball layout, with a custom AppRun. The updater
URL stays set: update information is embedded as
`gh-releases-zsync|hmziqagent|gpui-starter|latest|GPUI-Starter-*-<arch>.AppImage.zsync`
(override with `APPIMAGE_UPDATE_INFORMATION`). Users need FUSE 2: Ubuntu
22.04 and later stopped preinstalling it (`sudo apt install libfuse2` or
`libfuse2t64`), or run with `--appimage-extract-and-run`. The `.zsync`
sidecar enables delta updates in AppImageUpdate. CI builds with
`APPIMAGE_EXTRACT_AND_RUN=1` because hosted runners lack `/dev/fuse`.

### Flatpak

A local bundle, not Flathub, because the app id `com.gpui-starter.app`
violates three Flathub ID rules: `.app` is a banned generic suffix, a dash is
only allowed in the last component, and the project does not control
`gpui-starter.com`. Flathub requires `io.github.<owner>.<Product>`; renaming
is a project-level decision (see open decisions). The manifest lives at
`packaging/flatpak/com.gpui-starter.app.yml`, runtime org.freedesktop 25.08,
and the build enables `--features notifications-portal` (the ashpd XDG portal
notification backend, the correct notification path inside a sandbox). The
bundle embeds the Flathub runtime repo, so a plain
`flatpak install --user ./bundle` can fetch the Platform. x86_64 only for
now: the ~2GB rust-stable extension installs per `--user` cache, and aarch64
is a matrix duplicate on ubuntu-24.04-arm once this lane proves out. Bundles
are unsigned (no GPG key), so installs take the untrusted-bundle prompt.

### macOS

One universal DMG (lipo of the aarch64 and x86_64 builds, the default
download) plus two slim per-arch DMGs. The icns ships committed at
`assets/app-icon/gpui-starter.icns`, so CI needs no icon tooling. Developer
ID codesign and notarization are conditional on the `APPLE_*` secrets;
without them the apps are ad-hoc signed and Gatekeeper shows the
click-through.

### Windows

The Inno Setup installer (`packaging/windows/gpui-starter.iss`) installs
per-user into `%LOCALAPPDATA%\Programs\GPUI-Starter`
(`PrivilegesRequired=lowest`, `{autopf}`), needs no admin, and deliberately
registers no protocols or file associations: the app registers
`gpui-starter://` under HKCU and writes its own Start Menu shortcut on every
startup, and that shortcut carries
`System.AppUserModelID = com.gpui-starter.app`, without which Windows
silently drops every toast. Installer and app converge on the same files.
Per-user install is also what keeps the updater's destination writable. The
portable zip holds the exe, LICENSE and README; the bare exe exists as its
own asset because the updater swaps single files. The exe icon is embedded at
build time by `build.rs` via winresource. Signing is off by default: unsigned
exes show the SmartScreen click-through until download reputation builds,
and the Azure Artifact Signing upgrade path is documented in
`packaging/windows/README.md`. The installer's AppId GUID is
`uuid5(DNS, "com.gpui-starter.app")` and must never change: a new GUID means
a new app to Inno, so upgrades would install side by side.

### Nix

`flake.nix` builds from source on any commit (buildRustPackage, with a
postFixup patchelf adding the nix store's vulkan-loader and wayland to the
RUNPATH). No lockfile ships, so run `nix flake lock` once. Nix needs no
release artifact; inclusion in nixpkgs proper would be a community PR.

## Open decisions and deferred items

Everything the release work deliberately left for a maintainer, one line
each.

### Functional gaps (src/ changes, outside release infrastructure)

1. Windows updater swap: `src/services/updater/apply.rs` performs the swap
   with a single `fs::rename` onto the running exe, which Windows refuses.
   The fix is a two-step rename dance (running exe to `.old`, new exe into
   place, delete `.old` next launch) behind a windows cfg branch. Until it
   lands, Windows auto-update is download-verify-only. Recipe:
   `packaging/windows/README.md`.
2. gen-icons.sh real-tool path (rsvg-convert, png2icns, ImageMagick) has
   never executed: the dev box lacks the tools and the committed artifacts
   came from an offline stand-in chain. First real run must confirm the
   png2icns output via the in-script `icns2png -l` self-check. Releases are
   unaffected (CI consumes committed artifacts).

### Packaging decisions still open

3. Flathub app id: renaming to `io.github.<owner>.<Product>` diverges from
   the desktop id used by notify-rust's DesktopEntry hint and from the update
   manifest, so it is a project-level call, not a packaging detail. Detail:
   [research/linux-packages.md](research/linux-packages.md).
4. Metainfo screenshots and branding elements are absent (Flathub expects
   them, appstreamcli does not require them); add them alongside the app-id
   decision.
5. Keyring inside the flatpak sandbox: finish-args grant no
   `--talk-name=org.freedesktop.secrets`, so the Secret Service backend
   likely degrades to its mock store; portals cover notifications, not
   Secret Service.
6. Snap stays skipped. Reviving it needs a store account, manual-review name
   registration, live gpu-2404 driver testing, and LXD in CI. Detail:
   [research/linux-packages.md](research/linux-packages.md).
7. winget entry: a community PR to microsoft/winget-pkgs after the first
   tagged release, portable-zip flavor, byte-exact sha256. Detail:
   `packaging/windows/README.md`.
8. AppImage update channel: the embedded zsync information targets the
   fork's `latest` release; override `APPIMAGE_UPDATE_INFORMATION` if
   releases move to another repository or a stable channel is preferred.
9. glibc floor: the deb and rpm floors equal the build host (2.39). Widening
   via a debian:bookworm container lane is possible and so far unprompted.
10. Flatpak source staging: untracked root files (bun.lock, package.json,
    .zcodeignore, about 2 KB total) still ride the dir source; add them to
    the skip list if the manifest is ever edited again.
11. CHANGELOG.md has no 0.2.1 section (it jumps from 0.2.0 to 0.3.0), and
    the git tags run ahead of Cargo.toml: v0.2.2 and v0.2.3 exist, both
    pointing at the same commit, while Cargo.toml sits at 0.2.1; the
    metainfo release notes currently derive from git log. Reconcile.
12. Windows code signing stays off. Azure Artifact Signing (Basic at USD
    9.99 per month, individual onboarding limited to the USA and Canada) is
    the documented upgrade path. Detail: `packaging/windows/README.md`.
13. The Inno publisher URLs point at the origin (freeoxide/gpui-starter)
    while releases dispatch from the fork (hmziqagent); revisit when winget
    publisher consistency matters.
14. flatpak-cargo-generator is installed unpinned in CI (0.1.4 was current
    during research); pin it if the lane drifts.
15. Tarball icon set: the workflow stages only the single
    `resources/gpui-starter.png` into the tarball, so tarball installs get a
    512-only icon. Staging `assets/app-icon/hicolor/` into the tarball (the
    script already probes for a `hicolor/` directory next to itself) would
    match the full set the deb, rpm and flatpak carry. One `cp` in the
    release workflow's staging step.

### Accepted nits (recorded, no action expected)

16. The deb ships the LICENSE text twice (explicit asset plus cargo-deb's
    auto-generated `usr/share/doc/gpui-starter/copyright`); drop the asset
    line if the duplication bothers anyone.
17. Style nits kept for local convention: gen-icons.sh's 4-line header,
    install-linux.sh exiting half-installed on a malformed tarball this repo
    never produces, the 3-line justfile flatpak recipe comment.
18. Research-doc data nits: the Azure Premium price figure is unsourced (the
    Basic USD 9.99 figure is sourced); the Inno-versus-NSIS question was open
    in research and is now settled (Inno shipped); the libvulkan.so.1
    in-Platform claim is inferred from the runtime's dependency tree rather
    than shown on the cited page.
19. The justfile release-ops comment says `softprops/action-gh-release@v2`
    while release.yml pins @v3 (this doc follows the workflow); the fix is a
    one-word comment edit, user's call.
