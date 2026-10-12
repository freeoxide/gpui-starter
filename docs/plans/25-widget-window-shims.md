# 25 Widget Window Shims Plan

## Goal

A float spawned through plan 02 can be told to ignore all pointer input and to
toggle always-on-top at runtime, on macOS, Windows, X11, and Wayland, from the
same `FloatingSpec`/`FloatingHandle` API; platforms without a shim report
"not applied" instead of failing. The Windows gap the survey flags is closed
and pinned: `FloatLevel::AlwaysOnTop` floats on Windows are verified to spawn
as the PopUp kind (`WS_EX_TOOLWINDOW | WS_EX_TOPMOST`, never the non-topmost
`WS_EX_APPWINDOW` branch), recorded as an amendment note on 02's contract.
The gallery gains a "Click-Through & Topmost" section that drives both
toggles on a live float, and the diagnostics page gains a
`floating_click_through` capability row.

## Dependencies

Plan 02, landed (batch-1 foundation; none of plans 01-20 is implemented at
planning time, so 25 executes against 02's files after 02 merges). Consumed
from `02:Interface Contract` (02.md:286-407): `FloatingSpec` and its builder
set, `FloatingHandle::{Native, Overlay}` (02.md:330-334),
`FloatLevel::{Floating, AlwaysOnTop}` (02.md:292-300), `FloatingWindowId`,
`spawn_floating`/`close_floating` (02.md:347-353), and
`FloatingWindowRegistry` (02.md:406, reached through the same crate as
`pub(crate)` internals; 25 lives inside `src/ui/floating/`). 25 also touches
02's implementation files (`spec.rs`, `registry.rs`, `spawn.rs`, `probe.rs`,
`mod.rs`) and appends one amendment note to 02's Interface Contract section
(append-only prose, no reflow of the contract block or existing lines; the
discipline INDEX §5 records for 22/23 on plan 21).

Amendment to 02 (this plan, Step 1.1 appends it): `FloatingSpec::
click_through` builder + `is_click_through` reader, the two runtime toggle
functions, `CAPABILITY_FLOATING_CLICK_THROUGH`, a semantic note that
`FloatingWindowEntry.level` becomes "current level" (runtime toggles update
it), and the Windows level-to-kind verification (see Research Digest).

Consumer impact check (the spec mandates it; stated here so the implementer
re-runs it mechanically): grep the four 02 consumers for `FloatLevel::`,
`WindowKind`, and `floating_snapshot` level reads.

- 03 spawns `FloatLevel::Floating` (03.md:883) and its own PopUp window
  directly (03.md:836); unaffected by an AlwaysOnTop mapping pin.
- 04 spawns `FloatLevel::AlwaysOnTop` (04.md:667) and already documents the
  Windows result as "plan 02 `AlwaysOnTop`/PopUp" (04.md:831).
- 09 spawns `FloatLevel::AlwaysOnTop` (09.md:798) and already documents
  "WS_EX_TOPMOST tool window" as the Windows column (09.md:1185).
- 10 spawns both levels (10.md:530, 627) and documents the HUD's Windows
  behavior as "WS_EX_TOPMOST tool window" (10.md:844).
- No consumer reads `FloatingWindowEntry.level` (grep for `entry.level`,
  `.level` on snapshot items: builders only, verified this session).

All four consumers expect AlwaysOnTop to be the PopUp posture on Windows, so
pinning that mapping changes nothing they rely on. `FloatingWindowEntry.level`
staying truthful under runtime toggles is additive.

Peer coordination with plan 26 (Wayland layer-shell widgets; discovered as a
parallel amendment to 02's contract, `02:1129-1204`): 26's
`set_input_passthrough(handle, passthrough, cx)` is specced as "the Wayland
arm of plan 25's click-through toggle", i.e. it expects 25's dispatcher to
delegate to it on Wayland sessions. 25's Step 2.2 implements that arm inline
via the public `Window::set_input_region` (`GPUI/window.rs:2518`), which is
the same mechanism and works for both plain toplevels and 26's layer-shell
surfaces, so the plans do not conflict; whichever lands second reconciles at
the single dispatch site (25's `shims/mod.rs` Wayland arm either keeps the
inline call or forwards to `layer_shell::set_input_passthrough`; one
call-site edit, no signature change on either side). Shared-file touches
(`floating/mod.rs`, `spec.rs`, `floating::init`) are additive on both sides:
25 adds `click_through`/`mod shims`/`CAPABILITY_FLOATING_CLICK_THROUGH`,
26 adds `layer_shell`/`layer_shell_spec`/its own capability consts. 26 also
reads `FloatingWindowEntry.surface`, not `.level`, so the level-semantics
note above is unaffected.

## Research Digest

Path key (all verified this session unless noted; the desktop-widgets survey
`docs/plans/_survey/desktop-widgets.md` is the primary reference and was
re-read in full):

| Short | Path |
| --- | --- |
| `GPUI` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-0.3.9/src` |
| `MACOS` | `/tmp/gpui-pre-src/gpui-pre-macos-0.3.9/src` (crates.io tarball the survey extracted; re-read this session) |
| `WINDOWS` | `/tmp/gpui-pre-src/gpui-pre-windows-0.3.9/src` (same provenance) |
| `LINUX` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-linux-0.3.9/src/linux` |
| `OAK` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated` |
| `WIN` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/windows-0.61.3/src/Windows/Win32` |
| `XRB` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/x11rb-0.13.2/src` |
| `XRBPROTO` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/x11rb-protocol-0.13.2/src/protocol` |
| `RWH` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/raw-window-handle-0.6.2/src` |
| `REPO` | `/home/thragg/fo/gpui-starter` |

Raw-handle reach-in (the mechanism every shim rides):

- `impl HasWindowHandle for Window` delegates to the platform window
  (`GPUI/window.rs:7571-7576`); `impl HasDisplayHandle for Window` follows at
  `:7579-7585`. `Window` also has an inherent `window_handle(&self) ->
  AnyWindowHandle` (`GPUI/window.rs:2371`) that shadows the trait method; the
  repo precedent calls the trait explicitly
  (`REPO/src/platform/liquid_glass.rs:229-241`).
- macOS: the handle is `AppKitWindowHandle { ns_view: NonNull<c_void> }`
  (`MACOS/window.rs:2421-2429`; field at `RWH/appkit.rs:90`). `[ns_view
  window]` reaches the NSWindow; in objc2-app-kit that is the safe method
  `NSView::window(&self) -> Option<Retained<NSWindow>>`
  (`OAK/NSView.rs:215`).
- Windows: the handle is `Win32WindowHandle { hwnd: NonZeroIsize }`
  (`WINDOWS/window.rs:595-601`; field at `RWH/windows.rs:54`).
- X11: `X11Window::window_handle` returns `XcbWindowHandle { window:
  NonZeroU32 }` (`LINUX/x11/window.rs:340-348`; field at `RWH/unix.rs:124`);
  `display_handle` returns `XcbDisplayHandle { connection, screen }` built
  via `as_raw_xcb_connection` over the live connection
  (`LINUX/x11/window.rs:350-364`; fields at `RWH/unix.rs:87,94`). The backend
  connection is `Rc<x11rb::xcb_ffi::XCBConnection>`
  (`LINUX/x11/client.rs:37,204`), i.e. a real C `xcb_connection_t` pointer.
- `RawWindowHandle`/`RawDisplayHandle` and all handle structs are `Copy`, so
  the dispatcher copies them out and drops the borrow before acting.
- The test platform has no handle: `TestWindow::window_handle` returns
  `Err(NotSupported)` (`GPUI/platform/test/window.rs:64-71`), so every shim
  call through a test window degrades to `false` without panicking.

The gap this feature closes (per platform):

- macOS: gpui sets level and collection behavior only at creation
  (`MACOS/window.rs:1206-1247`: Floating -> `NSFloatingWindowLevel`,
  PopUp -> `NSPopUpWindowLevel` + utility animation +
  `CanJoinAllSpaces | FullScreenAuxiliary`; the nonactivating style bit at
  `:1020-1024`). Not exposed: `setIgnoresMouseEvents:`, runtime `setLevel:`,
  post-creation `setCollectionBehavior:` (survey Part 1). All three are safe
  objc2-app-kit methods: `setIgnoresMouseEvents(bool)`
  (`OAK/NSWindow.rs:2136`), `setLevel(NSWindowLevel)`
  (`OAK/NSWindow.rs:1327`), `setCollectionBehavior(
  NSWindowCollectionBehavior)` (`OAK/NSWindow.rs:1446`).
  `NSWindowLevel = NSInteger` (`OAK/NSWindow.rs:265`);
  `NSNormalWindowLevel = 0` and `NSFloatingWindowLevel = 3` are generated
  consts (`:268,271`) but `NSPopUpWindowLevel` is NOT generated (grep of
  `OAK/NSWindow.rs` and `OAK/mod.rs`: Normal/Floating/ModalPanel/Status
  only), so the shim defines `const NS_POPUP_WINDOW_LEVEL: NSWindowLevel =
  101`, the same value gpui-pre-macos defines locally
  (`MACOS/window.rs:104`). `NSWindowCollectionBehavior` is a bitflags type
  (`OAK/NSWindow.rs:147-170`) with `Default`, `CanJoinAllSpaces` (1<<0), and
  `FullScreenAuxiliary` (1<<8).
- Windows: `WindowKind::PopUp` maps to `WS_EX_TOOLWINDOW | WS_EX_TOPMOST`
  with window style 0; every other kind, `Floating` included, falls into the
  `WS_EX_APPWINDOW` branch (`WINDOWS/window.rs:492-515`). Not exposed:
  `WS_EX_TRANSPARENT` (click-through), `WS_EX_NOACTIVATE`, runtime topmost
  re-assert (survey Part 2). The app's direct `windows = "0.61"` dependency
  (`REPO/Cargo.toml:287`) already enables `Win32_Foundation` and
  `Win32_UI_WindowsAndMessaging` (`REPO/Cargo.toml:287-300`), which carry
  everything the shim needs: `GetWindowLongPtrW` and `SetWindowLongPtrW`
  (`WIN/UI/WindowsAndMessaging/mod.rs:1132,2262`; both cfg-gated to 64-bit
  pointer width; the app ships x64, wasm, no 32-bit lane), `GWL_EXSTYLE`
  (`:3755`), `WS_EX_LAYERED/NOACTIVATE/TOPMOST/TRANSPARENT` (`:7270,7276,
  7287,7288`), `HWND_NOTOPMOST/HWND_TOPMOST` (`:4061,4063`), `SWP_NOACTIVATE/
  NOMOVE/NOSIZE` (`:6397,6399,6404`), and
  `SetWindowPos(hwnd, Option<HWND>, x, y, cx, cy, SET_WINDOW_POS_FLAGS) ->
  windows_core::Result<()>` (`:2279`). The lock already carries windows
  0.61.3 and 0.62.2 side by side (`REPO/Cargo.lock:9870-9871` and
  `:9883-9884`; 0.62.2 is
  gpui-pre-windows's copy); the shim uses the app's 0.61 line and adds no
  copy.
- X11: the backend's atom list has no `_NET_WM_STATE_ABOVE`, `_BELOW`,
  `_SKIP_TASKBAR`, or `_STICKY` (`LINUX/x11/window.rs:65-82`); PopUp is
  override-redirect + `_NET_WM_WINDOW_TYPE_NOTIFICATION` (`:514,616-625`).
  Consequence, load-bearing for QA design: override-redirect windows
  bypass the WM entirely (survey Part 3), so EWMH `_NET_WM_STATE` client
  messages only affect WM-managed windows: a PopUp-kind float never has a
  taskbar entry to remove and is already drawn above; the EWMH topmost
  path targets Floating-level floats. XShape input regions are server-side
  window properties (xorg shape extension semantics, survey Part 3), so
  click-through works on managed and override-redirect windows alike.
  The shim sends EWMH `_NET_WM_STATE` client messages and an XShape input
  region over the existing connection. Verified x11rb 0.13.2 surface:
  `xcb_ffi` is behind the `allow-unsafe-code` feature
  (`XRB/lib.rs:147,161-162`), already enabled by gpui-pre-linux's own x11rb
  dependency (`gpui-pre-linux-0.3.9/Cargo.toml:241-252`);
  `XCBConnection::from_raw_xcb_connection(ptr: *mut c_void, should_drop:
  bool) -> Result<XCBConnection, ConnectError>` is the documented unsafe
  wrapper for a raw `xcb_connection_t` (`XRB/xcb_ffi/mod.rs:92-107`);
  `shape` is a standalone feature (`x11rb-0.13.2/Cargo.toml:118`) exposing
  `shape_rectangles` / `shape_combine` / `shape_mask`
  (`XRB/protocol/shape.rs:182-196`) with `SO::SET` and `SK::{BOUNDING, CLIP,
  INPUT}` (`XRBPROTO/shape.rs:45-52,110-116`);
  `ClientMessageEvent::new(format: u8, window, type_: impl Into<Atom>, data:
  impl Into<ClientMessageData>)` sets `response_type` and `sequence` itself
  (`XRBPROTO/xproto.rs:6168-6184`); `send_event(conn, propagate, destination,
  event_mask, event)` (`XRB/protocol/xproto.rs:1040-1044`); `Window` and
  `Atom` are `u32` aliases (`XRBPROTO/xproto.rs:64,76`); `intern_atom(conn,
  only_if_exists, name: &[u8])` (`XRB/protocol/xproto.rs:650`). gpui's own
  X11 backend uses exactly this client-message pattern
  (`LINUX/x11/window.rs:977-993`: `ClientMessageEvent::new(32, window,
  _NET_WM_STATE, [action, prop1, prop2, 1, 0])`, `send_event(false, root,
  SUBSTRUCTURE_REDIRECT | SUBSTRUCTURE_NOTIFY, msg)`, then flush). The root
  window comes from `conn.setup().roots[screen].root`
  (`LINUX/x11/client.rs:418`), with `screen` taken from
  `XcbDisplayHandle::screen`.
- Wayland: click-through is public API: `Window::set_input_region(Option<&
  [Bounds<Pixels>]>)`, where `Some(&[])` means "no pointer or touch input"
  and `None` restores the default region (`GPUI/window.rs:2514-2520`);
  overridden by the Wayland backend at `LINUX/wayland/window.rs:2134-2155`,
  a no-op default elsewhere (`GPUI/platform.rs:1336`). Runtime topmost is
  compositor-owned on Wayland (survey Part 3); the toggle returns `false`
  there and the creation-time kinds (`FloatLevel::AlwaysOnTop`, or a future
  LayerShell option) are the correct lever.

Windows level-to-kind verification (the survey's flagged gap, re-verified;
this contradicts the feature spec's premise, not the survey):

- The spec says "plan 02 maps `FloatLevel::AlwaysOnTop` onto
  `WindowKind::Floating`". Plan 02 on disk already maps uniformly:
  `kind: match spec.level() { FloatLevel::Floating => WindowKind::Floating,
  FloatLevel::AlwaysOnTop => WindowKind::PopUp }` (02.md:667), with the
  platform matrix showing "WS_EX_TOPMOST tool window" for Windows
  (02.md:971-977). The survey itself states "Plans 02/09 already consume
  PopUp through `FloatLevel::AlwaysOnTop`" (survey Part 2). So the demanded
  end state (AlwaysOnTop spawns PopUp-kind on Windows, hence topmost) is
  already 02's design; the real gap is that `WindowKind::Floating` on
  Windows is NOT topmost (`WINDOWS/window.rs:492-515`), which 25 closes at
  runtime via the topmost toggle rather than by changing the mapping.
  Step 1.1 pins this in the amendment note; Step 2.6 verifies the landed
  code and fixes it only if it drifted from the plan text.

Repo integration facts:

- Reach-in precedent: `REPO/src/platform/liquid_glass.rs:229-241` extracts
  the NSView from `HasWindowHandle::window_handle(window)`, matches
  `RawWindowHandle::AppKit`, and casts `ns_view` to `&NSView`; the exact
  pattern the macOS shim repeats. Plan 03 specs the same shape for its notch
  panel (`docs/plans/03-notch-floater.md:836-852`: open window, then
  `handle.update(cx, |_, window, _| ... apply_notch_policy(window, ...))`,
  treating `false` as a non-error on the test platform).
- `AnyWindowHandle::update(self, cx, impl FnOnce(AnyView, &mut Window,
  &mut App) -> R) -> Result<R>` (`GPUI/window.rs:7542-7553`) is the runtime
  entry into `&mut Window` from a handle.
- Registry global mutation notifies observers on every `default_global`
  access (`GPUI/app.rs:2414-2422`, re-verified against 0.3.9 this session);
02's registry helpers
  (`set_entry`/`remove_entry`, 02.md:605-609) are the pattern
  `set_entry_level` follows.
- Capability literals: `CapabilityStatus` has five pub fields:
  `supported, enabled, degraded, reason, last_error`
  (`REPO/src/runtime/capabilities.rs:6-12`; INDEX §8 corrects 02's "six").
  Reason strings are English literals (precedent
  `REPO/src/platform/input/shortcuts.rs:290-305`).
- Gallery hook: a section is `mod x;` plus `x::register(&mut sections,
  window, cx)` inside `overlays::register`
  (`REPO/src/gallery/sections/overlays/mod.rs:1-30`;
  `REPO/src/gallery/registry.rs:1-17`); section shape from
  `REPO/src/gallery/sections/overlays/sheet.rs:451-458`
  (`GallerySection::new(id, title, description, view)`).
- Lock graph: x11rb 0.13.2 (`REPO/Cargo.lock:10528`), raw-window-handle
  0.6.2 (`:6889`), objc2 0.6.4 (`:5581`), objc2-app-kit 0.3.2 (`:5607`),
  as-raw-xcb-connection 1.0.1 (`:266`). The app's macOS table already has
  objc2 0.6 + objc2-app-kit 0.3.2 with `NSView`+`NSWindow` features and
  raw-window-handle 0.6 (`REPO/Cargo.toml:223-248`); the Linux table
  (`:250-272`) has neither x11rb nor raw-window-handle today. No new package
  version enters the lock: 25 adds dependency edges to crates already
  locked, and moves raw-window-handle from the macOS table to the common
  table (raw-window-handle 0.6 compiles on wasm32; it defines the Web
  handle, `RWH/web.rs`; `just wasm-check` is the proof gate).
- CI lanes that compile the cfg'd modules: macos-latest check
  (`REPO/.github/workflows/ci.yml:31`), ubuntu-24.04 check (`:116`),
  windows-latest (`:174`); `cargo test -p gpui-starter --lib` runs on Linux
  and Windows lanes (`:245,285` per the seams survey). There is no macOS
  test lane, so `macos.rs` ships without a test file and is verified by the
  macos-latest check plus manual QA.
- Naming rule for the new reader: boolean readers use `is_`/`has_` next to a
  same-named builder (gpui-kit coding guides, "Public API design" and the
  vocabulary table), hence `click_through(...)` + `is_click_through()`.
  Note for 02's implementer (defect found this session, not fixed here):
  02.md:667-671 calls `spec.focus()`, `spec.resizable()`, `spec.movable()`
  as readers, which cannot coexist with the same-named consume-self
  builders in 02's contract block (Rust has no method overloading). 25's
  own additions avoid the collision; 02's implementer must rename its
  readers (e.g. `is_focus()`) or make builders `Option`-taking.

Unverified items the survey binds as risks/open questions, restated here so
they are never assumed: Windows click-through on the DirectComposition path
(`WS_EX_NOREDIRECTIONBITMAP`); Mutter honoring empty input regions;
`kCGDesktopWindowLevel` (desktop-below stays out of scope); sparse-MSIX
widget registration (out of scope: no OS-surface integration is planned).

## Architecture

Entities and state ownership:

- `shims` module (new, inside `src/ui/floating/`): stateless. Two
  `pub(crate)` entry points, `set_click_through(window: &mut Window,
  enabled: bool) -> bool` and `set_always_on_top(window: &mut Window,
  enabled: bool) -> bool`. Each copies the raw handles out of the `Window`,
  dispatches on the `RawWindowHandle` variant, and returns whether a
  platform shim acknowledged the request. No globals, no caches (X11 atoms
  are interned per call; toggles are rare user actions).
- `FloatingSpec` gains a private `click_through: bool` (default `false`),
  spawn-time intent, owned by the spec like `focus`/`movable`.
- `FloatingWindowRegistry` (02's global) gains a `pub(crate)
  set_entry_level` mutation so `floating_snapshot` reflects the current
  topmost state. The registry stays the single owner of float state; the
  shims never talk to it.
- `WindowShimsSection` (gallery): owns one `Option<FloatingHandle>` demo
  float plus the last toggle results, so the demo is inspectable without a
  debugger.

Events and data flow:

- spawn: `spawn_floating(spec, ...)` → (02's existing path) → after
  `gpui_kit::open_window` returns, if `spec.is_click_through()` the spec's
  `AnyWindowHandle` runs `shims::set_click_through(window, true)`; failure
  logs at debug and does not fail the spawn (test platform, unsupported
  session).
- runtime toggle: caller → `set_floating_click_through` /
  `set_floating_always_on_top(handle, enabled, cx)` → `FloatingHandle::
  Native(h)` → `h.update(cx, |_, window, _| shims::set_*(window, enabled))`
  → dispatcher → platform module → OS call → `bool`. On topmost success,
  `registry::set_entry_level(handle, level, cx)` rewrites the entry's spec
  level through `default_global` (notify-on-write), so overlay hosts and
  plan 09's observers see the change. `Overlay` handles and `update` errors
  return `false`.
- capability: `floating::init` (02's) registers `floating_click_through`
  statically per cfg at boot; the runtime bools in the gallery section are
  the live per-call truth.

Module tree (additions inside 02's `src/ui/floating/`):

```text
src/ui/floating/
├── mod.rs            (02) + `mod shims;` + re-export of the two toggle fns
├── spec.rs           (02) + click_through field/builder/reader + set_level
├── registry.rs       (02) + set_entry_level
├── spawn.rs          (02) + spawn-time apply + the two runtime fns
├── probe.rs          (02) + floating_click_through registration
└── shims/
    ├── mod.rs        dispatcher: copy raw handles, match variant, Wayland
    │                 arm (set_input_region), fallback false
    ├── macos.rs      NSWindow reach-in: ignoresMouseEvents, level,
    │                 collectionBehavior
    ├── windows.rs    ex-style bits + SetWindowPos (+ windows.test.rs)
    └── x11.rs        EWMH client messages + XShape input region
                      (+ x11.test.rs)
```

cfg-gate diagram (which module compiles where):

```text
                     mod.rs  macos.rs  windows.rs  x11.rs
macOS                 yes     yes        no        no
Windows               yes      no       yes        no
Linux (X11 session)   yes      no        no       yes   -> Xcb arm
Linux (Wayland session) yes    no        no       yes   -> mod.rs Wayland arm
wasm                  yes      no        no        no   -> Web handle -> false
test platform         (any host) window_handle = Err(NotSupported) -> false
```

Dispatch is runtime feature detection: the `RawWindowHandle` discriminant
itself selects the backend (AppKit on macOS, Win32 on Windows, Xcb on an X11
session, Wayland on a Wayland session), so no session sniffing or env checks.

## Interface Contract

Additions to `crate::ui::floating` (02's contract block is untouched; these
are also appended to 02 as an amendment note, Step 1.1). Types via
`gpui_kit::*`; no `use gpui::` anywhere (`tests/qa_migration.rs:386-405`).

```rust
impl FloatingSpec {
    /// Spawn-time click-through: the float ignores all pointer input.
    /// Applied over the platform shim immediately after the native window
    /// opens. Advisory on surfaces without a shim: wasm overlay floats and
    /// test-platform windows open normally and the spawn still succeeds.
    pub fn click_through(self, enabled: bool) -> Self;          // default false
    /// Reader (is_ prefix per the kit coding guides; a same-named reader
    /// cannot coexist with the builder).
    pub fn is_click_through(&self) -> bool;
    /// Registry-internal: rewrite the stored level after a runtime toggle.
    pub(crate) fn set_level(&mut self, level: FloatLevel);
}

/// Runtime mouse pass-through toggle on an open float.
/// - macOS: NSWindow setIgnoresMouseEvents.
/// - Windows: WS_EX_TRANSPARENT | WS_EX_NOACTIVATE ex-style bits.
/// - X11: empty XShape input region (set) / input := bounding region
///   (clear).
/// - Wayland: Window::set_input_region(Some(&[])) / (None).
/// Returns true when a shim acknowledged the request; false is a normal
/// outcome (overlay handle, wasm, test platform, missing handle, window
/// gone), never an error.
pub fn set_floating_click_through(
    handle: &FloatingHandle,
    enabled: bool,
    cx: &mut App,
) -> bool;

/// Runtime always-on-top toggle on an open float.
/// - macOS: on = NSPopUpWindowLevel (101) + CanJoinAllSpaces |
///   FullScreenAuxiliary; off = NSFloatingWindowLevel (3) + default
///   behavior. The toggle cannot add the NSWindowStyleMaskNonactivatingPanel
///   bit a spawn-time PopUp gets (MACOS/window.rs:1020-1024): a
///   Floating-spawned float toggled on sits at popup level but still
///   activates on click, while the registry reports AlwaysOnTop.
/// - Windows: on = SetWindowPos(HWND_TOPMOST); off = HWND_NOTOPMOST.
/// - X11: on = _NET_WM_STATE_ADD of ABOVE | SKIP_TASKBAR | STICKY; off =
///   _NET_WM_STATE_REMOVE of the same (EWMH request; honoring is
///   WM-dependent, best-effort, returns true when sent). Applies to
///   WM-managed windows only: a PopUp-kind float is override-redirect and
///   unmanaged on X11, so the WM never sees these messages (the window is
///   already stacked above and has no taskbar entry); the EWMH path
///   targets Floating-level floats.
/// - Wayland: no-op returning false (z-order is compositor-owned; use
///   FloatLevel::AlwaysOnTop at spawn).
/// On success the registry entry's level is updated, so
/// floating_snapshot reports AlwaysOnTop/Floating as the CURRENT level.
pub fn set_floating_always_on_top(
    handle: &FloatingHandle,
    enabled: bool,
    cx: &mut App,
) -> bool;

pub const CAPABILITY_FLOATING_CLICK_THROUGH: &str = "floating_click_through";
```

Guarantees: neither function panics or errors on any platform; both are
no-ops (`false`) for `FloatingHandle::Overlay` and on wasm; both tolerate a
closed/stale `Native` handle (`AnyWindowHandle::update` error -> `false`);
registry state changes only through the notify-on-write global (existing
observers, e.g. 02's overlay host and 09's session, re-render normally).
`FloatingWindowEntry.level` semantics widen from "spawn level" to "current
level": no current consumer reads it (Dependencies check), and the value
for an untouched float is unchanged.

## Module Layout

| File | Purpose |
| --- | --- |
| `src/ui/floating/shims/mod.rs` | Dispatcher: copy raw handles, match variant, cfg'd platform arms, Wayland arm, `false` fallback; `LOG` target |
| `src/ui/floating/shims/macos.rs` | cfg(macos): NSWindow reach-in, click-through, level + collection behavior |
| `src/ui/floating/shims/windows.rs` | cfg(windows): ex-style bits, SetWindowPos topmost; pure bit-math helper |
| `src/ui/floating/shims/windows.test.rs` | Bit-math tests (runs on the Windows test lane) |
| `src/ui/floating/shims/x11.rs` | cfg(linux): connection wrap, atom interning, EWMH state messages, XShape input region; pure builders |
| `src/ui/floating/shims/x11.test.rs` | Atom names + message-construction tests (runs on the Linux dev host and CI lane) |
| `src/gallery/sections/overlays/window_shims.rs` | Gallery demo section driving both toggles on one live float |

Test files wire with the repo convention `#[cfg(test)] #[path =
"<name>.test.rs"] mod <name>_test;` inside their module
(`src/platform/desktop_shell/tray/windows.rs:13-15` is the cfg'd-module
precedent). `spawn.test.rs` and `spec.test.rs` are 02's files, extended.

## Seam Touches

| # | File | Change | Why minimal |
| --- | --- | --- | --- |
| 1 | `docs/plans/02-floating-window-toolkit.md` (end of Interface Contract section, after the "Guarantees for consumers" paragraph at 02.md:420-427) | Append the plan-25 amendment note (Step 1.1 gives the exact text) | Append-only prose; no reflow of the contract block, INDEX §5 discipline |
| 2 | `src/ui/floating/spec.rs` | `click_through` field + builder + `is_click_through` reader + `pub(crate) set_level` | One field on 02's value object |
| 3 | `src/ui/floating/registry.rs` | `pub(crate) fn set_entry_level(handle, level, cx)` | One helper next to 02's `set_entry`/`remove_entry` |
| 4 | `src/ui/floating/spawn.rs` | Spawn-time click-through apply; `set_floating_click_through`; `set_floating_always_on_top` | Both runtime fns sit beside `close_floating`/`respawn_floating` |
| 5 | `src/ui/floating/mod.rs` | `mod shims;` + add the two fns to the `pub use spawn::{...}` list + `CAPABILITY_FLOATING_CLICK_THROUGH` | Three lines |
| 6 | `src/ui/floating/probe.rs` | Register `floating_click_through` inside 02's `register_capabilities` | One match arm per platform in the existing fn |
| 7 | `Cargo.toml` | Linux table: `x11rb` entry; move `raw-window-handle = "0.6"` from the macOS table (line 248) to the common `[dependencies]` table | x11rb needed only on Linux; raw-window-handle needed by the dispatcher on every target (wasm-clean, `RWH/web.rs`); no new lock versions |
| 8 | `Cargo.lock` | Regenerated and staged with the manifest edit | `tests/qa_migration.rs:167-186` requires lock/manifest sync |
| 9 | `src/gallery/sections/overlays/window_shims.rs` + `src/gallery/sections/overlays/mod.rs` | New section file + the two-line hook (`mod window_shims;`, `window_shims::register(sections, window, cx);`) | Gallery's standard two-line section hook |

No edits to `src/app/init.rs` (02's `floating::init` already runs; the
capability row registers inside it), the locale catalogs, the not-wasm
table, or any Windows/macOS table entry.

### Removal Recipe

1. Delete `src/gallery/sections/overlays/window_shims.rs` and its two hook
   lines in `src/gallery/sections/overlays/mod.rs`.
2. Delete `src/ui/floating/shims/` entirely.
3. In `src/ui/floating/mod.rs`: drop `mod shims;`, the two fns from the
   `pub use spawn::{...}` list, and `CAPABILITY_FLOATING_CLICK_THROUGH`.
4. Revert `src/ui/floating/spawn.rs` to 02's version (drop the spawn-time
   apply and both runtime fns), `src/ui/floating/registry.rs` (drop
   `set_entry_level`), `src/ui/floating/spec.rs` (drop the field, builder,
   reader, `set_level`), and `src/ui/floating/probe.rs` (drop the
   `floating_click_through` registration).
5. In `Cargo.toml`: remove the Linux-table `x11rb` entry and move
   `raw-window-handle = "0.6"` back to the macOS table; regenerate
   `Cargo.lock` and stage it.
6. Delete the plan-25 amendment note from the end of 02's Interface
   Contract section (it is an appended block; deleting it restores 02
   verbatim).
7. Run `cargo fmt --all -- --check`, `cargo clippy --workspace
   --all-features`, `cargo test --workspace --all-features`,
   `just wasm-check`; grep `floating_click_through`, `set_floating_click_through`,
   `set_floating_always_on_top`, `window_shims` across `src/ tests/ locales/
   scripts/`: zero hits.

## Implementation Phases

## Phase 1: Contract amendment and spec extension

### Step 1.1: Amendment note on plan 02 (already appended; verify)

Files: `docs/plans/02-floating-window-toolkit.md` (append only).

The amendment note was appended to 02 during planning (it is on disk now,
after 02.md:427, the end of the "Guarantees for consumers" paragraph,
before `## Module Layout`, as `### Amendment (2026-10-12, plan 25): widget
window shims`). Do NOT append it again. This step verifies the on-disk
block against the required content and corrects it in place only if a
point is missing. Required content:

- The added API, verbatim from this plan's Interface Contract: the
  `click_through` builder, `is_click_through` reader, `set_level`
  (pub(crate)), `set_floating_click_through`, `set_floating_always_on_top`,
  `CAPABILITY_FLOATING_CLICK_THROUGH`, with the per-platform behavior table
  and the "current level" semantic note for `FloatingWindowEntry.level`.
- The Windows mapping verification: `WindowKind::Floating` falls into the
  `WS_EX_APPWINDOW` branch and is not topmost on Windows
  (`WINDOWS/window.rs:492-515`); `WindowKind::PopUp` is
  `WS_EX_TOOLWINDOW | WS_EX_TOPMOST` with style 0; 02's Step 3.2 mapping
  (`FloatLevel::AlwaysOnTop => WindowKind::PopUp`, 02.md:667) therefore
  already makes AlwaysOnTop floats topmost on Windows; verified against the
  pinned sources, no mapping change required, and implementers must keep
  the mapping uniform across platforms.
- The consumer check result (03/04/09/10, from this plan's Dependencies
  section) with the grep command that reproduces it.

Tests: none (doc).
Done-check: `grep -c "Amendment (2026-10-12, plan 25)" docs/plans/02-floating-window-toolkit.md`
returns exactly `1` (a second copy must never exist); `git diff docs/plans/02-floating-window-toolkit.md`
shows only added lines (no deletions or reflowed lines).

### Step 1.2: `FloatingSpec::click_through`

Files: `src/ui/floating/spec.rs`, `src/ui/floating/spec.test.rs`.

Build:
- Private field `click_through: bool`, set to `false` in `new`.
- Builder `pub fn click_through(mut self, enabled: bool) -> Self`
  (consume-self, placed with the other boolean builders).
- Reader `pub fn is_click_through(&self) -> bool`.
- `pub(crate) fn set_level(&mut self, level: FloatLevel)` rewriting the
  stored level (used by the registry in Step 2.6).

Tests (`spec.test.rs`, plain `#[test]`, no gpui context needed): default
is `false`; `click_through(true).is_click_through()` is `true`;
`set_level(FloatLevel::AlwaysOnTop)` then asserting the private `level`
field directly: the sibling test module is a child of `spec`, so it sees
`spec`'s private fields, which is the only observable because 02's
contract exposes no level reader (the registry/snapshot path is exercised
in Step 2.6 and manual QA instead).
Done-check: `cargo test -p gpui-starter --lib floating::spec` green.

## Phase 2: Shims module and integration

### Step 2.1: Manifest and lock

Files: `Cargo.toml`, `Cargo.lock`.

Build:
- Linux table (`Cargo.toml:250-272`), after the `image` entry:

  ```toml
  # X11 EWMH/XShape window shims (src/ui/floating/shims/x11.rs) over the
  # same x11rb 0.13 the lock already carries for gpui-pre-linux; shape gates
  # the XShape protocol module, allow-unsafe-code gates xcb_ffi (wrapping
  # the live connection; no second X connection is opened).
  x11rb = { version = "0.13", default-features = false, features = [
      "allow-unsafe-code",
      "shape",
  ] }
  ```

  (x11rb 0.13.2 has no default feature set; verified: no `default =` under
  its `[features]`; the `default-features = false` is explicitness only.)
- Move `raw-window-handle = "0.6"` from the macOS table (`Cargo.toml:248`)
  into the common `[dependencies]` table with a one-line comment naming the
  shims as the reason (the dispatcher compiles on every target; the crate
  is wasm-clean, `RWH/web.rs`).
- Run `cargo check -p gpui-starter` once so the lock records the new
  gpui-starter dependency edges (x11rb, raw-window-handle now common), and
  stage `Cargo.lock` with the manifest (`tests/qa_migration.rs:167-186`).
  No new package versions may appear in the lock; if `git diff Cargo.lock`
  shows a version add, stop and re-pin to the locked versions.

Tests: none.
Done-check: `grep -n 'x11rb' Cargo.toml` shows the Linux-table entry;
`grep -n 'raw-window-handle' Cargo.toml` shows exactly one line, in the
common table; `cargo check -p gpui-starter` green on the dev host.

### Step 2.2: Dispatcher `shims/mod.rs`

Files: `src/ui/floating/shims/mod.rs` (new), `src/ui/floating/mod.rs`.

Build:
- `const LOG: &str = "gpui_starter::ui::floating";` (tracing-target rule,
  seams survey §9).
- Module gates mirroring the platform seam style
  (`src/platform/mod.rs:2-3`):

  ```rust
  #[cfg(target_os = "macos")]
  mod macos;
  #[cfg(target_os = "windows")]
  mod windows;
  #[cfg(target_os = "linux")]
  mod x11;
  ```

- `pub(crate) fn set_click_through(window: &mut Window, enabled: bool) -> bool`
  and `pub(crate) fn set_always_on_top(window: &mut Window, enabled: bool)
  -> bool`. Both start by copying the raw handle:

  ```rust
  let Ok(handle) = HasWindowHandle::window_handle(window) else { return false };
  let raw = handle.as_raw(); // RawWindowHandle is Copy
  ```

  then `match raw`:
  - `RawWindowHandle::AppKit(h)` (cfg macos) -> `macos::set_click_through(
    &h, enabled)` / `macos::set_always_on_top(&h, enabled)`.
  - `RawWindowHandle::Win32(h)` (cfg windows) -> `windows::set_click_through(
    &h, enabled)` / `windows::set_always_on_top(&h, enabled)`.
  - `RawWindowHandle::Xcb(h)` (cfg linux) -> pull the display handle the
    same way (`HasDisplayHandle::display_handle(window)`, copy
    `RawDisplayHandle::Xcb(d)`, take `d.connection` and `d.screen`), then
    `x11::set_click_through(h.window.get(), conn, d.screen, enabled)` /
    `x11::set_always_on_top(h.window.get(), conn, d.screen, enabled)` with
    `conn: NonNull<c_void>`; `None` connection or non-Xcb display -> false.
  - `RawWindowHandle::Wayland(_)` -> click-through arm only:
    `window.set_input_region(if enabled { Some(&[]) } else { None }); true`.
    The topmost function returns `false` on Wayland (compositor-owned).
  - `_ => false` (covers the Web handle on wasm and any future variant).
- Imports: `use gpui_kit::Window; use raw_window_handle::{HasDisplayHandle,
  HasWindowHandle, RawDisplayHandle, RawWindowHandle};` plus `std::ffi::
  c_void` / `std::ptr::NonNull` for the Xcb plumbing under cfg(linux). No
  `use gpui::` (qa_migration).
- `src/ui/floating/mod.rs`: add `mod shims;` to the module list.

Tests: none here (Phase 2.6 exercises the dispatcher through the public
functions on every host).
Done-check: `cargo check -p gpui-starter` green on the dev host with the
three platform module files from Steps 2.3-2.5 in place (create them in
order; this step's check is the tree compiling once all of Phase 2's
modules exist).

### Step 2.3: macOS shim

Files: `src/ui/floating/shims/macos.rs` (new).

Build (all behind the module's cfg from Step 2.2):
- `const NS_POPUP_WINDOW_LEVEL: NSWindowLevel = 101;` (objc2-app-kit 0.3.2
  does not generate it; value matches `MACOS/window.rs:104`).
- `fn ns_window(handle: &AppKitWindowHandle) -> Option<Retained<NSWindow>>`:

  ```rust
  // SAFETY: ns_view is the live GPUIView pointer gpui's own
  // HasWindowHandle impl hands out (MACOS/window.rs:2421-2429); shim calls
  // run on the app's main thread inside AnyWindowHandle::update, the only
  // thread AppKit permits window mutation on, and gpui keeps the view
  // alive for the window's lifetime.
  let view: &NSView = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
  view.window()
  ```

  (cast pattern from `REPO/src/platform/liquid_glass.rs:236-240`).
- `pub(crate) fn set_click_through(handle: &AppKitWindowHandle, enabled:
  bool) -> bool`: `ns_window` -> `window.setIgnoresMouseEvents(enabled)`
  (`OAK/NSWindow.rs:2136`) -> `true`; `None` -> `false`.
- `pub(crate) fn set_always_on_top(handle: &AppKitWindowHandle, enabled:
  bool) -> bool`: `ns_window` -> on: `window.setLevel(
  NS_POPUP_WINDOW_LEVEL)` + `window.setCollectionBehavior(
  NSWindowCollectionBehavior::CanJoinAllSpaces |
  NSWindowCollectionBehavior::FullScreenAuxiliary)` (the exact posture
  gpui gives PopUp at creation, `MACOS/window.rs:1234-1240`); off:
  `window.setLevel(NSFloatingWindowLevel)` + `setCollectionBehavior(
  NSWindowCollectionBehavior::Default)` -> `true`. The objc2 setters are
  safe methods; the only unsafe block in the file is the view cast.

Tests: none (no macOS test lane in CI; behavior covered by manual QA 3-5).
Done-check: `cargo check -p gpui-starter` green on the dev host (module
cfg'd out); the macos-latest check lane (`ci.yml:31`) is the compile proof;
this step is not done until that lane is green on the PR.

### Step 2.4: Windows shim

Files: `src/ui/floating/shims/windows.rs` (new),
`src/ui/floating/shims/windows.test.rs` (new).

Build (cfg windows):
- `fn with_ex_bits(current: u32, bits: u32, enable: bool) -> u32` is pure:
  `if enable { current | bits } else { current & !bits }`.
- `const CLICK_THROUGH_BITS: WINDOW_EX_STYLE = WS_EX_TRANSPARENT |
  WS_EX_NOACTIVATE;` (the spec's flag pair; `WS_EX_LAYERED` is deliberately
  NOT set: layered windows need a redirection surface, which conflicts
  with the `WS_EX_NOREDIRECTIONBITMAP` DirectComposition window
  (`WINDOWS/window.rs:514`); see Risks 1).
- `pub(crate) fn set_click_through(handle: &Win32WindowHandle, enabled:
  bool) -> bool`:

  ```rust
  let hwnd = HWND(handle.hwnd.get());
  // SAFETY: hwnd is this window's own handle from gpui's
  // HasWindowHandle impl (WINDOWS/window.rs:595-601); the call runs on the
  // thread that owns the window (gpui window updates run on the app's
  // main/UI thread), the documented safe context for SetWindowLongPtrW.
  let current = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
  let next = with_ex_bits(current, CLICK_THROUGH_BITS.0, enabled);
  // SAFETY: same hwnd ownership; GWL_EXSTYLE with a plain style word.
  unsafe { SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next as isize) };
  true
  ```

- `pub(crate) fn set_always_on_top(handle: &Win32WindowHandle, enabled:
  bool) -> bool`:

  ```rust
  let hwnd = HWND(handle.hwnd.get());
  let after = if enabled { HWND_TOPMOST } else { HWND_NOTOPMOST };
  // SAFETY: same hwnd ownership; NOMOVE|NOSIZE|NOACTIVATE restrict the
  // call to a z-order band change, per the documented API.
  unsafe {
      SetWindowPos(hwnd, Some(after), 0, 0, 0, 0,
          SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE)
  }
  .is_ok()
  ```

- Imports: `use raw_window_handle::Win32WindowHandle; use
  windows::Win32::Foundation::HWND; use windows::Win32::UI::
  WindowsAndMessaging::{...};` (all names verified at
  `WIN/UI/WindowsAndMessaging/mod.rs:1132,2262,2279,3755,4061,4063,6397,
  6399,6404,7276,7288`).
- `#[cfg(test)] #[path = "windows.test.rs"] mod windows_test;` wired inside
  `windows.rs` (cfg'd-module precedent
  `src/platform/desktop_shell/tray/windows.rs:13-15`).

Tests (`windows.test.rs`, plain `#[test]`): `with_ex_bits` sets both bits on
a zero word; clears both without touching neighbors (`WS_EX_TOPMOST` bit
survives a clear); idempotence (set twice, clear twice). Runs on the
Windows test lane (`ci.yml:255`).
Done-check: `cargo check -p gpui-starter` green on the dev host (module
cfg'd out); windows-latest check lane (`ci.yml:174`) green on the PR;
`cargo test -p gpui-starter --lib` green on a Windows host or the CI
Windows test lane.

### Step 2.5: X11 shim

Files: `src/ui/floating/shims/x11.rs` (new),
`src/ui/floating/shims/x11.test.rs` (new).

Build (cfg linux):
- `use x11rb::connection::Connection as _;` (brings `RequestConnection`
  methods incl. `flush`), `use x11rb::protocol::xproto::ConnectionExt;`
  (intern_atom, send_event), `use x11rb::protocol::shape::ConnectionExt as
  _;` (shape_rectangles, shape_combine), `use x11rb::protocol::xproto::
  {Atom, ClientMessageEvent, ClipOrdering, EventMask, Window}; use
  x11rb::protocol::shape::{SK, SO}; use
  x11rb::xcb_ffi::XCBConnection;`.
- `const STATE_ATOM_NAMES: [&str; 5] = ["_NET_WM_STATE",
  "_NET_WM_STATE_ABOVE", "_NET_WM_STATE_BELOW",
  "_NET_WM_STATE_SKIP_TASKBAR", "_NET_WM_STATE_STICKY"];`
- `struct StateAtoms { net_wm_state: Atom, above: Atom, below: Atom,
  skip_taskbar: Atom, sticky: Atom }` (below is carried because the pure
  message builder accepts any state atom; the public path never sends it;
  desktop-below is out of scope per the survey's unverified
  `kCGDesktopWindowLevel`).
- `const STATE_ADD: u32 = 1;` / `const STATE_REMOVE: u32 = 0;` (EWMH
  action codes; gpui's own hints use the same encoding,
  `LINUX/x11/window.rs:977-981`).
- `fn state_messages(atoms: &StateAtoms, action: u32, target: Window,
  states: &[Atom]) -> Vec<ClientMessageEvent>` is pure; chunks `states`
  into pairs (EWMH carries two state atoms per message, l[0]=action,
  l[1]=prop1, l[2]=prop2, l[3]=source indication 1), building each with
  `ClientMessageEvent::new(32, target, atoms.net_wm_state, [action, p1,
  p2_or_0, 1, 0])` (`XRBPROTO/xproto.rs:6177-6184`).
- `fn wrap_connection(ptr: NonNull<c_void>) -> Option<XCBConnection>`:

  ```rust
  // SAFETY: ptr is the live xcb_connection_t owned by gpui's X11 client
  // (LINUX/x11/window.rs:350-364 hands it out through HasDisplayHandle);
  // should_drop = false so the wrapper never disconnects, and the wrapper
  // is dropped at the end of each shim call, before gpui's client can go
  // away (the client outlives every window, hence every call).
  Some(unsafe { XCBConnection::from_raw_xcb_connection(ptr.as_ptr(), false) }.ok()?)
  ```

- `fn intern_state_atoms(conn: &XCBConnection) -> Option<StateAtoms>`:
  issue all five `intern_atom(true, name.as_bytes())` requests first, then
  collect `.reply()`s (pipelined round-trips); any `Err` or any atom equal
  to 0 (`only_if_exists` miss: the WM does not speak EWMH) -> log at debug
  and return `None`.
- `fn send_state_messages(conn: &XCBConnection, root: Window, msgs:
  &[ClientMessageEvent]) -> bool`: for each msg `conn.send_event(false,
  root, EventMask::SUBSTRUCTURE_REDIRECT | EventMask::
  SUBSTRUCTURE_NOTIFY, *msg)` then one `conn.flush()` (the exact pattern
  of `LINUX/x11/window.rs:977-993`); any `Err` -> debug log + `false`.
- `pub(crate) fn set_click_through(window_id: u32, connection: NonNull<
  c_void>, screen: i32, enabled: bool) -> bool`: wrap; on enable
  `conn.shape_rectangles(SO::SET, SK::INPUT, ClipOrdering::UNSORTED,
  window_id, 0, 0, &[])` (empty rectangle list = empty input region;
  XShape spec semantics per the survey); on disable
  `conn.shape_combine(SO::SET, SK::INPUT, SK::BOUNDING, window_id, 0, 0,
  window_id)` (input region := the window's own bounding region, the
  documented reset without a Pixmap argument). `conn.flush()`; `Err`
  (shape extension absent) -> debug log + `false`, else `true`. The
  `screen` parameter is unused here and carried for signature symmetry
  with `set_always_on_top`; mark it `let _ = screen;` or name it `_screen`
  in this fn only.
- `pub(crate) fn set_always_on_top(window_id: u32, connection: NonNull<
  c_void>, screen: i32, enabled: bool) -> bool`: wrap; intern atoms; root =
  `conn.setup().roots[screen as usize].root` (`LINUX/x11/client.rs:418`
  pattern); states = `[above, skip_taskbar, sticky]` with action
  `STATE_ADD`/`STATE_REMOVE`; `send_state_messages`. Returns `true` when
  the messages were flushed (honoring is WM policy; best-effort by
  design). A PopUp-kind float is override-redirect on X11
  (`LINUX/x11/window.rs:514`), so the WM never manages it: the messages
  are dropped harmlessly and the window is already stacked above; the
  EWMH path is the lever for Floating-level floats (manual QA 8).
- Test wiring: `#[cfg(test)] #[path = "x11.test.rs"] mod x11_test;`.

Tests (`x11.test.rs`, plain `#[test]`, no X connection needed): the five
atom names are byte-exact; `state_messages` with three states yields two
messages, first data `[action, s1, s2, 1, 0]`, second `[action, s3, 0, 1,
0]`, both `format == 32`, `window == target`, `type_ == net_wm_state`; one
state yields one message; zero states yield none; a message built with
`atoms.below` carries it (keeps the BELOW path exercised).
Done-check: `cargo test -p gpui-starter --lib floating::shims` green on
the dev host (Linux) and the ubuntu-24.04 CI test lane; `cargo check
-p gpui-starter` green.

### Step 2.6: Spawn integration and runtime functions

Files: `src/ui/floating/spawn.rs`, `src/ui/floating/registry.rs`,
`src/ui/floating/mod.rs`, `src/ui/floating/spawn.test.rs`.

Build:
- `registry.rs`: `pub(crate) fn set_entry_level(handle: &FloatingHandle,
  level: FloatLevel, cx: &mut App)`: mutate through
  `cx.default_global::<FloatingWindowRegistry>()` (notify-on-write, the
  `set_entry` pattern, 02.md:605-609); match entries by handle identity:
  `Native(a)` vs `Native(b)` when `a.window_id() == b.window_id()`
  (`GPUI/window.rs:7520`), `Overlay(a)` vs `Overlay(b)` when `a == b`; on
  match `entry.spec.set_level(level)`.
- `spawn.rs`, native branch, after `gpui_kit::open_window` returns the
  `AnyWindowHandle` and before the registry `set_entry`:

  ```rust
  if spec.is_click_through() {
      let applied = handle
          .update(cx, |_, window, _| shims::set_click_through(window, true))
          .unwrap_or(false);
      if !applied {
          tracing::debug!(
              target: LOG,
              "click-through not applied at spawn (no shim or unsupported handle)"
          );
      }
  }
  ```

  (03's post-spawn `handle.update` pattern, 03.md:846-852.)
- `spawn.rs` public functions (full signatures in Interface Contract):
  `set_floating_click_through` and `set_floating_always_on_top` matching on
  the handle: `Native(h)` -> `h.update(cx, |_, window, _|
  shims::set_*(window, enabled)).unwrap_or(false)`; `Overlay(_)` ->
  `false`. The topmost fn additionally calls
  `registry::set_entry_level(handle, if enabled { FloatLevel::AlwaysOnTop }
  else { FloatLevel::Floating }, cx)` when applied.
- Windows mapping verification (the spec's demanded check, executed here):
  grep the landed `spawn_floating` for the kind match; it must read
  `FloatLevel::AlwaysOnTop => WindowKind::PopUp` (02.md:667). If it maps
  AlwaysOnTop to `WindowKind::Floating` anywhere, change it to `PopUp`:
  on Windows only `PopUp` is topmost (`WINDOWS/window.rs:492-515`). Record
  the observed state in the PR description.
- `floating/mod.rs`: add both fns to the `pub use spawn::{...}` re-export
  list.

Tests (`spawn.test.rs`, `#[gpui_kit::test]`, test platform):
- spawn with `.click_through(true)` -> `Ok`, snapshot contains the entry
  (spawn-time apply degrades to `false` on the test platform without
  panicking);
- `set_floating_click_through` on a `Native` test-platform handle ->
  `false`, no panic;
- `set_floating_always_on_top` on a `Native` test-platform handle ->
  `false`, and the snapshot level is unchanged (still the spawn level);
- both fns on an `Overlay` handle (insert via 02's test-only
  `insert_overlay_for_test`) -> `false`;
- `set_floating_always_on_top` with a closed `Native` handle ->
  `false` (update error path).

Done-check: `cargo test -p gpui-starter --lib floating` green on the dev
host; `grep -n "set_floating_click_through" src/ui/floating/mod.rs` shows
the re-export.

## Phase 3: Capability and gallery demo

### Step 3.1: `floating_click_through` capability

Files: `src/ui/floating/probe.rs`, `src/ui/floating/mod.rs`.

Build:
- `pub const CAPABILITY_FLOATING_CLICK_THROUGH: &str =
  "floating_click_through";` in `floating/mod.rs` beside 02's capability
  constants (02.md:367-369).
- In 02's `register_capabilities` (probe.rs): append one entry:
  native platforms (macos/windows/linux): `CapabilityStatus::
  supported_enabled()`; wasm: a five-field literal `{ supported: false,
  enabled: false, degraded: false, reason: Some("overlay floats render
  inside the main window; no native window to pass input through".into()),
  last_error: None }` (five pub fields,
  `src/runtime/capabilities.rs:6-12`; literal-English reasons per the
  shortcuts.rs precedent).

Tests: extend `probe.test.rs` (02's) with: after `floating::init`, the
snapshot contains `floating_click_through` (run inside
`#[gpui_kit::test]`, the 02.md:870-873 pattern).
Done-check: `cargo test -p gpui-starter --lib floating::probe` green; the
diagnostics page shows the row (manual).

### Step 3.2: Gallery section `window_shims`

Files: `src/gallery/sections/overlays/window_shims.rs` (new),
`src/gallery/sections/overlays/mod.rs`.

Build:
- `WindowShimsSection` entity + `WindowShimsSection::view(window, cx)` +
  `pub fn register(sections, window, cx)` pushing
  `GallerySection::new("window_shims", "Click-Through & Topmost", "Toggle
  mouse pass-through and always-on-top on a live floating window.",
  WindowShimsSection::view(window, cx))` (shape from
  `sheet.rs:451-458`; literal English copy is the gallery discipline,
  02.md:988-990).
- State: `demo: Option<FloatingHandle>`, `click_through_on: bool`,
  `topmost_on: bool`, `last_applied: Option<(bool, bool)>` (the two most
  recent toggle results).
- Buttons (kit `Button`, label buttons, no icons; keeps the default-106
  icon policy out of scope). Two spawn buttons, because the X11 EWMH path
  is only observable on a WM-managed window (a PopUp-kind float is
  override-redirect there; Research Digest): "Spawn (topmost)" ->
  `spawn_floating` with `.level(FloatLevel::AlwaysOnTop)`; "Spawn
  (managed)" -> same spec with `.level(FloatLevel::Floating)`. Shared spec
  shape: `FloatingSpec::new(FloatingWindowId("window-shims-demo".into()),
  size(px(320.), px(180.))).title("Widget float").click_through(false)
  .anchor(FloatAnchor::Screen { display: None, placement: Placement9::
  Center, offset: point(px(0.), px(0.)) }).focus(false)`, build closure a
  placeholder view. Each spawn button first closes any existing demo float
  (`close_floating`), then spawns with its own level and stores the
  handle, resetting `click_through_on`/`topmost_on` to the spawn level's
  implied state (false/true for topmost, false/false for managed).
  "Click-through" / "Always on top" (enabled only when `demo` is present)
  -> call the runtime fns with the tracked bool, store the returned
  `bool`s in `last_applied` and render them as a status line ("applied /
  not applied on this platform"); "Close" -> `close_floating` + clear
  state. Uses only the public floating API (dogfoods the contract).
- Hook lines in `overlays/mod.rs`: `mod window_shims;` (alphabetical,
  after `tooltip`) and `window_shims::register(sections, window, cx);`
  last in `register()`.
- wasm behavior: floats spawn as overlay panes through 02's overlay path;
  both toggles return `false`; the status line states it. Degraded but
  routable, per the wasm rules.

Tests: none new (manual QA surface; the section compiles on wasm, which is
the point).
Done-check: `cargo check -p gpui-starter` green; the section appears under
the gallery's overlays group (manual).

## Phase 4: Lock, gates, verification

### Step 4.1: Full gates

Files: none new (verifies the tree).

Commands, in order (AGENTS.md):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

Plus the lane checks this feature uniquely needs (cfg'd modules compile on
their runners): the PR's macos-latest check (`ci.yml:31`), ubuntu-24.04
check + test (`ci.yml:116,245`), and windows-latest check + test
(`ci.yml:174,255`) jobs green.

Done-check: all five local gates pass; the three platform lanes pass;
`git diff --stat` shows only the files in Seam Touches.

## Platform & Wasm Gating

cfg gates:

- `src/ui/floating/shims/mod.rs`: compiled on every target (wasm included);
  platform arms carry their own `#[cfg(target_os = ...)]` on the match arms
  and module declarations (`src/platform/mod.rs:2-3` style).
- `shims/macos.rs`: `#[cfg(target_os = "macos")]`, compiled by the
  macos-latest lanes only.
- `shims/windows.rs` + `windows.test.rs`: `#[cfg(target_os = "windows")],
  windows-latest lanes.
- `shims/x11.rs` + `x11.test.rs`: `#[cfg(target_os = "linux")]`, dev host
  and ubuntu-24.04 lanes. The module is selected at runtime only when the
  raw handle is `Xcb` (X11 session); a Wayland session takes the mod.rs
  Wayland arm.
- wasm: the dispatcher compiles; the Web handle falls into `_ => false`;
  overlay handles return `false` in the public fns. No `cfg(target_family
  = "wasm")` block is needed anywhere in the feature; `Window::
  set_input_region` exists on all platforms (`GPUI/window.rs:2518`, no-op
  default `GPUI/platform.rs:1336`).

Capability-registry entries (registered by 02's `floating::init`, amended
by Step 3.1):

| Key | macOS | Windows | Linux | wasm |
| --- | --- | --- | --- | --- |
| `floating_click_through` | supported_enabled | supported_enabled | supported_enabled | unsupported, enabled:false, reason "overlay floats render inside the main window; no native window to pass input through" |

`floating_always_on_top` (02's entry) gains a runtime dimension from this
plan (macOS `setLevel:`, Windows `SetWindowPos`, X11 EWMH requests) but its
registration is unchanged; the amendment note documents the extension.

Native-only deps: `x11rb` sits in the Linux target table
(`cfg(target_os = "linux")`), which is inherently non-wasm; nothing joins
the `cfg(not(target_family = "wasm"))` table. `raw-window-handle` moves to
the common table; it is wasm-clean (`RWH/web.rs` defines the Web handle)
and `just wasm-check` is the proof. The macOS and Windows tables are
untouched: `objc2`/`objc2-app-kit` with `NSView`+`NSWindow` features
already cover the shim (`Cargo.toml:223-248`), and `windows = "0.61"`
already enables `Win32_Foundation` + `Win32_UI_WindowsAndMessaging`
(`Cargo.toml:287-300`). No new crate version enters the lock (all target
versions verified present: `Cargo.lock:266,5581,5607,6889,9883,10528`).

## i18n

No catalog keys. The only new user-visible strings are gallery section
copy (literal English, the gallery discipline shared by every section,
`sheet.rs:451-456`) and the capability reason string (English literal, the
`shortcuts.rs:290-305` precedent). Both catalogs stay untouched;
`tests/qa_localization.rs` passes unchanged.

| Key | en | zh-CN |
| --- | --- | --- |
| none | (no keys) | (no keys) |

## Design Compliance

Brief weight: platform plumbing plus one gallery section, no product UI of
its own.

- Components: the gallery section uses stock kit `Button` (label buttons,
  default/ghost per the local hierarchy of 02's neighboring section),
  "Components and composition" (use the platform component, do not build
  bespoke controls).
- State visibility: toggle outcomes are rendered, not silent: the status
  line shows applied/not-applied per call ("Design thesis" rule 5, "State
  must be visible"; the coding guides' platform-boundary rule that a
  platform branch preserves the semantic contract even when presentation
  differs).
- Interface language: button labels are immediate verbs ("Spawn", "Close")
  with no ellipsis, per "Interface language" (immediate actions); the
  section title states what it does.
- Density/spacing/tokens: no new tokens, no raw colors or literal radii;
  the section reuses the standard gallery section layout, so "Tokens
  before values" holds by construction.
- Accessibility: label buttons carry text names (no icon-only controls, so
  no tooltip obligation); no new always-present nodes outside the gallery
  page, so `qa_a11y` is unaffected.
- Capability visibility: the diagnostics row plus the gallery status line
  satisfy "State must be visible" for platform degradation.

## Verification

No cargo gate was run during planning (doc-only session, the convention
INDEX §8 records for every plan in this tree; all commands below are the
implementer's gates, quoted from AGENTS.md and the justfile).

Repo gates (Step 4.1, run exactly as named):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

`just wasm-check` (`justfile:24-25`) proves the dispatcher compiles on
wasm32 and the overlay/no-op path is the only reachable one there.

Per-platform manual QA: these features cannot be fully verified on one OS;
each lane names its own checks:

macOS (needs a Mac; dev flow `bash scripts/macos-dev-app.sh`):

1. Gallery -> Overlays -> "Click-Through & Topmost" -> Spawn: a 320x180
   AlwaysOnTop float opens, non-activating, on all spaces (Mission
   Control shows it on every desktop).
2. "Click-through" on: clicks over the float land on the window behind it
   (place it over Finder); the app does not activate. Off: the float
   takes clicks again. `xprop`-equivalent check is not available; verify
   behaviorally.
3. "Always on top" off: the float drops to floating level (above normal
   windows, below popups of other apps); on: returns to popup level and
   all-spaces behavior.
4. Spawn with click-through pre-set via a temporary `.click_through(true)`
   in the section: the float is pass-through from the first frame.

Windows (needs a Windows host; no runtime Windows check happened in
planning):

5. Same Spawn flow: the float is a borderless topmost tool window, above
   other apps, absent from the taskbar and Alt-Tab.
6. "Click-through" on: clicks pass through to windows beneath. This is the
   survey-flagged unverified item (WS_EX_TRANSPARENT on the
   DirectComposition/WS_EX_NOREDIRECTIONBITMAP path, `WINDOWS/window.rs:
   514`); record the outcome in the PR. If pass-through fails, do NOT add
   WS_EX_LAYERED blindly (redirection-surface conflict, Risks 1); file
   the result and leave the toggle returning true with the behavior gap
   documented.
7. "Always on top" off then on: z-order band visibly changes against
   another app's window; the float never activates on click while
   WS_EX_NOACTIVATE is set (click the float when click-through is off).

Linux X11 (dev host or any X session; a real X server, not headless):

8. Use "Spawn (managed)" (`FloatLevel::Floating`): the PopUp kind is
   override-redirect on X11 (`LINUX/x11/window.rs:514`), unmanaged, and
   never has a taskbar entry, so the EWMH path is invisible on it. On the
   managed float: `xprop _NET_WM_STATE` shows `_NET_WM_STATE_ABOVE,
   _NET_WM_STATE_SKIP_TASKBAR, _NET_WM_STATE_STICKY` after topmost on; the
   taskbar entry disappears; after off, the atoms are gone. Then toggle
   topmost once on the "Spawn (topmost)" float as well: applied=true with
   no stacking change expected (override-redirect window, already above;
   the messages are dropped by design).
9. "Click-through" on: clicks fall through to the window beneath; off:
   input restored. Works on both spawn kinds: XShape input regions are
   server-side window properties, not WM hints. `xshape` state is
   inspectable via `xwininfo` if needed.
10. EWMH-absent WM (e.g. a bare twm session, optional): toggles return
    applied=true (messages sent) but stacking does not change; the
    documented best-effort contract; no crash.

Linux Wayland (KWin or a wlroots compositor, plus GNOME/Mutter for the
open question):

11. KWin/Hyprland/Sway: "Click-through" on -> pointer events pass through
    the float (empty input region honored); off -> restored.
12. GNOME (Mutter): repeat 11 and record whether input passes through;
    this closes Open Question 2 either way; the gallery status line shows
    the call returned true (protocol-level) regardless.
13. Wayland "Always on top": returns not-applied by design.

wasm (`just wasm-check` + the existing web dev flow):

14. The gallery section renders; Spawn opens an overlay pane; both toggles
    report not applied; the page stays routable and `qa_localization`
    still passes (no keys added).

Acceptance criteria:

- All four local gates pass with only the Seam Touches files changed, and
  the macos/windows/ubuntu CI lanes pass on the PR.
- Every item in the Interface Contract exists with the exact signatures
  (mechanically checkable via grep/`cargo doc`).
- Both runtime fns return `false` (never panic, never error) on overlay
  handles, wasm, and the test platform; `floating_snapshot` reflects level
  changes after a successful topmost toggle.
- `Cargo.lock` gains no new package version; only gpui-starter's
  dependency edges change.
- The 02 amendment note exists, is append-only, and contains the Windows
  mapping verification plus the consumer check.

## Risks & Open Questions

Ranked:

1. **Windows click-through on the DirectComposition path is unverified at
   runtime.** `WS_EX_TRANSPARENT`'s documented semantics are sibling paint
   order; the common recipe adds `WS_EX_LAYERED`, but layered windows need
   a redirection surface while gpui creates its windows with
   `WS_EX_NOREDIRECTIONBITMAP` (`WINDOWS/window.rs:492-515`, mutually
   exclusive directions). Tried: read the pinned backend source and the
   survey's Windows evidence (`windows.md` Path B); no Windows host
   existed in any session to run it. Mitigation: ship
   `WS_EX_TRANSPARENT | WS_EX_NOACTIVATE` only, close by manual QA 6 on a
   real Windows host, and record the outcome in the PR. The fallback lever
   (recreating the window without DirectComposition, or trying
   `WS_EX_LAYERED`) is documented here, not implemented.
2. **EWMH honoring is WM policy.** X11 topmost/skip-taskbar/sticky are
   requests; non-EWMH window managers ignore them and the toggle still
   returns true ("sent"). Tried: read the backend atom list
   (`LINUX/x11/window.rs:65-82`, the backend itself never sets these
   atoms) and the EWMH quotes in the survey. Mitigation: best-effort
   contract documented in the Interface Contract; QA 8/10 verify on EWMH
   and non-EWMH sessions.
3. **Mutter may not honor empty input regions** (survey's unverified item).
   The Wayland arm uses the public protocol-level API, so the call
   succeeds regardless. Tried: docs.rs surface verification + survey; no
   GNOME session run in planning. Closure: manual QA 12 records the
   behavior; no code change is implied either way.
4. **Cross-thread xcb use while gpui's event loop polls.** The shim wraps
   the same `xcb_connection_t` gpui's client polls on another thread and
   blocks on `intern_atom` replies. lib xcb serializes request submission
   and reply routing internally, but this was not exercised under load
   here. Tried: verified the connection type (`LINUX/x11/client.rs:37,204`)
   and the wrapper contract (`XRB/xcb_ffi/mod.rs:92-107`); no multi-thread
   stress test run. Mitigation: toggles are rare user actions (five
   pipelined round-trips per topmost toggle); QA 8-10 cover it. If a hang
   ever appears, the documented lever is pre-interning atoms once at first
   float spawn on the X11 path.
5. **Plan 02's latent reader/builder name collision.** 02.md:667-671 calls
   `spec.focus()`-style readers that cannot coexist with its same-named
   builders (no method overloading in Rust). Found this session by
   re-reading 02's contract against its Step 3.2. 25's additions are
   immune (`is_click_through`). 02's implementer must rename its readers;
   if that lands after 25's Phase 1, no conflict arises (different names).
6. **macOS and Windows modules compile only on their CI lanes** (no such
   host in the dev environment; no macOS test lane at all). Done-checks
   name the lanes (`ci.yml:31,174,255`); macos.rs ships without unit
   tests, matching plan 03's posture for cfg'd platform code
   (03.md:802-806).
7. **Registry level semantics widen** ("spawn level" -> "current level").
   No consumer reads `FloatingWindowEntry.level` (grep this session);
   re-run the consumer grep at implementation (Dependencies section lists
   it) before merging.
8. **`NSPopUpWindowLevel` is not generated by objc2-app-kit 0.3.2** (grep
   of the generated sources). The shim defines the constant locally with
   the same value gpui-pre-macos uses (`MACOS/window.rs:104`); if a future
   objc2-app-kit generates it, drop the local const.
9. **Plan 26 amended 02 in parallel** (Wayland layer-shell; `02:1129-1204`)
   and expects 25's dispatcher to route Wayland click-through through its
   `set_input_passthrough`. Tried: read 26's amendment in full this
   session. No conflict exists today (both use the same public
   `Window::set_input_region` mechanism; the Dependencies coordination
   paragraph states the one-call-site reconciliation), but the implementer
   of whichever plan lands second must re-read the other's Wayland arm
   before wiring, so the two never diverge on Wayland click-through
   semantics.

Open questions (each with what was tried):

- Windows DComp click-through (risk 1): source-verified only; needs QA 6.
  No Windows host available in planning sessions.
- Mutter empty-input-region honoring (risk 3): docs-level only; needs QA
  12.
- `kCGDesktopWindowLevel` / desktop-below placement: the survey could not
  verify the citation, so desktop-below stays out of scope; the X11 shim's
  pure builder accepts `_NET_WM_STATE_BELOW` (tested) but no public API
  sends it. Tried: survey Part 1 + macos evidence; no Apple doc fetch
  verified the constant.
- Sparse-MSIX widget registration and all OS-surface integrations
  (WidgetKit, Windows widget board, plasmoids): out of scope by the
  survey's recommendation; nothing in this plan depends on them.
- Whether a future gpui-pre exposes set_level/topmost natively (upstream
  zed may grow it): not checked upstream beyond the snapshot; the shim is
  version-pinned to 0.3.9 and a pin bump re-opens this plan's Research
  Digest (INDEX §4 pin-update rule applies).
