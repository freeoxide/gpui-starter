# Release format matrix for desktop apps (Windows, macOS, Linux)

Research snapshot: 2026-10-09. Sources are inline; claims about specific projects
were verified against the linked page, not search snippets.

## The counted answer

There are two different things people call a "release format":

1. **Artifact formats**: files you build and upload. 15 exist in practice.
   - Linux (7): tarball, .deb, .rpm, pacman package (.pkg.tar.zst / PKGBUILD),
     Flatpak, Snap, AppImage.
   - macOS (3): .app in a .dmg, .app in a .zip, .pkg installer.
   - Windows (5): portable .zip, bare portable .exe, setup .exe installer
     (Inno Setup or NSIS), .msi, .msix.
2. **Distribution channels**: repos and stores that host or point at those
   artifacts. 14 exist in practice: apt repo, dnf/zypper repo, Arch official
   repos, AUR, Flathub, Snap Store, Homebrew, Nix, winget, Chocolatey, Scoop,
     Conda, Microsoft Store, Mac App Store.

Total: 15 artifact formats plus 14 channels = **29 delivery paths**. No project
ships all of them. Zed, the largest GPUI app, builds only 3 artifact formats
itself (M1 dmg, W3 Inno Setup exe, L1 tarball) and lets community packagers
handle every channel (source: [zed.dev/docs/linux](https://zed.dev/docs/linux),
[zed releases](https://github.com/zed-industries/zed/releases)).

## Master matrix

"Who ships it" lists verified examples; GPUI apps are marked (gpui).

| # | Format | Platform | Who ships it | Signing story | Auto-update story | Source |
|---|--------|----------|--------------|---------------|-------------------|--------|
| L1 | Tarball (.tar.gz / .tar.xz) | Linux | Zed, Helix, Lapce, every GPUI app below | SHA256SUMS optional; OpenLogi adds minisign | In-app updater possible (Zed does this) | [zed linux docs](https://zed.dev/docs/linux), [helix 25.07.1](https://github.com/helix-editor/helix/releases/tag/25.07.1) |
| L2 | .deb | Linux | Lapce (6 distro targets x 2 arches), OpenLogi (gpui), zedis (gpui) | dpkg-sig rare; SHA256 checksums usual | apt owns updates; no self-update | [lapce 0.4.6](https://github.com/lapce/lapce/releases/tag/v0.4.6), [openlogi](https://github.com/AprilNEA/OpenLogi/releases/latest) |
| L3 | .rpm | Linux | Lapce (fedora41/42), OpenLogi (gpui), zedis (gpui) | rpmsign rare | dnf/zypper own updates | same release pages |
| L4 | Pacman package (.pkg.tar.zst) / PKGBUILD | Linux | OpenLogi ships .pkg.tar.zst directly (gpui); Zed via Arch extra + AUR | makepkg sha256sums | pacman/AUR helper owns updates | [openlogi](https://github.com/AprilNEA/OpenLogi/releases/latest), [arch package](https://archlinux.org/packages/extra/x86_64/zed/) |
| L5 | Flatpak | Linux | Flathub dev.zed.Zed (community, unverified); sonora (gpui) ships raw .flatpak bundles | Flathub holds signing keys; direct bundles self-signed | System-managed (flatpak update); app cannot self-update | [flathub](https://flathub.org/apps/dev.zed.Zed), [sonora](https://github.com/sonorahq/sonora/releases/latest) |
| L6 | Snap | Linux | No GPUI app found on the Snap Store; rare for Rust GUI apps generally | Snap Store signs after review | snapd auto-refreshes up to 4x daily; no self-update | [snapcraft docs](https://snapcraft.io/docs/keeping-snaps-up-to-date) |
| L7 | AppImage | Linux | Helix, sonora (gpui), frame (gpui), zedis (gpui); not Zed | Optional gpg; .zsync enables delta updates | zsync deltas via AppImageUpdate, or in-app updater | [helix assets](https://github.com/helix-editor/helix/releases/tag/25.07.1), [sonora assets](https://github.com/sonorahq/sonora/releases/latest) |
| M1 | .app in .dmg | macOS | Zed (per-arch dmgs), every GPUI app below | Developer ID + hardened runtime + notarization required by Gatekeeper | Sparkle-style in-app; Zed rolls its own | [zed macos docs](https://zed.dev/docs/macos), [zed bundle-mac](https://github.com/zed-industries/zed/blob/main/script/bundle-mac) |
| M2 | .app in .zip | macOS | waku (gpui), frame (gpui), Zed's auto-update payloads | same as M1 | direct download; in-app updater rewrites .app | [waku](https://github.com/egoist/waku/releases/latest), [frame](https://github.com/66HEX/frame/releases/latest) |
| M3 | .pkg installer | macOS | Common for drivers/system software; no GPUI app found | Developer ID Installer cert + notarization | No built-in update story | [apple notarization docs](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution) |
| W1 | Portable .zip | Windows | Lapce, OpenLogi (gpui), zedis (gpui), waku (gpui) | Optional Authenticode | None; replace files manually | [lapce 0.4.6](https://github.com/lapce/lapce/releases/tag/v0.4.6) |
| W2 | Bare portable .exe | Windows | sonora (gpui) | Optional Authenticode | None | [sonora assets](https://github.com/sonorahq/sonora/releases/latest) |
| W3 | Setup .exe (Inno Setup or NSIS) | Windows | Zed uses Inno Setup; Tauri apps and waku (gpui) use NSIS-style setups | Authenticode; Zed signs via Azure Artifact Signing | In-app updater re-runs installer; winget can upgrade | [zed.iss](https://github.com/zed-industries/zed/blob/main/crates/zed/resources/windows/zed.iss), [tauri windows-installer](https://v2.tauri.app/distribute/windows-installer/) |
| W4 | .msi (WiX) | Windows | Lapce, OpenLogi (gpui), zedis (gpui); Tauri supports it | Authenticode | msiexec repair/uninstall semantics; winget upgrades | [tauri config ref](https://v2.tauri.app/reference/config/) |
| W5 | .msix (incl. Microsoft Store) | Windows | Rare for Rust GUI apps; MSIX is the Store format | Store signs; MSIX requires signed packages | Store auto-updates; containerized | [MSIX overview](https://learn.microsoft.com/en-us/windows/msix/overview) |
| C1 | apt repository | Linux | Zed has none; typical for larger projects only | repo GPG signing keys | apt | [zed linux docs](https://zed.dev/docs/linux) |
| C2 | dnf/zypper repository | Linux | Zed ships none; Fedora Terra provides zed | repo GPG keys | dnf | [terra packages](https://github.com/terrapkg/packages/tree/frawhide/anda/devs/zed/stable) |
| C3 | Arch official repos | Linux | zed in extra | packager keys | pacman | [arch package](https://archlinux.org/packages/extra/x86_64/zed/) |
| C4 | AUR | Linux | zed-git, zed-preview, zed-preview-bin | PKGBUILD sha256 | AUR helper | [zed linux docs](https://zed.dev/docs/linux) |
| C5 | Flathub | Linux | dev.zed.Zed, community-maintained, marked unverified | Flathub keys | flatpak update / software centers | [flathub](https://flathub.org/apps/dev.zed.Zed) |
| C6 | Snap Store | Linux | no GPUI/zed presence found | store review | snapd refresh | [snapcraft docs](https://snapcraft.io/docs/keeping-snaps-up-to-date) |
| C7 | Homebrew | macOS (cask) + Linux (formula) | brew install --cask zed; casks are macOS-only, formulae work on Linux | sha256 in formula/cask | brew upgrade | [brew cask zed](https://formulae.brew.sh/cask/zed), [cask-tap-is-macos-only](https://github.com/orgs/Homebrew/discussions/5375) |
| C8 | Nix (nixpkgs / flake) | Linux + macOS | zed-editor in nixpkgs | source-built, nix store integrity | nix flake update | [nixpkgs zed-editor](https://search.nixos.org/packages?channel=unstable&show=zed-editor) |
| C9 | winget | Windows | ZedIndustries.Zed, documented by Zed itself | SHA256 + publisher-domain validation, community-maintained manifests | winget upgrade | [zed windows docs](https://zed.dev/docs/windows), [winget-pkgs](https://github.com/microsoft/winget-pkgs) |
| C10 | Chocolatey | Windows | community "zed" package, stuck at 1.2.1 while Zed is at v1.23.x | package checksums | choco upgrade | [choco zed](https://community.chocolatey.org/packages/zed) |
| C11 | Scoop | Windows | third-party buckets; installs per-user, no UAC prompts | manifest checksums | scoop update | [scoop readme](https://github.com/ScoopInstaller/Scoop) |
| C12 | Conda | Linux/macOS/Windows | conda-forge zed | conda signing | conda update | [conda-forge zed](https://anaconda.org/conda-forge/zed) |
| C13 | Microsoft Store | Windows | hosts MSIX only | store signing | store updates | [MSIX overview](https://learn.microsoft.com/en-us/windows/msix/overview) |
| C14 | Mac App Store | macOS | requires App Sandbox; no GPUI app ships there | Apple signing | store updates | [apple distribution docs](https://developer.apple.com/documentation/xcode/preparing-your-app-for-distribution) |

## Zed's exact matrix (the GPUI reference point)

Release v1.23.2 ships exactly these user-facing artifacts
([source](https://github.com/zed-industries/zed/releases/tag/v1.23.2)):
`Zed-aarch64.dmg`, `Zed-x86_64.dmg` (per-arch, no universal dmg),
`Zed-aarch64.exe`, `Zed-x86_64.exe` (Inno Setup installers),
`zed-linux-x86_64.tar.gz`, `zed-linux-aarch64.tar.gz`, plus `bwrap-linux-*.gz`
sandbox helpers and `zed-remote-server-*` sidecar binaries.

Details worth copying:

- **Linux install is tarball-only.** `https://zed.dev/install.sh` detects arch,
  downloads the .tar.gz from cloud.zed.dev, and unpacks into `~/.local`
  ([source](https://zed.dev/install.sh)). There is no official apt/dnf repo.
  Docs list community packages instead: Arch extra, AUR, Fedora Terra,
  Homebrew, Manjaro, Conda, Nix, Solus, Parabola, ALT, AOSC, Flathub
  ([source](https://zed.dev/docs/linux)).
- **Flathub is community-run.** dev.zed.Zed is labeled "not verified by,
  affiliated with, or supported by Zed Industries" on Flathub
  ([source](https://flathub.org/apps/dev.zed.Zed)). Zed's official position is
  to let packagers do distro integration.
- **Windows uses Inno Setup**, not NSIS: `crates/zed/resources/windows/zed.iss`
  with `SignTool=` wired up, installed to Program Files
  ([source](https://github.com/zed-industries/zed/blob/main/crates/zed/resources/windows/zed.iss)).
  Binaries are signed through Azure Artifact Signing (formerly Trusted
  Signing): `sign.ps1` takes endpoint, account name, and certificate profile
  ([source](https://github.com/zed-industries/zed/blob/main/crates/zed/resources/windows/sign.ps1)).
- **macOS signing**: Developer ID codesign with `--options runtime` (hardened
  runtime) and notarization via an App Store Connect API key, all in
  `script/bundle-mac`
  ([source](https://github.com/zed-industries/zed/blob/main/script/bundle-mac)).
- **winget is documented officially**: `winget install -e --id ZedIndustries.Zed`
  ([source](https://zed.dev/docs/windows)). The Chocolatey package is community
  and stale at 1.2.1 ([source](https://community.chocolatey.org/packages/zed)).
- **Auto-update is in-app on all three platforms** for script/installer
  installs: download in background, apply on restart
  ([source](https://zed.dev/docs/update)). Windows has an
  `auto_update_helper.exe` sidecar
  ([source](https://github.com/zed-industries/zed/blob/main/script/bundle-windows.ps1)).
- **Icons**: Windows ships per-channel .ico files (app-icon.ico,
  app-icon-preview.ico, app-icon-nightly.ico); the Linux tarball carries a
  512x512 hicolor png and a .desktop file
  ([source](https://github.com/zed-industries/zed/tree/main/crates/zed/resources/windows),
  [source](https://zed.dev/docs/linux)).

## GPUI ecosystem precedent

Beyond Zed, checked every sizable GPUI-tagged app on GitHub (topic search,
2026-10-09). `gpui-kit` itself publishes crate tags only, no binaries
([source](https://github.com/longbridge/gpui-kit/releases)).

| App | What it ships | Notable |
|-----|---------------|---------|
| [OpenLogi](https://github.com/AprilNEA/OpenLogi/releases/latest) (23k stars) | Linux .deb/.rpm/.pkg.tar.zst (amd64+arm64), macOS dmg per-arch, Windows .msi + .zip per-arch, install.sh | Every artifact carries a minisign signature; broadest matrix of any GPUI app |
| [sonora](https://github.com/sonorahq/sonora/releases/latest) | Windows Setup.exe + bare .exe, AppImage + .zsync per arch, raw .flatpak bundle per arch, macOS dmg, bare Linux binaries | Only GPUI app found shipping Flatpak artifacts at all (direct bundles, not Flathub) |
| [frame](https://github.com/66HEX/frame/releases/latest) | macOS dmg + .app.zip per-arch, AppImage + .zsync, Linux tarballs, Windows .exe, update-manifest.json + Ed25519 .sig | Signed update manifest like this repo's manifest.json; also ships a CycloneDX SBOM |
| [waku](https://github.com/egoist/waku/releases/latest) | macOS dmg + zip, Windows Setup.exe + portable zip, Linux tar.gz | Publishes Sparkle-style appcast.xml per platform; single-maintainer project |
| [zedis](https://github.com/vicanso/zedis/releases/latest) | Linux .deb/.rpm/tar.gz/AppImage, macOS dmg, Windows .msi + .zip | latest.json + SHA256SUMS, Tauri-style updater layout |
| [Loungy](https://github.com/MatthiasGrandl/Loungy/releases/latest) | macOS universal dmg only | Minimal-end precedent: one artifact |
| [tty7](https://github.com/l0ng-ai/tty7/releases/latest) | Linux binaries, .apk for Android | Outlier; not a packaging model to copy |

Pattern: GPUI apps standardize on deb/rpm/tar.gz/AppImage for Linux, dmg for
macOS, msi/zip/Setup.exe for Windows. Flatpak, Snap, stores, and hosted repos
are rare or absent, matching the Zed position that packagers own distro
integration.

## Comparable Rust desktop apps

- **Lapce** v0.4.6 (floem, not GPUI): the fullest official matrix in the Rust
  GUI world: per-distro debs (bookworm/bullseye/jammy/noble/plucky, amd64 +
  arm64), fedora41/42 rpms, Linux tarballs, macOS dmg, Windows .msi plus
  portable zip ([source](https://github.com/lapce/lapce/releases/tag/v0.4.6)).
- **Helix** 25.07.1 (TUI): tar.xz per platform, AppImage with .zsync, a
  single .deb, Windows zip; everything else is community packaging
  ([source](https://github.com/helix-editor/helix/releases/tag/25.07.1)).
- **Tauri v2** (webview apps): the bundler defines the de facto Rust desktop
  matrix: macOS .app/.dmg, Windows NSIS .exe + MSI, Linux deb/rpm/AppImage,
  plus signed .tar.gz/.zip updater payloads
  ([source](https://v2.tauri.app/reference/config/),
  [updater](https://v2.tauri.app/plugin/updater/)).

## Per-platform notes

### Linux

- Tarball is the base layer everyone ships. Zed's glibc floors are 2.31
  (x86_64) and 2.35 (aarch64) ([source](https://zed.dev/docs/linux)); bundling
  non-glibc .so files (this repo already does) widens reach.
- .deb and .rpm are one-shot artifacts: build with cargo-deb /
  cargo-generate-rpm against an old-enough base image. Hosting an apt/dnf repo
  (C1/C2) is separate ongoing infrastructure no GPUI app runs itself.
- Flatpak apps live in OSTree trees the system deploys; updates come from
  `flatpak update` or software centers, and the app cannot replace its own
  files ([source](https://docs.flatpak.org/en/latest/using-flatpak.html)).
  Desktop portals are mandatory for file dialogs and secrets; this repo's
  `notifications-portal` feature is exactly that pattern
  (see Zed's portal requirements:
  [source](https://zed.dev/docs/linux)).
- Snaps auto-refresh up to four times a day and users cannot fully opt out;
  confinement is strict by default ([source](https://snapcraft.io/docs/keeping-snaps-up-to-date)).
- AppImage pairs with .zsync files for binary delta updates; Helix, sonora,
  and frame all publish them (see their release assets above).
- Homebrew casks are macOS-only; on Linux Homebrew serves formulae, which is
  why Zed's Linux docs listing a cask is a doc bug worth knowing about
  ([source](https://github.com/orgs/Homebrew/discussions/5375)).

### macOS

- Outside the App Store, Gatekeeper requires Developer ID signing with the
  hardened runtime plus notarization through `notarytool`, then stapling the
  ticket; the Apple Developer Program costs USD 99/year
  ([source](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)).
- The Mac App Store mandates App Sandbox, which blocks most desktop-app
  behavior and store apps update through the store, so Sparkle-style updating
  does not apply there ([source](https://developer.apple.com/documentation/xcode/preparing-your-app-for-distribution)).
- Sparkle 2 supports macOS 12+, handles sandboxed apps via an XPC service,
  and uses EdDSA signatures plus Apple code signing
  ([source](https://sparkle-project.org/)). Zed and frame prove a custom
  Ed25519-signed manifest updater is a workable substitute for GPUI apps.
- Universal vs per-arch is a size tradeoff: Zed ships per-arch dmgs; Loungy
  ships universal; this repo already builds all three.

### Windows

- Authenticode signing decides SmartScreen behavior. Azure Artifact Signing
  (renamed from Trusted Signing) is the budget route: Basic at USD 9.99/month
  for 5,000 signatures, Premium at USD 99.99/month; individual developers can
  onboard only in the USA and Canada with a paid Azure subscription
  ([source](https://azure.microsoft.com/en-us/pricing/details/artifact-signing),
  [source](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options)).
  Zed already uses it ([source](https://github.com/zed-industries/zed/blob/main/crates/zed/resources/windows/sign.ps1)).
  Unsigned is viable: users click through SmartScreen, and every GPUI app
  except Zed ships unsigned.
- Setup installers: Inno Setup (Zed) and NSIS (Tauri, waku) both run on GitHub
  Actions runners; both produce a single setup .exe with uninstall entries.
- MSI via WiX gives enterprise deployability; Lapce, OpenLogi, and zedis ship
  it. MSIX adds containerization and Store distribution but requires signed
  packages and is rare in this ecosystem
  ([source](https://learn.microsoft.com/en-us/windows/msix/overview)).
- winget is manifest-only: a YAML file in microsoft/winget-pkgs points at the
  publisher's installer URL and validates domain consistency plus SHA256; it
  is community-maintained with moderation
  ([source](https://github.com/microsoft/winget-pkgs/blob/master/doc/Validation.md)).
  winget also supports zip/portable packages, so it can ship before an
  installer exists.
- Scoop installs per-user with no UAC prompt; Chocolatey favors machine-wide
  installs and its community Zed package lags releases badly (1.2.1 vs
  v1.23.x) ([source](https://github.com/ScoopInstaller/Scoop),
  [source](https://community.chocolatey.org/packages/zed)).

## Auto-update and signing cross-cut

- Formats compatible with Sparkle-style in-app updaters: tarballs, dmg/zip
  .app, portable zip/exe, AppImage, Inno/NSIS installs. Formats that break
  them: Flatpak, Snap, stores (C5, C6, C13, C14) since the system owns the
  deploy, and deb/rpm/pacman where the package manager owns upgrades
  ([flatpak](https://docs.flatpak.org/en/latest/using-flatpak.html),
  [snap](https://snapcraft.io/docs/keeping-snaps-up-to-date)).
- Correct behavior, per Zed: enable the in-app updater only when installed
  from your own artifacts, and let package managers do their job otherwise
  ([source](https://zed.dev/docs/update)). For this repo that means baking
  GPUI_UPDATE_MANIFEST_URL into tarball/dmg/zip/AppImage builds and leaving
  it unset in deb/rpm/flatpak/snap builds.
- Signing ladder, cheapest first: SHA256SUMS (free), minisign/Ed25519
  manifests (free, OpenLogi and frame do it), Apple notarization (USD 99/yr,
  mandatory on macOS), Authenticode via Artifact Signing (USD 9.99/mo,
  regional restrictions above).
- Icons per format (details land in the icons research doc): .icns in the
  .app bundle, .ico embedded in the Windows exe/installer (Zed ships
  per-channel .ico), hicolor png set inside Linux packages
  ([source](https://github.com/zed-industries/zed/tree/main/crates/zed/resources/windows)).

## What this repo should ship, ranked by effort and impact

Already in place (keep): macOS dmgs (universal + 2 arches) with optional
notarization, Linux tarballs (2 arches) with desktop file/png/install.sh,
generated PKGBUILD, Ed25519-signed manifest.json on gh-pages, flake.nix for
Nix users (document it, do not rebuild it).

Recommended additions, in order:

1. **.deb via cargo-deb** (area 5). Config in Cargo.toml, builds inside the
   existing ubuntu CI job, arm64 cross-builds or native runner. Highest
   demand-to-effort ratio in the ecosystem (Lapce ships 10 deb variants).
2. **AppImage via linuxdeploy** (area 6). One script on the existing Linux
   artifacts; add .zsync for deltas. Solves "runs anywhere" without portals
   paperwork.
3. **Windows portable .zip** (area 9, first half). windows-latest runner,
   cargo build, zip the exe plus the .ico. First Windows artifact at all;
   enables a winget portable manifest immediately.
4. **winget manifest** (community PR once artifacts exist; zero CI cost).
5. **.rpm via cargo-generate-rpm** (area 8). Same shape as the deb job.
6. **Windows setup installer**, Inno Setup or NSIS (area 9, second half).
   Inno matches Zed, NSIS matches Tauri; both are single-dependency in CI.
7. **Homebrew cask** (submit after stable releases; community maintained).
8. **Flatpak** (area 7): metainfo + manifest + Flathub submission, plus portal
   testing behind the existing notifications-portal feature. Real ongoing
   maintenance; ship last.
9. Skip: Snap, MSIX/Microsoft Store, Mac App Store, hosted apt/dnf repos,
   Chocolatey. No GPUI ecosystem precedent, and each carries accounts,
   review queues, or signing infrastructure a single maintainer should not
   hold.

## Tooling survey

All statuses checked 2026-10-09.

| Tool | Status | Produces |
|------|--------|----------|
| [cargo-bundle](https://crates.io/crates/cargo-bundle) | active again, 0.12.0 (2026-09); repo unarchived | .app, .deb, .AppImage; .msi experimental, .rpm pending |
| [cargo-packager](https://crates.io/crates/cargo-packager) (CrabNebula) | 0.11.8 (2025-11) | deb, AppImage, dmg, msi, nsis; Tauri's packager split out |
| [cargo-deb](https://crates.io/crates/cargo-deb) | 3.8.0 (2026-09), actively maintained | .deb from Cargo metadata |
| [cargo-generate-rpm](https://crates.io/crates/cargo-generate-rpm) | 0.21.0 (2026-05) | .rpm from Cargo metadata |
| [cargo-wix](https://crates.io/crates/cargo-wix) | 0.3.9 (2025-03) | .msi from a WiX template |
| [cargo-appimage](https://crates.io/crates/cargo-appimage) | 2.4.0 (2025-11) | AppImage, wraps appimagetool |
| [cargo-dist](https://crates.io/crates/cargo-dist) | 0.32.0 (2026-05) | full release pipeline: tarballs, checksums, installers, GitHub Releases |
| [linuxdeploy](https://github.com/linuxdeploy/linuxdeploy) | maintained (2026-08) | AppDir to AppImage, desktop-file/icon integration |
| [appimagetool](https://github.com/AppImage/appimagetool) | maintained (2026-10) | AppImage from AppDir, .zsync metadata |
| [flatpak-builder](https://github.com/flatpak/flatpak-builder) | maintained (2026-10) | builds Flatpak apps from JSON manifests |
| [snapcraft](https://github.com/canonical/snapcraft) | maintained (2026-10) | .snap from snapcraft.yaml, store upload |
| [create-dmg](https://github.com/andreyvit/create-dmg) | maintained (2026-07) | styled macOS dmg from a .app |
| [WiX Toolset](https://wixtoolset.org/) | current line v6 | .msi installers; cargo-wix targets it |
| Inno Setup / NSIS | both current | setup .exe installers; run under wine or native windows runners |

Notable correction: the task brief named "cargo-packster"; the actual tool is
cargo-packager ([source](https://docs.rs/cargo-packager)). Flatpak has no cargo
subcommand; packaging goes through a flatpak-builder manifest.
