# App icons and Windows packaging for gpui-starter

> Status: research snapshot (2026-10-09), input to the icons and
> windows-installer campaign. Repo state below was read on that date.

Research for routing-map areas 4 (icons) and 9 (windows-installer). Every
claim carries its source. Repo reality was read before deciding:
`scripts/create-release-app.sh` (no icon step at all, `Info.plist` has no
`CFBundleIconFile`), `scripts/install-linux.sh` (installs a single 512px PNG
per-user), `src/platform/toast_identity.rs` and
`src/platform/deep_link_registration.rs` (runtime self-integration on
Windows), `src/services/updater/apply.rs` (generic single-step rename swap),
`Cargo.toml` (windows crate features, HKCU protocol registration comment).

Companion docs: `docs/research/release-matrix.md` (area 1: what apps ship,
signing ladder, updater policy) and `docs/research/linux-packages.md`
(area 2: deb/flatpak/appimage/rpm/snap).

## Part A. App icon pipeline

### A1. macOS .icns

Required contents. An .icns is a container of typed PNG entries. The types
macOS actually consumes for a modern app icon are
([Apple Icon Image format](https://en.wikipedia.org/wiki/Apple_Icon_Image_format)):

| OSType | Pixel size | Meaning |
|--------|-----------|---------|
| ic04 | 16 | 16 @1x (ARGB) |
| ic05 | 32 | 32 @1x (ARGB) |
| ic07 | 128 | 128 @1x |
| ic08 | 256 | 256 @1x |
| ic09 | 512 | 512 @1x |
| ic10 | 1024 | 512 @2x |
| ic11 | 32 | 16 @2x |
| ic12 | 64 | 32 @2x |
| ic13 | 256 | 128 @2x |
| ic14 | 512 | 256 @2x |

Practical consequence: render PNGs at 16, 32, 64, 128, 256, 512, 1024 px and
pack them; the container assigns the type per size. Legacy 32-bit mask types
(icm, ics, icl, it32) are not needed on any supported macOS
([source](https://en.wikipedia.org/wiki/Apple_Icon_Image_format)).

Placement in the bundle. `Contents/Resources/AppIcon.icns` plus
`CFBundleIconFile` in `Info.plist` naming it (the .icns extension is
optional in the value; macOS appends it). Without the plist key Finder shows
the generic executable icon
([CFBundleIconFile reference](https://developer.apple.com/documentation/bundleresources/information-property-list/cfbundleiconfile),
[Core Foundation Keys, archived](https://developer.apple.com/library/archive/documentation/General/Reference/InfoPlistKeyReference/Articles/CoreFoundationKeys.html),
[Bundle Structures](https://developer.apple.com/documentation/bundleresources/bundle-structure)).

Current script gap. `scripts/create-release-app.sh` writes `Info.plist`
with `CFBundleExecutable`, `CFBundleIdentifier`, versions, and
`NSHighResolutionCapable` only. It never copies an icon and never sets
`CFBundleIconFile`. Every DMG today ships an icon-less app.

Generating .icns on Linux. Three options:

1. `iconutil` (Apple) is the reference tool but exists only on macOS
   ([Wikipedia tool list](https://en.wikipedia.org/wiki/Apple_Icon_Image_format)).
   Our CI has a macOS job, so it is usable there; the dev box is not.
2. `png2icns` from `icnsutils` (apt) packs PNGs into an icns on Linux:
   `png2icns AppIcon.icns icon_16.png icon_32.png ... icon_1024.png`
   ([man page](https://manpages.ubuntu.com/manpages/jammy/man1/png2icns.1.html),
   [usage examples](https://askubuntu.com/questions/223215/how-can-i-convert-a-png-file-to-icns),
   [tutorial](https://dentrassi.de/2014/02/25/creating-mac-os-x-icons-icns-on-linux)).
   Caveat: its documentation does not state which OSTypes it emits per size
   and the project page is stale; Ubuntu ships version 0.8.1 which supports
   the PNG-based types
   ([icns project page](https://icns.sourceforge.io/),
   [Ubuntu package](https://ubuntu.pkgs.org/22.04/ubuntu-universe-arm64/icnsutils_0.8.1-3.1_arm64.deb.html)).
   The script should verify its own output with `icns2png -l`.
3. ImageMagick writes .icns from multiple input PNGs, but its writer has a
   documented history of mishandling the retina types ic10 through ic14
   compared to iconutil
   ([IM discussion #6713](https://github.com/ImageMagick/ImageMagick/discussions/6713),
   [multi-size example](https://gist.github.com/Qix-/f4090181e55ea365633da8c3d0ab5249)).

Missing ic11..ic14 entries are cosmetically survivable: macOS renders the
@1x entries at 2x. A boilerplate repo does not need pixel-perfect retina
small sizes on day one.

Decision for this repo. Author `scripts/gen-icons.sh` (area 4) to render the
seven PNG sizes from the SVG master, pack the icns with `png2icns`
(Debian/Ubuntu: `apt install icnsutils`), and list the emitted types with
`icns2png -l` as a self-check. Commit the result as
`assets/app-icon/AppIcon.icns`. Area 4 also copies it into the .app: but
note `scripts/create-release-app.sh` is outside area 4's allowed paths, so
the `Contents/Resources/AppIcon.icns` copy plus the `CFBundleIconFile`
plist key need a routing-map grant for area 4 (or area 10). Documented
fallback if png2icns output proves incomplete on the build host: run the
iconutil variant inside the existing macOS CI job.

### A2. Windows .ico

Required sizes. Microsoft's icon guidance for apps lists 16, 24, 32, 48,
64, 128, and 256 px in one .ico
([Windows app icons](https://learn.microsoft.com/en-us/windows/apps/design/style/icons/windows-app-icons)).
The ICO format stores entries as BMP or, since Vista, as embedded PNG; the
256 px entry is conventionally PNG-compressed because a 256 BMP is large
([ICO format overview](https://en.wikipedia.org/wiki/ICO_(file_format))). A
complete .ico is roughly 100 to 400 KB depending on art.

Generating .ico on Linux. ImageMagick is the cleanest single command:

```sh
magick icon-1024.png -resize 256x256 \
  -define icon:auto-resize=256,128,64,48,32,24,16 app.ico
```

`icon:auto-resize` writes all listed sizes from one input (it requires a
256x256 or larger input) and `icon:png-compression-size` defaults to 256,
so the 256 entry is stored PNG-compressed automatically
([ImageMagick defines](https://imagemagick.org/defines/),
[worked examples](https://stackoverflow.com/questions/11423711/recipe-for-creating-windows-ico-files-with-imagemagick),
[caveat thread](https://github.com/ImageMagick/ImageMagick/issues/1505)).
The alternative `icotool` (icoutils, apt) creates BMP entries by default and
only stores raw PNG per-file via `--raw=FILENAME`, which is awkward for
multi-size icons
([icotool man page](https://man.archlinux.org/man/icotool.1.en),
[icoutils home](https://www.nongnu.org/icoutils/)). Prefer ImageMagick.

Embedding the icon into the .exe. Windows reads the exe's resource section
for the window/taskbar icon; a bare .exe without an embedded icon shows the
generic one even when a shortcut points at a .ico file. Three crates exist:

- `winres`: abandoned; broken on Rust 1.61+
  ([crates.io winresource description](https://crates.io/crates/winresource),
  [docs.rs winres](https://docs.rs/winres/latest/winres/struct.WindowsResource.html)).
- `winresource`: maintained fork by Benjamin Richner; latest 0.1.31
  released 2026-03-16, active releases through 2025 and 2026, MIT
  ([crates.io API](https://crates.io/api/v1/crates/winresource),
  [repository](https://github.com/BenjaminRi/winresource)).
- `tauri-winres`: Tauri's internal fork, used by tauri itself
  ([docs.rs tauri-winres](https://docs.rs/tauri-winres)). Right tool when
  you are Tauri; winresource is the general-purpose pick.

The build.rs pattern from winresource's README, and it matters for our
Linux clippy gate:

```rust
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/app-icon/app.ico");
        res.compile().unwrap();
    }
}
```

`build.rs` runs on the host, so it must gate on `CARGO_CFG_TARGET_OS`
(target OS), never `#[cfg(target_os = "windows")]` (host OS). On this Linux
box building Linux targets, the branch never executes: `just clippy` and
`just test` stay toolchain-free no-ops. When the target IS windows, MSVC
builds need `rc.exe` from the Windows SDK and GNU builds need mingw
`windres` ([winresource README](https://github.com/BenjaminRi/winresource)).
The GHA Windows image ships the full SDK, so `rc.exe` is present
([runner image](https://github.com/actions/runner-images/blob/main/images/windows/Windows2025-Readme.md)).

The alternative is a hand-written `.rc` file compiled by `rc.exe` as a
separate CI step and linked via `-C link-arg=app.res`, no build-dependency.
That keeps Cargo.toml clean but loses the generated version-info block and
duplicates what winresource automates.

Decision for this repo. Generate `assets/app-icon/app.ico` with ImageMagick
`icon:auto-resize` in `scripts/gen-icons.sh`; commit it. Embed with
`winresource = "0.1"` as a target-gated build-dependency plus the
`CARGO_CFG_TARGET_OS` build.rs above (area 9). Flag for the decider: a root
`build.rs` is not currently inside area 9's allowed paths; either grant it
or fall back to the `rc.exe` + `link-arg` script variant (both documented
here). Window title-bar/taskbar icon then works for the bare exe, the
installer, and winget portable usage with zero runtime work.

### A3. Linux hicolor icon set

The size set. The hicolor fallback theme (0.18, current) ships directories
for 16, 22, 24, 32, 36, 48, 64, 72, 96, 128, 192, 256, 512 px and scalable,
under contexts like `apps`
([hicolor-icon-theme 0.18 file list](https://archlinux.org/packages/extra/any/hicolor-icon-theme/files),
[Debian package](https://packages.debian.org/sid/hicolor-icon-theme)).
Contrary to the task brief, hicolor does include 22x22 (GNOME's classic
panel size); 22 is simply lower priority for app icons than the Windows
set. The freedesktop spec's stated minimum is a 48x48 PNG in
`$prefix/share/icons/hicolor/48x48/apps`
([Icon Theme Specification](https://specifications.freedesktop.org/icon-theme/latest)),
and 256 is the common modern lookup size
([GNOME themed icons](https://developer.gnome.org/documentation/tutorials/themed-icons.html)).

Layout and naming. System packages install
`/usr/share/icons/hicolor/<size>/apps/gpui-starter.png` (plus
`scalable/apps/gpui-starter.svg`) and trigger the icon cache refresh.
`Icon=gpui-starter` in `resources/com.gpui-starter.app.desktop` already
follows the rule: Icon= carries the icon name (basename, no extension, no
path); desktop environments resolve it against the icon theme search path
([Icon Theme Specification directory layout](https://specifications.freedesktop.org/icon-theme/latest),
[ArchWiki Icons](https://wiki.archlinux.org/title/Icons)). Nothing to change
in the desktop file.

What ships today. `scripts/install-linux.sh` installs exactly one PNG:
`~/.local/share/icons/hicolor/512x512/apps/gpui-starter.png` sourced from
the tarball's `gpui-starter.png` (a single 512x512 file, verified). Small
sizes (taskbar, alt-tab, app grids on small displays) are scaled down from
512 by the toolkit, which is blurry or crunchy depending on DE.

Decision for this repo. Area 4 generates and commits the set
`16, 24, 32, 48, 64, 128, 256, 512` png plus `scalable/gpui-starter.svg`
(sizes 22, 36, 72, 96, 192 optional; diminishing returns). Areas 5 and 8
install the set into `/usr/share/icons/hicolor/...` in deb and rpm. The
tarball path needs two touches outside area 4's paths: ship the hicolor tree
inside the tarball (release.yml, area 10) and teach `install-linux.sh` to
loop all sizes instead of the single 512 (file owned by no area; needs a
routing-map grant). Keep `resources/gpui-starter.png` as-is for backward
compatibility or replace its consumers; that is area 4's call inside its
allowed paths.

### A4. Upstream artwork source

Current assets. `assets/icons/*.svg` are lucide-style UI glyphs, not the app
identity. `resources/gpui-starter.png` is the app icon: a single 512x512
PNG (13.6 KB, header verified). There is no vector master, so today the 512
raster is the only source and upscales are impossible.

Options. Author one SVG master (1024x1024 canvas, flat shapes so it scales
to 16px), then render every size from it:

- `rsvg-convert` (librsvg2-bin, apt, present in Ubuntu 24.04): the standard
  Linux renderer; `-w 512 icon.svg > icon-512.png`
  ([Launchpad noble package](https://launchpad.net/ubuntu/noble/+package/librsvg2-bin),
  [man page](https://man.archlinux.org/man/rsvg-convert.1.en)).
- `resvg` (Rust, `cargo install resvg` or prebuilt): higher fidelity on
  complex SVG, not in apt
  ([repository](https://github.com/linebender/resvg)).
- ImageMagick renders SVG too but delegates to librsvg internally, so it
  adds nothing here
  ([delegate note](https://stackoverflow.com/questions/9853325/how-to-convert-an-svg-to-png-with-imagemagick)).

CI availability. The ubuntu-24.04 runner image ships none of ImageMagick,
librsvg, icnsutils, or icoutils; one apt line installs all of them
([runner image README](https://github.com/actions/runner-images/blob/main/images/ubuntu/Ubuntu2404-Readme.md)).
So either way CI needs an install step; the difference is only whether CI
ever needs to run it.

Committed artifacts versus generate-in-CI. Comparable apps commit their
generated icons: Zed keeps per-arch .ico files and the installer icon in
the repo
([crates/zed/resources/windows](https://github.com/zed-industries/zed/tree/main/crates/zed/resources/windows),
referenced as `SetupIconFile` by
[zed.iss](https://github.com/zed-industries/zed/blob/main/crates/zed/resources/windows/zed.iss));
Tauri apps commit `icons/`. For a boilerplate repo the tradeoff favors
committing: local builds and packaging scripts work offline with zero
toolchain prerequisites, regeneration is rare (branding changes), and the
cost is a few hundred KB of binary files in git. Generate-in-CI keeps git
lean but makes every artifact job depend on renderer availability and
renderer version drift (librsvg 2.60 renders differently than 2.54).

Decision for this repo. Author `assets/app-icon/gpui-starter.svg` (simple
branded mark on a 1024 canvas). `scripts/gen-icons.sh` renders from it with
`rsvg-convert` when available, packs the icns (png2icns) and ico
(ImageMagick), and writes the hicolor tree. Commit all generated files
under `assets/app-icon/` and regenerate only when the SVG changes. The
script also keeps working when the tools are missing by skipping with a
clear message, since committed artifacts are the source of truth for
packaging.

## Part B. Windows packaging (CI-only, dev box is Linux)

### B1. Installer technology

The candidates and their Linux story:

- NSIS: `makensis` runs natively on Linux (apt `nsis`) and on Windows.
  Tauri explicitly supports cross-compiling Windows NSIS installers from
  Linux/macOS hosts "with caveats", and recommends NSIS over WiX for
  cross-builds
  ([Tauri Windows installer docs](https://v2.tauri.app/distribute/windows-installer/),
  [cross-platform guide](https://tauri.app/v1/guides/building/cross-platform/)).
- WiX v4/v5: the `wix` dotnet tool builds MSIs on Linux natively, no Wine;
  WiX v4+ is cross-platform .NET
  ([docker-wixtoolset](https://github.com/jkroepke/docker-wixtoolset),
  [wixtoolset issues #8623](https://github.com/wixtoolset/issues/issues/8623)).
  cargo-wix (0.3.9, area 1 verified) wraps the older toolchain.
- MSIX: `makemsix` from the MSIX SDK packs on Linux, but cannot sign, and
  MSIX must be signed to install at all
  ([MSIX on Linux](https://learn.microsoft.com/en-us/windows/msix/msix-sdk/msix-linux),
  [microsoft/msix-packaging](https://github.com/microsoft/msix-packaging),
  [signing gap #340](https://github.com/microsoft/msix-packaging/issues/340)).
  This kills MSIX for an unsigned OSS default (matches area 1's skip).
- Inno Setup: the compiler ISCC.exe is Windows-only; on Linux it needs
  Wine
  ([runner-images issue #11644](https://github.com/actions/runner-images/issues/11644),
  [Wine gist](https://gist.github.com/amake/3e7194e5e61d0e1850bba144797fd797)).
  But the GHA windows-2025 image preinstalls Inno Setup 6.7.1 (and WiX
  3.14), so on a Windows runner it costs nothing
  ([runner image README](https://github.com/actions/runner-images/blob/main/images/windows/Windows2025-Readme.md)).

What comparable apps ship. Zed builds Inno Setup installers
(`Zed-<arch>.exe`) on Windows runners: the release workflow runs on a
Windows 2022 runner and delegates to bundle-windows.ps1, which calls the
checked-in
[zed.iss](https://github.com/zed-industries/zed/blob/main/crates/zed/resources/windows/zed.iss)
and signs via Azure (env vars in the workflow; area 1 verified sign.ps1).
The fetched zed.iss installs per-user (`PrivilegesRequired=lowest`,
`DefaultDirName={autopf}`), embeds the ico via `SetupIconFile`, conditionally
signs with `SignTool`, and handles x64 and arm64 in one script. Tauri's
defaults are NSIS and WiX, built on Windows runners for Windows targets
([Tauri distribute docs](https://v2.tauri.app/distribute/windows-installer/)).

Decision for this repo. The exe must be built on a Windows runner anyway
(B2), so the installer is built on that same runner; the Linux makensis
advantage is moot. Choose Inno Setup: preinstalled on windows-2025 (zero
CI install), Zed's exact precedent including a copyable .iss skeleton, and
per-user install semantics that match this app's HKCU runtime registrations
and the updater's non-admin swap (B3, B4). NSIS stays documented as the
fallback if Windows exe builds ever move to Linux via cross-compilation.
Ship both the setup .exe and a portable .zip (area 1's rank 3 baseline).

### B2. GPUI on Windows: build requirements

Toolchain. Zed's official Windows build docs require Visual Studio (or Build
Tools) with the C++ workload: MSVC v143 x64/x86 build tools, Spectre-mitigated
libs, Windows SDK at least 10.0.20348, and CMake (for wasmtime's C API)
([Building Zed for Windows](https://zed.dev/docs/development/windows)).
Renderer: the current docs state "Zed uses Vulkan as its graphics API on
Windows" and call Vulkan drivers the common launch-failure cause. That
corrects the assumption in this task brief: the Windows backend today is
Vulkan through blade, not DirectX 12; the DirectX backend was the earlier
Windows-port effort
([docs](https://zed.dev/docs/development/windows),
[Windows progress report](https://zed.dev/blog/windows-progress-report),
[blade-to-wgpu discussion](https://news.ycombinator.com/item?id=47002825)).
For CI this means no DirectX SDK extras to install; the Windows SDK's
Vulkan loader is provided by the OS and GPU drivers, same as our Linux
Vulkan story (area 2).

Runner. GHA windows-2025 ships VS Enterprise 2022 with x64/x86 plus ARM64
toolsets, Windows SDK 10.0.26100, CMake, and Rust 1.98 with rustup
([runner image README](https://github.com/actions/runner-images/blob/main/images/windows/Windows2025-Readme.md)).
So the entire job is: `rustup target add x86_64-pc-windows-msvc` (already
default), `cargo build --release`, then the packaging script. Nothing else
to install.

Targets. `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc` are both
Tier 1 with host tools in Rust
([rustc platform support](https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html)).
The windows-rs based stack in this repo (windows 0.61, tray-icon,
global-hotkey, windows-registry) is MSVC-first; Zed ships both arches from
Windows runners. The GNU target would let Linux cross-build with mingw, but
it is untested territory for gpui and buys nothing while Windows runners are
in use.

Decision for this repo. Release CI gains a `build-windows` job on
`windows-2025` (or `windows-latest`): `cargo build --release --target
x86_64-pc-windows-msvc`, embed the icon via winresource (A2), run the Inno
script, publish `gpui-starter-<ver>-x86_64-windows-setup.exe` plus
`gpui-starter-<ver>-x86_64-windows.zip`. `aarch64-pc-windows-msvc` is a
cheap second target when someone asks for Windows-on-ARM (Zed ships it);
not in the initial matrix. No Linux cross-compilation of the exe.

### B3. Installer contents, integration, signing, winget

Install scope. Per-user into `%LOCALAPPDATA%\Programs\gpui-starter` (Inno
`PrivilegesRequired=lowest`, Zed's model, verified in zed.iss) rather than
Program Files. Two repo-specific reasons: the app registers its own
integration in HKCU at runtime, so it was designed for non-admin use, and
the in-app updater's file swap (B4) can write inside a per-user install dir
but never inside Program Files without UAC.

Start Menu shortcut and AUMID. Unpackaged Win32 apps need a Start Menu
shortcut carrying `System.AppUserModel.ID` or Windows silently drops every
toast; this repo already solves it at runtime:
`src/platform/toast_identity.rs` writes `gpui-starter.lnk` into the user's
Start Menu Programs folder with the AUMID property and sets the process
AUMID, idempotently, refreshing when the exe moves (source read; comment in
Cargo.toml lines 224+ points at it). Consequence for the installer: create
whatever plain shortcut Inno wants; on first launch the app overwrites the
same `gpui-starter.lnk` path with the AUMID-carrying version. The installer
must NOT need AUMID logic of its own. Uninstall should tolerate the app's
runtime files (the .lnk and HKCU keys) being user-created.

Protocol handler. `src/platform/deep_link_registration.rs` registers
`gpui-starter://` in HKCU at runtime via windows-registry (source read;
Cargo.toml comment). The installer needs no protocol registry writes and
none of the corresponding uninstall cleanup; this is the same self-service
model the Linux tarball uses.

SmartScreen and signing reality. Unsigned exes trigger the SmartScreen
"Windows protected your PC" click-through until enough downloads build
reputation. The paid route is Azure Artifact Signing (formerly Trusted
Signing): Basic at USD 9.99/month for 5,000 signatures; individual
developers can onboard only in the USA and Canada (organizations: USA,
Canada, EU, UK); certificates are Microsoft-trusted and build SmartScreen
reputation over time
([Azure Artifact Signing](https://azure.microsoft.com/en-us/products/artifact-signing),
[pricing](https://azure.microsoft.com/en-us/pricing/details/artifact-signing),
[code-signing options](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options)).
Self-signing with osslsigncode adds warning friction and earns no
reputation; it is worse than shipping unsigned for distribution. Every GPUI
app except Zed ships unsigned (area 1 survey).

winget. A winget entry is a community PR to microsoft/winget-pkgs: YAML
manifests with `PackageIdentifier` (`Publisher.Package`), `InstallerUrl` and
`InstallerSha256` that must match the hosted binary byte-for-byte, and
publisher/domain consistency checks under moderation
([submission requirements](https://learn.microsoft.com/en-us/windows/package-manager/package/repository),
[installer schema](https://github.com/microsoft/winget-pkgs/blob/master/doc/manifest/schema/1.12.0/installer.md),
[manifest authoring](https://learn.microsoft.com/en-us/windows/package-manager/package/manifest)).
winget supports ZIP portable packages, so the repo can be listed before any
installer exists by pointing at the release .zip.

Portable zip. Baseline Windows artifact (area 1 rank 3): the bare exe plus
a short README, name it `gpui-starter-<ver>-x86_64-windows.zip`. The bare
exe should also be a release asset because the current updater swaps single
files, not archives (B4).

Decision for this repo. Area 9 ships: portable .zip (required), Inno
setup .exe installing per-user (required), both unsigned with SHA256SUMS +
the Ed25519 manifest the release already publishes. Signing default-off:
wire an optional CI step in the pattern of Zed's sign.ps1 that runs only
when AZURE_* secrets exist, documented in the releasing doc (area 11) with
the eligibility caveat. winget: after the first tagged release, submit the
portable zip manifest to winget-pkgs (URLs are GitHub release assets,
which pass the domain checks; area 10's generate-manifest.sh already
computes sha256). No MSIX, no Store, no self-signed.

### B4. Auto-update on Windows

What the repo has today. `src/services/updater/types.rs` already emits the
`windows-<arch>` platform key (source read), so the manifest side needs no
code change. But `apply.rs` swaps with a single
`std::fs::rename(source, dest)` where dest is the currently running exe
(source read; only macOS gets a special branch). On Windows that fails:
a running exe cannot be replaced, though it CAN be renamed
([self-update technique](https://stackoverflow.com/questions/459734/is-there-a-way-to-make-a-self-updatable-executable-on-windows),
[in-use replacement bug report](https://github.com/metaist/cosmofy/issues/58)).
The established patterns:

- Rename dance: rename the running exe to `app.old`, move the new exe into
  the old name, restart, delete `.old` on next launch (SO above; the
  generic approach used by many updaters).
- Separate updater process plus versioned app dirs: Squirrel and its
  successor Velopack run Update.exe outside the app
  ([Velopack](https://docs.velopack.io/migrating/squirrel)).
- Exit-then-install: Tauri's updater quits the app during install "due to a
  limitation of Windows installers"
  ([Tauri updater docs](https://v2.tauri.app/plugin/updater/)).

Our swap runs at next launch via the pending-swap marker
(`check_pending_swap`), which is the right shape; it just needs the
two-step rename when `target_os = "windows"`. Installed-per-user (B3) keeps
the destination writable without elevation. With a Program Files install
this whole mechanism would be dead on arrival, which is the final argument
for per-user.

Scope warning for the decider: `src/services/updater/apply.rs` is under
`src/`, which every routing-map area excludes. The two-step Windows rename
is a small, self-contained src patch; it needs a new grant or a follow-up
task, otherwise the Windows updater will download, verify, and then fail at
the swap step.

Decision for this repo. Ship Windows update support in two stages. Stage 1
(no src changes): manifest gains `windows-x86_64` pointing at the bare exe
asset; updater bakes GPUI_UPDATE_MANIFEST_URL into zip and installer builds
per area 1's policy. Document that applying updates on Windows currently
fails at the swap and requires the apply.rs two-step rename patch
(windows branch: rename current exe to `.old`, move new exe into place,
delete `.old` at next start). Stage 2 is that patch, routed explicitly.

## Decision summary

| Question | Decision |
|---|---|
| icns generation on Linux | png2icns (icnsutils, apt), sizes 16..1024, self-check with icns2png -l; iconutil in the macOS CI job as fallback |
| icns placement | `assets/app-icon/AppIcon.icns` committed; copy to `Contents/Resources/AppIcon.icns` + `CFBundleIconFile` in create-release-app.sh (needs path grant) |
| ico generation | ImageMagick `icon:auto-resize=256,128,64,48,32,24,16` (256 entry PNG-compressed by default) |
| exe icon embedding | winresource 0.1 (latest 0.1.31, 2026-03-16) with CARGO_CFG_TARGET_OS-gated build.rs; rc.exe + link-arg variant if build.rs stays out of scope; no-op on Linux, clippy/test unaffected |
| Linux hicolor set | committed png 16,24,32,48,64,128,256,512 + scalable svg in `assets/app-icon/hicolor/`; deb/rpm install to /usr/share/icons/hicolor; tarball + install-linux.sh extension needs grants |
| Artwork pipeline | one SVG master `assets/app-icon/gpui-starter.svg`, rendered by rsvg-convert (apt); generated artifacts committed, script regenerates on demand |
| Installer tech | Inno Setup (preinstalled on windows-2025 runner, Zed precedent zed.iss); NSIS documented fallback; WiX/MSIX skipped |
| Runner matrix | new build-windows job on windows-2025, x86_64-pc-windows-msvc only; aarch64-pc-windows-msvc optional later (both Tier 1) |
| Install scope | per-user, %LOCALAPPDATA%\Programs (Zed model); matches HKCU self-registration and updater needs |
| AUMID / protocol | nothing in the installer: app self-installs AUMID shortcut (toast_identity.rs) and gpui-starter:// HKCU registration (deep_link_registration.rs) |
| Signing | default off; SHA256SUMS + Ed25519 manifest; optional Azure Artifact Signing step behind AZURE_* secrets (Basic $9.99/mo, individuals USA/Canada) |
| winget | community winget-pkgs PR for the portable zip after first release; sha256 from generate-manifest.sh |
| Auto-update | manifest windows-x86_64 -> bare exe asset; swap needs two-step rename patch in apply.rs (src/, unassigned) |

Interface flags for the decider (all outside current allowed paths):
`scripts/create-release-app.sh` (icns copy + plist key), root `build.rs`
(winresource), `scripts/install-linux.sh` (multi-size loop),
`src/services/updater/apply.rs` (windows rename dance).
