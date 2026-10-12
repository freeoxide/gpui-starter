# 03 Notch Floater Plan

## Goal

On macOS, a non-activating panel hovers over the notch area of the main
display: a collapsed pill showing a live clock that expands downward on hover
or click into mini-widgets (clock, mock now-playing, mock battery, quick
actions), and hides whenever another app's window goes fullscreen. On Windows,
Linux, and wasm the same gallery entry opens a small inset-anchored demo float
through plan 02's toolkit, and the capability registry states per platform
what works and why not.

## Dependencies

Plan 02 (`docs/plans/02-floating-window-toolkit.md`), implemented first
(`src/ui/floating` does not exist yet on this branch; verified this session).
Items consumed from its Interface Contract (signatures at `02:286-407`):

- `crate::ui::floating::probe(cx) -> FloatingPlatformInfo`, fields `notch:
  Option<NotchGeometry>`, `menu_bar: Option<Pixels>` (`02:371-391`).
- `NotchGeometry { safe_area: Edges<Pixels>, camera_housing:
  Option<Bounds<Pixels>> }` (`02:381-389`).
- `FloatingWindowId(pub SharedString)` (`02:290`); `FloatingSpec::new(id,
  size)` with `.title()`, `.level()`, `.anchor()`, `.focus()` builders
  (`02:318-328`); `FloatAnchor::Screen { display, placement, offset }`
  (`02:310-315`); `Placement9::Top` (`02:303-307`); `FloatLevel::Floating`
  (`02:293-300`).
- `spawn_floating`, `close_floating`, `floating_snapshot` (`02:347-361`) for
  the fallback demo float on non-macOS and wasm.
- Plan 02's manifest seam (adds the `NSScreen` feature to the macOS
  `objc2-app-kit` entry, `02` Seam Touches #5) and its boot hook
  `crate::ui::floating::init(cx);` (`02` Seam Touches #2); 03 will not
  compile until both land. 03's own `macos.rs` also uses `NSScreen`
  (`mainScreen`, `frame`) and depends on that feature.

Not consumed: `FloatingShell` (the notch window needs its own root view and a
transparent background), the overlay host directly (the wasm fallback goes
through `spawn_floating`, which routes to it).

## Research Digest

Verified this session against
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` (short paths:
`MAC` = gpui-pre-macos-0.3.8/src, `GPUI` = gpui-pre-0.3.8/src, `KIT` =
gpui-kit-0.7.1/src, `GC` = gpui-component-0.7.1/src, `AK` =
objc2-app-kit-0.3.2/src/generated, `OF` = objc2-foundation-0.3.2/src, `OCF` =
objc2-core-foundation-0.3.2/src) and repo files. Plan 02 facts 03 relies on
were re-read from its contract; two of its research notes were corrected here
(noted inline).

Window kind and level (macOS):

- `WindowKind::PopUp` creates a `PANEL_CLASS` instance and ORs
  `NSWindowStyleMaskNonactivatingPanel` into the style mask (const defined at
  `MAC/window.rs:94-96`, PopUp arm at `MAC/window.rs:1019-1022`):
  non-activating panel, the style the spec names.
- PopUp windows get `NSPopUpWindowLevel` (= 101, `MAC/window.rs:103`, applied
  at `:1231`), above the menu bar (level 24), so the window can cover the
  notch/menu-bar strip.
- PopUp windows get a mouse tracking area on the content view
  (`MAC/window.rs:1229`). `add_mouse_tracking_area` installs
  `MouseEnteredAndExited | MouseMoved | ActiveAlways | InVisibleRect` with
  the comment "Track even when another application is active so visible
  windows can respond to hover without being focused"
  (`MAC/window.rs:2488-2513`): hover works while the panel is not key.
- PopUp windows get `setCollectionBehavior_(CanJoinAllSpaces |
  FullScreenAuxiliary)` at creation (`MAC/window.rs:1236-1240`).
- `focus: false` + `show: true` → `orderFront_(nil)` without making key
  (`MAC/window.rs:1288-1292`): the panel never steals focus.
- `WindowOptions.window_background: WindowBackgroundAppearance` exists
  (`GPUI/platform.rs:2490`, default `:2627`); the enum carries
  `Opaque | Transparent | Blurred` (`GPUI/platform.rs:2711-2724`). The macOS
  backend applies non-opaque backgrounds in `set_background_appearance`:
  `setOpaque_(NO)` plus a near-zero-alpha background color
  (`MAC/window.rs:1857-1900`).
- `gpui_kit::open_window<V: Render>(options, cx, build) ->
  Result<(AnyWindowHandle, Entity<V>)>` hosts the kit `base::Root`
  (`KIT/lib.rs:149-164`).

Titlebar and style mask (load-bearing for the borderless look):

- `titlebar: None` yields style `titled | fullSizeContentView` with no
  closable/minimizable/resizable bits (`MAC/window.rs:1008-1011`), so no
  traffic-light buttons are created. `TitlebarOptions { appears_transparent:
  true }` would instead add closable+titled and hide nothing by itself
  (`MAC/window.rs:988-1006`).
- A `titled` window still carries system window chrome (rounded corners).
  The reach-in converts the panel to borderless after creation:
  `NSWindow::setStyleMask(&self, NSWindowStyleMask)` (`AK/NSWindow.rs:795`),
  `NSWindowStyleMask::Borderless` is `0` (`AK/NSWindow.rs:27-31`), and
  `NSWindowStyleMask::from_bits_retain(1 << 7)` reconstructs the
  non-activating bit exactly the way gpui does for its own const
  (`MAC/window.rs:95-96`). gpui itself flips an existing window to borderless
  at runtime for simple fullscreen (`MAC/window.rs:644`), so the runtime
  mask change is an in-ecosystem pattern.
- The `NSWindow` class decl is `#[thread_kind = MainThreadOnly]`
  (`AK/NSWindow.rs:473-478`): instance methods are safe on the main thread,
  which is where gpui window updates run. `NSWindow::setHasShadow(&self,
  bool)` exists (`AK/NSWindow.rs:1366`) if the system shadow ever needs
  disabling.

AppKit reach-in (repo precedent + objc2 sources):

- `impl HasWindowHandle for Window` exists (`GPUI/window.rs:7434-7437`); the
  repo already uses it to get the NSView:
  `HasWindowHandle::window_handle(window)` → `handle.as_raw()` → match
  `RawWindowHandle::AppKit(appkit)` → `unsafe {
  appkit.ns_view.cast::<NSView>().as_ref() }`
  (`src/platform/liquid_glass.rs:230-240`, including the comment about the
  inherent `Window::window_handle` shadowing the trait method).
- `NSView::window(&self) -> Option<Retained<NSWindow>>` is a safe generated
  method gated on the `NSWindow` crate feature, already enabled
  (`AK/NSView.rs:213-216`; features at `Cargo.toml:243-247`).
- `NSWindow::setCollectionBehavior(&self, NSWindowCollectionBehavior)`
  (`AK/NSWindow.rs:1446`); the behavior type is bitflags with
  `CanJoinAllSpaces = 1<<0` and `FullScreenAuxiliary = 1<<8`
  (`AK/NSWindow.rs:147-171`).
- `NSWindow::setFrame_display_animate(&self, frame_rect: NSRect, display_flag:
  bool, animate_flag: bool)` (`AK/NSWindow.rs:865-876`); its doc states it is
  equivalent to `setFrame:display:` when the animate flag is NO.
- NSScreen (via plan 02's `NSScreen` feature): `mainScreen(MainThreadMarker)
  -> Option<Retained<NSScreen>>` (`AK/NSScreen.rs:37-39`), `frame() -> NSRect`
  (`:54-56`), `safeAreaInsets() -> NSEdgeInsets` (`:107-109`),
  `auxiliaryTopLeftArea()`/`auxiliaryTopRightArea()` (`:111-117`). The
  feature pulls `objc2-foundation/NSArray`, `/NSGeometry`,
  `/objc2-core-foundation` and more (`objc2-app-kit-0.3.2/Cargo.toml:1232-1240`).
- `NSEdgeInsets { top, left, bottom, right: CGFloat }`
  (`OF/generated/NSGeometry.rs:37-44`). `NSPoint`/`NSSize`/`NSRect` are
  aliases of `CGPoint`/`CGSize`/`CGRect` re-exported from `objc2_foundation`
  (`OF/src/geometry.rs:12-33`, re-export at `OF/src/lib.rs:195`);
  `CGRect::new(origin, size)` is a const constructor (`OCF/src/geometry.rs:225-241`).
- `MainThreadMarker::new() -> Option<Self>` is safe
  (`objc2-0.6.4/src/main_thread_marker.rs:230`).

Geometry and lifecycle (corrects a plan 02 research note):

- gpui reports window bounds top-left-origin: `MacWindowState::bounds()` flips
  AppKit y (`MAC/window.rs:909-928`).
- Creation: `initWithContentRect` receives a rect whose y omits the height
  term (`MAC/window.rs:1056-1067`, call at `:1069-1075`), but the immediately
  following `setFrameTopLeftPoint_(native_window, window_rect.origin)`
  (`MAC/window.rs:1296-1300`) re-positions using that same point as a
  TOP-LEFT point, which lands the window at the top-left-origin
  interpretation. Net effect: `WindowBounds::Windowed` origins are honored
  as top-left after creation. Plan 02's "creation and readback disagree by
  one window height" note overstates it; the discrepancy is the transient
  init rect, not the final position. 03 still pins the frame after open
  because it must also switch the style mask and collection behavior.
- `Window::resize()` routes to `setContentSize:` through a foreground spawn
  and detaches (`MAC/window.rs:1440-1453`), pinning the AppKit bottom-left
  origin and racing any same-frame origin fix. 03 resizes only through
  `setFrame_display_animate` (atomic origin+size), never `Window::resize`.
- Resizes driven through NSWindow land back in gpui: the content view's
  `setFrameSize:` override is registered (`MAC/window.rs:227-230`) and the
  `windowDidResize:` observer is wired (`MAC/window.rs:440-443`).
- `Window::remove_window()` (`GPUI/window.rs:2324-2328`);
  `App::on_window_closed(handler) -> Subscription` (`GPUI/app.rs:2631-2640`;
  detached-observer precedent `src/app/init.rs:265-267`).
- Window-update re-entrancy: event listeners run inside
  `app.update_window` (`window.dispatch_event` is invoked from within it,
  `GPUI/app/test_app.rs:463-464`), and `update_window_erased` removes the
  window from the map for the duration (`GPUI/app.rs:3020-3045`). A
  `handle.update` on the window whose listener is currently firing
  therefore returns `Err("window not found")` (`GPUI/app.rs:2095`) rather
  than panicking or running. Consequence: close-from-inside must use the
  listener's `&mut Window` (`remove_window`); close-from-outside (gallery
  toggle) goes through `handle.update`.
- `App::reduce_motion()` (`GPUI/app.rs:1222-1224`) gates the animate flag.
- `Window::set_window_title(&str)` (`GPUI/window.rs:2924`) sets the title on
  a `titlebar: None` window after open (Mission Control / a11y name).
- `cx.set_global` pushes `Effect::NotifyGlobalObservers` before insert
  (`GPUI/app.rs:2305-2310`), so `cx.observe_global` fires on every write;
  `try_global` (`GPUI/app.rs:2275-2279`), `observe_global`
  (`GPUI/app.rs:2330-2343`).

View APIs:

- `on_hover` is a `StatefulInteractiveElement` method (trait at
  `GPUI/elements/div.rs:1305`, fluent method at `:1661-1669`, listener
  `Fn(&bool, &mut Window, &mut App) + 'static`): the root div needs `.id(...)`.
  `on_mouse_down` is on `Interactivity`/`InteractiveElement`
  (`GPUI/elements/div.rs:123-131`).
- `Context::spawn` takes `AsyncFnOnce(WeakEntity<T>, &mut AsyncApp) -> R`
  (`GPUI/app/context.rs:229-237`); timers via
  `background_executor().timer(Duration) -> Task<()>`
  (`GPUI/executor.rs:183-188`).
- `Entity<V> -> AnyView` conversion: `impl<V: Render> From<Entity<V>> for
  AnyView` (`GPUI/view.rs:31-38`).
- `IntoElement::into_any_element(self) -> AnyElement` is a default trait
  method (`GPUI/element.rs:153-158`); it is the arm-unification path when an
  if/else yields two distinct opaque `impl IntoElement` types (a plain
  if/else there is E0308).
- `FocusHandle::new` is `pub(crate)` (`GPUI/window.rs:551-552`); the public
  constructor is `App::focus_handle(&self) -> FocusHandle`
  (`GPUI/app.rs:2927-2928`), reached from `Context<T>` through its `Deref`
  to `App` (`GPUI/app/context.rs:26-38`; repo precedent
  `src/gallery/sections/forms/questionnaire.rs:461`).
- `Context<T>` derefs to `App` (`GPUI/app/context.rs:26-38`), so listeners
  can pass `cx` where `&mut App` is expected (used by the wasm arm of the
  view's Close button, which calls `controller::close`).
- `AnyWindowHandle::update(self, cx, FnOnce(AnyView, &mut Window, &mut App)
  -> R) -> Result<R>` (`GPUI/window.rs:7404-7415`).
- Pixels arithmetic: the inner field is `pub(crate)`
  (`GPUI/geometry.rs:2689`), so use `as_f32()` (`:2802`) or `to_f64()`
  (`:2884`) for numeric math; `Mul<f32> for Pixels` exists (`:2719-2724`)
  and `Div` is `Pixels / Pixels -> f32` (`:2691-2698`); halve with `* 0.5`,
  never `/ 2.`. `Bounds::center() -> Point<T>` (`GPUI/geometry.rs:1001`).
- `text_xl` used in-repo (`src/features/pages/notifications.rs:55`);
  `rounded_b(cx.theme().radius_lg)` precedent (`GC/dialog/footer.rs:55`);
  `w_2_5()` = 40% width suffix generated by the macro
  (`gpui-pre-macros-0.3.8/src/styles.rs:1128-1132`); `border_b_1()`
  (`src/gallery/page.rs:141`), `border_t_1()`
  (`src/gallery/sections/editor/large_text.rs:126`), `.accessibility_label`
  (`src/gallery/sections/tabs/carousel.rs:202`) all used in-repo.
- `Edges<Pixels>` is a pub gpui type (`GPUI/platform.rs:40` export list;
  struct used by `WindowInsets`, `GPUI/platform.rs:1104-1126`).
- `cx.primary_display() -> Option<Rc<dyn PlatformDisplay>>`
  (`GPUI/app.rs:1492-1494`), `PlatformDisplay::bounds()` (survey-verified;
  the test platform returns `Some(active_display)` for it,
  `GPUI/platform/test/platform.rs:464-466`, and opens `TestWindow`s at
  `:493-506`).

Kit components:

- `Button::new(id)` (`GC/button/button.rs:241`), `.label(...)` (`:371`),
  `.accessibility_label(...)` (`:391`), `.on_click(impl Fn(&ClickEvent,
  &mut Window, &mut App))` (`:446-450`), `.ghost()` on the button-variants
  trait (`:81-83`), `.small()` via `Sizable` (in-repo
  `src/gallery/sections/overlays/popover.rs:421`); listener idiom
  `.on_click(cx.listener(|this, _, window, cx| ...))`
  (`src/gallery/sections/overlays/sheet.rs:375`).
- `cx.theme().popover` / `.muted` / `.muted_foreground` / `.primary` /
  `.border` / `.foreground` resolve through `Theme`'s `Deref` to
  `ThemeColor` (`GC/theme/mod.rs:219-226`; `pub popover: Hsla` at
  `GC/theme/theme_color.rs:199`); `theme().radius` / `radius_lg`
  (`GC/theme/mod.rs:185-187`).
- Import pattern for new UI modules: `use gpui_kit::component::{...}` named
  items plus `use gpui_kit::{prelude::FluentBuilder as _, *};`
  (`src/gallery/sections/overlays/sheet.rs:3-15`). Never `use gpui::`
  (`tests/qa_migration.rs:386-405`).
- Icons verified as svg files in `gpui-kit-assets-0.7.1/assets/icons/`:
  `music.svg`, `battery-full.svg`, `chevron-up.svg`, `x.svg`,
  `audio-lines.svg`.
- `chrono::Local::now()` precedent `src/gallery/sections/pickers/
  date_picker.rs:40`; chrono is a workspace dep (`Cargo.toml:27`), native
  entry with `serde` (`:176`), wasm entry swaps to `wasmbind`
  (`Cargo.toml:367-371`), so the clock compiles on every platform.
- `#[gpui_kit::test]` + `&TestAppContext` in-crate
  (`src/state/config_store.test.rs:280-287`).

Fullscreen hide mechanism: after creation, 03 overwrites the panel's
collection behavior to `CanJoinAllSpaces` only, dropping the
`FullScreenAuxiliary` bit gpui set (`MAC/window.rs:1236-1240`). Apple's
documented meaning of `fullScreenAuxiliary` is "The window displays on the
same space as the full screen window"; a window without the bit is not shown
on a fullscreen space, so the system hides the panel whenever any window is
fullscreen on its display and restores it after, with no notification
subscription needed. Verification attempt this session: direct fetch of the
Apple page returned 404 (bot-blocked, two attempts across sessions); the
quoted sentence comes from web-search results of that page plus community
reports of `canJoinAllSpaces` windows failing to stay visible over
fullscreen without the flag (sources in Risks item 3). Runtime confirmation
is manual QA step 5.

Crate picks: none new. Everything rides the existing macOS table
(`Cargo.toml:223-248`: objc2 0.6, objc2-app-kit 0.3.2 with std/NSView/NSWindow,
raw-window-handle 0.6, objc2-foundation 0.3.x) plus plan 02's `NSScreen`
feature addition. No manifest edit belongs to this plan.

Battery and now-playing are mocks: real battery needs IOPowerSources bindings
(no in-tree crate; `Cargo.toml` has none) and real now-playing needs
MediaRemote (private SPI). Adding either to a boilerplate demo breaks the
repo's no-speculative-dependencies rule (`AGENTS.md` "De-bloat").

## Architecture

Entities and state ownership:

- `NotchLayout` (value): `collapsed` and `expanded` `Bounds<Pixels>` in
  main-display top-left coordinates, computed once at open from plan 02's
  `probe` output. Pure math, no behavior.
- `NotchFloaterView` (`Render` + `Focusable` entity, root view of the notch
  window or the fallback float's content): owns `hovered: bool`,
  `pinned: bool`, `minute: String`, `date: String`, `layout: NotchLayout`,
  `resizable: bool`, `focus_handle: FocusHandle`. Expanded = `hovered ||
  pinned`. A `Context::spawn` loop ticks every second and calls `cx.notify()`
  only when the minute or date string changed. Pointer listeners flip
  `hovered`/`pinned`; on a state change the view asks the platform module to
  resize the window top-anchored (macOS only; `resizable == false` on the
  fallback float and everywhere off macOS).
- `macos::` helpers (cfg(macos), stateless): the reach-in path, `ns_window`
  from `HasWindowHandle`, `appkit_rect` y-flip math, `apply_notch_policy`
  (borderless + collection behavior + initial frame pin), `resize_to`
  (animated frame change). No retained state; free functions.
- `NotchFloaterWindow` (app `Global`): `Option<NotchHandle>` where
  `NotchHandle` is `Native(AnyWindowHandle)` on macOS or
  `Floating(crate::ui::floating::FloatingHandle)` elsewhere. An
  `on_window_closed` tracker clears the `Native` variant when the window goes
  away (mirrors plan 02's close-tracker design and the
  `src/app/init.rs:265-267` detach precedent). Written through
  `cx.set_global`, which notifies `observe_global` consumers
  (`GPUI/app.rs:2305-2310`).

Events and data flow:

- open: gallery button → `toggle(cx)` → `is_open` false → macOS:
  `probe(cx)` → `notch_layout(...)` → `gpui_kit::open_window` with PopUp
  options, `titlebar: None`, transparent background, `focus: false` → inside
  the same `App` call, `handle.update` runs `macos::apply_notch_policy`
  (convert to borderless + non-activating, drop `FullScreenAuxiliary`, pin
  frame to `layout.collapsed`) and `window.set_window_title` → global set →
  gallery re-renders through its `observe_global` subscription. Non-macOS:
  build `FloatingSpec` (Screen-Top anchor) → plan 02 `spawn_floating` →
  global set with the returned handle.
- expand/collapse: pointer over the panel → `on_hover` listener →
  `entity.update` → `set_hovered` → mode change → macOS:
  `setFrame_display_animate` to `layout.expanded`/`layout.collapsed`
  (animate = `!cx.reduce_motion()`) → `cx.notify()` → content re-renders at
  the new size. Click toggles `pinned` the same way. Fallback platforms:
  state change re-renders the swapped content at fixed window size.
- close: quick action "Close" → `window.remove_window()` on the window the
  listener already holds (macOS: the panel's own NSPanel; native fallbacks:
  the float window) because a `handle.update` on the window whose listener
  is firing fails (see Step 2.2); on wasm the same button routes to
  `controller::close`, whose `close_floating` call removes the overlay
  registry entry. The gallery toggle (running in the main window, a
  different window) routes through `controller::close` everywhere: macOS
  `remove_window` via the handle, fallback `close_floating`. Both `close`
  arms also clear the global directly so `is_open` flips in the same
  update. Closures 03 does not initiate are caught by two safety nets
  installed at `init`: on macOS a window-close tracker clears the `Native`
  handle; on other platforms an observer on plan 02's
  `FloatingWindowRegistry` global clears the `Floating` handle when the
  demo entry disappears (covers 02's FloatingShell close button, the Esc
  `DismissFloating` action, the wasm overlay pane's own close, the in-panel
  Close on native fallbacks, and external `close_all_floating`).
- capability: `init(cx)` registers `notch_floater` once at boot from static
  platform knowledge; the diagnostics page renders it with no further wiring.

Module tree:

```text
src/features/notch/
├── mod.rs            seam: init re-export, CAPABILITY_NOTCH_FLOATER, LOG, module gates
├── geometry.rs       NotchLayout + notch_layout() pure geometry
├── geometry.test.rs
├── view.rs           NotchFloaterView: hover/pin state machine, clock tick, render
├── view.test.rs
├── widgets.rs        collapsed pill + expanded widget rows (RenderOnce fns, no listeners)
├── controller.rs     NotchFloaterWindow global, open/close/toggle routing, capability
├── controller.test.rs
└── macos.rs          cfg(macos): ns_window, appkit_rect, apply_notch_policy, resize_to
```

## Interface Contract

Public surface at `crate::features::notch` (leaf feature; only the gallery
section and the diagnostics page consume it):

```rust
/// Capability-registry key for the notch floater.
pub const CAPABILITY_NOTCH_FLOATER: &str = "notch_floater";

/// Registers the `notch_floater` capability and, on macOS, the window-close
/// tracker. Call once from `app::init`, after
/// `crate::ui::floating::init(cx)` (03 reads nothing from it at init; the
/// ordering keeps all float-related rows together on the diagnostics page).
pub fn init(cx: &mut App);

/// Opens the notch floater if closed, closes it if open. Returns the
/// resulting open state. Never panics: on failure the capability is written
/// degraded with the error and `false` is returned.
pub fn toggle(cx: &mut App) -> bool;

/// Whether the notch floater window is currently open.
pub fn is_open(cx: &App) -> bool;
```

Crate-internal (used by the gallery section and tests, which are in-crate):

```rust
pub(crate) const LOG: &str = "gpui_starter::features::notch";

pub(crate) struct NotchLayout {
    pub collapsed: Bounds<Pixels>,
    pub expanded: Bounds<Pixels>,
}

/// Pure placement math; `screen` is the full main-display bounds (top-left
/// origin), `safe_top` the notch/menu-bar strip height, `housing` the
/// camera-housing rect in screen coordinates (None on displays without one).
pub(crate) fn notch_layout(
    screen: Bounds<Pixels>,
    safe_top: Option<Pixels>,
    housing: Option<Bounds<Pixels>>,
) -> NotchLayout;
```

Guarantees: `toggle` is idempotent-safe (an already-open floater closes and
re-opens nothing); on macOS the opened window never takes keyboard focus
(`focus: false` → `orderFront_`, `MAC/window.rs:1288-1292`) and never becomes
the key window (non-activating panel style, `MAC/window.rs:1019-1022`);
on non-macOS the fallback float follows plan 02's guarantees verbatim
(spawn cannot panic; wasm spawns as an overlay pane once the main window
exists).

## Module Layout

Sibling test files wired with the repo convention
`#[cfg(test)] #[path = "<name>.test.rs"] mod <name>_test;`
(`src/services/i18n.rs:19-21`, `CONTRIBUTING.md:110`).

| File | Purpose |
| --- | --- |
| `src/features/notch/mod.rs` | Re-export seam, `init`/`toggle`/`is_open` re-exports, `CAPABILITY_NOTCH_FLOATER`, `LOG`, cfg module gates, doc naming the removal seams |
| `src/features/notch/geometry.rs` | `NotchLayout`, `notch_layout()` placement math (pure) |
| `src/features/notch/geometry.test.rs` | Placement/clamp/fallback table tests (plain `#[test]`) |
| `src/features/notch/widgets.rs` | Collapsed pill and expanded widget rows as free `RenderOnce`-style fns |
| `src/features/notch/view.rs` | `NotchFloaterView` entity: state machine, clock tick task, render, quick-action buttons |
| `src/features/notch/view.test.rs` | Hover/pin transitions + entity smoke (`#[gpui_kit::test]`) |
| `src/features/notch/controller.rs` | Global handle, open/close/toggle routing, capability registration |
| `src/features/notch/controller.test.rs` | Toggle lifecycle + capability presence (`#[gpui_kit::test]`) |
| `src/features/notch/macos.rs` | cfg(macos): `ns_window`, `appkit_rect`, `apply_notch_policy`, `resize_to` NSWindow reach-in |
| `src/gallery/sections/overlays/notch_floater.rs` | Gallery section: open/close button, probe facts, capability status |

## Seam Touches

| # | File | Change | Why minimal |
| --- | --- | --- | --- |
| 1 | `src/features/mod.rs:1-3` | Add `pub mod notch;` | One-line mount beside `pages`/`palette` |
| 2 | `src/app/init.rs:274` (after `crate::notifications::initialize(cx);`) | `crate::features::notch::init(cx);` | Single boot hook; capability + tracker only, spawn stays lazy. Sits after plan 02's `crate::ui::floating::init(cx);` line (02's seam, expected immediately after the notifications block) |
| 3 | `src/gallery/sections/overlays/mod.rs:6-14,23-30` | `mod notch_floater;` (alphabetical: between `hover_card` and `notification`) + `notch_floater::register(sections, window, cx);` in `register()` | Gallery's documented two-line hook (`src/gallery/registry.rs:3-8`) |
| 4 | `locales/en.yml`, `locales/zh-CN.yml` | 8 `notch_*` keys in both | Lockstep rule `tests/qa_localization.rs:188-198` |
| 5 | `Cargo.toml` / `Cargo.lock` | Nothing | `NSScreen` feature is plan 02's seam; every objc2 symbol 03 touches (`NSView`, `NSWindow`, `NSWindowStyleMask`, `NSWindowCollectionBehavior`, `NSScreen`, NSGeometry via feature union) is covered by 02's feature line or already enabled (`Cargo.toml:243-247`). No new dep, nothing joins the not-wasm table |

### Removal Recipe

Style mirrors `src/gallery/mod.rs:1-7`:

1. Delete `src/gallery/sections/overlays/notch_floater.rs` and its two hook
   lines in `src/gallery/sections/overlays/mod.rs` (`mod notch_floater;` and
   the `notch_floater::register` call).
2. Delete the `notch_*` keys from `locales/en.yml` and `locales/zh-CN.yml`.
3. Remove `crate::features::notch::init(cx);` from `src/app/init.rs`.
4. Delete `src/features/notch/` entirely.
5. Revert `src/features/mod.rs` to its previous three module lines.
6. Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-features`,
   `cargo test --workspace --all-features`, `just wasm-check`. An open notch
   window dies with the app; no runtime cleanup exists or is needed.

## Implementation Phases

Prerequisite for every phase: plan 02 is implemented and
`cargo check -p gpui-starter` passes with `src/ui/floating` in the tree.

## Phase 1: Module mount and geometry

### Step 1.1: Module mount and compilable stubs

Files: `src/features/mod.rs`, `src/features/notch/mod.rs`,
`src/features/notch/controller.rs`, `src/features/notch/view.rs`,
`src/features/notch/widgets.rs`, `src/features/notch/geometry.rs`,
`src/features/notch/macos.rs` (all new except `src/features/mod.rs`).

Build:
- `src/features/mod.rs`: add `pub mod notch;` (keep the existing three lines).
- `src/features/notch/mod.rs`:
  - Module doc comment (1-2 lines) naming the removal seams: gallery section,
    `features/mod.rs` line, `app/init.rs` hook, locale keys.
  - `pub mod controller; pub mod geometry; pub mod view; pub mod widgets;`
  - `#[cfg(target_os = "macos")] pub(crate) mod macos;` (module-gate style of
    `src/platform/mod.rs:2-3`).
  - `pub use controller::{init, is_open, toggle};`
  - `pub const CAPABILITY_NOTCH_FLOATER: &str = "notch_floater";`
  - `pub(crate) const LOG: &str = "gpui_starter::features::notch";`
    (tracing-target rule, survey §9).
  - Interim stubs in `controller.rs` so the crate compiles: `pub fn init(_cx:
    &mut App) {}`, `pub fn is_open(_cx: &App) -> bool { false }`, `pub fn
    toggle(_cx: &mut App) -> bool { false }` (replaced in Step 4.2).
- `view.rs`, `widgets.rs`, `geometry.rs`: `//!` doc line only (contents
  arrive in their steps).
- `macos.rs`: `//!` doc line plus a compilable stub of the one symbol Phase
  2 references, so a macOS host compiles Phase 2 before Phase 3 writes the
  real body: `use gpui_kit::{Bounds, Pixels, Window}; pub(crate) fn
  resize_to(window: &Window, target: Bounds<Pixels>, animate: bool)
  { let _ = (window, target, animate); }`. `apply_notch_policy` needs no
  stub; it is referenced only by Phase 3+ code.

Tests: none.
Done-check: `cargo check -p gpui-starter` passes on the dev host;
`grep -n "pub mod notch" src/features/mod.rs` shows the line.

### Step 1.2: `NotchLayout` and `notch_layout` in `geometry.rs`

Files: `src/features/notch/geometry.rs`, `src/features/notch/geometry.test.rs`.

Build:
- `NotchLayout { pub collapsed: Bounds<Pixels>, pub expanded: Bounds<Pixels> }`
  (crate-internal record type; pub fields deliberate, contract above).
- `pub(crate) fn notch_layout(screen: Bounds<Pixels>, safe_top:
  Option<Pixels>, housing: Option<Bounds<Pixels>>) -> NotchLayout`. All math
  in `f32` via `Pixels::as_f32()` (`GPUI/geometry.rs:2802`); halving via
  `* 0.5` on `Pixels` (`Mul<f32>` at `:2719`); never `Pixels / f32` (no such
  impl, `Div` is `Pixels / Pixels -> f32` at `:2691`):
  - `let pill_h = px(safe_top.map(|p| p.as_f32()).unwrap_or(24.).clamp(20., 40.));`
    (pill height matches the notch/menu-bar strip height)
  - `let collapsed_w = px(housing.map(|h| h.size.width.as_f32()).unwrap_or(240.).clamp(160., (screen.size.width * 0.6).as_f32()));`
    (pill spans the camera housing width, clamped)
  - `let center_x = screen.center().x;` (`Bounds::center`,
    `GPUI/geometry.rs:1001`).
  - `collapsed = Bounds::new(point(center_x - collapsed_w * 0.5, px(0.)), size(collapsed_w, pill_h))`.
  - `let expanded_w = px(720f32.min((screen.size.width * 0.9).as_f32()));`
  - `let expanded_h = px(200f32.min((screen.size.height - pill_h).as_f32()).max(pill_h.as_f32()));`
    (never smaller than the pill, so the pill stays reachable on short
    displays)
  - `expanded = Bounds::new(point(center_x - expanded_w * 0.5, px(0.)), size(expanded_w, expanded_h))`.
  - Constants doc (1-2 lines): window geometry must match the physical
    screen, so `px()` values here are the audited exception to the rem-scale
    rule (design guides "Zoom, base font, and rem": geometry that must match
    an external surface).
- Wire `#[cfg(test)] #[path = "geometry.test.rs"] mod geometry_test;`.

Tests (`geometry.test.rs`, plain `#[test]`, no gpui context):
- `notched_display_places_pill_over_housing`: screen `1512x982` at `(0,0)`,
  `safe_top = px(37.)`, housing 200 wide centered at top → collapsed
  `200x37` at `(656, 0)`; expanded `720x200` at `(396, 0)`.
- `housing_width_clamps_to_screen_share`: housing 1200 on a 1512 screen →
  collapsed width 907.2; housing 100 → 160.
- `defaults_without_probe_data`: `safe_top: None` → pill height 24;
  `housing: None` → collapsed width 240.
- `narrow_and_short_screens_clamp_expanded`: screen 500 wide → expanded
  width 450; screen 100 tall with pill 24 → expanded height 76; screen 30
  tall → expanded height equals pill height (floor).

Done-check: `cargo test -p gpui-starter --lib features::notch::geometry`
green.

## Phase 2: Widgets and view

### Step 2.1: `widgets.rs`

Files: `src/features/notch/widgets.rs`.

Build: free functions returning `impl IntoElement` (pattern
`src/ui/widgets/virtual_list.rs:9-88`), imports
`use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _, Icon, IconName};`
plus `use gpui_kit::{prelude::FluentBuilder as _, *};`, never `use gpui::`:
- `pub(crate) fn collapsed_pill(minute: &str, cx: &App) -> impl IntoElement`:
  `div().size_full().flex().justify_center().items_center()` with one
  `h_flex().gap_1()` child: `Icon::new(IconName::ChevronUp)` colored
  `cx.theme().muted_foreground` (expand hint), clock `text_xs()`.
  `text_color(cx.theme().foreground)`. The pill surface (background, radius,
  hairline) is owned by `view.rs` so collapsed and expanded share one surface
  treatment.
- `pub(crate) fn clock_block(minute: &str, date: &str, cx: &App) -> impl IntoElement`:
  `v_flex().gap_1()`: clock `text_xl()` `accessibility_label(localize(
  "notch_time_label"))`, date `text_xs()` muted
  (`text_color(cx.theme().muted_foreground)`).
- `pub(crate) fn now_playing(cx: &App) -> impl IntoElement`: `v_flex().gap_1()
  .min_w_0()`: header `h_flex().gap_1()` with `Icon::new(IconName::Play)`
  (no music-note icon exists in the default 106 the app registers; Play is
  the verified default-enum stand-in)
  plus `text_xs()` muted `localize("notch_now_playing")`; title `text_sm()`
  `localize("notch_track_title")`; artist `text_xs()` muted
  `localize("notch_track_artist")`; progress `h_1().w_full().rounded_full()
  .bg(cx.theme().muted)` with inner `h_full().w_2_5().rounded_full()
  .bg(cx.theme().primary)` (static 40%, mock).
- `pub(crate) fn battery_block(cx: &App) -> impl IntoElement`:
  `v_flex().gap_1().items_end()`: row `h_flex().gap_1()` with
  `Icon::new(IconName::BatteryFull)` plus `text_sm()` `"84%"`; caption
  `text_xs()` muted `localize("notch_battery")`.
- All strings through `crate::i18n::localize` (facade
  `src/services/i18n.rs:3-5`); the literal `"84%"` is mock data, matching the
  demo label.

Tests: none (pure composition; covered by view tests + manual QA).
Done-check: `cargo check -p gpui-starter` passes; grep finds no listener
(`on_click`/`cx.listener`) in `widgets.rs`.

### Step 2.2: `NotchFloaterView` in `view.rs`

Files: `src/features/notch/view.rs`.

Build:
- `pub(crate) struct NotchFloaterView { hovered: bool, pinned: bool,
  minute: String, date: String, layout: NotchLayout, resizable: bool,
  focus_handle: FocusHandle }` implementing `Render` + `Focusable`
  (`fn focus_handle(&self, _: &App) -> FocusHandle { self.focus_handle.clone() }`).
- Constructors (no `Window` param; nothing in them needs one, and an unused
  param fails CI's `clippy -D warnings`. `FocusHandle::new` is `pub(crate)`
  (`GPUI/window.rs:551-552`), so the handle comes from `cx.focus_handle()`
  (`App::focus_handle`, `GPUI/app.rs:2927-2928`, reached through
  `Context<T>`'s `Deref` to `App`; repo precedent
  `src/gallery/sections/forms/questionnaire.rs:461`)):
  - `pub(crate) fn new(layout: NotchLayout, cx: &mut Context<Self>) -> Self` for the
    macOS notch window; `resizable: true`.
  - `pub(crate) fn demo(cx: &mut Context<Self>) -> Self` for the
    fallback float; `layout = notch_layout(Bounds::new(point(px(0.),
    px(0.)), size(px(360.), px(140.))), None, None)` (values only feed the
    layout math; unused at fixed size), `resizable: false`.
  - Both call a private `fn start_clock(&mut self, cx: &mut Context<Self>)`:
    seed `minute = now.format("%H:%M").to_string()`, `date =
    now.format("%a, %b %e").to_string()` from one `chrono::Local::now()`
    (precedent `src/gallery/sections/pickers/date_picker.rs:40`), then
    `cx.spawn(async move |this, cx| { loop {
    cx.background_executor().timer(Duration::from_secs(1)).await;
    if this.update(cx, |this, cx| this.tick(cx)).is_err() { break; } } })
    .detach();`. `tick` recomputes both strings and calls `cx.notify()`
    only when either changed (signatures `GPUI/app/context.rs:229-237`,
    `GPUI/executor.rs:183-188`; loop-break on dropped entity mirrors the
    `src/platform/desktop_shell/tray/macos.rs:144-153` drain pattern).
- State machine:
  - `fn is_expanded(&self) -> bool { self.hovered || self.pinned }`.
  - `fn set_hovered(&mut self, hovered: bool, window: &mut Window, cx: &mut
    Context<Self>)` and `fn toggle_pinned(&mut self, window: &mut Window, cx:
    &mut Context<Self>)`: both early-return when the derived expanded state
    cannot change (`set_hovered` when `self.hovered == hovered`;
    `toggle_pinned` when flipping `self.pinned = !self.pinned` leaves
    `is_expanded()` where it was, e.g. hover held), else `self.sync_frame(
    window, cx)`, `cx.notify()`. The early return also skips a redundant
    animated `setFrame` to the identical rect.
  - `fn sync_frame(&mut self, window: &mut Window, cx: &mut Context<Self>)`:
    `if !self.resizable { return; }` then
    `#[cfg(target_os = "macos")] crate::features::notch::macos::resize_to(
    window, if self.is_expanded() { self.layout.expanded } else {
    self.layout.collapsed }, !cx.reduce_motion());`
    (`#[cfg(not(target_os = "macos"))] let _ = (&window, cx);` keeps the
    signature uniform; on non-macOS `resizable` is always false so the
    early return fires first).
- Render:
  - Root: `div().id("notch-floater").size_full().track_focus(
    &self.focus_handle)` (stateful for `on_hover`, trait
    `GPUI/elements/div.rs:1305`), with:
    - `.on_hover({ let entity = cx.entity(); move |hovered, window, cx| {
      entity.update(cx, |this, cx| this.set_hovered(*hovered, window, cx));
      } })` (fluent method `GPUI/elements/div.rs:1661-1669`).
    - `.on_mouse_down(MouseButton::Left, { let entity = cx.entity(); move
      |_, window, cx| { entity.update(cx, |this, cx| this.toggle_pinned(
      window, cx)); } })` (click pins/unpins; the pill is a click target
      whether or not hover fired).
  - Surface: `.bg(cx.theme().popover)` (via `Deref`,
    `GC/theme/mod.rs:219-226` + `theme_color.rs:199`), hairline bottom edge
    only `.border_b_1().border_color(cx.theme().border)`, bottom-only
    rounding `.rounded_b(cx.theme().radius)` (pattern
    `GC/dialog/footer.rs:55`); top edge flush with the screen top.
  - Content: `.child(if self.is_expanded() {
      self.render_expanded(cx).into_any_element()
    } else {
      widgets::collapsed_pill(&self.minute, cx).into_any_element()
    })`. The two arms are distinct opaque `impl IntoElement` types, so a
    plain if/else is E0308; both arms go through
    `IntoElement::into_any_element(self) -> AnyElement`
    (`GPUI/element.rs:153-158`).
  - `fn render_expanded(&mut self, cx: &mut Context<Self>) -> impl IntoElement`
    (no `Window` param; the button listeners receive one from
    `cx.listener`):
    `v_flex().size_full().p_4().gap_3()` with
    - `h_flex().gap_4().justify_between().items_start()` row of
      `widgets::clock_block(&self.minute, &self.date, cx)`,
      `widgets::now_playing(cx)`, `widgets::battery_block(cx)` (three peer
      groups, `gap_4` = lg per design-guides spatial grammar);
    - a `div().border_t_1().border_color(cx.theme().border)` hairline;
    - actions row `h_flex().justify_end().gap_2()`:
      - "Collapse" `Button::new("notch-collapse").ghost().small().label(
        localize("notch_collapse"))` with `.on_click(cx.listener(|this, _,
        window, cx| { this.pinned = false; this.hovered = false;
        this.sync_frame(window, cx); cx.notify(); }))`;
      - "Close" `Button::new("notch-close").ghost().small().label(
        localize("notch_close"))` with `.on_click(cx.listener(|this, _, window,
        cx| this.close(window, cx)))` calling a private
        `fn close(&mut self, window: &mut Window, cx: &mut Context<Self>)`:
        ```rust
        #[cfg(any(target_os = "macos", not(target_family = "wasm")))]
        {
            let _ = cx;
            window.remove_window();
        }
        #[cfg(all(not(target_os = "macos"), target_family = "wasm"))]
        {
            let _ = window;
            super::controller::close(cx);
        }
        ```
        Why not `controller::close` everywhere: click listeners run inside
        `app.update_window` (`window.dispatch_event` is called from within
        it, `GPUI/app/test_app.rs:463-464`), and `update_window_erased`
        takes the window out of the map for the duration
        (`GPUI/app.rs:3020-3045`), so a `handle.update` on the window whose
        listener is firing returns `Err("window not found")`
        (`GPUI/app.rs:2095`) instead of closing it. The listener already
        holds `&mut Window`, so `window.remove_window()`
        (`GPUI/window.rs:2324-2328`) is the correct close on macOS (own
        NSPanel) and on native fallbacks (the float window; plan 02's close
        tracker then removes the registry entry and 03's registry watcher
        clears the global). On wasm the held `window` is the ROOT window, so
        `remove_window()` there would quit the app; the wasm arm routes to
        `controller::close`, whose `close_floating` call removes the overlay
        registry entry without touching any window. `Context<Self>` derefs
        to `App`, so `close(cx)` typechecks inside the listener
        (`GPUI/app/context.rs:26-38`); `close` is `pub(crate)` in
        `controller.rs` for the gallery toggle and this wasm arm.
      Imports: `gpui_kit::component::{button::{Button, ButtonVariants as _},
      Sizable as _, ...}` (trait-import pattern
      `src/gallery/sections/overlays/sheet.rs:3-15`).

Tests: in `view.test.rs` (Step 2.3).
Done-check: `cargo check -p gpui-starter` passes; grep finds `text_xl`/
`BatteryFull` only in `widgets.rs`/`view.rs`.

### Step 2.3: `view.test.rs`

Files: `src/features/notch/view.test.rs` (wired in Step 2.2's module file via
`#[cfg(test)] #[path = "view.test.rs"] mod view_test;`).

Build (`#[gpui_kit::test]`, `cx: &TestAppContext`; harness precedent
`src/state/config_store.test.rs:280-287`):
- `hover_and_pin_drive_expanded_state`: open a window hosting the demo view
  (`cx.update(|cx| gpui_kit::open_window(WindowOptions::default(), cx,
  |_, cx| cx.new(|cx| NotchFloaterView::demo(cx)))`; the test
  platform opens `TestWindow`s, `GPUI/platform/test/platform.rs:493-506`, and
  `open_window` draws once before returning), keep both the window handle and
  the view entity, then drive the state machine inside
  `window.update(cx, |_, window, cx| view.update(cx, |this, cx| { ... }))`:
  `toggle_pinned` → `is_expanded()` true; `set_hovered(true)` then
  `set_hovered(false)` with pin held → still expanded; `toggle_pinned` again
  → collapsed. `resizable == false` keeps every `sync_frame` on the early
  return, so nothing touches the window beyond layout.
- `render_smoke_does_not_panic`: open the demo view in its own window
  (construction + first draw are the smoke; element-text assertions need the
  kit's `test-support` feature, which this repo does not enable (survey
  Open Questions), so the assertion level is non-panic + state, honestly
  scoped).

Done-check: `cargo test -p gpui-starter --lib features::notch::view` green.

## Phase 3: macOS native window

### Step 3.1: `macos.rs` reach-in

Files: `src/features/notch/macos.rs`.

Build: entire module exists only under the `#[cfg(target_os = "macos")]`
gate in `mod.rs`; imports mirroring `src/platform/liquid_glass.rs:4-9`:
`use gpui_kit::{Bounds, Pixels, Window}; use objc2::MainThreadMarker; use
objc2_app_kit::{NSScreen, NSView, NSWindow, NSWindowCollectionBehavior,
NSWindowStyleMask}; use objc2_foundation::{NSPoint, NSRect, NSSize}; use
raw_window_handle::{HasWindowHandle, RawWindowHandle};` plus
`use super::geometry::NotchLayout;`.
- `const NONACTIVATING_PANEL: NSWindowStyleMask =
  NSWindowStyleMask::from_bits_retain(1 << 7);` with a 1-2 line comment: the
  bit is absent from objc2-app-kit's generated bitflags; gpui constructs the
  same value the same way (`MAC/window.rs:94-96`).
- `fn ns_window(window: &Window) -> Option<objc2::rc::Retained<NSWindow>>`:
  `HasWindowHandle::window_handle(window).ok()?` (trait call is deliberate:
  the inherent `Window::window_handle` returns `AnyWindowHandle` and shadows
  it, `src/platform/liquid_glass.rs:230-240`), match
  `RawWindowHandle::AppKit(appkit)` from `handle.as_raw()`, then
  `unsafe { appkit.ns_view.cast::<NSView>().as_ref() }.window()`; the cast
  is the `liquid_glass.rs` SAFETY pattern (GPUI guarantees a valid NSView),
  `NSView::window` is the safe generated method (`AK/NSView.rs:213-216`).
  `None` on test windows (`TestWindow::window_handle` returns
  `Err(NotSupported)`, plan 02 research).
- `fn appkit_rect(target: Bounds<Pixels>, screen_frame: NSRect) -> NSRect`:
  `NSRect::new(NSPoint::new(screen_frame.origin.x + target.origin.x.to_f64(),
  screen_frame.origin.y + screen_frame.size.height -
  target.origin.y.to_f64() - target.size.height.to_f64()), NSSize::new(
  target.size.width.to_f64(), target.size.height.to_f64()))`; the exact
  inverse of gpui's readback flip (`MAC/window.rs:917-919`); constructors at
  `OCF/src/geometry.rs:225-241`, `Pixels::to_f64` at `GPUI/geometry.rs:2884`.
- `pub(crate) fn apply_notch_policy(window: &mut Window, layout:
  &NotchLayout) -> bool`:
  1. `let Some(mtm) = MainThreadMarker::new() else { return false; };`
     (`objc2-0.6.4/src/main_thread_marker.rs:230`).
  2. `let Some(ns_window) = ns_window(window) else { return false; };`
  3. `let Some(screen) = NSScreen::mainScreen(mtm) else { return false; };`
     (`AK/NSScreen.rs:37-39`).
  4. `ns_window.setStyleMask(NSWindowStyleMask::Borderless |
     NONACTIVATING_PANEL);` (`AK/NSWindow.rs:795`), dropping the `titled` bit
     gpui set (`MAC/window.rs:1008-1011`) so no system corner rounding or
     titlebar chrome sits over the notch; keeps the non-activating bit. gpui
     itself flips a live window to borderless the same way
     (`MAC/window.rs:644`).
  5. `ns_window.setCollectionBehavior(
     NSWindowCollectionBehavior::CanJoinAllSpaces);`
     (`AK/NSWindow.rs:1446`), overwriting gpui's `CanJoinAllSpaces |
     FullScreenAuxiliary` (`MAC/window.rs:1236-1240`); without
     `FullScreenAuxiliary` the system hides the panel while any window is
     fullscreen on its display (Research Digest, mechanism + sources).
  6. `ns_window.setFrame_display_animate(appkit_rect(layout.collapsed,
     screen.frame()), true, false);` (`AK/NSWindow.rs:865-876`); pins the exact
     frame at the screen top, top-left-origin semantics.
  7. Return `true`.
- `pub(crate) fn resize_to(window: &Window, target: Bounds<Pixels>, animate:
  bool)`: `MainThreadMarker::new()` and `NSScreen::mainScreen(mtm)` again,
  then `ns_window(window)?.setFrame_display_animate(appkit_rect(target,
  screen.frame()), true, animate)`; the rect recomputes origin from the
  layout's constant `target.origin.y`, so the top edge stays flush across
  expand/collapse. `&Window` suffices: gpui learns of the resize through the
  view's `setFrameSize:` override and `windowDidResize:` observer
  (`MAC/window.rs:227-230`, `:440-443`), and 03 never calls `Window::resize`
  (it detaches through `setContentSize:` and pins the origin,
  `MAC/window.rs:1440-1453`).

Tests: none runnable off macOS; the module is cfg'd out on Linux/CI hosts.
`controller.test.rs` exercises the `false`-return path implicitly on any test
platform whose window handle is unsupported.
Done-check: `cargo check -p gpui-starter` passes on the dev host (Linux
compiles the module away; no Apple toolchain exists here, so the macOS
compile is first exercised on a Mac or macOS CI; Risk 1).

### Step 3.2: controller native path and global

Files: `src/features/notch/controller.rs`.

Build:
- `#[derive(Default)] pub(crate) struct NotchFloaterWindow(Option<NotchHandle>);`
  `impl Global` (shape of `src/runtime/capabilities.rs:53-58`).
- `#[derive(Clone)] pub(crate) enum NotchHandle` with
  variants `#[cfg(target_os = "macos")] Native(AnyWindowHandle)` variant;
  `#[cfg(not(target_os = "macos"))] Floating(
  crate::ui::floating::FloatingHandle)` variant. `Clone` is required for the
  close tracker read (plan 02's `FloatingHandle` derives `Clone`, `02:330`).
- `pub fn is_open(cx: &App) -> bool`: `cx.try_global::<NotchFloaterWindow>()
  .is_some_and(|g| g.0.is_some())` (`GPUI/app.rs:2275-2279`).
- `#[cfg(target_os = "macos")] fn open(cx: &mut App) -> anyhow::Result<()>`:
  1. `let info = crate::ui::floating::probe(cx);`
  2. `let screen = cx.primary_display().map(|d| d.bounds()).unwrap_or(
     Bounds::new(point(px(0.), px(0.)), size(px(1512.), px(982.))));`
     (`GPUI/app.rs:1492-1494`; the constant only exists so a headless
     environment still computes a layout).
  3. `let safe_top = info.notch.map(|n| n.safe_area.top).or(info.menu_bar);`
  4. `let layout = notch_layout(screen, safe_top, info.notch.and_then(|n|
     n.camera_housing));`
  5. Options: `window_bounds: Some(WindowBounds::Windowed(layout.collapsed))`,
     `titlebar: None` (no traffic lights: style mask gets no closable bit,
     `MAC/window.rs:1008-1011`), `kind: WindowKind::PopUp`, `focus: false`,
     `show: true`, `is_resizable: false`, `is_minimizable: false`,
     `is_movable: false`, `window_background:
     WindowBackgroundAppearance::Transparent`, `..Default::default()`.
  6. `let (handle, _) = gpui_kit::open_window(options, cx, |_, cx| {
     cx.new(|cx| NotchFloaterView::new(layout.clone(), cx)) })?;`
  7. `let pinned = handle.update(cx, |_, window, _| { window.set_window_title(
     &crate::i18n::localize("notch_window_title")); super::macos::
     apply_notch_policy(window, &layout) }).unwrap_or(false);`. The title is for
     Mission Control/a11y (`GPUI/window.rs:2924`); `pinned == false` is not
     an error (test platform windows have no NSWindow handle), log it:
     `if !pinned { tracing::debug!(target: LOG, "notch policy not applied
     (no native window handle)"); }`.
  8. `cx.set_global(NotchFloaterWindow(Some(NotchHandle::Native(handle))));`
     (notifies `observe_global` consumers, `GPUI/app.rs:2305-2310`).
- `#[cfg(target_os = "macos")] pub(crate) fn close(cx: &mut App)` (the
  gallery toggle lands here, running in the main window so the handle
  update below targets a window that is not mid-update): read global →
  `Native(h)` → `let _ = h.update(cx, |_, window, _|
  window.remove_window());` (Err means already closed); then always
  `cx.set_global(NotchFloaterWindow(None));` so `is_open` flips in the same
  update. The close tracker (installed by `init`, Step 4.2) is the safety
  net for window-manager-initiated closure and the panel's own in-window
  Close (Step 2.2), which bypasses this fn.
- `fn install_close_tracker(cx: &mut App)` (macOS only; called from `init`
  under cfg): `cx.on_window_closed(|cx, window_id| { let open =
  cx.try_global::<NotchFloaterWindow>().and_then(|g| g.0.clone()); if
  matches!(open, Some(NotchHandle::Native(h)) if h.window_id() == window_id)
  { cx.set_global(NotchFloaterWindow(None)); } }).detach();`
  (`AnyWindowHandle::window_id` `GPUI/window.rs:7382`; detach precedent
  `src/app/init.rs:265-267`).

Tests: Step 4.3 (single controller test file after the fallback exists).
Done-check: `cargo check -p gpui-starter` passes on the dev host.

## Phase 4: Fallback path, capabilities, boot seam

### Step 4.1: fallback open/close

Files: `src/features/notch/controller.rs`.

Build:
- `#[cfg(not(target_os = "macos"))] fn open(cx: &mut App) ->
  anyhow::Result<()>`:
  1. Spec: `let spec = crate::ui::floating::FloatingSpec::new(
     crate::ui::floating::FloatingWindowId("notch-floater-demo".into()),
     size(px(360.), px(140.))).title(crate::i18n::localize(
     "notch_window_title")).level(crate::ui::floating::FloatLevel::Floating)
     .anchor(crate::ui::floating::FloatAnchor::Screen { display: None,
     placement: crate::ui::floating::Placement9::Top, offset: point(px(0.),
     px(8.)) }).focus(false);`. Every builder per plan 02's contract
     (`02:318-328`); the offset insets the demo below the menu bar/panel
     strip, and plan 02's resolve logic clamps into visible bounds.
  2. `let handle = crate::ui::floating::spawn_floating(spec, |_, cx| {
     cx.new(|cx| NotchFloaterView::demo(cx)).into() }, cx)?;`.
     `build` returns `AnyView` (`From<Entity<V>>` `GPUI/view.rs:31-38`); the
     closure is `Fn` as the contract requires.
  3. `cx.set_global(NotchFloaterWindow(Some(NotchHandle::Floating(handle))));`
- `#[cfg(not(target_os = "macos"))] pub(crate) fn close(cx: &mut App)`
  (callers: the gallery toggle from the main window, and the wasm arm of
  the view's Close; the native-fallback in-panel Close bypasses it, see
  Step 2.2): read global → `Floating(h)` → `let _ =
  crate::ui::floating::close_floating(&h, cx);` (removes the native float
  window via `remove_window`, or the wasm overlay registry entry directly)
  then always `cx.set_global(NotchFloaterWindow(None));`. The registry
  watcher below is the safety net for closes 03 does not initiate.
- `#[cfg(not(target_os = "macos"))] fn install_float_registry_watch(cx: &mut
  App)`: `cx.observe_global::<crate::ui::floating::FloatingWindowRegistry>(
  |cx| { let ours = cx.try_global::<NotchFloaterWindow>().and_then(|g|
  g.0.clone()); if matches!(ours, Some(NotchHandle::Floating(_))) &&
  !crate::ui::floating::floating_snapshot(cx).iter().any(|e| e.id.0 ==
  "notch-floater-demo") { cx.set_global(NotchFloaterWindow(None)); }
  }).detach();`. Plan 02 re-exports the registry global for observers
  (`02:405-406`); every registry mutation runs through
  `default_global`/`set_global`, which pushes
  `Effect::NotifyGlobalObservers` (`GPUI/app.rs:2294-2310`), and
  `observe_global` callbacks run as effects flushed at each update boundary
  (`GPUI/app.rs:2330-2343`). This catches the FloatingShell header close,
  the Esc `DismissFloating` action, the wasm overlay pane's own close
  button, and external `close_all_floating`. `e.id.0 == "literal"` works
  because `SharedString` implements `PartialEq<&str>`
  (gpui-pre-shared-string-0.3.8/gpui_shared_string.rs:98-109).
- `pub fn toggle(cx: &mut App) -> bool` final form: `if is_open(cx) {
  close(cx); false } else { match open(cx) { Ok(()) => true, Err(e) => {
  crate::capabilities::set(CAPABILITY_NOTCH_FLOATER,
  CapabilityStatus::degraded(e), cx); false } } }` (helpers
  `src/runtime/capabilities.rs:28-37,63-79`).

Tests: none here.
Done-check: `cargo check -p gpui-starter` and `just wasm-check` pass (the
wasm arm of `spawn_floating` is plan 02's; 03's cfg split compiles on both).

### Step 4.2: capability registration + boot seam

Files: `src/features/notch/controller.rs`, `src/app/init.rs`.

Build:
- `pub fn init(cx: &mut App)` replacing the Step 1.1 stub:
  - capability registration:
    `#[cfg(target_os = "macos")]`:
    `crate::capabilities::set(CAPABILITY_NOTCH_FLOATER,
    CapabilityStatus::supported_enabled(), cx);`
    `#[cfg(all(not(target_os = "macos"), not(target_family = "wasm")))]`:
    `CapabilityStatus::degraded("notch APIs are macOS-only; showing
    inset-anchored demo window")`
    `#[cfg(target_family = "wasm")]`:
    `CapabilityStatus::degraded("native notch unavailable on wasm; showing
    in-window overlay demo")`
    (helper semantics `src/runtime/capabilities.rs:28-37`; reason strings are
    English literals, precedent `src/platform/input/shortcuts.rs:290-305`).
  - close safety nets:
    `#[cfg(target_os = "macos")] install_close_tracker(cx);`
    `#[cfg(not(target_os = "macos"))] install_float_registry_watch(cx);`
- `src/app/init.rs`: one line after `crate::notifications::initialize(cx);`
  (`src/app/init.rs:274`): `crate::features::notch::init(cx);`, placed
  after plan 02's `crate::ui::floating::init(cx);` line (that line is plan
  02's seam, expected immediately after the notifications block).
- Replace the remaining Step 1.1 stubs with the real `is_open`/`toggle`.

Tests: Step 4.3.
Done-check: `grep -n "features::notch::init" src/app/init.rs` shows the line;
`cargo check -p gpui-starter` passes.

### Step 4.3: `controller.test.rs`

Files: `src/features/notch/controller.test.rs` (wired in `controller.rs` via
`#[cfg(test)] #[path = "controller.test.rs"] mod controller_test;`).

Build (`#[gpui_kit::test]`); every test calls
`cx.update(|cx| { crate::ui::floating::init(cx);
crate::features::notch::init(cx); })` first: the floating tracker owns
registry cleanup on close, so it must be installed exactly as the boot seam
orders it:
- `init_registers_capability`: `crate::capabilities::snapshot(cx)`
  (`src/runtime/capabilities.rs:81-85`) contains
  `CAPABILITY_NOTCH_FLOATER` with `supported: true` on every host this test
  runs on.
- `toggle_opens_and_closes_floater`: `assert!(!is_open(cx)); assert!(
  toggle(cx)); assert!(is_open(cx));` then `assert!(!toggle(cx));
  assert!(!is_open(cx));`. On non-macOS hosts additionally assert
  `crate::ui::floating::floating_snapshot(cx).len() == 1` after open and
  `== 0` after close (plan 02 snapshot is ordered by id); guard those two
  lines with `#[cfg(not(target_os = "macos"))]`. On a macOS host the same
  test runs against the test platform: `open` reaches
  `gpui_kit::open_window` (`TestWindow`), `apply_notch_policy` returns false
  harmlessly (no native handle), and the `is_open` assertions still hold.
- `toggle_is_idempotent_across_cycles`: open → close → open again; assert
  final state open and (non-macOS) snapshot len 1. The fallback rebuilds the
  demo view on reopen (plan 02 retained-factory semantics); clock state
  resets, which is invisible since it re-reads the wall clock.
- `external_float_close_clears_state` (whole test under
  `#[cfg(not(target_os = "macos"))]`): open via `toggle`, then call
  `crate::ui::floating::close_all_floating(cx)` (`02:354-355`) as an
  externally initiated close 03 did not drive; at the next `cx.update`
  boundary the registry watcher has run (effects flush per update) and
  `is_open(cx)` is false. Regression test for the close-desync path behind
  gallery label honesty (manual QA 6/10).

Done-check: `cargo test -p gpui-starter --lib features::notch` green on the
dev host.

## Phase 5: Gallery section

### Step 5.1: section

Files: `src/gallery/sections/overlays/notch_floater.rs` (new),
`src/gallery/sections/overlays/mod.rs`.

Build:
- `NotchSection` entity + `NotchSection::view(window, cx) -> Entity<Self>`
  (idiom `src/gallery/sections/overlays/sheet.rs:204-215`) + `pub fn
  register(sections, window, cx)` pushing
  `GallerySection::new("notch-floater", "Notch Floater", "macOS notch-area
  floater with hover expansion; inset-anchored demo on other platforms.",
  NotchSection::view(window, cx))` (idiom `sheet.rs:450-458`; literal
  English strings follow the gallery precedent, `sheet.rs:452-455`).
- Entity state: `open: bool` plus the `observe_global` subscription. In
  `view(window, cx)` (or the section's constructor):
  `cx.observe_global::<crate::features::notch::controller::
  NotchFloaterWindow>(|_, cx| cx.notify()).detach();` because `set_global` pushes
  a notify-observers effect (`GPUI/app.rs:2305-2310`), so the button label
  stays honest even when the floater is closed from its own "Close" action.
  (The global type is `pub(crate)` in the notch module; the gallery is
  in-crate, so the path is reachable. If `controller` is private in
  `notch/mod.rs`, re-export the type `pub(crate) use controller::
  NotchFloaterWindow;` from `notch/mod.rs`; one line, no behavior.)
- Section body uses the shared demo scaffolding (`section(id, title)`
  builder, `src/gallery/sections/overlays/demo.rs:104-118`):
  - `section("notch-toggle", "Floater").description("Toggle the notch
    floater window.")` containing a default `Button` whose label follows
    `crate::features::notch::is_open(cx)` ("Open notch floater" /
    "Close notch floater") with `on_click(cx.listener(|this, _, window,
    cx| { this.open = crate::features::notch::toggle(cx); cx.notify(); }))`
    (button listener idiom `sheet.rs:375-377`; the local field keeps the
    click response immediate, the global observation keeps it correct).
  - A muted `text_sm` status line from plan 02 `probe(cx)`: safe-area top,
    housing width or "no camera housing", menu bar height, as one
    `format!`-built line.
  - A muted `text_sm` capability line: `notch_floater` status + reason from
    `crate::capabilities::snapshot(cx)`.
- Hook lines in `overlays/mod.rs`: `mod notch_floater;` (alphabetical,
  between `hover_card` and `notification`) and
  `notch_floater::register(sections, window, cx);` in `register()`.

Tests: none new (manual QA surface; gallery compiles on wasm, which is part
of the point).
Done-check: `cargo check -p gpui-starter` passes; the section appears under
the gallery's Overlays group (manual).

## Phase 6: i18n and gates

### Step 6.1: Locale keys

Files: `locales/en.yml`, `locales/zh-CN.yml`.

Build: the 8 keys from the i18n table below, quoted scalars, top-level
snake_case (parser rules `tests/qa_localization.rs:20-71`). Appended at the
end of each catalog, after the existing `desktop_action_failed` /
`about_tagline` block.

Done-check: `cargo test -p gpui-starter --test qa_localization` green.

### Step 6.2: Full gates

Files: none (verification only).

Commands, in order:
1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-features`
3. `cargo test --workspace --all-features`
4. `just wasm-check`

Done-check: all four pass; `git diff --stat` shows only the files in Seam
Touches.

## Platform & Wasm Gating

cfg gates:

- `src/features/notch/macos.rs`: `#[cfg(target_os = "macos")] pub(crate) mod
  macos;` in `mod.rs` (module-gate style `src/platform/mod.rs:2-3`);
  `view.rs`'s `sync_frame` reaches it under the same cfg with a `let _` sink
  elsewhere. `resize_to`/`apply_notch_policy` have no non-macOS fallback;
  every call site is cfg-gated.
- `controller.rs`: `open`/`close`/`NotchHandle` variants split
  `#[cfg(target_os = "macos")]` vs `#[cfg(not(target_os = "macos"))]`;
  `close` is `pub(crate)` on both arms (gallery toggle on every platform,
  plus the wasm arm of the view's in-panel Close). The view's in-panel Close
  splits on `any(target_os = "macos", not(target_family = "wasm"))` for
  `window.remove_window()` vs the wasm controller route (Step 2.2).
  `toggle`/`is_open`/`init`-shell uncfg'd so tests exercise them everywhere.
  Close-state safety nets split the same way: macOS installs the
  window-close tracker, other platforms the `FloatingWindowRegistry`
  watcher. The non-macOS arm covers Windows, Linux, and wasm: native builds
  spawn a plan 02 float, wasm spawns an overlay pane through the same
  `spawn_floating` call.
- No cfg in the gallery section: every platform renders the same entry
  point; behavior differs underneath.

Capability-registry entries (written by `notch::init`, rendered by the
diagnostics page):

| Key | macOS | Windows/Linux | wasm |
| --- | --- | --- | --- |
| `notch_floater` | supported_enabled | degraded: "notch APIs are macOS-only; showing inset-anchored demo window" | degraded: "native notch unavailable on wasm; showing in-window overlay demo" |

Plan 02 separately registers `notch_geometry` (macOS supported, others
"macOS only", `02:904`); 03 reads it through `probe(cx)` and does not
duplicate it.

Native-only deps: none. This plan adds no manifest entry; every objc2 symbol
it touches (`NSView`, `NSWindow`, `NSWindowStyleMask`,
`NSWindowCollectionBehavior`, all under the already-enabled `NSWindow`/
`NSView` features, `Cargo.toml:243-247`; `NSScreen` + NSGeometry under
plan 02's feature addition) or kit/gpui item is already in the graph.
Nothing joins `[target.'cfg(not(target_family = "wasm"))'.dependencies]`
(`Cargo.toml:310-353`). The macOS table is inherently non-wasm.

Platform behavior matrix:

| Behavior | macOS | Windows/Linux | wasm |
| --- | --- | --- | --- |
| Window | own PopUp NSPanel (non-activating, level 101) converted to borderless, pinned over the notch strip | plan 02 float, screen-top anchored, inset 8px | plan 02 overlay pane at the same anchor |
| Expansion | hover or click; top-anchored `setFrame:display:animate:` | fixed window size; content swaps pill/grid | same |
| Focus | never key (`orderFront_`, non-activating panel) | `focus: false` (Linux best-effort per plan 02) | same window |
| Fullscreen of other apps | system hides the panel (no `FullScreenAuxiliary`) | n/a | n/a |
| Close | in-panel Close holds the panel's own window: `remove_window`; gallery toggle → controller `close` → handle `remove_window`; window-close tracker clears state on external closure | in-panel Close holds the float window: `remove_window`; registry watcher clears state for it, shell close, Esc; gallery toggle → controller `close` → `close_floating` | in-panel Close → controller `close` → overlay entry removal (the held window is the root window and must not close) |

## i18n

Crate facade `crate::i18n::localize` (`src/services/i18n.rs:3-5`); both
catalogs in lockstep (`tests/qa_localization.rs:188-198`). Gallery section
strings stay literal English (precedent
`src/gallery/sections/overlays/sheet.rs:452-455`). Capability reasons are
English literals (precedent `src/platform/input/shortcuts.rs:290-305`).
`notch_track_artist` ("GPUI Starter") is a proper noun and stays identical
in both locales.

| Key | en | zh-CN |
| --- | --- | --- |
| `notch_window_title` | "Notch floater" | "刘海浮窗" |
| `notch_close` | "Close" | "关闭" |
| `notch_collapse` | "Collapse" | "收起" |
| `notch_now_playing` | "Now playing" | "正在播放" |
| `notch_track_title` | "Demo track" | "演示曲目" |
| `notch_track_artist` | "GPUI Starter" | "GPUI Starter" |
| `notch_battery` | "Demo battery" | "演示电量" |
| `notch_time_label` | "Current time" | "当前时间" |

## Design Compliance

- Surfaces: the floater uses the popover family treatment
  (`cx.theme().popover` background, `border` hairline, `theme().radius`),
  matching plan 02's `FloatingShell` so every floating surface in the repo
  shares one treatment ("Surfaces and elevation": all surfaces of a kind
  share one treatment). No raw colors anywhere.
- Elevation: the panel relies on the OS window shadow plus the bottom
  hairline; no added shadows ("use elevation to explain stacking, not
  importance"). If the transparent window's system shadow misbehaves,
  `setHasShadow(false)` (`AK/NSWindow.rs:1366`) is the documented escape.
- Density: collapsed pill is a compact strip (`text_xs`, centered clock);
  expanded content row uses `p_4` panel padding and `gap_3` rhythm with
  `gap_4` between the three peer widget groups ("Radius, spacing, and
  density"; "Spatial grammar": one content group `md`, separate groups in
  one section `lg`); actions row `gap_2` (closely related controls).
- `px()` values appear only for window geometry (720x200, pill height from
  probe data, 360x140 demo), geometry that must match the physical screen,
  the documented exception in "Zoom, base font, and rem"; all in-view
  spacing and type use rem helpers (`p_4`, `gap_3`, `text_xs`).
- Interaction states: quick actions are kit `Button` ghost/small with the
  component's hover/pressed/focus-visible states; the collapsed pill shows
  a `chevron-up` affordance plus the pointer hover itself ("State must be
  visible"; hover is feedback, not the only cue; click pins, so the expand
  path does not depend on hover).
- Buttons: "Close" and "Collapse" are result verbs, sentence case, no
  ellipsis ("Buttons and confirmation dialogs"). Mock widgets carry captions
  ("Now playing", "Demo battery") so the demo never misstates mocked data as
  real ("Make the result understandable before the click").
- Motion: expansion animates through AppKit's `setFrame:display:animate:`
  only when `!cx.reduce_motion()`; collapse mirrors it ("Motion explains
  change"; "Honor reduced-motion preferences"). The pill/grid swap on
  fallback platforms is an immediate state change, no added animation.
- A11y: clock text carries `accessibility_label` ("Current time"); buttons
  have text labels; no color-only state. The surface is pointer-driven by
  nature (a non-activating panel never owns keyboard focus, which is the
  feature), so the keyboard/AT path to open and close it is the gallery
  button; this limitation is recorded in Risks, matching how the checklist's
  "every action is reachable and operable by keyboard" is met at the entry
  point rather than inside the panel.
- Platform fit: macOS gets the native behavior; other platforms keep the
  same information architecture (clock, media, battery, actions) in the demo
  float, per "Internationalization and platform fit": stable IA, honest
  capability reporting.

## Verification

Repo gates (AGENTS.md; run in Step 6.2 exactly as named):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

`just wasm-check` proves the wasm arm: the fallback branch compiles and the
capability reason path is cfg-selected (`justfile:24-25`, lib-only nightly
check). `tests/qa_migration.rs` must stay green (no `use gpui::` anywhere in
the new modules). No cargo gate was run during this planning session
(doc-only scope, same as the survey); the commands above are quoted from
`AGENTS.md` and the `justfile`.

Feature-specific manual QA (macOS flow `bash scripts/macos-dev-app.sh`,
`CONTRIBUTING.md:32-38`; requires a Mac; not runnable in this planning
session, listed as the implementer's checklist):

1. Gallery → Overlays → Notch Floater → "Open notch floater": a pill appears
   centered over the camera housing, clock readable, main app keeps focus.
2. Hover the pill: it expands downward to 720x200 with the three widget
   blocks and quick actions; leaving collapses it.
3. Click the pill: it stays expanded (pinned); clicking again or "Collapse"
   collapses it.
4. With another app frontmost, click the pill: that app stays active (no
   focus steal) and hover still expands (tracking area is `ActiveAlways`).
5. Make any other app's window fullscreen on the main display: the floater
   disappears; leaving fullscreen restores it.
6. "Close" quick action and the gallery toggle both close the floater; the
   gallery button label flips via the global observation.
7. Non-notched Mac: pill centers over the menu bar, height matches the menu
   bar.
8. Diagnostics page: `notch_floater` supported/enabled (macOS);
   `notch_geometry` values nonzero on a notched display.
9. Linux dev host: the same gallery button opens the 360x140 demo float
   top-center below the panel; hovering it swaps pill/grid at fixed size;
   capability row shows the inset-demo reason.
10. Wasm (`just wasm-build` + existing web dev flow): the overlay pane
    renders at the top-center anchor with the compact content; capability
    shows the overlay reason; the in-panel Close removes the pane without
    closing the app window, and the gallery button label flips (registry
    watcher path).
11. Linux dev host, close paths: close the demo float from plan 02's
    FloatingShell header (or Esc) without touching the gallery button; the
    gallery label still flips to "Open notch floater" (registry watcher),
    and `is_open` is false.

Acceptance criteria:

- All four gates pass with only the Seam Touches files changed.
- The Interface Contract items exist with the exact signatures
  (grep-checkable: `pub fn toggle`, `pub fn is_open`, `pub fn init`,
  `CAPABILITY_NOTCH_FLOATER`).
- On macOS the floater never becomes the key window and never appears above
  another app's fullscreen window (manual QA 4-5).
- `notch_floater` appears in the diagnostics capability list on every
  platform with the table's status.

## Risks & Open Questions

Ranked:

1. **macOS compile path unverified here.** The dev host is Linux; every
   `macos.rs` / `#[cfg(target_os = "macos")]` claim is source-verified
   (citations in Research Digest) but never compiled in this session, and no
   `cargo check --target *-apple-darwin` toolchain was probed. Tried: none
   beyond reading; cross-compiling needs an Apple target + SDK not present.
   Mitigation: the reach-in mirrors `src/platform/liquid_glass.rs:230-240`
   line-for-line, every objc2 signature was read from the generated sources,
   and `Pixels::to_f64` is verified (`GPUI/geometry.rs:2884`); first macOS
   CI/dev build may still need mechanical fixes (import paths, borrow shape).
2. **Borderless conversion at runtime is source-verified, not
   runtime-verified.** `setStyleMask` (`AK/NSWindow.rs:795`) with
   `Borderless | from_bits_retain(1 << 7)` follows gpui's own const
   construction (`MAC/window.rs:95-96`) and gpui's own runtime borderless
   flip (`MAC/window.rs:644`), but whether the flip preserves the PopUp
   level, tracking area, and transparency in all macOS versions is a runtime
   property. Tried: read the creation path where level/tracking are set
   (`MAC/window.rs:1205-1240`); they are set on the window/view, not the
   style mask, so the flip should not disturb them. Fallback if QA 1-2
   shows chrome or lost hover: drop the `setStyleMask` call (keep
   `titled|fullSizeContentView` (no traffic lights either way, only
   possible corner rounding): a one-line deletion in `apply_notch_policy`.
3. **Fullscreen auto-hide relies on `FullScreenAuxiliary` semantics not read
   from Apple's primary page.** Tried this session: `WebFetch` on the
   Apple documentation URL returns HTTP 404 (bot-blocked); the generated
   objc2-app-kit doc comments do not carry the sentence. Corroboration: web
   search quoting Apple's page ("The window displays on the same space as
   the full screen window") plus community reports of `canJoinAllSpaces`
   windows failing to stay visible over fullscreen without the flag:
   [Apple's fullScreenAuxiliary page on developer.apple.com](https://developer.apple.com/documentation/appkit/nswindow/collectionbehavior/fullscreenauxiliary),
   Monkeybread constants listing (NSWindowCollectionBehaviorFullScreenAuxiliary = 256,
   matching `AK/NSWindow.rs:167`), and an Electron `visibleOnAllWorkspaces`
   bug report. If manual QA 5 fails on hardware, the fallback is: keep
   gpui's collection behavior, subscribe to `NSWorkspace`
   active-space/frontmost-app notifications (needs the `NSWorkspace`
   feature + `block2` observer plumbing) and `orderOut`/`makeKeyAndOrderFront`
   the panel; that change stays inside `macos.rs` + one feature line.
4. **Hover on a non-key panel.** gpui installs a tracking area with
   `ActiveAlways` for PopUp windows (`MAC/window.rs:1229`, `:2488-2513`, the
   code comment states the intent), but end-to-end
   hover-while-another-app-is-active is a runtime property this session
   cannot run. Mitigation: manual QA 2/4; the click-pin path expands even if
   hover never fires (the pill is a click target either way).
5. **System shadow around the transparent panel.** With
   `WindowBackgroundAppearance::Transparent` the backend sets
   `setOpaque_(NO)` + alpha-0.0001 background (`MAC/window.rs:1862-1874`);
   macOS normally computes the window shadow from opaque content, so the
   shadow should hug the pill, but this is runtime behavior. Fallback: one
   `setHasShadow(false)` call (`AK/NSWindow.rs:1366`) in
   `apply_notch_policy`.
6. **Battery and now-playing are mocks.** No in-tree binding for
   IOPowerSources or MediaRemote; adding one breaks the repo's dependency
   rules for a demo. Widgets are labeled as demos. Real battery is a
   separate feature with its own crate pick; out of scope.
7. **Plan 02 must be implemented first; its contract is the only coupling.**
   If 02's contract changes (e.g. `NotchGeometry` fields), 03's probe reads,
   the fallback spec, and the `FloatingWindowRegistry` watcher break at
   compile time; acceptable, both plans live in the same repo and review
   together.
8. **Clock timer keeps a weak-entity loop alive.** The 1s loop detaches and
   exits when the view drops (`this.update` returns `Err`); while open it
   wakes once per second; bounded work, and `cx.notify()` fires only on
   minute change ("Keep animation work bounded").
9. **Fallback float content state on `toggle`.** Closing and reopening the
   non-macOS demo rebuilds the view (plan 02's retained-factory respawn
   semantics); clock state resets, which is invisible since it re-reads the
   wall clock. Not a defect; recorded so an implementer does not try to
   preserve it.
