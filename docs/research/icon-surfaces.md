# Runtime icon surfaces for gpui-starter

Research for run 6 area 1 (audit:icon-surfaces). Every runtime surface
where an icon reaches the user was read in the code before deciding;
external claims carry fetched sources. The branded asset set
(`assets/app-icon/`: SVG master, hicolor 16-512 PNGs, scalable SVG,
.icns, .ico) landed in run 5; this audit decides how areas 2-5 wire it
into surfaces that still show generic or off-brand imagery.

Companion docs: `docs/research/icons-and-windows.md` (asset pipeline,
AUMID/ico embed), `docs/research/linux-packages.md` (hicolor installs per
package format), `docs/releasing.md` (open-decisions list, swept in
section 6 below).

## 1. Notifications

### 1a. Send-sites and the abstraction layer

Layer: `NotificationBackend` trait
(`src/services/notifications/backend/mod.rs:31`), per-platform backends
cfg-gated at `backend/mod.rs:1-18`:

| Backend | File | Icon today | Fresh-machine behavior |
|---|---|---|---|
| notify-rust (Linux native + mac/win fallback) | `backend/notify_rust.rs` | none — sets `.appname` + `Hint::DesktopEntry` only (`:55-66`) | see 1b |
| XDG portal (Flatpak, `notifications-portal` feature) | `backend/portal.rs` | none — `Notification::new(title).body(...)` (`:56-58`) | see 1c |
| user-notify (macOS) | `backend/user_notify.rs` | none; builder has title/body/user_info/thread/category (`:170-183`) | see 1e |
| win_toast (Windows) | `src/platform/win_toast.rs` | none in XML; identity via `toast_identity.rs` | see 1d |
| Web Notification API | `backend/web.rs` | `"/icons/icon-192.png"` (`:18`, applied `:100`) | Lucide glyph (see section 3) |

Send-sites (none pass per-event icons; `NotificationRequest`
`src/services/notifications/service/types.rs:93-104` has no icon field):

- Settings test buttons: `src/features/pages/settings/notifications_section.rs:129,169,184,201`
- Updater background notification: `src/services/updater/mod.rs:210`

### 1b. Linux native (notify-rust 4.18.0, locked)

Current behavior. `send()` sets `.appname(APP_DISPLAY_NAME)`,
`.summary`, `.body`, `Hint::DesktopEntry(DESKTOP_ENTRY_ID)` ("com.gpui-starter.app"),
urgency, sound (`notify_rust.rs:55-82`). No `.icon()` call anywhere in
the file. What that yields per install type:

- deb/rpm/flatpak-native installs: the spec's `desktop-entry` hint
  exists precisely so "the daemon [can] retrieve the correct icon for the
  application" ([notification spec, hints
  table](https://specifications.freedesktop.org/notification-spec/notification-spec-latest.html#hints)),
  and those formats install `hicolor/*/apps/gpui-starter.png` plus
  `Icon=gpui-starter` (`resources/com.gpui-starter.app.desktop:6`). The
  daemon resolves the branded icon. Already correct.
- Portable tarball run without `install-linux.sh`, un-integrated
  AppImage: the daemon's themed lookup runs in host icon paths, finds no
  `gpui-starter`, `app_icon` is empty, and the banner falls back to a
  generic/dimmed app icon. This is the gap.
- AppImage: run 5 stages the hicolor tree inside the image under
  `usr/share/icons/hicolor`, visible at `<mount>/usr/share/...` while
  running. Not in the host's search path unless desktop integration
  (appimaged/AppImageLauncher) installed it. Same gap as the tarball.

How icon-by-name vs icon-by-path works in notify-rust 4.x (verified in
the locked 4.18.0 source and on
[docs.rs/notify-rust](https://docs.rs/notify-rust/latest/notify_rust/struct.Notification.html)):
`.icon(&str)` writes the spec's `app_icon` field, which the spec defines
as "either an URI (file:// is the only URI schema supported right now) or
a name in a freedesktop.org-compliant icon theme" ([spec, `icon` argument
table](https://specifications.freedesktop.org/notification-spec/notification-spec-latest.html#icons-and-images-formats));
notify-rust's docs add that common theme names and absolute file paths
are accepted. Separate and not wanted here: `.image_path()` /
`Hint::ImagePath` set the body's content image ("image-path" hint), and
`Hint::ImageData` raw ARGB is the legacy pre-1.1 content-image path;
both are for per-notification content, not the app icon. notify-rust also
offers `.auto_icon()` (sets `app_icon` to the binary basename, i.e.
"gpui-starter"), which covers the installed case but not the portable
one, and is documented as a no-op for the icon on macOS.

Gap. One portable case shows a generic icon: exe-adjacent PNGs exist in
every shipped layout but are never referenced.

Decision for area 2 (linux-notifications). In
`notify_rust.rs::send`, after the `DesktopEntry` hint, set
`notification.icon(icon_value)` where `icon_value` comes from a
`resolve_icon()` helper, cfg-gated `target_os = "linux"` like the
existing hint:

1. Probe exe-relative bundled PNGs, first hit wins, sizes in order
   48, 64, 128, 256, 512 (daemons scale; 48 is the typical banner size):
   `<exe_dir>/../share/icons/hicolor/<s>x<s>/apps/gpui-starter.png`
   (deb/rpm/AppImage-mount layouts), then
   `<exe_dir>/hicolor/<s>x<s>/apps/gpui-starter.png` (tarball layout —
   ships once the section-6 staging fix lands), then
   `<exe_dir>/resources/gpui-starter.png` (today's single-512 tarball).
   On hit: pass the absolute path (spec allows path or `file://` URI).
2. No bundled PNG found (e.g. `cargo run` from a checkout): fall back to
   the theme name `"gpui-starter"` — a no-op when unresolvable, correct
   when the user installed icons.

Factor the probe so it takes the exe dir as a parameter (unit-testable
without faking `current_exe`); tests assert candidate ordering and the
fallback. Do NOT: add the `image-path` hint or `Hint::ImageData`
(content imagery, out of scope), add an icon-theme-probing dependency,
touch urgency/sound logic, or set icons on the macOS/Windows notify-rust
fallback path (macOS ignores manual icons per notify-rust docs; Windows
toasts derive identity from the AUMID shortcut).

### 1c. Flatpak portal path (ashpd 0.13.11)

The portal spec supports icons: `AddNotification`'s vardict has an
`icon (v)` key — a serialized GIcon; `themed (as)` is the array-of-names
form, a plain single-name string is accepted for historical reasons, and
`bytes (ay)` is deprecated since interface version 2
([org.freedesktop.portal.Notification XML](https://github.com/flatpak/xdg-desktop-portal/blob/main/data/org.freedesktop.portal.Notification.xml),
[icon validation](https://flatpak.github.io/xdg-desktop-portal/docs/icons.html)).
ashpd 0.13.11 exposes it: `Notification::icon(impl Into<Option<Icon>>)`
(`ashpd-0.13.11/src/desktop/notification.rs:333`) with
`Icon::Names(Vec<String>)` / `Icon::with_names`
(`ashpd-0.13.11/src/desktop/icon.rs:16-33`).
[docs.rs/ashpd::desktop::Icon](https://docs.rs/ashpd/latest/ashpd/desktop/enum.Icon.html)
also lists `Uri`, `Bytes` (deprecated at the spec level), `FileDescriptor`.
Current code (`portal.rs:56-59`) passes no icon; it does pass
`DESKTOP_ENTRY_ID` as the portal's app id.

Resolution reality: the flatpak manifest installs the hicolor set into
`/app/share/icons/hicolor` (`packaging/flatpak/com.gpui-starter.app.yml`
build-commands), and Flatpak exports `/app/share/icons` to
`~/.local/share/flatpak/exports/share` or
`/var/lib/flatpak/exports/share`, which distributions append to the
host's `XDG_DATA_DIRS` ([Flatpak
conventions](https://docs.flatpak.org/en/latest/conventions.html)); the
host-side portal can therefore resolve the themed name. In practice the
portal's app id plus the exported desktop file already brand most
backends.

Decision for area 2. Add `.icon(Icon::with_names(["gpui-starter"]))`
(the themed name; never `Bytes` — deprecated) to the portal
notification in `portal.rs`. Effort S; belt-and-braces on top of the
existing id-based resolution. A unit test can assert the serialized
notification carries the name (the type implements `Serialize`).

### 1d. Windows

A toast's default app logo is pulled from the app's identity (the
shortcut carrying the AUMID); an `<image placement="appLogoOverride">`
would replace it in the content area, but on Windows 11 the attribution
area shows the app logo and "is not overridden by the appLogoOverride
image placement" ([app notification
content](https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/app-notifications-content)).
The identity chain is already complete: `toast_identity.rs` writes the
Start-Menu shortcut with `SetIconLocation(exe_path, 0)` (`:135`) and
`SetCurrentProcessExplicitAppUserModelID` (`:46`), called from
`user_notify.rs:41`; run 5 embedded the branded `.ico` in the exe, and
Inno's `SetupIconFile` brands the installer-created shortcut. Installed
and portable both resolve the branded icon — the self-registered
portable shortcut points at the exe itself, which now carries the icon.

Gap. None. Decision: no code. Document (this file + releasing.md
cross-link in area 6) that the Windows toast icon closes itself through
the exe-embedded ico. Do NOT add `appLogoOverride` XML — Windows 11
ignores it for the attribution logo anyway, and a content-area logo per
toast is per-notification theming (rejected in 1f).

### 1e. macOS

`user_notify.rs` sends through `UNUserNotificationCenter`
(`user_notify::NotificationBuilder`, `:170-183`); macOS takes the
notification icon from the app bundle automatically — there is no
per-notification icon API to call, and notify-rust's docs confirm manual
icons are unsupported on macOS. Run 5 put the branded `.icns` in the
bundle. Nothing to do; verified no icon-related code exists to change.

### 1f. Per-notification thematic icons

`NotificationRequest` has no icon field and no send-site wants one.
Platform norm for application notifications is one app icon; per-event
imagery (update available, download done) would ride the spec's
`image-path` hint / web `image` option / a toast image — three more
per-backend code paths for a starter. Recommendation: do not add it.
Revisit only if a real feature (e.g. showing an update's screenshot)
asks for content imagery.

## 2. Tray

Current behavior. `src/platform/desktop_shell/tray/mod.rs:19-77` draws a
magnifying-glass glyph in code (comment: "to avoid an asset") as black +
alpha RGBA; `macos.rs:11-13` builds it at 36px with
`with_icon_as_template(true)`; `windows.rs:16-17,163-176` builds it at
16px with a full menu. Linux has no tray at all: `tray/mod.rs:1-9`
cfg-gates the module to macOS/Windows, `src/web.rs:43-44` calls
`crate::tray::setup` only under `cfg(any(macos, windows))`, and the
Linux dependency table (`Cargo.toml:250-266`) has no tray crate.

Gap. The tray shows a generic glyph, not the brand, on both platforms
that have a tray.

Decisions for area 3 (tray-icon).

- macOS: keep template monochrome. Apple's guidance for menu bar extras
  is a simple icon that adapts to the menu bar, and the template-image
  mechanism (black + clear pixels, `isTemplate`) is how macOS recolors
  it for light/dark/highlight states
  ([HIG: the menu bar](https://developer.apple.com/design/human-interface-guidelines/the-menu-bar),
  [Bjango: designing menu bar extras](https://bjango.com/articles/designingmenubarextras)).
  A full-color branded icon would be wrong there. The branded mark is a
  "G" (arc + crossbar on a rounded square, `assets/app-icon/gpui-starter.svg`);
  as a template only alpha survives, so the square would read as a blob.
  Change the glyph to the brand G silhouette instead of the magnifier:
  same in-code rasterization (the arc is a radius/angle-range test on
  the existing circle code plus one filled rect for the crossbar),
  `with_icon_as_template(true)` stays. Keeps the no-asset property and
  the 36px sizing. Do NOT ship the color SVG/PNG to the macOS tray.
- Windows: switch to the branded PNG. The tray convention on Windows is
  a full-color 16px icon. Embed the hicolor 16 (and 24/32 while there,
  ~1-2 KB each) via a small `rust-embed` struct over
  `assets/app-icon/hicolor` cfg-gated to windows (native
  `CombinedAssets` embeds only `themes/` today, `src/app/assets.rs:11-14`),
  decode with the `image` crate (already in the lock at 0.25.10
  transitively; add as a direct windows-target dep for the decode call)
  and feed `tray_icon::Icon::from_rgba`. Keep `ICON_SIZE = 16` and the
  existing DPI comment; per-DPI swapping stays future work.
- Linux: document-gap, do not implement in this run. Feasibility facts:
  the tray-icon crate pinned for macOS/Windows (0.24.1,
  `default-features = false`) supports Linux only through its `gtk`
  feature — libappindicator, which requires a GTK event loop on the
  creating thread (verified in `tray-icon-0.24.1/Cargo.toml` features
  and `src/platform_impl/gtk/`; [docs.rs
  tray-icon](https://docs.rs/tray-icon) states the GTK-loop requirement).
  gpui's Linux backend is raw xcb/Wayland and runs no GTK loop, so 0.24
  does not fit. Newer tray-icon adds a `ksni` feature (first landing in
  0.25.0; pure D-Bus StatusNotifierItem, no GTK; verified in the crates.io
  sparse index `features2`: `ksni = ["dep:ksni", "muda-snapshot"]`), and
  the standalone [ksni crate](https://github.com/iovxw/ksni) (0.3.6, zbus,
  own service thread) works without any GUI loop — Tauri 3 made ksni its
  default tray backend for the same reason
  ([tauri#11293](https://github.com/tauri-apps/tauri/issues/11293),
  [ksni docs](https://docs.rs/ksni/latest/ksni)). So implementing is
  feasible (bump tray-icon for all three OSes, or add ksni directly and
  keep mac/win on 0.24). The honest call is still punt: GNOME, the
  default desktop on Fedora/Ubuntu, ships no StatusNotifier host — the
  icon silently no-ops without the AppIndicator extension — and the
  menu/event plumbing is a full third backend (L). This file is the
  documentation of the gap; releasing.md gets the cross-link in area 6.

## 3. Web favicon

Current behavior. First paint uses the static
`<link rel="icon" href="./icons/icon.svg">` (`wasm/index.html:24`),
the PWA manifest (`wasm/manifest.webmanifest` icons), and the service
worker precache (`wasm/sw.js:30-33`). Every file in `wasm/icons/` is a
Lucide terminal glyph under an ISC `LICENSE-LUCIDE` — unbranded. After
boot, `src/platform/web/document_meta.rs:91-145` replaces the favicon
with a canvas drawing: a `#2563eb` blue square plus connectivity dot and
unread badge, applied via a `data-app-favicon`-marked link (`:148-188`).
So the tab shows the Lucide glyph, then switches to a blue square: two
off-brand states. The web Notification icon
(`/icons/icon-192.png`, `backend/web.rs:18`) is the same Lucide file.

Gap. No branded icon reaches the browser tab, PWA install, or web
notification.

Decisions for area 4 (web-favicon).

- Replace the files, not the references: keep the five names
  (`icon.svg`, `icon-192.png`, `icon-512.png`, `icon-512-maskable.png`,
  `apple-touch-icon.png`) so `index.html`, the manifest, and `sw.js`
  need no edits and the SW cache list stays valid. Content becomes
  renders of the branded SVG: `icon.svg` a copy of
  `assets/app-icon/gpui-starter.svg`; 192/512 straight renders;
  the maskable variant padded into the safe zone; apple-touch at 180.
  Repo precedent (run 5) accepts offline stand-in generation when the
  real toolchain is absent. Delete `LICENSE-LUCIDE` with the last
  Lucide file.
- Draw the branded base in `document_meta.rs`: the wasm build already
  embeds all of `assets/` (`src/app/assets.rs:17-21`, `WasmAssets` over
  `folder = "assets"`), so
  `app-icon/hicolor/64x64/apps/gpui-starter.png` (2.3 KB) is already in
  the binary — zero new bytes. Build a blob URL from the embedded bytes
  (`Blob` + `Url` web-sys features are already enabled for the OPFS
  lane, `Cargo.toml:382-407`), load it into an `HtmlImageElement` once
  at `install()`, cache it, and `draw_image` it as the canvas base with
  the existing dot/badge overlays on top. Until the async image is
  ready, leave the static branded link untouched (skip the canvas
  rewrite rather than draw the old blue square). The `LAST` no-op guard
  (`document_meta.rs:24,46-52`) carries over.
- Title: static `<title>gpui-starter (wasm)</title>` (`index.html:23`)
  vs the post-boot compose (`"<app> — <route> [(unread)]"`,
  `document_meta.rs:62-63`). Optional one-line alignment of the static
  title to `gpui-starter`; the boot-time swap is inherent to a
  JS-rendered app. Do NOT touch `web/` (Astro marketing site — user's
  untracked work), add an SVG rasterizer dependency, or change the
  notification icon URL (replacing `icon-192.png` brands it for free;
  its leading `/` assumes the harness is served at the domain root,
  which `wasm/serve.sh` does — an accepted limitation, and
  `backend/web.rs` belongs to area 2's fence anyway).

## 4. Window icon (Linux)

Current behavior. `src/app/window.rs:59-80` builds `WindowOptions` with
`app_id: Some("gpui-starter")` and no icon. The pinned gpui DOES expose
the API: `WindowOptions.icon: Option<Arc<image::RgbaImage>>`,
doc-commented "Icon image (X11 only)"
(`gpui-pre-0.3.8/src/platform.rs:2501-2503`), consumed at window
creation into an EWMH `_NET_WM_ICON` property
(`gpui-pre-linux-0.3.8/src/linux/x11/window.rs:805-824`, spec:
[wm-spec _NET_WM_ICON](https://specifications.freedesktop.org/wm-spec/1.4/ar01s05.html)).
Wayland has no icon path in gpui-pre 0.3.8 (the wayland window code has
no icon references; `xdg_toplevel_icon_v1` —
[protocol](https://wayland.app/protocols/xdg-toplevel-icon-v1) — is
unsettable through it).

Adoption as of 2026: KWin ships `xdg_toplevel_icon_manager_v1` since
Plasma 6.3 ([Electron tracking
issue](https://github.com/electron/electron/issues/45638)); GNOME/mutter
has not merged support and maintains an open issue, preferring icon
resolution through `.desktop` files
([mutter#4100](https://gitlab.gnome.org/GNOME/mutter/-/issues/4100)).
GNOME's position matters less than it sounds: installed layouts already
brand the window through `app_id` + `Icon=gpui-starter`, and GNOME
removed the X11 session, so the gpui X11 path and GNOME barely intersect.

Gap. X11 sessions (and XWayland taskbars) of portable/tarball runs show
a generic window icon; installed runs resolve via the desktop entry.

Decision. Implement — but note the fence: no run-6 area owns
`src/app/window.rs` (area 3's fence is `desktop_shell/**`). Cheapest
resolution is a one-line fence extension for area 3 in the routing map;
failing that this records the recipe for the next run. Recipe: in
`window.rs`, set `icon: Some(Arc::new(decoded))` under
`cfg(target_os = "linux")`, decoding the embedded 48px hicolor PNG via
the same `rust-embed`-over-`assets/app-icon/hicolor` struct +
`image::load_from_memory` prescribed for the Windows tray in section 2
(share it, e.g. in `src/app/assets.rs`, so one embed serves both).
Raw xlib through the app itself to reach anything gpui does not expose:
over-engineering, rejected. Watching item for Wayland: mutter#4100
above; nothing to do until gpui-pre exposes a toplevel-icon API.

## 5. In-app surfaces (quick pass)

`src/features/pages/about.rs:20-47` renders a text-only About page
(heading + version). No icon placeholder exists anywhere in
`src/features/pages/` (grep for icon usage in settings/mod.rs and
pages/* found none). Nothing to fix or replace; adding an in-app brand
logo is a design choice, not an icon-surface bug. Recommendation:
skip. If ever wanted, it is a gpui-kit design-guides decision, not a
packaging/branding fix.

## 6. Finishing-sweep input (releasing.md open decisions)

Mechanical items for area 5, each re-verified against the tree today:

- Item 19 (justfile @v2 comment): still accurate — `justfile:69` says
  `softprops/action-gh-release@v2`, `release.yml:592` pins `@v3`. Exact
  fix: in `justfile:69`, `@v2` to `@v3` (one word).
- Item 15 (tarball hicolor staging): still accurate —
  `release.yml:304` stages only `resources/gpui-starter.png`, while
  `install-linux.sh:39` already probes `$SCRIPT_DIR/hicolor` first.
  Exact fix: replace line 304 with
  `cp -r assets/app-icon/hicolor "$STAGE/hicolor" 2>/dev/null || true`
  (drop the single-PNG cp; the hicolor tree supersedes it and the
  script's legacy fallback then simply has no png to find, same as
  today's failure mode under `|| true`).
- Item 16 (deb LICENSE duplication): still accurate — `Cargo.toml:106`
  assets `LICENSE` to `usr/share/doc/gpui-starter/` while cargo-deb
  auto-generates `usr/share/doc/gpui-starter/copyright` from
  `license-file`. Exact fix: delete the
  `["LICENSE", "usr/share/doc/gpui-starter/", "644"],` asset line.
- Item 10 (flatpak skip-list residuals): still accurate — `bun.lock`,
  `package.json`, `.zcodeignore` are untracked root files (git status)
  riding the dir source. Exact fix: add `bun.lock`, `package.json`,
  `.zcodeignore` to the `skip:` list in
  `packaging/flatpak/com.gpui-starter.app.yml`. (`.gitignore` is
  tracked; `com.gpui-starter.app.instance` is the runtime single-instance
  lock, gitignored at the repo root rather than tracked — neither is a
  residual.)

Items that stay genuinely-user (not area 5): Flathub app id (3),
metainfo screenshots tied to that id (4), keyring-in-flatpak (5), snap
revival (6), winget entry (7), AppImage update channel (8), glibc floor
(9), CHANGELOG/tag reconciliation (11), code signing (12), fork-vs-origin
publisher URLs (13), flatpak-cargo-generator pin (14), and the Windows
updater swap in `apply.rs` (1). Plus gen-icons.sh real-tool validation
(2) — mechanical to run but needs tools the box lacks.

## Summary

| Surface | Owning area | Action | Effort |
|---|---|---|---|
| Linux notify-rust icon (portable/AppImage) | 2 | implement: exe-relative PNG probe, name fallback | S-M |
| Flatpak portal notification icon | 2 | implement: `Icon::with_names` one-liner + test | S |
| Windows toast icon | — | none: exe-embedded ico + AUMID shortcut already brand it | S (doc only) |
| macOS notification icon | — | none: bundle icon automatic | — |
| Per-notification thematic icons | — | rejected: over-engineering for a starter | — |
| Tray macOS glyph | 3 | implement: template brand-G silhouette, keep template mode | S |
| Tray Windows glyph | 3 | implement: branded 16px embedded PNG | S-M |
| Tray Linux | 3 | document-gap: ksni feasible, GNOME no host by default | L if ever |
| Web static favicon/manifest/PWA icons | 4 | implement: replace `wasm/icons/*` with branded renders | S |
| Web canvas favicon + web notification icon | 4 | implement: embedded-PNG base under badges; file swap covers notifications | M |
| Window icon X11 | 3 (fence extension needed) | implement: `WindowOptions.icon` from embedded PNG; Wayland punt (mutter#4100) | S |
| In-app branding | — | skip: no placeholder exists | — |
| Finishing-sweep mechanicals | 5 | implement: four one-line fixes above | S |

## Sources

External claims were fetched, not quoted from memory:

- notify-rust 4.x icon API:
  https://docs.rs/notify-rust/latest/notify_rust/struct.Notification.html
  (and locked 4.18.0 source, local registry).
- Notification spec (app_icon, image-path, desktop-entry):
  https://specifications.freedesktop.org/notification-spec/notification-spec-latest.html
- Portal notification vardict + deprecated bytes icon:
  https://github.com/flatpak/xdg-desktop-portal/blob/main/data/org.freedesktop.portal.Notification.xml
- Portal icon validation (square, 512px, SVG 4096B):
  https://flatpak.github.io/xdg-desktop-portal/docs/icons.html
- ashpd Notification/Icon (0.13.x):
  https://docs.rs/ashpd/latest/ashpd/desktop/notification/struct.Notification.html
  and https://docs.rs/ashpd/latest/ashpd/desktop/enum.Icon.html (and
  locked 0.13.11 source, local registry).
- Flatpak /app/share exports and XDG_DATA_DIRS:
  https://docs.flatpak.org/en/latest/conventions.html
- Windows toast app logo / appLogoOverride on Windows 11:
  https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/app-notifications-content
- Apple menu-bar-extras guidance:
  https://developer.apple.com/design/human-interface-guidelines/the-menu-bar
  and https://bjango.com/articles/designingmenubarextras
- tray-icon GTK-loop requirement and ksni backend:
  https://docs.rs/tray-icon and https://github.com/tauri-apps/tray-icon
  (0.24.1 / 0.26.1 manifests, local registry + crates.io sparse index).
- ksni (pure D-Bus SNI): https://github.com/iovxw/ksni,
  https://docs.rs/ksni/latest/ksni; Tauri context
  https://github.com/tauri-apps/tauri/issues/11293
- Wayland toplevel icon: protocol
  https://wayland.app/protocols/xdg-toplevel-icon-v1; adoption
  https://github.com/electron/electron/issues/45638 (KDE 6.3) and
  https://gitlab.gnome.org/GNOME/mutter/-/issues/4100 (GNOME open).
- EWMH _NET_WM_ICON:
  https://specifications.freedesktop.org/wm-spec/1.4/ar01s05.html

Code-level claims come from the named files in this repository, or from
the pinned crates' sources in the local cargo registry
(`gpui-pre-0.3.8`, `gpui-pre-linux-0.3.8`, `ashpd-0.13.11`,
`notify-rust-4.18.0`, `tray-icon-0.24.1`).
