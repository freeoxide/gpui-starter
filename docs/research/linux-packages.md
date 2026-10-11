# Linux package formats for gpui-starter

Research snapshot: 2026-10-09. Deep dive into every Linux package format for this
app: mechanics, required files, the GPU/sandbox story, and one concrete tool
decision per format for the implementers of areas 5-8. Companion doc:
[release-matrix.md](release-matrix.md) (counted format matrix, tool statuses,
updater policy). Sources are inline; load-bearing claims were verified by
fetching the linked page, not from search snippets.

## Baseline: what this repo already ships

Facts the per-format decisions build on (all from this repo's files):

- The Linux tarball (`scripts/` in release.yml) bundles every non-glibc `.so`
  next to the binary under `lib/` and sets `patchelf --set-rpath '$ORIGIN/lib'`.
  glibc, libm, ld-linux and friends are left to the host.
- `resources/com.gpui-starter.app.desktop` carries app id
  `com.gpui-starter.app`, `StartupWMClass=gpui-starter`, and
  `MimeType=x-scheme-handler/gpui-starter;` (deep links).
- `resources/gpui-starter.png` is a single 512x512 PNG; the full hicolor set is
  area 4's deliverable.
- `scripts/gen-pkgbuild.sh` generates a PKGBUILD inside the tarball that
  installs the bundled layout into `/usr/lib/gpui-starter` and declares
  `depends=('vulkan-driver' 'libxkbcommon-x11' 'wayland' 'fontconfig' 'freetype2')`.
- Cargo feature `notifications-portal` (Cargo.toml, Linux target table) routes
  notifications through `org.freedesktop.portal.Notification` via ashpd 0.13
  (`notification` + `async-io` features) for sandboxed runs; native installs
  never need it.
- `flake.nix` builds from source with `buildRustPackage`, patchelf-adding the
  Nix-store `vulkan-loader` and `wayland` to the RUNPATH.
- Rendering is Vulkan-only on Linux (GPUI's blade backend); there is no OpenGL
  fallback, so no `libgl1` dependency anywhere. The runtime library set matches
  `scripts/install-linux-deps.sh` and `flake.nix`: Vulkan loader, xkbcommon
  (+x11), wayland, xcb, fontconfig, freetype, openssl.

## 1. .deb

### Tooling

- **cargo-deb 3.8.0** (2026-09), actively maintained by kornelski
  ([crates.io](https://crates.io/crates/cargo-deb),
  [repo](https://github.com/kornelski/cargo-deb)). Reads identity from standard
  Cargo.toml fields (name, version, license, description, repository); Debian
  specifics go under `[package.metadata.deb]`: `maintainer`, `copyright`,
  `extended-description-file`, `depends` (auto-generated from shlibs when
  absent, `$auto` to mix), `recommends`, `suggests`, `assets` (`[source, dest,
  mode]` triples), `maintainer-scripts` (preinst/postinst/prerm/postrm),
  `conf-files`, variants via `[package.metadata.deb.variants.<name>]`, and
  `--no-build` for packaging an already-built binary. Output:
  `target/debian/<name>_<version>-1_<arch>.deb`.
- **Hand-rolled dpkg-deb tree**: full control, zero Rust tooling, but you own
  control-file syntax, md5sums, permissions and compression yourself. This is
  what the only real Zed deb does (below).
- **cargo-packager** (0.11.8, CrabNebula): multi-format packager split from
  Tauri ([docs.rs](https://docs.rs/cargo-packager)). Capable, but deb/rpm via
  cargo-deb/cargo-generate-rpm have deeper per-format control and more
  GPUI-ecosystem precedent (OpenLogi, Lapce-adjacent tooling).

### Zed's actual deb story (correction to the brief)

Zed ships **no .deb at all**. `script/bundle-linux` produces only the tarball:
`zed.app/` with `bin/`, `libexec/`, bundled `lib/`, desktop file in
`share/applications/`, hicolor icons at 512 and 1024
([bundle-linux](https://github.com/zed-industries/zed/blob/main/script/bundle-linux),
[zed.dev/docs/linux](https://zed.dev/docs/linux)). The known community deb
([dariogriffo/zed-debian](https://github.com/dariogriffo/zed-debian))
hand-rolls a dpkg tree in Debian/Ubuntu Docker containers by repackaging the
official tarball; its control file declares only
`Depends: ${misc:Depends}, libxkbcommon-x11-0, libopenblas0` because the
tarball already bundles everything else
([debian/control](https://github.com/dariogriffo/zed-debian/blob/master/debian/control)).
That is the closest thing to "Zed's .deb approach": bundle everything, depend
on almost nothing.

### Mechanics and files

A minimal binary deb needs `DEBIAN/control` (Package, Version, Architecture,
Maintainer, Depends, Description; field grammar in
[deb-control(5)](https://manpages.ubuntu.com/manpages/trusty/man5/deb-control.5.html)),
`DEBIAN/md5sums`, and the payload tree:

| File | Path |
|------|------|
| binary | `/usr/bin/gpui-starter` (or bundled layout: `/usr/lib/gpui-starter/gpui-starter` + `lib/`, as the PKGBUILD does) |
| desktop entry | `/usr/share/applications/com.gpui-starter.app.desktop` |
| icons | `/usr/share/icons/hicolor/<size>/apps/gpui-starter.png` (16..512) + optional `scalable/apps/gpui-starter.svg` |
| metainfo | `/usr/share/metainfo/com.gpui-starter.app.metainfo.xml` |
| license/doc | `/usr/share/doc/gpui-starter/copyright`, `changelog.gz` |

**Maintainer scripts: none needed.** `update-desktop-database` and the icon
cache refresh are owned by dpkg triggers: `desktopfile-utils` declares interest
in `/usr/share/applications` and `hicolor-icon-theme` in the icon directories,
so dpkg runs them automatically after unpack when those packages are installed
([DpkgTriggers wiki](https://wiki.debian.org/DpkgTriggers);
[Signal-Desktop #3694](https://github.com/signalapp/Signal-Desktop/issues/3694)
records the same conclusion: calling it in postinst is redundant). A postinst
that guards on `command -v update-desktop-database` is harmless belt-and-braces
but lintian-clean packages omit it.

**Dependency lines.** Two coherent strategies:

1. *Bundled (tarball-equivalent)*: reuse the existing `$ORIGIN/lib` tree.
   `Depends: $auto` resolves to near-empty; `Suggests: mesa-vulkan-drivers`
   documents the ICD requirement without apt force-installing 137 MB of Mesa on
   NVIDIA systems ([mesa-vulkan-drivers](https://packages.debian.org/sid/mesa-vulkan-drivers),
   [libvulkan1](https://packages.debian.org/sid/libvulkan1)). zed-debian proves
   the pattern.
2. *System-libs*: `Depends: libvulkan1, libxkbcommon0, libxkbcommon-x11-0,
   libwayland-client0, libxcb1, libfontconfig1, libfreetype6, libssl3` (the
   runtime set from `flake.nix` / `install-linux-deps.sh`; cargo-deb's `$auto`
   derives most of it). Cleaner lintian, but the deb then only runs where the
   build host's glibc and sonames are satisfied.

**lintian interaction with the bundled strategy**: `binary-or-shlib-defines-rpath`
(recently renamed `custom-library-search-path`,
[lintian tags](https://lintian.debian.org/tags/binary-or-shlib-defines-rpath.html))
and `embedded-library` will fire on `$ORIGIN` rpath plus private `.so` copies.
Debian policy forbids both for archive packages; a third-party GitHub-release
deb is not an archive package, zed-debian ships exactly this way, and lintian
is advisory there. Fixing them (system-libs variant) costs the portability the
tarball strategy exists for.

**glibc floor**: bundling does not bundle glibc. A deb built on ubuntu-24.04
(glibc 2.39) installs only on glibc >= 2.39 systems regardless of bundling.
Lapce covers this by building 10 debs across 5 distro targets
([v0.4.6](https://github.com/lapce/lapce/releases/tag/v0.4.6)). The pragmatic
middle for a starter repo: build the deb inside a `debian:bookworm` container
(glibc 2.36) in CI, or accept 24.04+ initially. Zed's published floors are
2.31/2.35 ([docs](https://zed.dev/docs/linux)).

**Debian archive vs own repo**: getting into Debian proper requires the New
Maintainer/sponsor process ([Debian wiki](https://wiki.debian.org/DebianMaintainer));
a one-package upstream project distributes its own deb from GitHub Releases or
its own apt repo and needs none of that.

**apt repo hosting** (if ever wanted): Zed has none (verified,
[zed.dev/docs/linux](https://zed.dev/docs/linux)); the brief's "zed's apt repo
via Cloudsmith" does not exist. Real options: Launchpad PPA (free, Ubuntu-only,
builds from source on Launchpad's buildds), Cloudsmith (managed, free
open-source tier, [debian hosting](https://cloudsmith.com/product/formats/debian-repository)),
or reprepro on GitHub Pages using the same gh-pages branch pattern as
manifest.json ([create-apt-repository action](https://github.com/marketplace/actions/create-apt-repository),
[walkthrough](https://linsomniac.com/post/2025-03-18-building_and_publishing_apt_repos_to_github_pages)).
All require a GPG signing key for `Release`/`InRelease`.

### Decision for this repo

Use **cargo-deb 3.8.0** with the **bundled layout** (binary + `lib/` under
`/usr/lib/gpui-starter`, mirroring the PKGBUILD, or `/usr/bin` + rpath
`$ORIGIN/../lib`; area 5 picks one and documents it). Configure
`[package.metadata.deb]` in Cargo.toml, build via `scripts/build-deb.sh` that
first runs the existing bundle step then `cargo deb --no-build`. No maintainer
scripts. `Depends: $auto`, `Suggests: mesa-vulkan-drivers`. Accept the two
lintian tags as documented cost of the bundling strategy. No apt repo; the deb
is a GitHub Release asset. Later reach widening: rebuild inside
`debian:bookworm` container (area 10's call).

## 2. Flatpak

### Runtime and the Vulkan story

Use **org.freedesktop.Platform/SDK 25.08**: 24.08 is EOL (final release
24.08.37; users directed to 25.08/26.08)
([freedesktop-sdk releases](https://gitlab.com/freedesktop-sdk/freedesktop-sdk/-/releases),
[25.08.0 announcement](https://discourse.flathub.org/t/freedesktop-sdk-25-08-0-released/10399)).

Vulkan lands without bundling: the freedesktop Platform runtime's `platform.bst`
depends on `components/vulkan-icd-loader.bst`, so `libvulkan.so.1` ships in the
Platform itself
([vulkan-icd-loader element](https://freedesktop-sdk.gitlab.io/documentation/reference/api-reference/fdsdk/elements/components/vulkan-icd-loader.html)).
Drivers come from the host at install time: Mesa via the default
`org.freedesktop.Platform.GL.*` extension, NVIDIA via
`org.freedesktop.Platform.GL.nvidia`, which is generated per driver version and
fetched from NVIDIA servers as extra-data
([Igalia: Flatpak with NVIDIA](https://blogs.igalia.com/vjaquez/digging-further-into-flatpak-with-nvidia/),
[GL extension discussion](https://gitlab.com/freedesktop-sdk/freedesktop-sdk/-/issues/964)).
The app needs no driver code of its own, only device access.

### Manifest and finish-args

Structure per [flatpak manifest docs](https://docs.flatpak.org/en/latest/manifests.html)
and [flatpak-manifest(5)](https://manpages.debian.org/testing/flatpak-builder/flatpak-manifest.5.en.html).
Finish-args for this app (adapted from the closest precedent, the Rust GPU
editor on Flathub, verified from its manifest):

- `--device=dri` (GPU device nodes; the canonical GPU-app arg,
  [flatpak-builder reference](https://docs.flatpak.org/en/latest/flatpak-builder-command-reference.html))
- `--socket=wayland` + `--socket=fallback-x11` + `--share=ipc` (X11 needs ipc;
  fallback-x11 keeps Wayland native and X11 as fallback)
- `--share=network` (the app has reqwest/websocket features; note Flathub
  requires permissions be kept to a minimum,
  [requirements](https://docs.flathub.org/docs/for-app-authors/requirements))
- **No** `--talk-name=org.freedesktop.portal.*`: the sandbox's session-bus
  filter always allows talking to `org.freedesktop.portal.*` names; only the
  app's own name, `org.freedesktop.DBus`, and portals pass by default
  ([sandbox permissions](https://docs.flatpak.org/en/latest/sandbox-permissions.html)).
  The brief's suggestion to add the talk-name is unnecessary.
- No pulseaudio (no audio in the app).

[dev.lapce.lapce on Flathub](https://flathub.org/en/apps/dev.lapce.lapce) is
the template: runtime `org.freedesktop.Sdk 25.08` with the
`org.freedesktop.Sdk.Extension.rust-stable` extension, finish-args
`--device=dri --share=ipc --share=network --socket=wayland --socket=fallback-x11`
(plus `--filesystem=host`/`--talk-name=org.freedesktop.Flatpak` for its
host-shell needs, which this app does not have). Manifest verified:
[dev.lapce.lapce.yaml](https://github.com/flathub/dev.lapce.lapce/blob/master/dev.lapce.lapce.yaml).

### Building Rust inside flatpak-builder

Flathub builds have no network access; everything must be in the manifest
([requirements](https://docs.flathub.org/docs/for-app-authors/requirements)).
The standard flow (also Lapce's):

1. `flatpak-cargo-generator.py Cargo.lock -o generated-sources.json` from
   [flatpak-builder-tools](https://github.com/flatpak/flatpak-builder-tools/blob/master/cargo/README.md).
2. Manifest module uses `org.freedesktop.Sdk.Extension.rust-stable`, sets
   `CARGO_HOME=/run/build/<module>/cargo`, and runs
   `cargo --offline build --release` with `PATH+=/usr/lib/sdk/rust-stable/bin`.
3. **This app's difference**: the build command must pass
   `--features notifications-portal` so the ashpd portal backend (the
   `dep:ashpd` optional dependency behind the feature) compiles into the
   sandboxed build; native builds keep it off (Cargo.toml Linux target table).

flatpak-builder also has a native `cargo` source type now
([command reference](https://docs.flatpak.org/en/latest/flatpak-builder-command-reference.html));
the generated-sources approach remains the Flathub norm.

### Deep links and desktop integration

Desktop file must be named after the app id and placed in
`/app/share/applications/` before `build-export`
([conventions](https://docs.flatpak.org/en/latest/conventions.html)). The
existing `MimeType=x-scheme-handler/gpui-starter;` in the desktop file is
exported automatically; flatpak rewrites `Exec` to `flatpak run ...` at install
([command reference](https://docs.flatpak.org/en/latest/flatpak-command-reference.html)).

### AppStream requirements

Mandatory metainfo passing `appstreamcli validate` with zero errors *and*
warnings for Flathub; needs `<launchable type="desktop-id">`, `<releases>`,
OARS `<content_rating type="oars-1.1">`, screenshots, and a `<developer>` tag
with reverse-DNS id
([requirements](https://docs.flathub.org/docs/for-app-authors/requirements),
[metainfo guidelines](https://docs.flathub.org/docs/for-app-authors/metainfo-guidelines)).

### Flathub submission: the app-id blocker

`com.gpui-starter.app` **cannot be submitted to Flathub as-is**. Three
independent violations of the ID rules
([requirements](https://docs.flathub.org/docs/for-app-authors/requirements)):

1. IDs must not end in generic terms; the list explicitly includes `.app`.
2. A dash is allowed only in the last component; `gpui-starter` is a middle
   component.
3. The domain in the ID must be controlled by the submitter (here
   `gpui-starter.com`, which the project does not own).

GitHub-hosted projects must use `io.github.<owner>.<Product>` (4+ components,
`com.github.*` reserved). Renaming the desktop id for Flatpak diverges from the
app id used by notify-rust's `DesktopEntry` hint and the update manifest, so a
Flathub ID change is a project-level decision, not a packaging detail.

### Updater inside the sandbox

`--share=network` would let the binary fetch manifest.json and GitHub URLs,
but the app cannot replace its own files under `/app`; updates are owned by
`flatpak update` ([using flatpak](https://docs.flatpak.org/en/latest/using-flatpak.html)).
Policy carried from release-matrix.md: `GPUI_UPDATE_MANIFEST_URL` stays unset
in flatpak builds.

### CI

GitHub-hosted Ubuntu runners have no FUSE, so raw flatpak-builder struggles;
the standard fix is the `flatpak/flatpak-github-actions` action running in the
bilelmoussaoui container that builds flatpak-builder without FUSE
([action](https://github.com/marketplace/actions/flatpak-builder),
[Docker image](https://hub.docker.com/r/bilelmoussaoui/flatpak-github-actions)).
Flathub's own CI uses the same machinery
([Flathub docs](https://docs.flathub.org/docs/for-app-authors/github-actions)).
Alternative: `andyholmes/flatter` for bundle+repo export.

### Decision for this repo

Ship a **local `.flatpak` bundle** (flatpak-builder manifest in
`packaging/flatpak/`, ID `com.gpui-starter.app` kept, runtime 25.08,
rust-stable extension, generated-sources, `--features notifications-portal`,
finish-args as above) following the sonora precedent of direct bundles rather
than Flathub ([sonora](https://github.com/sonorahq/sonora/releases/latest)).
Flathub submission is blocked on the app id; document the rename decision
(`io.github.<owner>.<Product>`) as the prerequisite and defer. The bundle is
built in CI via `flatpak/flatpak-github-actions` (area 10).

## 3. AppImage

### Tooling

**linuxdeploy + linuxdeploy-plugin-appimage** (`--output appimage`):
linuxdeploy maintains the AppDir (deploys dependencies, desktop file, icons,
generates AppRun), the plugin wraps it into an AppImage via appimagetool
([linuxdeploy](https://github.com/linuxdeploy/linuxdeploy),
[plugin-appimage](https://github.com/linuxdeploy/linuxdeploy-plugin-appimage),
[distinction thread](https://discourse.appimage.org/t/distinction-between-linuxdeploy-and-appimagetool/1128)).
`cargo-appimage` 2.4.0 exists but wraps appimagetool only
([crates.io](https://crates.io/crates/cargo-appimage)); linuxdeploy is the
standard with the GPUI/Helix/Lapce precedent.

### AppDir layout vs the existing bundle

linuxdeploy's canonical layout is `usr/bin/<binary>` + `usr/lib/...` with
rpath `$ORIGIN/../lib`, desktop file under `usr/share/applications`, icons
under `usr/share/icons` (auto-placed from `--desktop-file`/`--icon-file`
options, [linuxdeploy user guide](https://docs.appimage.org/packaging-guide/from-source/linuxdeploy-user-guide.html)).
The tarball's `$ORIGIN/lib` (binary at root, `lib/` beside it) survives inside
an AppDir unchanged if a small custom AppRun script `exec`s the root-level
binary; AppRun is just a launcher ([AppImage packaging guide](https://docs.appimage.org/packaging-guide/)).
Either layout works; the custom-AppRun route reuses the staged tarball tree
byte-for-byte, which is what sonora/frame-style AppImage builds do. Since
every non-glibc library is already bundled, linuxdeploy's dependency walk
finds nothing left to deploy.

### FUSE story on user machines

Type-2 AppImages need FUSE 2: Ubuntu 22.04+ stopped preinstalling libfuse2 and
24.04 renamed it `libfuse2t64`
([troubleshooting](https://docs.appimage.org/user-guide/troubleshooting/fuse.html),
[OMG!Ubuntu](https://www.omgubuntu.co.uk/2023/04/appimages-libfuse2-ubuntu-23-04)).
Fallbacks: `sudo apt install libfuse2`/`libfuse2t64`, or
`./AppImage --appimage-extract-and-run`. Not a build blocker; a README note
covers it. On Wayland an AppImage is just an executable; display-server
support comes from the binary (which speaks both Wayland and X11), not the
format.

### CI: FUSE again

GitHub-hosted runners lack `/dev/fuse`; linuxdeploy and appimagetool are
themselves AppImages, so builds must set `APPIMAGE_EXTRACT_AND_RUN=1` (or pass
`--appimage-extract-and-run`)
([AppImageKit FUSE wiki](https://github.com/appimage/appimagekit/wiki/fuse),
[issue #841](https://github.com/AppImage/AppImageKit/issues/841)).

### Arch support and precedent

aarch64 is first-class: official `linuxdeploy-aarch64.AppImage` and
`appimagetool-aarch64.AppImage` builds; `ARCH=aarch64` selects the target
([appimagetool](https://github.com/AppImage/appimagetool),
[linuxdeploy #258](https://github.com/linuxdeploy/linuxdeploy/issues/258)).
The release workflow's `ubuntu-24.04-arm` runner builds the aarch64 AppImage
natively. Precedent: Zed ships no official AppImage
([issue #8137](https://github.com/zed-industries/zed/issues/8137), community
builds like [zedx-appimage](https://github.com/zed-industries/zed/discussions/49225));
Lapce, Helix, sonora, frame, zedis all ship AppImages with `.zsync` delta
metadata (see release-matrix.md). Tauri's `tauri-action` AppImage output is
further webview-app precedent for CI wiring.

### Decision for this repo

`scripts/build-appimage.sh` (area 6): stage the existing tarball layout into an
AppDir root (binary + `lib/` + custom AppRun + existing desktop file and icon),
run linuxdeploy `--appdir ... --desktop-file ... --icon-file ...
--output appimage` with `APPIMAGE_EXTRACT_AND_RUN=1`, `OUTPUT=` naming the
artifact `GPUI-Starter-<ver>-<arch>.AppImage`, per arch (x86_64 + aarch64),
plus the matching `.zsync`. `GPUI_UPDATE_MANIFEST_URL` stays set (AppImage is
on the updater-compatible list).

## 4. .rpm

### Tooling and config

**cargo-generate-rpm 0.21.0** (cat-in-136, maintained; builds a binary rpm
from Cargo metadata with no spec file and no rpmbuild)
([repo](https://github.com/cat-in-136/cargo-generate-rpm),
[crates.io](https://crates.io/crates/cargo-generate-rpm)). Config in
`[package.metadata.generate-rpm]`: `assets = [[source, dest, mode], ...]`
(with optional user/group/config flags), `requires`, `requires-compat`,
`obsoletes`, `conflicts`, `provides`; `--set-metadata` can override per build.

Same two strategies as deb. Bundled layout keeps `requires` minimal; the
system-libs variant needs the Fedora/openSUSE names, which differ from Debian's
(`vulkan-loader` on Fedora vs `libvulkan1` on SUSE; `libxkbcommon`/
`libwayland-client`; consult
[Fedora packages](https://packages.fedoraproject.org/) vs
[openSUSE](https://build.opensuse.org/)). Bundling sidesteps the split
entirely, which is why Lapce/OpenLogi/zedis all bundle.

### Signing

`rpmsign` with a GPG key is required only when users install through a repo
with `gpgcheck=1`; `dnf install ./file.rpm` on a GitHub-release artifact shows
a bypassable warning ([rpmsign(8)](https://rpm.org/docs/4.20.x/man/rpmsign.8)).
Fedora-based users additionally hit the unsigned-localinstall prompt. For
GitHub-release-only rpms, signing is not worth the CI key management; the
existing SHA256/Ed25519 manifest covers integrity. Revisit only if a COPR/OBS
repo ever ships.

### openSUSE/Fedora differences

With bundled libraries the payload is distro-neutral; only the glibc floor
matters. Fedora 41 ships glibc 2.40 and openSUSE Tumbleweed is newer still, so
an rpm built on ubuntu-24.04 (glibc 2.39) installs on both; older enterprise
targets would need an older build base. Cross-distro smoke-testing is optional
via a Fedora container on the ubuntu runner.

### Decision for this repo

**cargo-generate-rpm 0.21.0**, `[package.metadata.generate-rpm]` mirroring the
deb's asset map, bundled layout, `requires` minimal (mirror the PKGBUILD's
`vulkan-driver` equivalent: nothing beyond what rpm auto-detects; add
`vulkan-loader` only in the system-libs variant). No signing. Built by
`scripts/build-rpm.sh` (area 8) on the same runners as the deb.

## 5. Snap

### Mechanics

`snapcraft.yaml` with `base: core24` (current LTS base; core22 is the older
one). GPU access for Vulkan/OpenGL on core24 comes from the `gpu` extension /
`gpu-2404` content snap, which provides the driver stack including NVIDIA
([gpu extension docs](https://ubuntu.com/docs/snapcraft/9/reference/extensions/gpu-extension),
[forum: GPU support](https://forum.snapcraft.io/t/adding-opengl-gpu-support-to-a-snap/6273),
[gpu-2404 how-to](https://canonical.com/mir/docs/2.30/contributing/how-to/enable-graphics-for-snaps-on-a-device)).
Sockets/interfaces needed: `opengl`, `wayland`, `x11`, `desktop`,
`desktop-legacy`, `network`
([opengl interface](https://snapcraft.io/docs/reference/interfaces/opengl-interface),
[wayland interface](https://snapcraft.io/docs/reference/interfaces/wayland-interface)).
The `gnome` extension + `gnome-46-2404` content snap is for GTK/GLib apps;
GPUI uses neither, so it is not needed, and legacy `desktop-glib-interface`
stacks are superseded on core24
([gnome extension](https://ubuntu.com/docs/snapcraft/9/reference/extensions/gnome-extension/),
[forum threads](https://forum.snapcraft.io/t/gnome-extension-for-core24/40228)).

### Confinement tradeoffs

Strict confinement works with the interfaces above (opengl/wayland/x11/desktop
auto-connect on classic Ubuntu desktops). Classic confinement requires a
per-snap forum review; the community Zed snaps went through exactly that
contention ([classic confinement for Zed](https://forum.snapcraft.io/t/classic-confinement-for-zed/45782)).
Strict + gpu extension is the viable path for a Vulkan renderer.

### Store publishing effort

Developer account, then name registration, which is now under manual review
for all new names after fraud incidents
([manual review policy](https://forum.snapcraft.io/t/manual-review-of-all-new-snap-name-registrations/39440)),
then store review on each release. Zed has no official snap and even the name
`zed` is held by a third party
([forum: snap-requests](https://forum.snapcraft.io/t/zed-rust-based-ide/40993);
[unofficial zed-online](https://snapcraft.io/zed-online) is flagged stale).
Lapce is on the store ([snapcraft.io/lapce](https://snapcraft.io/lapce)), so a
Rust GPU editor snap is provably doable. CI-wise, builds need LXD via
`canonical/action-build` (formerly snapcore/action-build,
[action](https://github.com/canonical/action-build)).

### Decision for this repo

**Document-only, skip building** (matches release-matrix.md's skip list). The
repo has no store account, the GPU snap story needs live testing against
driver stacks a CI cannot provide, and the per-release review queue is ongoing
cost. Area 8 keeps a commented `packaging/snap/snapcraft.yaml` sketch (strict,
core24, gpu extension, `notifications-portal` feature enabled) plus a README
section pointing at the interfaces above.

## 6. AUR

`scripts/gen-pkgbuild.sh` already emits a PKGBUILD into the tarball. For actual
AUR submission the rules that matter
([AUR submission guidelines](https://wiki.archlinux.org/title/AUR_submission_guidelines)):

- Prebuilt-binary packages use the `-bin` suffix convention: submit as
  `gpui-starter-bin` (source-building `gpui-starter` would need the full Rust
  toolchain chain in makedepends; possible but slower for users).
- `sha256sums=('SKIP')` in the generated file is fine for an in-tarball
  convenience copy but **not acceptable on the AUR**: SKIP is only sanctioned
  for VCS sources. The AUR copy must carry the real per-release sha256 of the
  release tarball.
- Every PKGBUILD change needs a regenerated `.SRCINFO`
  (`makepkg --printsrcinfo > .SRCINFO`) committed beside it.
- Maintenance: pkgver bumps per release, responding to comments; AUR packages
  that build things already in official repos are rejected (not a risk here).

The existing `depends` list in gen-pkgbuild.sh (`vulkan-driver`
`libxkbcommon-x11` `wayland` `fontconfig` `freetype2`) is correct: Arch's
`vulkan-driver` is the virtual package every ICD (mesa, nvidia) provides.

### Decision for this repo

Keep the in-tarball PKGBUILD as-is (it targets manual `makepkg` use and works
today). AUR submission is a documented community/later step: submit
`gpui-starter-bin` with real checksums once releases are routine. No build
work in areas 5-8.

## 7. Nix

`flake.nix` already builds the app from source on any commit: `buildRustPackage`
with the Cargo.lock, `allowBuiltinFetchGit` for the git deps, and a postFixup
patchelf adding the Nix store's `vulkan-loader` and `wayland` to the RUNPATH.
Nix users consume `github:<owner>/gpui-starter` directly; nothing about
releases needs to change. Inclusion in nixpkgs proper is a community process
initiated by a nixpkgs PR (as happened for `zed-editor`,
[nixpkgs](https://search.nixos.org/packages?channel=unstable&show=zed-editor)).

### Decision for this repo

Document, do not build. Area 11's release docs mention the flake and the
RUNPATH mechanism so future dependency changes keep it working.

## 8. Cross-cutting

### AppStream metainfo (shared by deb / rpm / flatpak)

One file, `com.gpui-starter.app.metainfo.xml`, installed at
`/usr/share/metainfo/` (deb/rpm) and `/app/share/metainfo/` (flatpak).
Required/expected tags per the spec
([desktop applications](https://www.freedesktop.org/software/appstream/docs/sect-Metadata-Application.html),
[upstream metadata](https://www.freedesktop.org/software/appstream/docs/chap-Metadata.html)):

- `<id>com.gpui-starter.app</id>` matching the desktop-file name
  ([Debian guidelines](https://wiki.debian.org/AppStream/Guidelines))
- `<launchable type="desktop-id">com.gpui-starter.app.desktop</launchable>`
- `<releases>` with one `<release version="X.Y.Z" date="...">` per release
- `<content_rating type="oars-1.1"/>` (Flathub requires OARS;
  [metainfo guidelines](https://docs.flathub.org/docs/for-app-authors/metainfo-guidelines))
- `<provides>` (binary `gpui-starter`, mediatype `x-scheme-handler/gpui-starter`)
- `<developer id="...">`, `<branding>` for Flathub screenshots/brand color
- screenshots in `<screenshots>` (Flathub wants at least one)

Area 7 owns the file; areas 5/8 install the same file unchanged.

### Validation commands (CI gate)

```sh
desktop-file-validate resources/com.gpui-starter.app.desktop
appstreamcli validate --explain resources/com.gpui-starter.app.metainfo.xml
```

Flathub treats warnings as blockers
([metainfo guidelines](https://docs.flathub.org/docs/for-app-authors/metainfo-guidelines));
flatpak's conventions doc recommends exactly these two commands
([conventions](https://docs.flatpak.org/en/latest/conventions.html)).

### Icons

hicolor layout is `<theme>/<size>x<size>/apps/<name>.png` plus `scalable/apps`
([icon theme spec](https://specifications.freedesktop.org/icon-theme/latest));
the `hicolor-icon-theme` package's index lists 16, 22, 24, 32, 36, 48, 64, 72,
96, 128, 192, 256, 512 and scalable/symbolic
([package](https://packages.debian.org/sid/hicolor-icon-theme)). Packages
should ship the full raster set (16..512) plus an SVG in scalable; Flatpak
requires at least a 256x256 PNG or SVG
([requirements](https://docs.flathub.org/docs/for-app-authors/requirements));
linuxdeploy picks icons from `--icon-file` (largest wins). Generation is area
4's job (`scripts/gen-icons.sh`); areas 5-8 consume the set.

### Checksums and signing

Per-release `SHA256SUMS` plus the existing Ed25519-signed manifest.json cover
integrity for every format here. GPG signing enters only with hosted repos
(apt Release/InRelease, rpm gpgcheck) or Flathub (store-held keys); none of
those are in the ship set.

### Which CI runner builds what

| Format | Runner | Notes |
|--------|--------|-------|
| deb | ubuntu-24.04 + ubuntu-24.04-arm (existing matrix) | cargo-deb native per arch; bookworm container optional for reach |
| rpm | same runners | cargo-generate-rpm needs no rpmbuild; Fedora container optional for smoke tests |
| AppImage | same runners | APPIMAGE_EXTRACT_AND_RUN=1; aarch64 tools on the arm runner |
| flatpak bundle | ubuntu runner in flatpak-github-actions container | host flatpak-builder lacks FUSE on runners ([issue](https://github.com/actions/runner-images/issues/11359), [image](https://hub.docker.com/r/bilelmoussaoui/flatpak-github-actions)) |
| snap (if ever) | ubuntu runner | LXD via canonical/action-build ([action](https://github.com/canonical/action-build)) |

## Comparison table

| Format | Tool | Maintainer effort | GPU/sandbox story | Updates owned by | Verdict |
|--------|------|-------------------|-------------------|------------------|---------|
| tarball + install.sh | existing | already shipped | host Vulkan ICDs, bundled loader | in-app updater (manifest URL baked) | keep |
| .deb | cargo-deb 3.8.0 | config only | bundled loader+libs; host ICDs; no sandbox | apt / manual dpkg -i | ship |
| .rpm | cargo-generate-rpm 0.21.0 | config only | same as deb | dnf / manual | ship |
| AppImage (+.zsync) | linuxdeploy + plugin | one script | same as deb; FUSE2 on host or extract-and-run | in-app updater + zsync deltas | ship |
| .flatpak bundle | flatpak-builder manifest | manifest + cargo-sources regen per release | runtime ships libvulkan; host drivers via GL extensions; portals for notifications | flatpak update | ship (local bundles) |
| Flathub | same + submission | ID rename + review + per-release builds | same | Flathub | blocked on app id; document |
| AUR | PKGBUILD (exists) | per-release bump + checksums | host ICDs (vulkan-driver virt) | AUR helper | document (community) |
| nix | flake.nix (exists) | none | Nix-store loader on RUNPATH | nix flake update | document |
| snap | snapcraft.yaml | store account, manual name review, per-release review | gpu-2404 content snap, strict confinement | snapd (4x daily refresh) | document-only |
| apt repo | reprepro on gh-pages or Cloudsmith | GPG key + repo upkeep | n/a | apt | defer |

## Recommended set (maximum breadth, ranked)

Ship, in build order:

1. **.deb** (area 5): biggest demand-per-effort; cargo-deb config only.
2. **AppImage + .zsync** (area 6): runs-anywhere artifact, reuses the staged
   tarball tree verbatim.
3. **.rpm** (area 8): same shape as the deb job.
4. **.flatpak bundle** (area 7): real sandbox testing for the
   notifications-portal feature; direct bundles, no store.
5. **AUR `gpui-starter-bin`**: zero CI; a PKGBUILD variant with real
   checksums, submitted once releases are routine.

Document-only:

6. **Flathub**: blocked by the `.app`/dash/domain app-id rules; write the
   rename decision down, revisit if the project adopts a real domain.
7. **nix**: already works; a docs paragraph.
8. **Snap**: sketch yaml + pointers; store overhead not justified for a
   starter repo (no GPUI app is on the store; Zed's own name is squatted).
9. **apt repo**: defer until users ask; gh-pages reprepro is the zero-infra
   route when needed.

## Interface notes for areas 5-8 and 10

- **Area 5 (deb)**: cargo-deb 3.8.0; `[package.metadata.deb]` in Cargo.toml;
  `scripts/build-deb.sh` runs the existing bundle step then
  `cargo deb --no-build`; assets map binary, `lib/`, desktop file, hicolor set,
  metainfo, LICENSE; NO maintainer scripts (dpkg triggers own desktop/icon
  caches); `Depends: $auto`, `Suggests: mesa-vulkan-drivers`; lintian
  `custom-library-search-path` + `embedded-library` accepted and documented;
  GPUI_UPDATE_MANIFEST_URL unset for this artifact.
- **Area 6 (AppImage)**: linuxdeploy + appimage plugin, AppDir = staged
  tarball layout + custom AppRun; `APPIMAGE_EXTRACT_AND_RUN=1` in the script;
  `OUTPUT` names `GPUI-Starter-<ver>-<arch>.AppImage` (+ `.zsync`); aarch64
  via the arm runner with aarch64 linuxdeploy; updater URL stays set.
- **Area 7 (flatpak)**: manifest in `packaging/flatpak/`; runtime+sdk 25.08;
  `org.freedesktop.Sdk.Extension.rust-stable`; sources from
  flatpak-cargo-generator (`generated-sources.json`, regenerated per release);
  build-commands append `--features notifications-portal`; finish-args:
  `--device=dri --socket=wayland --socket=fallback-x11 --share=ipc
  --share=network`; no portal talk-names; metainfo at
  `resources/com.gpui-starter.app.metainfo.xml` passing `appstreamcli validate`
  with zero warnings; updater URL unset.
- **Area 8 (rpm/snap)**: cargo-generate-rpm 0.21.0 mirroring the deb assets;
  minimal requires; unsigned. Snap: commented sketch only, strict + core24 +
  gpu extension + `notifications-portal` in build-features.
- **Area 10 (CI)**: add deb/rpm/AppImage steps to build-linux matrix jobs;
  flatpak bundle job uses `flatpak/flatpak-github-actions` in container;
  per-artifact GPUI_UPDATE_MANIFEST_URL control (set for tarball/AppImage,
  unset for deb/rpm/flatpak) is the one workflow change the policy needs.
- **Shared dependency (area 4)**: the hicolor 16..512 set + SVG must exist
  before deb/rpm/flatpak assets can be mapped; all three consume
  `assets/app-icon/` output.
