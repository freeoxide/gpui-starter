# Desktop widgets survey: GPUI-native floating panels vs OS widget surfaces

Question: can gpui-starter ship desktop widgets, and should they be GPUI-rendered
floating panels or live inside the OS widget surfaces (macOS Notification Center
and desktop widgets, the Windows 11 widget board, KDE plasmoids)? Facts were
verified this session against the pinned crate sources (gpui-pre 0.3.9 = a
snapshot of zed@93d7456; line numbers match upstream raw files at that commit)
and fetched primary platform docs. Evidence: `.z-flashflow/evidence/
{gpui-windowing,macos,windows,linux}.md`. Doc-only survey: no cargo gate
applies. Unverified items are flagged inline, not guessed.

## Answers

- Yes. GPUI renders the widget UI natively on all three desktops with the API
  already in `Cargo.lock`. `WindowKind::PopUp` is the widget kind on macOS
  (non-activating NSPanel at popup level 101, all spaces), Windows (borderless
  topmost tool window, no taskbar or Alt-Tab), and X11 (override-redirect
  notification window); `WindowKind::LayerShell` is a first-class Wayland
  dock/widget surface. No fork, no native-UI detour.
- On macOS, Windows, and X11 the gaps are click-through and runtime
  always-on-top toggles; neither is public API there, and each closes with a
  small unsafe shim over the raw-window-handle 0.6 handles every gpui window
  exports (NSView pointer, HWND, X window plus xcb connection): a few dozen
  lines at minimum, 100-200 per module, a few hundred for X11, using objc2 /
  windows-rs / x11rb, all already in the graph. Wayland differs on both
  counts: click-through is public API (`Window::set_input_region`,
  Wayland-only), and runtime topmost is compositor-owned, solved by
  `LayerShell` at creation rather than a toggle.
- Widgets inside the OS surfaces are rendered by the OS, not the app. A macOS
  widget is SwiftUI, archived and drawn by a system process; a Windows 11
  widget board tile is Adaptive Cards JSON rendered by the host; a KDE plasmoid
  is QML in plasmashell. GPUI pixels cannot appear in any of them. The app's
  role is a data bridge: App Group files plus a reload trigger on macOS, a COM
  provider shipping card JSON on Windows, DBus or file IPC into QML on KDE.
- Effort splits accordingly. GPUI-native panels are days: plans 02/03/04/09
  already build most of the path. OS-surface integrations are per-platform side
  projects: macOS is ~150-300 lines of Swift plus a week-plus of bundle/signing
  plumbing; Windows is weeks (own WinRT projection, COM server, MSIX lane);
  KDE is days per plasmoid but a second UI codebase in QML; GNOME is a
  non-path.
- Verdict: build GPUI-native panels first; plans 02 and 03 absorb the shim
  scope. Treat OS-surface integrations as optional data-bridge extensions, one
  platform at a time, only when placement in the OS widget gallery is a hard
  requirement.

## Path key

| Short | Path |
| --- | --- |
| `GPUI` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-0.3.9/src` |
| `MACOS` | gpui-pre-macos-0.3.9 sources (crates.io tarball, extracted for this survey) |
| `WINDOWS` | gpui-pre-windows-0.3.9 sources (same provenance as `MACOS`) |
| `LINUX` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-linux-0.3.9/src/linux` |
| `REPO` | `/home/thragg/fo/gpui-starter` |

Upstream zed citations use snapshot commit 93d7456 (raw.githubusercontent.com);
the macOS tarball was spot-checked against the upstream file at that commit and
the cited lines match exactly. Doc targets:
developer.apple.com (WidgetKit, AppKit), learn.microsoft.com (widget providers,
Win32 styles), wayland.app (layer-shell), specifications.freedesktop.org
(EWMH), develop.kde.org (plasmoids).

## Part 1: macOS

### GPUI-native floating panel (recommended path)

`WindowKind::PopUp` maps to the canonical floating-widget recipe in one kind
(`MACOS/window.rs:1020-1024, 1206-1247`): an NSPanel with
`NSWindowStyleMaskNonactivatingPanel` (private bit `1 << 7`), level
`NSPopUpWindowLevel` (101), utility-window animation, and collection behavior
`CanJoinAllSpaces | FullScreenAuxiliary`: never activates the app, above
normal windows, follows the active space. `WindowKind::Floating` is an
activating NSPanel at floating level 3. `WindowBackgroundAppearance::
{Transparent, Blurred}` (NSVisualEffectView) is runtime-settable via
`Window::set_background_appearance` (`GPUI/window.rs:3059`).

Not exposed: `setIgnoresMouseEvents:` (click-through), runtime `setLevel:`,
arbitrary levels, post-creation collection-behavior changes. Each is one
objc2 call on the NSWindow: `Window` implements `HasWindowHandle`
(raw-window-handle 0.6), the backend hands out the NSView pointer
(`AppKitWindowHandle`, `MACOS/window.rs:2421-2431`), and `[ns_view window]`
reaches the NSPanel. gpui-pre-macos already depends on objc2 and
objc2-app-kit; the repo has on-disk precedent for the reach-in
(`REPO/src/platform/liquid_glass.rs:230-240`), and plan 03 specs the same
pattern in its `macos.rs`.

Dock-icon visibility is an activation-policy decision (`Regular | Accessory`),
not a per-window flag. Menu-bar extras are NSStatusItem, covered by the
existing tray support. A below-desktop level lives in the CGWindowLevel
constants; the `kCGDesktopWindowLevel` citation was not verified, so
desktop-below placement stays out of scope.

Effort: days. The reach-in pattern is proven on disk; plan 03 (unimplemented)
specs the full panel, and only the click-through shim is new.

### OS surface: WidgetKit (Notification Center / desktop widget)

A macOS widget is not a drawable surface. The app ships a widget extension
(`.appex` under `Contents/PlugIns/`) whose UI is SwiftUI; the extension
declares views plus a `TimelineProvider`, and the system renders archived
representations in its own process. WWDC23: "Your view code only runs during
archiving." GPUI cannot be the renderer there, by design. Interactive widgets
(macOS 14 Sonoma) stay declarative: only Button and Toggle driven by
AppIntents. Desktop placement arrived with Sonoma; WidgetKit is macOS 11+.

Data bridge: the entitlement `com.apple.security.application-groups` shares a
group container; the Rust app writes JSON into
`~/Library/Group Containers/<group.id>/` and the Swift `TimelineProvider`
reads it in `getTimeline`. Reload: `WidgetCenter.reloadTimelines(ofKind:)` is
documented as callable from the containing app but exposes a Swift-only
interface (no Objective-C symbol in DocC; objc2 ships no WidgetKit crate). A
Rust host either links a ~30-line Swift shim via swift-rs (`@_cdecl` calling
`reloadAllTimelines()`) or accepts the system-scheduled reload policies.
Apple publishes no refresh-budget figure; commonly cited numbers are
unverified and omitted here.

Distribution: Developer ID signing plus notarization covers distribution
outside the Mac App Store; OnlySwitch is a fetched existence proof (Homebrew
or DMG, desktop and Notification Center widget on Sonoma+, no App Store).

Effort: days for the Swift surface (one `WidgetBundle`, one `Widget`, one
`TimelineProvider`, 1-2 views, roughly 150-300 LOC), a week-plus for build
plumbing: Xcode project for the .appex, embedding, signing, notarization, and
a macOS bundle pipeline the repo does not have (packaging today is deb/rpm;
plan 20 owns bundling). Ongoing tax: widget UI changes live in Swift, not
GPUI.

## Part 2: Windows

### GPUI-native floating panel (recommended path)

`WindowKind::PopUp` is the widget kind: `WS_EX_TOOLWINDOW | WS_EX_TOPMOST`
with borderless style 0 (`WINDOWS/window.rs:492-515`). A tool window does not
appear in the taskbar or the Alt-Tab dialog (learn.microsoft.com,
extended-window-styles), and topmost puts it above non-topmost windows.
`WindowKind::Floating` falls into the Normal branch on Windows: not topmost,
not a tool window. A real per-platform gap, so widget code must use PopUp
here. Transparency and backdrops are first-class:
`WindowBackgroundAppearance::{Transparent, Blurred (acrylic),
MicaBackdrop, MicaAltBackdrop}`, runtime-settable, composed through Direct3D
11 with DirectComposition (`WS_EX_NOREDIRECTIONBITMAP`).

Not exposed: `WS_EX_TRANSPARENT` (click-through), `WS_EX_NOACTIVATE`, runtime
topmost re-assert (topmost comes from the creation style). The shim is a few
dozen lines: `Window` implements `HasWindowHandle`, the backend builds a
`Win32WindowHandle` with the HWND (`WINDOWS/window.rs:595-607`), and
`SetWindowLongPtrW(GWL_EXSTYLE)` plus `SetWindowPos(HWND_TOPMOST)` via the
`windows` crate (already a dependency) do the rest. Caveat: documented
`WS_EX_TRANSPARENT` semantics are about sibling paint order; the common
ignore-mouse recipe is `WS_EX_LAYERED | WS_EX_TRANSPARENT`, and the combined
behavior on the DirectComposition path is unverified at runtime.

Prior art: Desktop Gadgets are dead (discontinued in Windows 8, Security
Advisory 2719662); Rainmeter, the surviving desktop-widget framework, is plain
layered Win32 windows, the same path recommended here. Effort: days. Plans
02/09 already consume PopUp through `FloatLevel::AlwaysOnTop`; only the shim
is new.

### OS surface: Windows 11 widget board

The Windows App SDK 1.4+ model: the app implements `IWidgetProvider`
(`CreateWidget`, `DeleteWidget`, `OnActionInvoked`, `Activate`,
`Deactivate`) as an out-of-proc COM local server (`CoRegisterClassObject`,
`CLSCTX_LOCAL_SERVER`); the OS starts and kills the process. Content is
Adaptive Cards JSON (template plus data, pushed via
`WidgetManager::GetDefault().UpdateWidget`); the host renders the card and the
app never draws pixels. No API supplies a surface, HWND, or bitmap. The
HTML/web-widget escape hatch is EEA-only. Interaction is `Action.Execute`
verbs; the design guidance forbids scrolling and nested pages inside widgets
and points taps at launching the app.

Registration is manifest-only (`com:ComServer`/`com:ExeServer` plus a
`uap3:AppExtension` named `com.microsoft.windows.widgets`), and Microsoft
states that only packaged apps can be registered as widget providers: MSIX,
signed with a certificate trusted on the target machine, Store or sideload.
A sparse package (external location) can grant package identity to an
unpackaged exe, but whether widget-provider registration is accepted inside
one is unverified; the docs show only the fully packaged flow.

Rust support: the `windows` crate has no Widgets feature (feature list for
0.62.2, the version gpui-pre-windows depends on, checked); the official
`windows-app` crate is dead (every version published July 2021, all yanked).
A Rust provider means generating a WinRT projection from Windows App SDK
metadata (windows-bindgen) and writing the COM local server by hand. No
real-world Rust `IWidgetProvider` example was found (GitHub code search needs
auth here); the absence is consistent with the dead crate.

Effort: weeks, and the visible result is not GPUI-rendered. Not planned unless
Widgets Board presence becomes a distribution requirement.

## Part 3: Linux

### GPUI-native floating panel (recommended path)

Wayland is the headline result. `WindowKind::LayerShell(LayerShellOptions)` is
first-class in gpui-pre 0.3.9: the backend binds `zwlr_layer_shell_v1`
versions 1..=5 and maps `Layer::{Background, Bottom, Top, Overlay}`, anchor
bitflags, exclusive zone/edge, margin, and
`KeyboardInteractivity::{None, Exclusive, OnDemand}` onto the protocol
(`LINUX/wayland/window.rs:161-217`; options in `GPUI/src/platform/
layer_shell.rs`). Layer surfaces are not toplevels, so there is no taskbar or
Alt-Tab entry to suppress. Click-through is `Window::set_input_region
(Some(&[]))` (documented Wayland-only; wl_surface.set_input_region semantics
in wayland.xml). Note on sources: one evidence pass (gpui-windowing) scores
Wayland click-through as unavailable because it counts only the raw-surface
route, binding wl_compositor by hand over the raw handle; that reading misses
the public wrapper. The docs.rs `Window` page for 0.3.9 lists
`set_input_region` as a public, Wayland-only method, which settles API
existence in favor of the native reading; what stays unverified is
per-compositor runtime honoring of an empty region, explicitly so on Mutter.
Desktop pinning is `Layer::Background` or `Bottom`. Blur is
the KDE `org_kde_kwin_blur` protocol, so KWin only. Upstream ships a worked
example (`crates/gpui/examples/layer_shell.rs`, a transparent bottom-anchored
clock). Compositor support (wayland.app table): KWin 6.7 (protocol v5) plus
the wlroots family (Hyprland, Sway, niri, COSMIC, river, labwc, Wayfire);
Mutter, Weston, and Cage do not support it. Known upstream issue: zed#46378,
displays unavailable at startup breaks layer-shell output selection (open); a
deferred open is the workaround.

GNOME Wayland has no layer shell (mutter#973, unimplemented). Degrade to a
normal xdg toplevel: no keep-above, skip-taskbar, or desktop pin, those are
compositor policy. Transparency is normal per-pixel alpha on Wayland.
Click-through via `set_input_region` is protocol-level, but whether Mutter
honors empty input regions is unverified. Feature-detect on
`LayerShellNotSupportedError` and fall back to a plan 02 float.

X11 (KDE or GNOME): `WindowKind::PopUp` is override-redirect plus
`_NET_WM_WINDOW_TYPE_NOTIFICATION` (`LINUX/x11/window.rs:514, 616-625`),
which bypasses the WM: drawn above, no taskbar entry, no focus stealing. For
Normal windows the backend sets no `_NET_WM_STATE_ABOVE/BELOW/SKIP_TASKBAR/
STICKY`; the shim adds them. `HasWindowHandle`/`HasDisplayHandle` return
`XcbWindowHandle` plus `XcbDisplayHandle` (window id and live `XCBConnection`
pointer, `LINUX/x11/window.rs:318-360`; as-raw-xcb-connection is already in
the tree), so an x11rb shim can send EWMH client messages and an XShape input
region for click-through without opening a second connection. A few hundred
lines. Transparency needs a running compositor (32-bit ARGB visual).

Effort: layer-shell on KDE Wayland and wlroots compositors is configuration
only; the X11 shim is days; GNOME Wayland is a documented graceful
degradation.

### OS surface: plasmoids and GNOME extensions

KDE plasmoids are QML running in plasmashell's engine; a third-party widget is
a directory (`metadata.json` plus QML) under `~/.local/share/plasma/plasmoids/`.
A C++ `Plasma::Applet` plugin is possible but needs root-installed
distribution, which fits the existing deb/rpm lane. There is no documented way
to embed foreign (GPUI) pixels in a plasmoid; doing it would be unsupported
in-plasmashell-process work with no upstream examples. The realistic bridge:
re-implement the widget UI in QML and feed it from the Rust app over DBus
(Data Engines are slated for deprecation) or a file. Days per widget, but a
second UI codebase in a second language, Plasma-themed rather than
app-themed.

GNOME has no widget system. Shell extensions are GJS/JavaScript, the review
guidelines ban binary executables and libraries, and per-release churn is real
(the GNOME 45 ESM migration broke extensions wholesale). A GPUI widget can
never live in an extension; a data-only GJS widget is the ceiling.

## Comparison

| Platform | Path | Render path | Distribution | Effort | Risks |
| --- | --- | --- | --- | --- | --- |
| macOS | GPUI panel | GPUI over non-activating NSPanel (`PopUp`), objc2 shims | no macOS bundle pipeline yet (plan 20) | days | transparent-panel hover/level behavior needs on-Mac QA (plan 03 risks) |
| macOS | WidgetKit | SwiftUI in `.appex`, drawn by a system process | Developer ID + notarization works (OnlySwitch proof) | days Swift + 1wk+ packaging/signing | GPUI cannot render there; Swift codebase; no macOS bundle pipeline yet |
| Windows | GPUI panel | GPUI over `WS_EX_TOOLWINDOW\|TOPMOST`, D3D11/DComp | portable exe, existing lane | days | click-through on the DComp path unverified at runtime |
| Windows | widget board | Adaptive Cards JSON rendered by the OS host | MSIX only, Store or signed sideload | weeks | own WinRT projection + COM server; result is not GPUI |
| Linux (KDE/wlroots Wayland) | GPUI layer-shell | GPUI on `zwlr_layer_surface_v1` | deb/rpm, existing lane | config only | zed#46378 startup output selection; blur KWin-only |
| Linux (X11) | GPUI panel + shim | GPUI over override-redirect, EWMH + XShape via shim | deb/rpm | days | WM-dependent placement for non-override-redirect windows |
| Linux (GNOME Wayland) | GPUI, degraded | normal xdg toplevel | deb/rpm | zero extra | no topmost/taskbar/pin; Mutter input-region honoring unverified |
| Linux (KDE) | plasmoid | QML in plasmashell | deb/rpm (C++ plugin needs root) | days per widget | second UI codebase; no GPUI pixels |
| Linux (GNOME) | extension | GJS/JS only | EGO review | n/a | binaries banned, per-release churn; non-path |

## Recommendation

Build GPUI-native widget panels. Keep GPUI as the only widget renderer on all
three desktops; do not build widget UI in AppKit, Win32, or Qt.

1. Plan 02 owns the window-management helper and should absorb the shim scope.
   It already owns `FloatLevel`, anchors, spawn/close/respawn, the registry,
   and capability rows; the kit platform layer adds nothing here. Add a
   cfg-gated `shims` module inside `src/ui/floating/`: `macos.rs`
   (`setIgnoresMouseEvents:`, `setLevel:`, `setCollectionBehavior:` over the
   NSView from `HasWindowHandle`; precedent is `liquid_glass.rs` and plan 03's
   `macos.rs`), `windows.rs` (`SetWindowLongPtrW` for
   `WS_EX_TRANSPARENT | WS_EX_NOACTIVATE`, `SetWindowPos(HWND_TOPMOST)`), and
   `x11.rs` (x11rb EWMH client messages plus an XShape input region). A few
   dozen lines at minimum, 100-200 per module, a few hundred for `x11.rs`; no
   new dependency family.
2. Adopt `WindowKind::LayerShell` as the desktop-widget kind on Linux: extend
   plan 02's spec with a layer-shell level/anchor option for Wayland sessions,
   feature-detect with `LayerShellNotSupportedError`, and degrade to the
   existing float or a normal window on GNOME Wayland. X11 keeps PopUp plus
   the shim. Watch zed#46378 for output selection at startup.
3. Plans 03/04/09 need no changes for this verdict. 03 specs the macOS panel
   reach-in (the on-disk proof the pattern works is `liquid_glass.rs`), 04
   anchors its popover off the existing tray, 09 consumes
   `FloatLevel::AlwaysOnTop`. The one runtime unknown worth
   closing early is Windows click-through on the DirectComposition path: a
   short manual test once the shim exists.
4. OS-surface integrations are optional data-bridge extensions, one platform
   at a time, none planned now: macOS WidgetKit (Swift extension plus App
   Group bridge, `WidgetCenter` reload through a ~30-line swift-rs shim,
   blocked on a macOS bundle/signing pipeline, plan 20 territory); Windows
   widget board only if board presence is a distribution requirement (weeks of
   WinRT projection, COM server, MSIX, for a result GPUI does not render;
   keep explicitly not-planned); KDE plasmoid only if deep Plasma-panel
   integration is wanted (QML fed by DBus or file).
   The bridge data already exists: plan 17's `SystemControlsSnapshot` global
   and plan 04's chart feeds are the snapshots a bridge would serialize. The
   bridge reads those; it does not re-derive them.
5. Beyond the existing plans this survey adds: the shim scope (click-through,
   runtime topmost, X11 EWMH) sized and source-verified, the layer-shell
   option on Linux, the Windows Floating-is-not-topmost gap, and the
   OS-surface effort numbers that justify the ordering.

## Sources

All fetched 2026-10-12 (consolidated from the four evidence files; a 12-URL
subset re-checked via curl this session, all HTTP 200).

gpui crate and upstream zed:

- https://crates.io/api/v1/crates/gpui-pre (snapshot provenance: zed@93d7456)
- https://docs.rs/gpui-pre/0.3.9/gpui/enum.WindowKind.html
- https://docs.rs/gpui-pre/0.3.9/gpui/struct.Window.html
- https://raw.githubusercontent.com/zed-industries/zed/93d7456/crates/gpui_macos/src/window.rs
- https://raw.githubusercontent.com/zed-industries/zed/93d7456/crates/gpui_windows/src/window.rs
- https://raw.githubusercontent.com/zed-industries/zed/93d7456/crates/gpui_linux/src/linux/wayland/window.rs
- https://raw.githubusercontent.com/zed-industries/zed/93d7456/crates/gpui_linux/src/linux/x11/window.rs
- https://raw.githubusercontent.com/zed-industries/zed/93d7456/crates/gpui/src/platform/layer_shell.rs
- https://github.com/zed-industries/zed/issues/46378 (open: layer-shell output selection)

macOS:

- https://developer.apple.com/documentation/widgetkit
- https://developer.apple.com/documentation/widgetkit/widgetcenter
- https://developer.apple.com/documentation/widgetkit/timelineprovider
- https://developer.apple.com/documentation/widgetkit/creating-a-widget-extension
- https://developer.apple.com/videos/play/wwdc2023/10028/ (system-process rendering)
- https://developer.apple.com/videos/play/wwdc2023/10027/ (desktop widgets, Sonoma)
- https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.application-groups
- https://developer.apple.com/documentation/appkit/nswindow/ignoresmouseevents
- https://developer.apple.com/documentation/appkit/nswindow/collectionbehavior-swift.struct
- https://developer.apple.com/developer-id/
- https://github.com/Brendonovich/swift-rs
- https://github.com/jacklandrin/OnlySwitch (Developer-ID widget existence proof)

Windows:

- https://learn.microsoft.com/en-us/windows/apps/develop/widgets/widget-providers
- https://learn.microsoft.com/en-us/windows/apps/develop/widgets/implement-widget-provider-win32
- https://learn.microsoft.com/en-us/windows/apps/develop/widgets/web-widget-providers (EEA-only)
- https://learn.microsoft.com/en-us/windows/apps/design/widgets/widgets-interaction-design
- https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/grant-identity-to-nonpackaged-apps
- https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles
- https://docs.rs/crate/windows/latest/features (no Widgets feature)
- https://crates.io/api/v1/crates/windows-app (yanked, 2021)
- https://docs.rainmeter.net/manual/skins/
- https://en.wikipedia.org/wiki/Windows_Desktop_Gadgets

Linux:

- https://wayland.app/protocols/wlr-layer-shell-unstable-v1 (protocol + compositor table)
- https://gitlab.freedesktop.org/wayland/wayland/-/raw/main/protocol/wayland.xml (set_input_region)
- http://specifications.freedesktop.org/wm/latest/ar01s05.html (EWMH atoms)
- https://xorg.freedesktop.org/archive/X11R7.6/doc/xextproto/shape.html (XShape input)
- https://develop.kde.org/docs/plasma/widget/
- https://hanyoung.uk/blog/plasmoid-with-cpp
- https://gjs.guide/extensions/review-guidelines/review-guidelines.html
- https://gjs.guide/extensions/upgrading/gnome-shell-45.html
- https://gitlab.gnome.org/GNOME/mutter/-/issues/973 (no layer shell in Mutter)

Flagged as unverified and not relied on: WidgetKit refresh-budget numbers
(Apple publishes no figure); the `kCGDesktopWindowLevel` citation; Windows
click-through on the DirectComposition path; widget registration inside a
sparse MSIX; any real-world Rust `IWidgetProvider` example; Mutter honoring
empty input regions or transparent toplevels. Evidence conflict, resolved:
gpui-windowing's capability table scores Wayland click-through as unavailable
(raw-surface shim only); linux.md and the docs.rs `Window` page show the
public `set_input_region` wrapper, which this survey follows.
