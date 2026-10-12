# 02 Floating Window Toolkit Plan

## Goal

Callers spawn borderless utility windows that sit above the app (anchored to a
main-window rect or a screen rect, explicit size, no focus steal), track and
close them by handle, and query per-platform support (always-on-top, notch
geometry, menu-bar height). On wasm the same API opens in-window overlay floats
instead of OS windows, and every platform registers capability-registry entries
that name what works and why not.

## Dependencies

None upstream. This plan is a foundation; it consumes only the repo baseline
(capability registry `src/runtime/capabilities.rs`, the `RootWindow` global
`crate::app::window::root_window`, the `src/ui` mount seam, the gallery
section registry). Consumers, all on disk and all building on the Interface
Contract below (verified by reading their Dependencies sections this session):

- `docs/plans/03-notch-floater.md` — `probe`/`FloatingPlatformInfo`
  (`notch`, `menu_bar`), `NotchGeometry`, `FloatingWindowId`, `FloatingSpec`
  builders, `FloatAnchor::Screen`, `spawn_floating`, `close_floating`,
  `floating_snapshot`, `FloatingHandle` (by path
  `crate::ui::floating::FloatingHandle`), and `floating::init` ordering
  (03's init runs after it).
- `docs/plans/04-menubar-charts.md` — `FloatingWindowId`, `FloatingSpec`
  builders, `FloatLevel::AlwaysOnTop`, `FloatAnchor::Screen`,
  `Placement9::TopLeft`, `spawn_floating`, `close_floating`,
  `floating_snapshot` (entries `id`/`handle`), `DismissFloating` Esc.
- `docs/plans/09-floating-ui.md` — everything above plus
  `FloatingHandle::{Native, Overlay}`, `probe(cx).native_windows`, and the
  `FloatingWindowRegistry` Global type by path (it observes the global), so
  `floating::mod.rs` must re-export `FloatingWindowRegistry`.
- `docs/plans/10-floating-canvas.md` — full `FloatingSpec` builder set
  including `min_size`, `FloatAnchor::Window`, `respawn_floating`,
  `DismissFloating`.

Consumer docs cite line numbers into this file (`02:280-281` style). Those
pointers are informational; the signatures in Interface Contract are the
contract. This revision keeps the contract section content identical to the
version those plans were written against (only attribute placement and the
registry re-export note changed).

## Research Digest

All gpui/gpui-kit/objc2 claims below were re-verified this session against
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` (short paths
`GPUI/src`, `MAC/src`, `WIN/src`, `X11/...`, `WL/...`, `WEB/src` for
gpui-pre-0.3.8, gpui-pre-macos-0.3.8, gpui-pre-windows-0.3.8,
gpui-pre-linux-0.3.8/src/linux, gpui-pre-web-0.3.8), plus `KIT/src` for
gpui-kit-0.7.1, the objc2 crates, and repo files. Survey docs
(`docs/plans/_survey/seams.md`, `ecosystem.md`) were re-checked where
load-bearing; one stale survey claim corrected below (macOS window-handle
reachability).

Window creation and options:

- `gpui_kit::open_window<V: Render>(options, cx, build) -> Result<(AnyWindowHandle, Entity<V>)>`
  hosts the kit `base::Root` in the new window (`KIT/src/lib.rs:149-164`). App
  precedent: `src/app/window.rs:85-98`.
- `WindowOptions` full field set at `GPUI/src/platform.rs:2438-2507`: window_bounds,
  titlebar, focus, show, kind, is_movable, app_owns_titlebar_drag,
  inactive_frame_interval, is_resizable, is_minimizable, display_id,
  window_background, app_id, window_min_size, window_decorations, icon,
  tabbing_identifier.
- `WindowKind` variants at `GPUI/src/platform.rs:2653-2683`: `Normal`, `PopUp`,
  `AnchoredPopup(popup::PopupOptions)`, `Floating`,
  `LayerShell` (linux + wayland feature), `Dialog`.
- `TitlebarOptions { title, appears_transparent, traffic_light_position }` at
  `GPUI/src/platform.rs:2641-2649`; `WindowBounds::Windowed(Bounds<Pixels>)` at
  `:2576-2585` with `get_bounds()` at `:2594-2602`. All re-exported through
  `gpui_kit::*` (`GPUI/src/gpui.rs:195` does `pub use platform::*`; the kit
  globs `::gpui::*` at `KIT/src/lib.rs:97`).
- Level is fixed by `WindowKind` at creation; there is no public set-level or
  set-position API. The `PlatformWindow` trait method list
  (`GPUI/src/platform.rs:1145-1235`) has `resize` and `set_title`, nothing for
  origin or level. Reposition therefore means close-and-reopen.

Per-platform meaning of the kinds (load-bearing for the level policy):

| Platform | `Floating` | `PopUp` | Citation |
| --- | --- | --- | --- |
| macOS | NSPanel + `NSFloatingWindowLevel` (above normal windows) | NSPanel + `NSWindowStyleMaskNonactivatingPanel` + `NSPopUpWindowLevel` + `CanJoinAllSpaces \| FullScreenAuxiliary` | `MAC/src/window.rs:1013-1023, 1211-1240`; nonactivating const `:95-96` |
| Windows | normal style (`WS_EX_APPWINDOW`, not topmost) | `WS_EX_TOOLWINDOW \| WS_EX_TOPMOST`, style 0 (borderless, no resize frame) | `WIN/src/window.rs:491-518` |
| X11 | `WM_TRANSIENT_FOR` to parent | `_NET_WM_WINDOW_TYPE_NOTIFICATION` | `X11/x11/window.rs:614-627, 629-645` |
| Wayland | `xdg_toplevel.set_parent` | same parent path | `WL/wayland/window.rs:262-263` |
| wasm | rejected `UnsupportedWindowKind("floating windows")` | rejected | `WEB/src/platform.rs:375-400` |

`AnchoredPopup` is implemented on Linux only (`X11/x11/window.rs:479`,
`WL/wayland/window.rs:206`, `WL/wayland/client.rs:1244`) and rejected on macOS
(`MAC/src/platform.rs:770-774`), Windows (`WIN/src/window.rs:444-446`), and web
(`WEB/src/platform.rs:382`); rejected as the base for our anchor API.

Titlebar/borderless behavior: `titlebar: None` on macOS still yields
`titled | full-size-content` (`MAC/src/window.rs:1008-1011`); the borderless
look is `TitlebarOptions { appears_transparent: true, .. }` (adds
`NSFullSizeContentView`, `:1004-1006`), which on Windows sets
`hide_title_bar = true` (`WIN/src/window.rs:481-484`; note the Windows default
is `unwrap_or(true)`, so `titlebar: None` hides it there too).
`NSBorderlessWindowMask` is not reachable through `WindowOptions` (only
simple-fullscreen uses it, `MAC/src/window.rs:640-645`).

Non-focus-stealing: `focus: false` + `show: true` → `orderFront` (no key) on
macOS (`MAC/src/window.rs:1288-1292`), `SW_SHOWNOACTIVATE` on Windows
(`WIN/src/window.rs:578-581`). Linux ignores the flag (no reference to
`params.focus` in `X11/x11/window.rs` or `WL/wayland/window.rs`; grep run this
session). On macOS `PopUp` additionally uses a non-activating panel.

Lifecycle and control: `Window::remove_window()` (`GPUI/src/window.rs:2324`,
sets `removed = true`), `cx.on_window_closed(fn(&mut App, WindowId)) ->
Subscription` (`GPUI/src/app.rs:2631-2640`; repo detach precedent
`src/app/init.rs:265-267`), `AnyWindowHandle::window_id()`
(`GPUI/src/window.rs:7382`), `AnyWindowHandle::update(self, cx, FnOnce(AnyView,
&mut Window, &mut App) -> R)` (`:7407-7414`), `WindowHandle::update`
(`:7273-7290`), `Window::window_bounds()` (`:2464`), `resize` (`:2774`),
`start_window_move` (`:2888`; macOS `MAC/src/window.rs:2209`, X11 `:1828`,
Wayland `:2035`, Windows inherits the trait no-op default
`GPUI/src/platform.rs:1272` — WIN has no override; grep run this session).

Displays/anchoring: `cx.displays()` (`GPUI/src/app.rs:1487`),
`cx.primary_display()` (`:1492`), `cx.find_display(id)` (`:1509`);
`PlatformDisplay::{id, bounds, visible_bounds, default_bounds}`
(`GPUI/src/platform.rs:566-593`). macOS `visible_bounds` is
`NSScreen.visibleFrame` converted to top-left origin
(`MAC/src/display.rs:121-129`), so menu-bar height is derivable as
`visible.origin.y - bounds.origin.y` without objc.

Observation: `cx.observe_window_bounds(window, cb)` fires on resize
(`GPUI/src/app/context.rs:402-417`); repo model
`src/shell/root/app_root/state.rs:132-138`.

In-window overlay layer: `Position::Absolute` + macro-generated `absolute()`,
`top_*`, `left_*` style methods (`GPUI/src/style.rs:1236-1248`;
`gpui-pre-macros-0.3.8/src/styles.rs:113-125`). An absolute child's offsets
resolve against its closest positioned ancestor, else the origin
(`GPUI/src/style.rs:1241-1243`); `Position::Relative` is the default for every
element (`:1233-1240`), so a plain parent's box is the containing block. No
`z_index` in this snapshot (grep across `GPUI/src` empty this session);
stacking is element order. Two more layout facts that fix the overlay host
mount: `flex_shrink` defaults to `1.0` (`GPUI/src/style.rs:802`), so an
in-flow `size_full()` child appended to the already-full AppRoot column would
steal height from `content_area`'s `flex_1()`
(`src/shell/root/app_root/render.rs:267`, with `content_v` at `:139`); and the
numeric style suffix `"0"` generates `top_0()` / `left_0()`
(`gpui-pre-macros-0.3.8/src/styles.rs:925-933`, `box_style_suffixes`), so the
host can be taken out of flow with `absolute().top_0().left_0().size_full()`.

Stable identity: `ElementId` has `From` for `&'static str`, `String`,
`SharedString`, and the tuples `(&'static str, usize|u64|u32|EntityId)`
(`GPUI/src/window.rs:7514-7582`). There is NO `From<(&'static str,
SharedString)>` — a close button keyed by a `FloatingWindowId` must use a
`format!`-built `String` (`From<String>` at `:7538`) or
`(ElementId, T: Into<SharedString>)` composition (`:7579-7584`).

macOS interop (survey correction): `impl HasWindowHandle for Window` exists
(`GPUI/src/window.rs:7434`), and the repo already reaches the NSView/NSWindow
through it (`src/platform/liquid_glass.rs:230-240`;
`AppKitWindowHandle { ns_view }` in raw-window-handle-0.6.2
`src/platform/appkit.rs:88-91`). The ecosystem survey's "not reachable from
user code" claim is stale; this plan still does not need NSWindow reach-in
because level/space policy is covered by `WindowKind`.

Notch/menu-bar (macOS extras): `objc2-app-kit` 0.3.2 exposes
`NSScreen::safeAreaInsets() -> NSEdgeInsets` (`objc2-app-kit-0.3.2/src/generated/NSScreen.rs:107-109`),
`auxiliaryTopLeftArea()` (`:111-113`), `auxiliaryTopRightArea()`
(`:115-117`), `frame()` (`:54`), `visibleFrame()` (`:58`),
`backingScaleFactor()` (`:99-101`), `mainScreen(mtm)` (`:37`), behind the
crate feature `NSScreen` (`objc2-app-kit-0.3.2/Cargo.toml:1232`), which the
repo does not enable yet (`Cargo.toml:243-246` enables std, NSView, NSWindow).
`MainThreadMarker::new() -> Option<Self>` is safe
(`objc2-0.6.4/src/main_thread_marker.rs:230`). Repo pins objc2 0.6 /
objc2-app-kit 0.3.2 (`Cargo.toml:230,243-246`); the lock also carries a
transitive objc2-app-kit 0.2.2, which the feature addition does not touch.

Actions and keys: the `Action` derive supports `#[action(namespace = <ident>,
no_json)]`; namespace is stringified, not path-resolved
(`gpui-pre-macros-0.3.8/src/derive_action.rs:36-42, 113-116`). Repo precedent
for the exact derive shape: `src/app/actions.rs:17-19`
(`#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)] #[action(namespace = app, no_json)]`).
`App::bind_keys(&mut self, impl IntoIterator<Item = KeyBinding>)`
(`GPUI/src/app.rs:2486`); `KeyBinding::new(keystrokes: &str, action, context:
Option<&str>)` (`GPUI/src/keymap/binding.rs:31-33`). `key_context` and
`track_focus` are `InteractiveElement`/`StatefulInteractiveElement` methods
(`GPUI/src/elements/div.rs:830`, `:788`); `Focusable` is
`fn focus_handle(&self, cx: &App) -> FocusHandle` (`GPUI/src/window.rs:710-713`).

Components: `Button::label` (`gpui-component-0.7.1/src/button/button.rs:371`),
`Button::icon(impl Into<ButtonIcon>)` (`:397`), `Button::ghost` (`:82`),
`Button::accessibility_label` (`:391`). Theme carries `popover`
(`gpui-component-0.7.1/src/theme/mod.rs:524,616`) and `radius: Pixels`
(`:185`). Icon assets verified present in
`gpui-kit-assets-0.7.1/assets/icons/`: `x.svg`, `app-window.svg`,
`layers.svg`, `monitor.svg`.

Test surface: the test platform implements `open_window` with `TestWindow`
(`GPUI/src/platform/test/platform.rs:493-506`); `TestWindow::window_handle`
returns `Err(NotSupported)` (`GPUI/src/platform/test/window.rs:64-71`), so
probes must tolerate handle failure. `#[gpui_kit::test]` + `TestAppContext`
usable in-crate (`src/state/config_store.test.rs:280-281`). `Context<T>`
derefs to `App` (`GPUI/src/app/context.rs:26-38`), so page listeners can pass
`cx` where `&mut App` is expected.

Globals: `cx.default_global` pushes a notify-observers effect on every call
(`GPUI/src/app.rs:2294-2302`); `observe_global` at `:2330-2343`. Mirrors the
`CapabilityRegistry::set` pattern (`src/runtime/capabilities.rs:62-79`).
Effects flush at the end of every `App::update` (`App::flush_effects`,
`GPUI/src/app.rs:1874-1877`).

`SharedString` derives `PartialOrd, Ord, Hash` and impls `Serialize`/
`Deserialize` (`gpui-pre-shared-string-0.3.8/gpui_shared_string.rs:14,194,203`),
plus `PartialEq<str>`/`PartialEq<&str>` (`:98,104`), so
`FloatingWindowId(pub SharedString)` can derive the full key set and
consumers can compare `id.0 == "literal"`.

Crate picks: none new. macOS extras ride the existing `objc2-app-kit = 0.3.2`
macOS dep (`Cargo.toml:243-246`), feature list extended with `NSScreen`; no
new crate enters the lock.

## Architecture

Entities and state ownership:

- `FloatingSpec`: value object describing one float: identity, size, title,
  level, anchor, focus/resizable/movable/min-size. No behavior beyond
  geometry resolution.
- `FloatingWindowRegistry`: app `Global` (same shape as
  `CapabilityRegistry`, `src/runtime/capabilities.rs:53-85`): map
  `FloatingWindowId -> RegistryEntry { spec, resolved_bounds, handle }`.
  Owned by the App. Mutation via `set`/`remove` helpers that go through
  `default_global` so `observe_global` consumers (overlay host, gallery demo,
  plan 09's palettes) re-render.
- `FloatingShell`: `Render` entity that is the content view of every native
  floating window (the kit's `base::Root` wraps it). Owns a header strip
  (title, close), the consumer `AnyView`, and the `DismissFloating` action.
  One per open native float.
- `OverlayFloatHost`: `Render` entity held by `AppRoot`; renders all
  overlay-mode floats as absolutely positioned panes inside the main window.
  Its root is itself out of flow (`absolute().top_0().left_0().size_full()`)
  so the AppRoot column's flex layout is untouched and pane offsets are
  window coordinates (see Step 4.1 for the containing-block chain). Empty on
  native platforms; the only float surface on wasm.
- `floating::init(cx)`: registers capabilities, binds the Esc action,
  installs the app-level `on_window_closed` tracker (detached Subscription,
  `src/app/init.rs:265-267` pattern), and creates the registry global lazily
  via first use.

Events and data flow:

- spawn: caller → `spawn_floating(spec, build, cx)` → resolve bounds from
  anchor → native: `gpui_kit::open_window` with a `FloatingShell` content
  view → registry `set`; wasm: build view inside the root window's context →
  registry `set` with `Overlay` handle → host observes registry global →
  paints pane.
- close: `close_floating(&handle, cx)` → native:
  `handle.update(cx, |_, window, _| window.remove_window())` → the
  `on_window_closed` tracker removes the registry entry; overlay: remove
  registry entry directly.
- probe: `probe(cx)` returns static per-platform facts plus live macOS
  notch data; `init` writes the three capability entries.

Module tree:

```text
src/ui/floating/
├── mod.rs            module seam: re-exports, init(cx), capability names
├── spec.rs           FloatingSpec, FloatLevel, FloatAnchor, Placement9, resolve geometry
├── spec.test.rs
├── registry.rs       FloatingWindowRegistry Global, FloatingWindowEntry, window-close tracking
├── registry.test.rs
├── spawn.rs          spawn_floating / close_floating / close_all_floating / respawn_floating
├── spawn.test.rs
├── shell.rs          FloatingShell view for native windows
├── overlay_host.rs   OverlayFloatHost entity + absolute pane layer
├── probe.rs          FloatingPlatformInfo probe + capability registration
├── macos_probe.rs    cfg(macos) NSScreen notch and menu-bar probe
└── probe.test.rs
```

## Interface Contract

Public surface at `crate::ui::floating` (all types via `gpui_kit::*` re-exports;
no `use gpui::` anywhere, per `tests/qa_migration.rs:386-405`). `mod.rs`
re-exports every item below, including the `FloatingWindowRegistry` Global
type (plan 09 observes it via `cx.observe_global::<FloatingWindowRegistry>`).

```rust
// `PartialOrd, Ord` are required: the registry is a BTreeMap keyed by this.
// `SharedString` itself derives them (gpui-pre-shared-string-0.3.8/gpui_shared_string.rs:14).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FloatingWindowId(pub SharedString);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatLevel {
    /// Above the app's normal windows. Not OS-topmost on Windows/X11.
    Floating,
    /// PopUp window kind: NSPopUp level + all spaces (macOS),
    /// WS_EX_TOPMOST (Windows), notification type (X11). Implies
    /// non-activating on macOS and non-resizable on Windows.
    AlwaysOnTop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement9 {
    TopLeft, Top, TopRight,
    Left, Center, Right,
    BottomLeft, Bottom, BottomRight,
}

#[derive(Clone, Debug)]
pub enum FloatAnchor {
    /// Place relative to a window's current bounds (usually `root_window(cx)`).
    Window { window: AnyWindowHandle, placement: Placement9, offset: Point<Pixels> },
    /// Place relative to a display's visible bounds; None = primary display.
    Screen { display: Option<DisplayId>, placement: Placement9, offset: Point<Pixels> },
}

pub struct FloatingSpec { /* private fields */ }
impl FloatingSpec {
    pub fn new(id: FloatingWindowId, size: Size<Pixels>) -> Self;
    pub fn title(self, title: impl Into<SharedString>) -> Self;      // default: id string
    pub fn level(self, level: FloatLevel) -> Self;                    // default Floating
    pub fn anchor(self, anchor: FloatAnchor) -> Self;                 // default centered on primary display
    pub fn focus(self, focus: bool) -> Self;                          // default false (non-stealing)
    pub fn resizable(self, resizable: bool) -> Self;                  // default false
    pub fn movable(self, movable: bool) -> Self;                      // default true
    pub fn min_size(self, min: Size<Pixels>) -> Self;                 // default None
    pub fn id(&self) -> &FloatingWindowId;
}

#[derive(Clone, Debug)]
pub enum FloatingHandle {
    Native(AnyWindowHandle),
    Overlay(FloatingWindowId),
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FloatingWindowEntry {
    pub id: FloatingWindowId,
    pub title: SharedString,
    pub level: FloatLevel,
    /// Screen-space bounds (for overlay floats: main-window space).
    pub bounds: Bounds<Pixels>,
    pub handle: FloatingHandle,
}

pub fn spawn_floating(
    spec: FloatingSpec,
    build: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    cx: &mut App,
) -> anyhow::Result<FloatingHandle>;
/// Returns true if the handle was open and is now closing/closed.
pub fn close_floating(handle: &FloatingHandle, cx: &mut App) -> bool;
pub fn close_all_floating(cx: &mut App) -> usize;
/// Recompute the anchor and reopen. Native has no move API
/// (`GPUI/src/platform.rs:1145-1235`), so the OS window is closed and
/// reopened; content is rebuilt through the view factory retained at spawn
/// time. Overlay floats only update their stored bounds. Content state is
/// not carried across a native respawn; caller responsibility.
pub fn respawn_floating(id: &FloatingWindowId, cx: &mut App) -> bool;
pub fn floating_snapshot(cx: &App) -> Vec<FloatingWindowEntry>;

/// Call once from `app::init` before any spawn: capabilities, Esc binding,
/// window-close tracking.
pub fn init(cx: &mut App);

pub const CAPABILITY_FLOATING_WINDOWS: &str = "floating_windows";
pub const CAPABILITY_ALWAYS_ON_TOP: &str = "floating_always_on_top";
pub const CAPABILITY_NOTCH_GEOMETRY: &str = "notch_geometry";

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FloatingPlatformInfo {
    pub native_windows: bool,
    pub always_on_top: bool,
    pub non_focus_stealing: bool,
    pub notch: Option<NotchGeometry>,
    pub menu_bar: Option<Pixels>,
}

#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct NotchGeometry {
    /// Screen-edge insets in logical pixels; top includes the menu bar region
    /// on notched displays.
    pub safe_area: Edges<Pixels>,
    /// Camera-housing rect in screen coordinates (top-center), if present.
    pub camera_housing: Option<Bounds<Pixels>>,
}

pub fn probe(cx: &App) -> FloatingPlatformInfo;

/// Esc action inside a floating shell or overlay pane; bound by `init`.
#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = floating, no_json)]
pub struct DismissFloating;

/// In-window overlay layer host; `AppRoot` owns one and renders it last.
pub struct OverlayFloatHost { /* private */ }
impl OverlayFloatHost {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self;
    pub fn is_empty(&self) -> bool;
}

/// Registry global re-exported for observers (plan 09).
pub use registry::FloatingWindowRegistry;
```

Note on the attribute placement: `non_exhaustive` is a built-in attribute, not
a derive; it sits on its own line above the struct, never inside
`#[derive(...)]`.

Builder retention: `spawn_floating` stores `build` in the registry entry as
`Rc<dyn Fn(&mut Window, &mut App) -> AnyView>` (the closure must be callable
more than once, so callers pass view constructors, not one-shot captures,
the same constraint kit dialog content closures have). `respawn_floating`
reruns that factory. Content state is not carried across a native respawn;
the `spawn_floating` signature therefore takes `Fn`, not `FnOnce`.

Guarantees for consumers: `spawn_floating` never panics on unsupported
platforms. Native `Err` cases are exactly two: the anchor cannot be resolved
(window closed, no display) or `open_window` itself failed. Wasm `Err` case
is exactly one: no main window has been recorded yet
(`root_window(cx) == None`, `src/app/window.rs:20-22`); once the app has a
main window, wasm spawns always succeed via the overlay path.
`floating_snapshot` is ordered by id (BTreeMap). All handles stay valid
across `App` lifetime; a closed handle makes `close_floating` return `false`.

## Module Layout

Every module with a sibling test file wires it inside the module file with
the repo convention `#[cfg(test)] #[path = "<name>.test.rs"] mod <name>_test;`
(`src/ui/widgets/virtual_list.rs:90-92`; seams survey §7); the table below
lists the files, the wiring line is part of each step's work.

| File | Purpose |
| --- | --- |
| `src/ui/floating/mod.rs` | Re-export seam, `init`, capability-name constants |
| `src/ui/floating/spec.rs` | Spec types + pure `resolve_bounds` geometry |
| `src/ui/floating/spec.test.rs` | Placement/clamping table tests (plain `#[test]`) |
| `src/ui/floating/registry.rs` | Registry global, entry tracking, close-all |
| `src/ui/floating/registry.test.rs` | Registry set/remove/snapshot tests (`#[gpui_kit::test]`) |
| `src/ui/floating/spawn.rs` | Native + overlay spawn/close/respawn routing |
| `src/ui/floating/spawn.test.rs` | Headless spawn/close lifecycle tests |
| `src/ui/floating/shell.rs` | `FloatingShell` content view for native floats |
| `src/ui/floating/overlay_host.rs` | Overlay panes layer entity |
| `src/ui/floating/probe.rs` | Platform info + capability registration |
| `src/ui/floating/macos_probe.rs` | NSScreen notch/menu-bar probe (cfg macos) |
| `src/ui/floating/probe.test.rs` | Probe degradation tests |

## Seam Touches

| # | File | Change | Why minimal |
| --- | --- | --- | --- |
| 1 | `src/ui/mod.rs:1` | Add `pub mod floating;` | One-line mount, mirrors `pub mod widgets;` |
| 2 | `src/app/init.rs` (after `notifications::initialize` at `:274`) | `crate::ui::floating::init(cx);` | Single boot hook; capabilities/keys/tracker live in the module |
| 3 | `src/shell/root/app_root/state.rs:17-38,168-185` | Field `floating_host: Entity<OverlayFloatHost>` + construct in `AppRoot::new` | Host must live in the main window's tree |
| 4 | `src/shell/root/app_root/render.rs:287-291` | Append `.child(self.floating_host.clone())` as last child of the root `v_flex` | Overlay layer needs topmost tree order (no z_index in gpui) |
| 5 | `Cargo.toml:243-246` | Add `"NSScreen"` to `objc2-app-kit` features | Only new capability; no new dep |
| 6 | `Cargo.lock` | Regenerate and stage with the manifest change | `tests/qa_migration.rs:167-186` requires lock blob in sync |
| 7 | `src/gallery/sections/overlays/mod.rs` + new `floating.rs` | `mod floating;` + `floating::register(...)` line | Gallery's two-line section hook (`src/gallery/registry.rs:4-8`) |
| 8 | `locales/en.yml`, `locales/zh-CN.yml` | `floating_close` key in both | Lockstep rule `tests/qa_localization.rs:188-198` |

### Removal Recipe

Mirror of `src/gallery/mod.rs:2-7` style:

1. Delete `src/gallery/sections/overlays/floating.rs` and its two hook lines
   in `src/gallery/sections/overlays/mod.rs` (`mod floating;` and the
   `floating::register` call).
2. Delete the `floating_close` key from `locales/en.yml` and
   `locales/zh-CN.yml`.
3. Delete `src/ui/floating/` entirely.
4. Revert `src/ui/mod.rs` to `pub mod widgets;` only.
5. Remove `crate::ui::floating::init(cx);` from `src/app/init.rs`.
6. Remove the `floating_host` field and constructor line from
   `src/shell/root/app_root/state.rs` and the
   `.child(self.floating_host.clone())` line from
   `src/shell/root/app_root/render.rs`.
7. Revert the `"NSScreen"` feature in `Cargo.toml`'s macOS
   `objc2-app-kit` entry and regenerate `Cargo.lock`.
8. Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-features`,
   `cargo test --workspace --all-features`; open floats cannot exist anymore,
   so no runtime cleanup is needed.

## Implementation Phases

## Phase 1: Spec and pure geometry

### Step 1.1: Types in `spec.rs`

Files: `src/ui/floating/spec.rs` (new).
Build: `FloatingWindowId`, `FloatLevel`, `Placement9`, `FloatAnchor`,
`FloatingSpec` with private fields and the builder methods from the contract
(fluent, consume-self, per coding-guide naming; no `set_`).
`Default` not implemented (id+size are mandatory identity).
Doc comments on `FloatLevel` variants state the platform matrix from
Research Digest with the MAC/WIN/X11 citations.

Tests: none yet (pure data).
Done-check: `cargo check -p gpui-starter` compiles with the new module wired
into a stub `mod.rs` (`pub mod spec;` + re-export).

### Step 1.2: `resolve_bounds` geometry

Files: `src/ui/floating/spec.rs`.
Build: `pub(crate) fn resolve_bounds(
    anchor_rect: Bounds<Pixels>,
    clamp_rect: Bounds<Pixels>,
    size: Size<Pixels>,
    placement: Placement9,
    offset: Point<Pixels>,
) -> Bounds<Pixels>`: places the float's box at the given edge/center of
`anchor_rect` (offset applied after placement), then clamps origin so the box
stays inside `clamp_rect` (visible display bounds); when the box is larger
than `clamp_rect` it is centered in `clamp_rect`. Pure; no gpui context.
Also `pub(crate) fn anchor_rect(anchor: &FloatAnchor, cx: &App) -> Option<(Bounds<Pixels>, Bounds<Pixels>)>`
returning (anchor_rect, clamp_rect): Window variant reads
`window_bounds().get_bounds()` (`GPUI/src/platform.rs:2594-2602`) via
`AnyWindowHandle::update` (`:7407-7414`), clamp = the display containing the
anchor rect's center (`cx.displays()`/`visible_bounds()`,
`GPUI/src/app.rs:1487`, `platform.rs:566-593`); Screen variant reads the
given/primary display's `visible_bounds()` and uses it as both. `None` when
the anchor window is closed or no display resolves.

Tests: `spec.test.rs`: 9 placements at a 100x100 anchor inside a 1000x1000
clamp; offset pushes; clamping at each edge; oversized-box centering; window
anchor with a stubbed display list. Plain `#[test]` for `resolve_bounds`
(no gpui context), `#[gpui_kit::test]` for `anchor_rect`.
Done-check: `cargo test -p gpui-starter --lib floating::spec` green.

### Step 1.3: Module mount

Files: `src/ui/floating/mod.rs` (new), `src/ui/mod.rs`.
Build: `mod.rs` with `mod shell; mod overlay_host; pub mod probe; pub mod
registry; pub mod spec; pub mod spawn;` re-exports of the contract types
(`pub use registry::FloatingWindowRegistry;` included for plan 09),
`mod.rs` doc comment naming removal seams; `src/ui/mod.rs` gains
`pub mod floating;` above `pub mod widgets;`.
Done-check: `grep -n "pub mod floating" src/ui/mod.rs` returns the line;
`cargo check -p gpui-starter` passes with empty stub files for the not-yet-built
modules.

## Phase 2: Registry

### Step 2.1: Registry global

Files: `src/ui/floating/registry.rs` (new).
Build:
- `pub(crate) struct RegistryEntry { spec: FloatingSpec, bounds: Bounds<Pixels>, handle: FloatingHandle, build: ViewFactory }`
  where `type ViewFactory = Rc<dyn Fn(&mut Window, &mut App) -> AnyView>`.
- `#[derive(Default)] pub struct FloatingWindowRegistry { entries: BTreeMap<FloatingWindowId, RegistryEntry> }`
  `impl Global`.
- `pub(crate) fn set_entry(entry: RegistryEntry, cx: &mut App)` /
  `remove_entry(id, cx) -> Option<RegistryEntry>`: both mutate via
  `cx.default_global::<FloatingWindowRegistry>()` (notify-on-write,
  `GPUI/src/app.rs:2294-2302`), mirroring `capabilities::set`
  (`src/runtime/capabilities.rs:62-79`).
- `pub fn floating_snapshot(cx: &App) -> Vec<FloatingWindowEntry>` mapping to
  the contract's entry type (clone-out).
- `pub(crate) fn install_close_tracker(cx: &mut App)`:
  `cx.on_window_closed(|cx, window_id| { remove entries whose
  `handle == Native(h) && h.window_id() == window_id` }).detach()`: pattern
  `src/app/init.rs:265-267`.

Tests: `registry.test.rs` (`#[gpui_kit::test]`): set then snapshot contains
entry; remove then snapshot empty; default_global mutation triggers
`cx.observe_global::<FloatingWindowRegistry>` (flag set in observer).
Done-check: `cargo test -p gpui-starter --lib floating::registry` green.

## Phase 3: Native windows

### Step 3.1: `FloatingShell`

Files: `src/ui/floating/shell.rs` (new).
Build:
- `pub(crate) struct FloatingShell { id: FloatingWindowId, title: SharedString, closable: bool, content: AnyView, focus_handle: FocusHandle }`
  implementing `Render` + `Focusable` (`fn focus_handle(&self, _: &App) ->
  FocusHandle`, `GPUI/src/window.rs:710-713`).
- Render: `v_flex().size_full().key_context("floating").track_focus(...)` with
  a 32px header row (`h_flex`): `text_sm` muted title, flex spacer, icon close
  `Button` (`IconName::Close`, verified in the default 106 the app registers) with
  `.accessibility_label(crate::i18n::localize("floating_close"))`,
  dispatching `DismissFloating`; below it the content pane. Background
  `cx.theme().popover`, hairline `border_1().border_color(cx.theme().border)`,
  `rounded(cx.theme().radius)`: one themed popover surface per design guides
  ("Surfaces and elevation": all surfaces of a kind share one treatment).
- Close-button id: `Button::new(format!("floating-close-{}", self.id.0))` —
  a `String` converts to `ElementId` (`From<String>`,
  `GPUI/src/window.rs:7538`); the `(&'static str, SharedString)` tuple form
  has no `From` impl (`:7514-7582` lists them all), so do not use it.
  Domain-derived and unique per float, per the stable-identity rule.
- `.on_action(cx.listener(|this, _: &DismissFloating, window, _| {
  window.remove_window(); }))` (`GPUI/src/window.rs:2324`).
- Drag: header `on_mouse_down` (left button) calls
  `window.start_window_move()` only when `cfg(not(target_os = "macos"))`
  (macOS keeps native titlebar drag via full-size content view; Windows
  backend's `start_window_move` is a no-op trait default,
  `GPUI/src/platform.rs:1272`).
Tests: none separate (covered via spawn tests).
Done-check: `cargo check -p gpui-starter` passes; close button id contains
the float id (grep in shell.rs).

### Step 3.2: Native spawn path + `init`

Files: `src/ui/floating/spawn.rs` (new), `src/ui/floating/mod.rs`.
Build:
- `pub fn spawn_floating(spec, build, cx)` (`build: impl Fn(&mut Window, &mut App) -> AnyView + 'static`,
  `Fn` not `FnOnce` because the factory is retained for respawn): native branch:
  1. `let Some((anchor_rect, clamp_rect)) = anchor_rect(&spec.anchor(), cx)`
     else return `Err(anyhow!("floating anchor unavailable"))`.
  2. `let bounds = resolve_bounds(...)` with `spec.size()`.
  3. `WindowOptions`:
     `window_bounds: Some(WindowBounds::Windowed(bounds))`,
     `titlebar: Some(TitlebarOptions { title: Some(spec.title()), appears_transparent: true, traffic_light_position: None })`,
     `kind: match spec.level() { FloatLevel::Floating => WindowKind::Floating, FloatLevel::AlwaysOnTop => WindowKind::PopUp }`,
     `focus: spec.focus()`, `is_resizable: spec.resizable() && level != AlwaysOnTop`
     (PopUp on Windows has style 0, `WIN/src/window.rs:491-493`),
     `is_minimizable: false`, `is_movable: spec.movable()`,
     `window_min_size: spec.min_size()`,
     `app_id: Some("gpui-starter".into())`, rest `..Default::default()`.
  4. `gpui_kit::open_window(options, cx, |window, cx| { let view = build(window, cx); cx.new(|cx| FloatingShell::new(spec.id(), spec.title(), view, window, cx)) })`
     (`KIT/src/lib.rs:149-164`); the kit wraps the shell in `base::Root`, so
     the returned `Entity<FloatingShell>` is the app content and the window's
     `AnyWindowHandle` is the first tuple element.
  5. Wrap the returned `AnyWindowHandle` in `FloatingHandle::Native`, build
     `RegistryEntry` with the view factory as `Rc::new(build)`,
     `set_entry(...)`, return the handle.
- `pub fn close_floating(handle, cx)`: Native: `h.update(cx, |_, window, _|
  window.remove_window()).is_ok()` (registry cleanup happens in the close
  tracker); Overlay: `remove_entry` returns Some → true.
- `pub fn close_all_floating(cx)`: snapshot handles, close each, return count.
- `pub fn respawn_floating(id, cx) -> bool`: take entry, rerun
  `spawn_floating(entry.spec, entry.build clone, cx)`; on success remove old
  entry first (same id) so no duplicate; return false when id unknown.
- `pub fn init(cx: &mut App)`: `install_close_tracker(cx)`;
  `cx.bind_keys([KeyBinding::new("escape", DismissFloating, Some("floating"))])`
  (`GPUI/src/app.rs:2486`, `keymap/binding.rs:31-33`);
  capability registration (Phase 5 wires the probe; Phase 3 registers
  `floating_windows` as `supported_enabled` on native only; the wasm-correct
  call arrives with the probe phase, so `init` delegates to
  `probe::register_capabilities(cx)` guarded by a Phase-3 stub that is
  native-supported_enabled / wasm-degraded).
Tests: none here.
Done-check: `cargo check -p gpui-starter` passes.

### Step 3.3: Headless lifecycle tests

Files: `src/ui/floating/spawn.test.rs` (new).
Build (`#[gpui_kit::test]`, test platform opens windows,
`GPUI/src/platform/test/platform.rs:493-506`):
- spawn anchored `Screen { display: None, Center, .. }` → snapshot len 1,
  handle is `Native`, window title matches;
- spawn `Window`-anchored to a window opened in the test;
- `close_floating` on the native handle: assert the returned bool is true,
  then a second `close_floating` on the same handle returns false. App
  effects (including the close tracker) run at each `cx.update` boundary:
  `App::flush_effects` is called at the end of every update
  (`GPUI/src/app.rs:1874-1877`), same boundary the config-store tests rely
  on (`src/state/config_store.test.rs:280-287`). Registry removal is also
  asserted directly via `remove_entry` in `registry.test.rs`, so a
  test-platform quirk in close-observer ordering cannot fail this step
  silently;
- `close_all_floating` returns prior count.
Done-check: `cargo test -p gpui-starter --lib floating::spawn` green on the
dev host.

### Step 3.4: Boot seam

Files: `src/app/init.rs`.
Build: one line after `crate::notifications::initialize(cx);`:
`crate::ui::floating::init(cx);`.
Tests: none (existing boot tests cover init not panicking;
`tests/e2e_lifecycle.rs` smoke).
Done-check: `grep -n "floating::init" src/app/init.rs` shows the line;
`cargo test -p gpui-starter --lib` green.

## Phase 4: Overlay fallback (wasm parity)

### Step 4.1: `OverlayFloatHost`

Files: `src/ui/floating/overlay_host.rs` (new).
Build:
- `pub struct OverlayFloatHost { focus_handle: FocusHandle }` `Render` +
  `Focusable`; subscribes
  `cx.observe_global::<FloatingWindowRegistry>(|_, cx| cx.notify())` in
  `new`.
- Render: for every snapshot entry with `FloatingHandle::Overlay(id)`, a pane
  `div().id(SharedString::from(format!("floating-pane-{}", id.0)))
  .absolute().top(px(bounds.origin.y)).left(px(bounds.origin.x))
  .w(px(bounds.size.width)).h(px(bounds.size.height))`. Pane coordinates are
  main-window space because the overlay `resolve_bounds` call in Step 4.2
  subtracts the root window origin, and the host's own box coincides with the
  window (below). Pane chrome identical to `FloatingShell` (same tokens),
  `.key_context("floating")`, same close button dispatching `DismissFloating`;
  `on_action` removes the registry entry.
- Host root MUST be out of flow: `div().absolute().top_0().left_0().size_full()`.
  Two reasons, both verified: (a) an in-flow `size_full()` child appended to
  the AppRoot column joins flex layout with default `flex_shrink: 1.0`
  (`GPUI/src/style.rs:802`) and steals height from `content_area`'s
  `flex_1()` (`src/shell/root/app_root/render.rs:267`, with `content_v` at `:139`), collapsing the
  native layout; (b) absolute panes resolve against the closest positioned
  ancestor (`GPUI/src/style.rs:1241-1243`; every element defaults to
  `Position::Relative`, `:1233-1240`), so panes must sit inside a box that
  equals the window. With `absolute().top_0().left_0().size_full()` the
  host's offsets resolve against the root `v_flex` (window origin, full
  size, `src/shell/root/app_root/render.rs:287-291`), making the host box
  the window box and pane offsets window coordinates. `top_0()`/`left_0()`
  exist via the macro's numeric `"0"` suffix
  (`gpui-pre-macros-0.3.8/src/styles.rs:925-933`). When no overlay entries
  exist the host renders no children: it paints nothing, inserts no hitbox
  (plain childless `div`, no `.id()`), and adds no a11y node on native
  platforms.
Tests: `overlay_host.test.rs` (`#[gpui_kit::test]`): insert a registry
entry with Overlay handle + fixed bounds via a test-only
`pub(crate) fn insert_overlay_for_test(entry, cx)` (plain `set_entry` with
Overlay handle, no cfg); mount the host twice in a test window, once alone
and once as the last child of `v_flex().size_full()` that already carries a
fixed-height sibling (mimicking the AppRoot column), and assert both mounts
render the same pane count. This keeps the real tree shape in the test;
geometric proof that panes sit at window coordinates needs element-measure
tooling this repo does not enable (kit `test-support`, survey Open
Questions), so placement is verified by manual QA step 10.
Pure-cross-platform (no wasm cfg needed to test).
Done-check: `cargo test -p gpui-starter --lib floating::overlay` green.

### Step 4.2: Wasm routing in `spawn.rs`

Files: `src/ui/floating/spawn.rs`.
Build: in `spawn_floating`, `#[cfg(target_family = "wasm")]` branch: read
`root_window(cx)` (`crate::app::window::root_window`,
`src/app/window.rs:20-22`); when `None`, return
`Err(anyhow!("no main window to host overlay float"))` (the contract's one
wasm error). Otherwise resolve bounds against it as anchor-space: compute
screen bounds, then store
`bounds - root_origin` (window space for the host); build the content view via
`root.update(cx, |_, window, cx| build(window, cx))` so the `AnyView` is
created in the root window's context; register entry with
`FloatingHandle::Overlay(id)`. `#[cfg(not(target_family = "wasm"))]` keeps the
native branch; shared helpers (registry writes, snapshot) uncfg'd.
`close_floating` Overlay arm and `respawn_floating` overlay arm (recompute +
`set_entry` new bounds, no rebuild) live here, uncfg'd where possible.
Tests: compile-only on native hosts; behavior covered by 4.1 host tests.
Done-check: `just wasm-check` passes after Phase 7 deps are in place (run at
Step 7.2; interim `cargo check -p gpui-starter` stays green).

### Step 4.3: AppRoot mount

Files: `src/shell/root/app_root/state.rs`, `src/shell/root/app_root/render.rs`.
Build: `pub(crate) floating_host: Entity<crate::ui::floating::OverlayFloatHost>`
field; `let floating_host = cx.new(|cx| OverlayFloatHost::new(window, cx));`
in `AppRoot::new`; render appends `.child(self.floating_host.clone())` as the
final child of the root `v_flex` (`render.rs:287-291`) so panes paint last
(gpui has no z_index; stacking is tree order). Because the host root is
`absolute().top_0().left_0().size_full()` (Step 4.1), appending it does not
join the column's flex layout and cannot perturb `content_area`'s `flex_1()`
(`render.rs:267`).
Tests: existing app-root/a11y suites must stay green; panes absent on native
(empty host renders nothing).
Done-check: `cargo test -p gpui-starter --lib` green;
`tests/qa_a11y.rs` unaffected (no new always-present nodes).

## Phase 5: Probes and capabilities

### Step 5.1: macOS notch probe

Files: `src/ui/floating/macos_probe.rs` (new), `Cargo.toml`, `Cargo.lock`.
Build:
- `Cargo.toml` macOS table: `objc2-app-kit` features `["std", "NSView", "NSWindow", "NSScreen"]`.
- `pub(crate) fn notch_geometry() -> Option<NotchGeometry>`: requires
  `MainThreadMarker::new()` (`objc2-0.6.4/src/main_thread_marker.rs:230`);
  `NSScreen::mainScreen(mtm)`; `safeAreaInsets()` and
  `auxiliaryTopLeftArea()`/`auxiliaryTopRightArea()`
  (`objc2-app-kit-0.3.2/src/generated/NSScreen.rs:33-117`); camera housing
  present when the two auxiliary rects do not span `frame().size.width`;
  housing is the gap between them. Convert NSEdgeInsets and NSRect (points ==
  logical pixels) to `Edges<Pixels>`/`Bounds<Pixels>`.
Tests: none runnable off macOS; the macos file is cfg'd out on other hosts
(module gate, Platform & Wasm Gating).
Done-check: `grep -n "NSScreen" Cargo.toml` shows the feature;
`cargo check -p gpui-starter` on the dev host passes.

### Step 5.2: `probe.rs` + capability registration

Files: `src/ui/floating/probe.rs` (new), `src/ui/floating/mod.rs`.
Build:
- `pub(crate) fn menu_bar_height(cx: &App) -> Option<Pixels>`: primary
  display `visible_bounds().origin.y - bounds().origin.y` delta, gpui-native,
  no objc (`MAC/src/display.rs:121-129` for the macOS source of both rects).
- `pub fn probe(cx: &App) -> FloatingPlatformInfo`:
  - native: `native_windows: true`, `always_on_top: true` (PopUp exists on all
    native platforms per matrix), `non_focus_stealing: true` on macOS/Windows,
    `false` on Linux (flag ignored, Research Digest),
    `menu_bar: menu_bar_height(cx)` (Some on all native: top-gap formula; 0
    when no menu bar), `notch: macos_probe::notch_geometry()` (cfg macos,
    else None).
  - wasm: `native_windows: false`, `always_on_top: false`,
    `non_focus_stealing: false`, `menu_bar: None`, `notch: None`.
- `pub(crate) fn register_capabilities(cx: &mut App)` consumed by `init`:
  - native: `floating_windows` supported_enabled;
    `floating_always_on_top` supported_enabled;
    `notch_geometry`: macOS supported_enabled; other native
    `{ supported: false, enabled: false, degraded: false,
    reason: Some("macOS only"), last_error: None }`.
  - wasm: `floating_windows` `{ supported: false, enabled: true, degraded: true,
    reason: Some("native floating windows unavailable on wasm; in-window overlay fallback"),
    last_error: None }`;
    `floating_always_on_top` `{ supported: false, enabled: false, degraded: false,
    reason: Some("overlay floats have no OS-level topmost"), last_error: None }`;
    `notch_geometry` unsupported as above.
  Every literal lists all six fields: `CapabilityStatus` has six pub fields
  (`supported, enabled, degraded, reason, last_error`,
  `src/runtime/capabilities.rs:6-13`) and no `Default`, so four-field
  literals do not compile. The existing helpers `supported_enabled()` /
  `degraded()` / `error()` (`capabilities.rs:16-51`) cover every shape except
  supported-false-plus-enabled, hence the struct literals. Reason strings are
  English literals, matching existing capability reasons
  (`src/platform/input/shortcuts.rs:290-305` precedent).
Tests: `probe.test.rs`: on any host: `probe(cx).native_windows ==
  !cfg(wasm)`; capability names present in `crate::capabilities::snapshot`
  after `init` (run inside `#[gpui_kit::test]`).
Done-check: `cargo test -p gpui-starter --lib floating::probe` green;
diagnostics page shows the three rows (manual).

### Step 5.3: Probe tolerance

Files: `src/ui/floating/probe.rs`.
Build: `notch_geometry()` returns None when `MainThreadMarker::new()` is None
(possible in tests / off-main-thread callers) or `mainScreen` is None;
`TestWindow` handle unsupported (`GPUI/src/platform/test/window.rs:64-71`)
never reached because probe does not touch window handles.
Tests: call `probe(cx)` inside `#[gpui_kit::test]`; assert no panic and
`notch.is_none()` off macOS.
Done-check: covered by 5.2 test run.

## Phase 6: Gallery demo section

### Step 6.1: Section

Files: `src/gallery/sections/overlays/floating.rs` (new),
`src/gallery/sections/overlays/mod.rs`.
Build:
- `FloatingSection` entity + `FloatingSection::view(window, cx)` +
  `pub fn register(sections, window, cx)` pushing
  `GallerySection::new("floating", "Floating Windows", "Child utility windows anchored to the window or screen; overlay fallback on web.", ...)`
  (literal strings follow the gallery precedent, `sheet.rs:451-456`).
- Buttons (kit `Button`, default/ghost per local hierarchy): "Anchor top
  right" (`FloatAnchor::Window` + `root_window`), "Screen center"
  (`FloatAnchor::Screen`), "Always on top" (level toggle), "Close all",
  each spawning a small `AnyView` (a `div` with `text_sm` muted placeholder
  and the spec id). Status line: `floating_snapshot(cx).len()` open count and
  `probe(cx)` facts. Icons `IconName::WindowRestore` / `PanelLeft` /
  `Maximize` — all verified in the default 106 (`gpui-kit-assets-0.7.1/
  default-icons.txt`); the app's native asset source embeds only that subset,
  so full-catalog names like `AppWindow`/`Layers`/`Monitor` would compile via
  the assets enum but render blank (wasm's CDN source is the only path that
  serves them).
- Uses `crate::ui::floating::*` public API only (dogfoods the contract).
Tests: none new (manual QA surface; gallery compiles on wasm which is the
point).
Done-check: `cargo check -p gpui-starter` passes; section appears in the
gallery overlays group.

## Phase 7: i18n, lock, gates

### Step 7.1: Locale keys

Files: `locales/en.yml`, `locales/zh-CN.yml`.
Build: `floating_close: "Close"` / `floating_close: "关闭"`: quoted scalars,
top-level snake_case (parser rules `tests/qa_localization.rs:20-71`).
Done-check: `cargo test -p gpui-starter --test qa_localization` green.

### Step 7.2: Lock regeneration and full gates

Files: `Cargo.lock` (regenerated), none else.
Build/commands, in order:
1. Run `cargo check -p gpui-starter` once so the Cargo.toml feature edit from
   Step 5.1 rewrites `Cargo.lock`; stage the lock together with the manifest
   change (rule: `tests/qa_migration.rs:167-186`).
2. `cargo fmt --all -- --check`
3. `cargo clippy --workspace --all-features`
4. `cargo test --workspace --all-features`
5. `just wasm-check`
Done-check: all five pass; `git diff --stat` shows only the files in
Seam Touches.

## Platform & Wasm Gating

cfg gates:

- `src/ui/floating/macos_probe.rs`: module-gated
  `#[cfg(target_os = "macos")] mod macos_probe;` in `mod.rs`
  (module-gate style of `src/platform/mod.rs:2-3`); `probe.rs` holds a
  `#[cfg(not(target_os = "macos"))]` fallback returning `None`.
- `spawn.rs`: native branch `#[cfg(not(target_family = "wasm"))]`, overlay
  branch `#[cfg(target_family = "wasm")]`; shared registry code uncfg'd so
  tests exercise it everywhere.
- No `cfg` in `AppRoot`: the host exists on all platforms and renders
  nothing when no overlay entries exist.

Capability-registry entries (written by `floating::init`, rendered by the
diagnostics page):

| Key | macOS | Windows | Linux | wasm |
| --- | --- | --- | --- | --- |
| `floating_windows` | supported_enabled | supported_enabled | supported_enabled | supported:false, enabled, degraded, overlay reason |
| `floating_always_on_top` | supported_enabled | supported_enabled | supported_enabled | unsupported, reason "no OS-level topmost" |
| `notch_geometry` | supported_enabled | unsupported, "macOS only" | unsupported, "macOS only" | unsupported, "macOS only" |

Native-only deps: none new. The only manifest change adds a feature to the
existing macOS-table `objc2-app-kit = 0.3.2` entry
(`Cargo.toml:243-246`); nothing joins the
`[target.'cfg(not(target_family = "wasm"))'.dependencies]` table
(`Cargo.toml:310-353`). The macOS table is inherently non-wasm.

Platform behavior matrix (what each platform gets, including hidden):

| Behavior | macOS | Windows | X11 | Wayland | wasm |
| --- | --- | --- | --- | --- | --- |
| Window opens above app | NSFloating panel | normal window (not topmost) | transient-for | xdg parent | n/a (overlay) |
| AlwaysOnTop | NSPopUp + all spaces | WS_EX_TOPMOST tool window | NOTIFICATION type (WM-dependent) | parent + level via kind | n/a |
| Borderless look | appears_transparent titlebar | hide_title_bar | WM decorations (Server default) | compositor CSD | always (pane) |
| Non-focus-stealing | focus:false → orderFront; PopUp non-activating | SW_SHOWNOACTIVATE | not honored by gpui-pre (documented best-effort) | same | n/a (same window) |
| User drag | native titlebar strip | not available (no-op `start_window_move`, no caption on PopUp) | `start_window_move` | `start_window_move` | pane fixed |
| Resizable | resizable flag honored | honored for Floating; ignored for PopUp (style 0) | honored | honored | n/a |
| Reposition | close+reopen (no API) | same | same | same | registry bounds update |

## i18n

Crate facade `crate::i18n::localize` (`src/services/i18n.rs:3-5`); keys must
land in both catalogs (`tests/qa_localization.rs:188-198`).

| Key | en | zh-CN |
| --- | --- | --- |
| `floating_close` | "Close" | "关闭" |

Gallery section strings stay literal English, matching every existing gallery
section (`src/gallery/sections/overlays/sheet.rs:451-456`). Capability reason
strings are English literals (precedent `src/platform/input/shortcuts.rs:290-305`).

## Design Compliance

- Surfaces: floats use the popup family treatment (`cx.theme().popover`
  background, `border` hairline, theme radius) per "Surfaces and elevation"
  ("all surfaces of the same kind should share one treatment"); no raw colors
  or `px`-based radii anywhere in shell/host.
- Elevation: overlay panes carry the shadow treatment of the popover family;
  the OS-window floats rely on the OS shadow and draw only the hairline:
  "use elevation to explain stacking, not importance".
- Density: header strip is a compact control row (32px, `text_sm`, muted
  foreground title), "compact for toolbars and repeated professional data";
  content padding is the caller's, the shell adds none beyond the header.
- Spacing: header internals use `gap_1()`/`gap_2()` scale helpers; no literal
  spacing tokens ("Tokens before values"; rem-based helpers per
  "Zoom, base font, and rem").
- Interaction states: close button is a kit `Button` (ghost, icon) with
  hover/pressed/focus-visible from the component; icon-only control carries
  an accessible name (`floating_close`), "icon-only buttons require a
  tooltip and an accessible name" (name delivered via accessibility_label).
- Keyboard: Esc dismisses through a bound action scoped to the
  `floating` key context ("every action is reachable and operable by
  keyboard"); focus is tracked in the shell and overlay panes so focus is
  visible and restored when the surface closes ("focus remains visible and
  restored after overlays").
- Interface language: the only control label is "Close": a result verb,
  sentence case, no ellipsis (immediate action) per "Buttons and confirmation
  dialogs".
- Motion: none added; window appearance animation is the OS utility-window
  behavior on macOS PopUp (system-owned), honoring "Motion explains change".
- Capability/degradation states are visible on the diagnostics page with
  reasons, "State must be visible" and coding-guide "platform and capability
  boundaries" (semantic contract preserved across platforms: spawn always
  succeeds on wasm via overlay).

## Verification

Repo gates (AGENTS.md; run in Step 7.2 exactly as named):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

`just wasm-check` is the wasm proof: lib-only nightly check
(`justfile:24-25`) compiles the overlay branch and the wasm capability path.

Feature-specific manual QA (macOS dev flow `bash scripts/macos-dev-app.sh`,
CONTRIBUTING.md:32-38):

1. Gallery → Overlays → "Floating Windows".
2. "Anchor top right": a borderless float opens flush with the main window's
   top-right corner minus offsets; main window keeps focus (no key steal).
3. Drag the float by its header strip: window moves (macOS native strip).
4. Esc with the float focused closes it; snapshot count on the page drops.
5. "Always on top": float stays above another app's full-screen-normal
   window (all spaces on macOS); resizing disabled.
6. "Screen center": float centers on the primary display, below the menu bar
   (clamped to `visible_bounds`).
7. "Close all" closes every open float; count returns to 0.
8. Diagnostics page: `floating_windows` supported/enabled;
   `notch_geometry` supported on a notched Mac (values nonzero), unsupported
   with reason elsewhere.
9. Notch QA (macOS, notched display): probe reports `safe_area.top > 0` and
   camera-housing bounds; a `Screen { TopCenter }` spawn does not overlap the
   menu bar.
10. Wasm (`just wasm-build` + existing web dev flow): same gallery section
    opens in-window overlay panes at the same anchor positions; Esc and Close
    work; `floating_windows` shows degraded with the overlay reason.

Acceptance criteria:

- All five gates pass with only the Seam Touches files changed.
- Every public item in the Interface Contract exists with the exact
  signatures (mechanically checkable via `cargo doc`/grep).
- `floating_snapshot` round-trips id/level/bounds for both handle kinds.
- No `use gpui::` / `gpui_component::` imports added
  (`tests/qa_migration.rs:386-405` passes).
- Locale catalogs stay in lockstep (`tests/qa_localization.rs` passes).

## Risks & Open Questions

Ranked:

1. **X11/Wayland always-on-top is WM-dependent.** `_NET_WM_WINDOW_TYPE_NOTIFICATION`
   placement is compositor policy; some WMs will not keep the float above.
   Tried: read `X11/x11/window.rs:614-627`: no `_NET_WM_STATE_ABOVE` is ever
   set by gpui-pre 0.3.8 and there is no public API to set it. Mitigation:
   capability entry stays "supported" (window opens), gallery demo and probe
   facts document the caveat; manual QA on Linux is best-effort.
2. **Windows PopUp windows cannot be dragged or resized** (style 0, no
   caption; `start_window_move` is a no-op, `GPUI/src/platform.rs:1272`).
   Consumers needing movable topmost windows on Windows must use
   `FloatLevel::Floating` (not topmost). Verified from source; no Windows
   host available in this session to confirm at runtime.
3. **Linux ignores `focus: false`.** Verified by grep over both Linux
   backends this session; non-focus-stealing is macOS/Windows only. Probe
   reports `non_focus_stealing: false` there so consumers can adapt.
4. **Consumer contract drift.** Plans 03/04/09/10 exist on disk and cite this
   contract's items, several by line number into this file (`02:280-281`
   style). Tried: grep of all four consumer docs for every contract symbol
   (this session); all referenced symbols (`FloatingWindowId` tuple field
   `.0`, `FloatingSpec` builder set, `FloatHandle::{Native, Overlay}`,
   `respawn_floating`, `FloatingWindowRegistry` path, `probe().native_windows`,
   `DismissFloating`) are present and unchanged in this revision. Their line
   pointers drift when this file is edited; the signatures are the contract,
   pointers are informational. Any future change to a contract signature
   requires re-reading the four consumer docs first.
5. **Overlay pane coordinate space.** Overlay bounds are main-window space
   (screen bounds minus root window origin at spawn). If the user moves or
   resizes the main window, panes keep stale positions until
   `respawn_floating`. Native floats have the same one-shot anchoring (no
   move API). Tried: searched gpui for a public move/position API
   (`GPUI/src/platform.rs:1145-1235` method list); none exists in 0.3.8.
   Mitigation: `respawn_floating` is the documented refresh path;
   `observe_window_bounds` could drive it later if a consumer needs it.
6. **`gpui_kit::test` window-close tracker timing.** The `on_window_closed`
   tracker fires through App effects, which flush at each `cx.update`
   boundary (`App::flush_effects`, `GPUI/src/app.rs:1874-1877`). If the test
   platform surfaces window removal asynchronously, Step 3.3's double-close
   assertion plus `registry.test.rs`'s direct `remove_entry` test keep
   coverage honest. Test-authoring risk, not product risk.
7. **objc2-app-kit feature addition rewrites Cargo.lock.** Feature flags are
   additive to the same 0.3.2 crate; no second binding set. Lock must be
   staged with the manifest or `tests/qa_migration.rs:167-186` fails;
   Step 7.2 covers it.
8. **macOS native header-drag on a transparent full-size titlebar is not
   provable from crate source.** `MAC/src/window.rs:1004-1011` shows the
   style mask but hit-testing for drags over the invisible titlebar is
   AppKit behavior. Skipped as a fix (reviewer note, best-effort acceptable):
   the plan already hedges with the `cfg(not(target_os = "macos"))`
   `start_window_move` path, and manual QA step 3 exercises the drag on a
   real Mac; if drag fails there, the fix is enabling
   `app_owns_titlebar_drag` plus a `start_window_move` header strip on macOS
   too (`GPUI/src/platform.rs:2461-2477`, `GPUI/src/window.rs:2888`).
