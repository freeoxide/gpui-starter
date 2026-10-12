# 06 Infinite Canvas Plan

## Goal

A sidebar page mounts plan 01's canvas engine over a procedurally generated field of sticky notes and dot-grid cards (1k/5k/20k presets) with unbounded pan (wheel, space-drag, middle-drag, platform pinch), zoom-to-cursor clamped to 0.1x–8x, engine viewport culling plus label LOD so interaction stays smooth at thousands of items. A minimap shows the whole field as dots plus the live viewport rect and supports click-to-jump; toolbar buttons give zoom out/in, zoom-to-fit, view reset, grid toggle, and regenerate. A perf budget with measurement steps (page-level timing atomics mirroring `src/shell/root/frame_time.rs`) makes the 60fps target checkable.

## Dependencies

Plan `01-canvas-foundation.md`. The engine is still contract text, not code: `src/ui/` contains only `mod.rs` + `widgets/` (verified this session with `ls`). Every item below is consumed from 01's Interface Contract verbatim; sibling consumers already on disk are 05, 07, 08, 10 (read in 01's Dependencies note). Items consumed:

- `CanvasState` entity: `new`, `transform`, `pan`, `zoom`, `pan_by`, `zoom_at`, `set_zoom`, `zoom_to_fit`, `reset_view`, `grid`/`set_grid`, `document`, `document_revision`, `edit_document`, `set_layer_visible`, `set_tool`, `set_builtin_tool(BuiltinTool::Pan)`, `active_tool_id`; the observe-driven repaint contract (`cx.observe(&state, |_, _, cx| cx.notify())`).
- `Canvas` element: `Canvas::new(id, Entity<CanvasState>)`, fills parent, `BlockMouse` hitbox, engine-side paint culling and grid LOD.
- Scene API: `CanvasShape::{rect, text}`, `ShapeStyle::both`, `StrokeStyle`, `doc_bounds`, `CanvasDocument::{clear, add_layer, add_shape, layer}` (`is_visible`), `LayerId`, `CanvasGrid`.
- Tool protocol: `trait CanvasTool` (pointer/scroll methods taking `&mut CanvasState` + `&mut Context<CanvasState>`, default `scroll -> false`), `CanvasScrollEvent`, `ToolContext`, `pub(crate) dispatch_tool_event` (test-only dispatch, same-crate callable per 05's Research Digest), router rule "middle-drag always pans; wheel defaults apply only when the tool does not consume the scroll".
- Constants `MIN_ZOOM = 0.05`, `MAX_ZOOM = 32.0` (engine clamp; this page clamps tighter, Phase 3).

Coordinate spaces: 01's current revision pins viewport-space (element-local) semantics end to end — events converted by `position - bounds.origin`, everything downstream element-local, painters re-adding `bounds.origin` once (01 contract "Router rules" bullet; the ambiguity this plan originally flagged is resolved in 01's Risk 1). This plan still funnels every window-space input through two seams (`local_viewport` in `viewport.rs`, `minimap` projection in `minimap.rs`) and keeps a window-level regression test (Phase 5.2) so a router regression surfaces as a test failure, not a visual offset.

Page-seam coexistence: plan 05 (`05-canvas-showcase.md`) also appends a `Page` variant after `Page::Gallery`. Both appends are safe together: digit keys bind only `i < 9` (`src/shell/root/app_root/state.rs:144-166`), so indices ≥ 10 rebind nothing. The e2e count is bumped by one from its current literal, whatever plans landed first.

No other dependencies. No new crates: `gpui-kit 0.7.1` (pinned facade) and workspace deps only; `Cargo.toml`/`Cargo.lock` untouched.

## Research Digest

Verified this session against `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` (short names `GPUI` = `gpui-pre-0.3.8/src`, `KIT` = `gpui-kit-0.7.1/src`, `GC` = `gpui-component-0.7.1/src`, `GB` = `gpui-base-0.7.1/src`) and the repo.

Pinch:

- `PinchEvent { position: Point<Pixels>, delta: f32, modifiers, phase: TouchPhase }`; `delta` doc: "0.1 represents a 10% zoom increase"; `Deref<Target = Modifiers>` (`GPUI/interactive.rs:571-601`).
- Native producers, all verified by reading the emit sites: macOS `NSEventTypeMagnify` → `delta = magnification()` (`gpui-pre-macos-0.3.8/src/events.rs:238-258`); X11 `XinputGesturePinch{Begin,Update,End}` → `PlatformInput::Pinch` (`gpui-pre-linux-0.3.8/src/linux/x11/client.rs:1446-1486`); Wayland `zwp_pointer_gesture_pinch_v1` Dispatch impl builds `PinchEvent` (`gpui-pre-linux-0.3.8/src/linux/wayland/client.rs:2652-2670`); Windows DirectManipulation `GestureKind::Pinch` → `PlatformInput::Pinch` (`gpui-pre-windows-0.3.8/src/direct_manipulation.rs:133, 180-184`).
- Wasm: `gpui-pre-web-0.3.8/src/platform.rs` has zero hits for `Pinch`, `TouchEvent`, or touch forwarding (grep run this session). Core can synthesize gestures from raw `TouchEvent`s (`GPUI/window.rs:5668-5678`, `dispatch_touch_event` runs `TouchGestureRecognizer`), but the web platform never forwards touches, so no synthesis input exists. Pinch therefore never fires on wasm; the handler is platform-neutral code and simply stays idle.
- Runtime gesture-capability query is unreachable: `Platform::gestures()` exists (`GPUI/platform.rs:440-443`) and `GestureKinds { pinch: bool }` (`GPUI/gestures.rs:357-363`), but `App` exposes no public platform accessor (grep `pub fn platform` in `GPUI/app.rs`: no hits). Pinch availability is compile-time knowledge per platform (producers above); no capability-registry entry is possible or useful.

Keyboard and focus:

- `KeyDownEvent { keystroke, is_held, prefer_character_input }` (`GPUI/interactive.rs:24-41`), `KeyUpEvent { keystroke }` (`:44-49`). Space key string is `"space"` (`GPUI/platform/keystroke.rs:249`).
- `window.on_key_event::<E: KeyEvent>(Fn(&E, DispatchPhase, &mut Window, &mut App))`: paint-phase only (`debug_assert_paint`), listener cleared when the next frame renders, so it must be re-registered every paint (`GPUI/window.rs:5317-5333`).
- `FocusHandle::contains_focused(&self, window, cx)` (`GPUI/window.rs:513`), `FocusHandle::focus(&self, window, cx)` (`:610`), `Window::focused(cx)` (`:2329`). Repo `track_focus` precedent: `src/shell/root/app_root/render.rs:225-227`.

Elements and paint:

- `canvas(prepaint, paint)` value element: prepaint `FnOnce(Bounds<Pixels>, &mut Window, &mut App) -> T`, paint `FnOnce(Bounds<Pixels>, T, &mut Window, &mut App)` (`GPUI/elements/canvas.rs:10-22`; exported at `GPUI/elements/mod.rs:17` and through the kit glob; repo import `use gpui_kit::{..., canvas, ...}` at `src/gallery/sections/theme/brush.rs:4-20`). It inserts no hitbox and registers no listeners — overlay usage precedent `brush.rs:239-301`.
- `ElementExt::on_prepaint(callback: FnOnce(Bounds<Pixels>, &mut Window, &mut App))` — an absolute full-size `canvas()` child; blanket impl for every `ParentElement` (`GB/element_ext.rs:14-26`, re-export `GB/lib.rs:109`; import as `gpui_kit::component::ElementExt`, repo precedent `brush.rs:6`). Resolved bounds are window-space. Storing them into an entity without notify is the kit's own pattern: `GC/menu/popup_menu.rs:1505` `.on_prepaint(move |bounds, _, cx| view.update(cx, |r, _| r.bounds = bounds))`.
- Hitboxes insert in prepaint (`GPUI/window.rs:5157`); `BlockMouse` makes hitboxes behind report not-hovered (`:888-910`). Div mouse/pinch listeners are window-level wrappers that always invoke the listener; hover filtering is the listener's job via the hitbox (`GPUI/elements/div.rs:2828-2843`). A div behind the engine's `BlockMouse` hitbox therefore cannot rely on `is_hovered`; window-level `on_mouse_event` listeners with manual bounds checks are the correct page-side shape (same technique as 01's router).
- Paint: `window.paint_quad(quad)` (`GPUI/window.rs:4550`), `fill(bounds, background)` (`:7694`); `Bounds::{intersects :965, intersect :1143, center :1001, contains :1468}`, `Bounds` derives `PartialEq` (`:720`). `Hsla::opacity` (`GPUI/color.rs:637`). `Pixels * f32` exists; `Pixels / f32` does not, so divide through `as_f32()` (01 Research Digest).

Theme tokens (pub `ThemeColor` fields, `GC/theme/theme_color.rs`): `accent :61`, `background :67`, `border :69`, `danger :149`, `foreground :165`, `info :167`, `muted_foreground :197`, `popover :199`, `primary :203`, `secondary :221`, `success :251`; `radius_lg` (`GC/theme/mod.rs:187`). `cx.theme()` on `&App` via `gpui_kit::component::ActiveTheme` (`GC/theme/mod.rs:44-50`).

Kit components:

- `Button::new(id)` (`GC/button/button.rs:241`), `.outline() :332`, `.label :371`, `.accessibility_label :391`, `.icon(impl Into<ButtonIcon>) :397` — an `IconName` converts directly, `src/gallery/sections/buttons/toolbar.rs:82-86` passes `IconName` without wrapping, `.tooltip :403`, `.compact :441`, `.on_click :447`, `.toggled :497`; `ghost` is the variant trait method `:82`.
- Icons verified in the component enum the app registers — `gpui-kit-assets-0.7.1/default-icons.txt` cross-checked against the generated `component::IconName` (`target/debug/build/gpui-kit-assets-*/out/icon_name.rs`): `frame`, `maximize`, `refresh-cw`, `plus`, `minus`, `map`, `layout-dashboard` are all in the default 106. The svg files `zoom-in`/`zoom-out`/`locate-fixed`/`layout-grid` exist only in the full `gpui_kit::assets::IconName` catalog, whose bytes the app's default asset source does NOT register — those variants are unusable here (compile error via `component::IconName`, missing icon bytes via the assets enum). Used: `IconName::Frame` (page), `Plus` (zoom in), `Minus` (zoom out), `Maximize` (fit), `Map` (reset view), `LayoutDashboard` (grid toggle), `RefreshCw` (regenerate).
- accesskit 0.24.1 (kit re-export, `tests/qa_a11y.rs:6` pattern): `Role::Image` at `accesskit-0.24.1/src/lib.rs:68`, `Role::Group` at `:152`. Repo a11y builders: `.a11y(Role, label)`, `.aria_level`, `.a11y_live` via the repo's `A11yExt` (`src/features/pages/notifications.rs:48-54`; the `accesskit::Live` path there shows the re-exported crate name works with the kit glob import).

Modifiers: `Modifiers::secondary()` = platform key on macOS, control elsewhere (`GPUI/platform/keystroke.rs:483`); matches the repo's `secondary-` binding convention (`src/app/keys.rs`) and 01's "ctrl/cmd+wheel" router intent.

Styling and subscription arities (verified after review):

- `Styled` size/inset helpers are macro-generated from a fixed suffix token set `{0, 0p5, 1, 1p5, 2, 2p5, 3, 3p5, 4, 5, 6, 7, 8, 9, 10, 11, 12, 16, 20, 24, 32, 40, 48, 56, 64, 72, 80, 96, 112, 128}` plus fractions/`px`/`full` (`gpui-pre-macros-0.3.8/src/styles.rs`, `box_style_suffixes` suffix strings, grepped this session). There is no `h_30`; `h_24` (96 px), `w_40`, `w_12` exist. The minimap uses `w_40().h_24()`.
- `Context::observe` closure arity is `FnMut(&mut T, Entity<W>, &mut Context<T>)` — three parameters (`GPUI/app/context.rs:63-70`); observers are wired `|this, _, cx|`.
- `FocusHandle` is cloneable via a manual `impl Clone` (`GPUI/window.rs:648`), so listeners clone the handle before calling `focus(window, cx)` on it (an entity `read` borrow is alive across the call otherwise).

Repo patterns:

- Page seam rows 1–15: `docs/plans/_survey/seams.md` §1, re-checked against `src/shell/sidebar.rs`, `src/shell/route.rs:10-21`, `src/shell/root/app_root/state.rs:22-31, 47-56, 211-223`, `tests/e2e_navigation.rs:12-19` this session; current count is 10.
- frame_time pattern: `SLOW_FRAME_THRESHOLD_US = 4_000`, `LAST_FRAME_TIME_US: AtomicU64`, `store_frame_time`/`last_frame_time_us` (`src/shell/root/frame_time.rs:1-25`); measured with `crate::platform::clock::Instant` (`render.rs:32-33`) and stored in `AppRoot::render` (`src/shell/root/app_root/render.rs:270-283`, store + slow-frame warn); status-bar readout `#[cfg(debug_assertions)]`, gated on config `show_frame_time` (`src/shell/status_bar.rs:228-254`, `"Frame: {ms:.2}ms"` at `:238`; config field `src/state/config_store.rs:88-89`); re-export path `crate::root::last_frame_time_us()` (`src/shell/root/mod.rs:7`).
- Wasm-safe clock: `crate::platform::clock::Instant` re-exports `std::time` native / `web_time` wasm (`src/platform/clock.rs:1-8`); `std::time` clocks panic at runtime on wasm.
- i18n facade: `crate::i18n::localize("key")`, `localize_with_args("key", &[("count", v)])` with `%{count}` (`src/services/i18n.rs`); catalogs must stay in lockstep (`tests/qa_localization.rs:188-198`); nav block `locales/en.yml:53-62`.
- Page construction `pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self` (`src/features/pages/notifications.rs:12`); directory-page pattern `src/features/pages/diagnostics/`.
- Tests: sibling `#[cfg(test)] #[path = "x.test.rs"] mod x_test;`; `#[gpui_kit::test]` with `TestAppContext` in-crate (`src/state/config_store.test.rs:280-281`; dev-dep gpui `test-support` at `Cargo.toml:213-216`); `VisualTestContext` mouse simulation (`GPUI/app/test_context.rs:894-940`); `simulate_window_resize` (`:461`); `simulate_keystrokes` fires complete down+up sequences (`:587`), so a held-key drag is not simulable; space-pan is tested at the handler level plus manual QA.
- Snapshot schema: default config pins `active_route: Page: Home` only (`tests/snapshot_tests.rs:24-45`); `Page` derives `Serialize, Deserialize` (`src/shell/sidebar.rs:4`), so `InfiniteCanvas` becomes a valid persisted `active_route` value with no migration.

Crate picks: none new. `gpui-kit 0.7.1` plus existing workspace deps; `Cargo.toml`/`Cargo.lock` untouched.

## Architecture

Entities and ownership:

- `AppRoot` owns `Entity<InfiniteCanvasPage>` (page seam). The page is the only entity this feature creates.
- `InfiniteCanvasPage` owns: `state: Entity<CanvasState>` (engine), `field: NoteField` (generator parameters), `layers: (LayerId, LayerId)` (bodies, labels), `palette: [Hsla; 5]` (raw theme roles, snapshotted at build; card fills apply `opacity(0.85)` at build time, minimap dots use the raw roles), `viewport: Bounds<Pixels>` (window-space canvas-area bounds, probe-updated), `minimap_bounds: Bounds<Pixels>` (window-space, measured by the minimap's own `on_prepaint`), `space_pan: bool`, `preset_ix: usize`, `subscriptions: Vec<Subscription>`, `focus_handle: FocusHandle` (canvas-area focus scope).
- Engine document layout: layer "bodies" = one `CanvasShape::rect` per note plus six small dot rects per dot-grid card; layer "labels" = one `CanvasShape::text` per sticky note. LOD = `set_layer_visible(labels, zoom >= LABEL_LOD_ZOOM)`; paint-side culling, grid LOD, and path caching are the engine's (plan 01), and the page never iterates shapes during render.

Events and data flow:

- Pointer/wheel reach 01's router. The page installs `ExploreTool` as the steady-state tool: left-drag pans, plain wheel pans, `secondary()`+wheel zooms at the cursor through the page clamp; it consumes every scroll so the engine's 0.05–32 default zoom clamp never applies. Middle-drag pans at router level regardless of tool (01 router rule).
- Page-side window-level listeners are registered each paint by the interaction probe (a hitbox-free `canvas()` overlay stacked above the engine element): `PinchEvent` → page-clamped `zoom_at` anchored at `event.position - viewport.origin`; `KeyDownEvent`/`KeyUpEvent` for `"space"` → swap tool to `BuiltinTool::Pan` while held, restore `ExploreTool` on release, guarded by the page focus scope so a focused HUD button keeps normal Space activation; `MouseDownEvent` inside the viewport (and outside the minimap) → `focus_handle.focus` so space-pan works after a canvas click.
- Engine → page: `cx.observe(&state, ...)` runs `reconcile_lod` (change-guarded `set_layer_visible`) then `cx.notify()`. The guard is mandatory: `set_layer_visible` notifies per call (01 contract), so an unconditional call inside the observer is a permanent notify loop.
- Bounds bookkeeping: the canvas area's `on_prepaint` stores `viewport`, the minimap wrapper's `on_prepaint` stores `minimap_bounds`; both are change-guarded, notify-free stores (every bounds change coincides with a system-driven redraw; `popup_menu.rs:1505` precedent). The minimap's `on_prepaint` child is its first child, so the bounds land before the minimap's paint child in the same frame.
- HUD buttons → engine through page methods: `zoom_view`, `fit_view`, `reset_view`, `toggle_grid`, `regenerate`.
- Regeneration: one `edit_document` batch (`doc.clear()`, two `add_layer`, N `add_shape`) = one revision bump, one `DocumentChanged`, one notify (01 contract); duration measured with `crate::platform::clock::Instant` and stored in `perf.rs`.
- Perf: `render()` duration stored per frame in `perf.rs` (mirrors `frame_time.rs`); the root-level number stays available at `crate::root::last_frame_time_us()`.

Module tree:

```text
src/features/pages/infinite_canvas/
├── mod.rs           page seam: InfiniteCanvasPage + Render (HUD, canvas area, minimap),
│                    removal-recipe header, module decls
├── content.rs       NoteField/NoteSpec/NoteKind, splitmix64, layout + dot math, build_field()
├── explore_tool.rs  PAGE_MIN_ZOOM/PAGE_MAX_ZOOM/LABEL_LOD_ZOOM, page_factor, ExploreTool
├── viewport.rs      local_viewport(), interaction probe (space/pinch/focus), space-pan state
├── minimap.rs       projection math + minimap element (dots, viewport rect, click-to-jump)
├── perf.rs          LAST_PAGE_RENDER_US / LAST_GEN_US atomics + threshold (frame_time pattern)
└── *.test.rs        content, explore_tool, minimap, perf, page integration
```

## Interface Contract

Leaf feature: nothing downstream consumes it. The public surface is the page type plus the app-shell integration.

```rust
// src/features/pages/infinite_canvas/mod.rs
use gpui_kit::{prelude::*, *};

pub struct InfiniteCanvasPage { /* fields private */ }

impl InfiniteCanvasPage {
    /// Mounts the engine, builds the default 5,000-note field, installs
    /// ExploreTool. Constructed once by AppRoot; nothing else required.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self;
}

impl Render for InfiniteCanvasPage { /* HUD + canvas area + minimap */ }

// App-shell surface (seam touches below):
//   crate::sidebar::Page::InfiniteCanvas  (title "Infinite Canvas", key nav_infinite_canvas,
//     host "infinite-canvas", icon IconName::Frame, appended after Page::Gallery)
//   AppRoot.infinite_canvas_page: Entity<InfiniteCanvasPage> + unchecked_active_page_view arm

// pub(crate) within the feature module (not exported past the page):
//   content::{NoteField, NoteSpec, NoteKind, build_field, note_palette,
//             NOTE_W, NOTE_H, NOTE_GAP, PALETTE_LEN, DOT_SIZE, dot_offsets}
//   explore_tool::{ExploreTool, PAGE_MIN_ZOOM, PAGE_MAX_ZOOM, LABEL_LOD_ZOOM,
//                  clamp_zoom, page_factor}
//   viewport::{local_viewport, interaction_probe}
//   InfiniteCanvasPage inherent methods (mod.rs, private): set_space_pan,
//     reconcile_lod, store_viewport, store_minimap_bounds, zoom_view, fit_view,
//     reset_view, toggle_grid, regenerate; consts PRESETS, DEFAULT_PRESET_IX
//   minimap::{minimap, MiniProjection, fit_projection,
//             project_doc_to_minimap, unproject_minimap_to_doc}
//   perf::{store_page_render_us, last_page_render_us, store_gen_us, last_gen_us,
//          PAGE_SLOW_RENDER_THRESHOLD_US}
```

Consumption notes: constructed once in `AppRoot::new` like every other page; deep link `gpui-starter://infinite-canvas` routes to it; persisted `active_route` accepts the new variant with no config migration.

## Module Layout

| File | Purpose |
|---|---|
| `src/features/pages/infinite_canvas/mod.rs` | Page seam, render tree (HUD; canvas area with viewport probe + focus scope + `Canvas` mount + interaction probe + minimap), zoom/fit/reset/grid/regenerate handlers, LOD reconcile, removal-recipe header |
| `src/features/pages/infinite_canvas/content.rs` | Deterministic field generator: `splitmix64`, `NoteField`, `NoteSpec`, `NoteKind`, span/layout and dot-offset math, `build_field` (single-batch document populate), `note_palette` |
| `src/features/pages/infinite_canvas/explore_tool.rs` | Page zoom clamps (`0.1..8.0`), `page_factor`, `ExploreTool` (`CanvasTool` impl: drag pan, secondary+wheel clamped zoom-at-cursor, wheel pan, consumes all scrolls) |
| `src/features/pages/infinite_canvas/viewport.rs` | Window↔element-local conversion, interaction probe element (space keydown/keyup, pinch, click-to-focus), space-pan tool swap |
| `src/features/pages/infinite_canvas/minimap.rs` | Projection math and minimap element: note dots, live viewport rect, click-to-jump, self-measured bounds |
| `src/features/pages/infinite_canvas/perf.rs` | Atomic last-timing stores + slow threshold, mirroring `src/shell/root/frame_time.rs` |
| `src/features/pages/infinite_canvas/*.test.rs` | Sibling test modules per repo convention |

## Seam Touches

All edits outside `src/features/pages/infinite_canvas/` (page-seam rows from seams §1):

| # | File | Edit |
|---|---|---|
| 1 | `src/features/pages/mod.rs` | `mod infinite_canvas;` + `pub use infinite_canvas::InfiniteCanvasPage;` |
| 2 | `src/shell/sidebar.rs` | `Page::InfiniteCanvas` variant; arms in `title()` ("Infinite Canvas"), `title_key()` (`nav_infinite_canvas`), `host()` (`infinite-canvas`), `icon()` (`IconName::Frame`); `all()` entry appended last (index 10+; digit keys bind only `i < 9`, `state.rs:144-166`) |
| 3 | `src/shell/route.rs` | `Page::InfiniteCanvas.host(),` in `VALID_HOSTS` (`route.rs:10-21`) |
| 4 | `src/shell/root/app_root/state.rs` | Field `infinite_canvas_page: Entity<InfiniteCanvasPage>`; construction `cx.new(|cx| InfiniteCanvasPage::new(window, cx))`; import; `unchecked_active_page_view` arm |
| 5 | `tests/e2e_navigation.rs` | Count literal +1 from its current value (10 today; 11 if plan 05 landed first — both plans append); add `Page::InfiniteCanvas` to the `expected` array; update the count comment |
| 6 | `locales/en.yml`, `locales/zh-CN.yml` | `nav_infinite_canvas` + the `canvas_*` keys from the i18n table, identical key sets in both files |

Deliberate non-touches: no `Cargo.toml`/`Cargo.lock` change (qa_migration lock check, seams §9); no command-palette entry (page reachable via sidebar, context menu, deep link; `CommandId` additions are optional per seams §1 and skipped); no capability-registry entry (see Platform & Wasm Gating); no gallery section; no `src/ui/` change (engine edits belong to plan 01).

### Removal Recipe

`mod.rs` opens with this header (mirrors `src/gallery/mod.rs:1-7`):

```rust
//! Removable infinite-canvas page over the ui::canvas engine.
//! `InfiniteCanvasPage` is the only public surface. Removal = delete this
//! directory and revert its six seam touches: the `mod`/`pub use` lines in
//! features/pages/mod.rs, the Page::InfiniteCanvas arms and all() entry in
//! shell/sidebar.rs, the VALID_HOSTS entry in shell/route.rs, the AppRoot
//! wiring in shell/root/app_root/state.rs, the page count in
//! tests/e2e_navigation.rs, and the nav_infinite_canvas/canvas_* keys in
//! locales/en.yml and locales/zh-CN.yml.
```

Ordered removal steps:

1. Revert `src/features/pages/mod.rs` (drop `mod infinite_canvas;` and the re-export).
2. Revert `src/shell/sidebar.rs` (variant + five arms + `all()` entry).
3. Revert `src/shell/route.rs` (`VALID_HOSTS` entry).
4. Revert `src/shell/root/app_root/state.rs` (field, construction, import, view arm).
5. Revert `tests/e2e_navigation.rs`: count back by one, drop the `expected` entry.
6. Remove the `nav_infinite_canvas` and `canvas_*` keys from both locale files.
7. `rm -r src/features/pages/infinite_canvas/`.
8. Grep `InfiniteCanvas\|infinite-canvas\|infinite_canvas` across `src/ tests/ locales/`: zero hits required.
9. Run the Verification gates; all must pass.

## Implementation Phases

Plan 01 (`src/ui/canvas/`) is implemented and green before Phase 1.

## Phase 1: Page seam scaffold

### Step 1.1: Locale keys

- Files: `locales/en.yml`, `locales/zh-CN.yml`.
- Build: add the eleven keys from the i18n table (nav key in the `nav_*` block after `nav_gallery`; `canvas_*` block appended near the other page blocks; same order in both files). Values exactly as in the i18n section.
- Tests: none new (`tests/qa_localization.rs` is the guard).
- Done-check: `cargo test -p gpui-starter --test qa_localization` passes.

### Step 1.2: Page variant, route, page shell, and AppRoot wiring (one commit-sized unit)

Single step on purpose: the `Page` enum gains an exhaustive-match arm in `state.rs` the moment the variant exists, or the lib does not compile between the two edits — a split step's intermediate done-check cannot pass.

- Files: `src/shell/sidebar.rs`, `src/shell/route.rs`, `tests/e2e_navigation.rs`, `src/features/pages/infinite_canvas/mod.rs` (new; header comment from the Removal Recipe), `src/features/pages/mod.rs`, `src/shell/root/app_root/state.rs`.
- Build, in order within one edit pass: `InfiniteCanvasPage` with `pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self` whose struct is empty for now (fields land in Phase 4; an empty struct compiles, no placeholder field needed); `impl Render` returning `v_flex().size_full().p_6().child(crate::i18n::localize("nav_infinite_canvas"))` as a placeholder. `mod`/`pub use` in `features/pages/mod.rs`. Then `InfiniteCanvas` variant last in the `Page` enum; `title()` → `"Infinite Canvas"`; `title_key()` → `"nav_infinite_canvas"`; `host()` → `"infinite-canvas"`; `icon()` → `IconName::Frame`; `all()` entry appended after `Page::Gallery` (if plan 05 already appended `Page::CanvasShowcase`, append after that — last position either way). `VALID_HOSTS` entry. Field `infinite_canvas_page: Entity<InfiniteCanvasPage>` + construction `cx.new(|cx| InfiniteCanvasPage::new(window, cx))` + import + `unchecked_active_page_view` arm in `state.rs`, copying the `notifications_page` rows (`state.rs:25, 50, 216`). Bump the e2e count literal by one, extend `expected`, fix the count comment.
- Tests: extend the parse cases in `test_route_parsing` with `("gpui-starter://infinite-canvas", AppRoute::Page(Page::InfiniteCanvas))`.
- Done-check: `cargo test -p gpui-starter --test e2e_navigation` passes; `cargo check -p gpui-starter` passes; `grep -n "infinite-canvas" src/shell/route.rs` prints the new line; the sidebar shows the placeholder page (later phases replace it).

## Phase 2: Content generator and document build

### Step 2.1: Generator math

- Files: `src/features/pages/infinite_canvas/content.rs`, `content.test.rs`; `mod.rs` gains `mod content;` + `#[cfg(test)] #[path = "content.test.rs"] mod content_test;`.
- Build (all `pub(crate)`): consts `NOTE_W: Pixels = px(140.)`, `NOTE_H: Pixels = px(90.)`, `NOTE_GAP: Pixels = px(40.)`, `PALETTE_LEN: usize = 5`, `DOT_SIZE: Pixels = px(10.)`; `fn splitmix64(x: u64) -> u64` (standard mixer: `x = x.wrapping_add(0x9E3779B97F4A7C15); let mut z = x; z = (z ^ z>>30).wrapping_mul(0xBF58476D1CE4E5B9); z = (z ^ z>>27).wrapping_mul(0x94D049BB133111EB); z ^ z>>31`); `#[derive(Clone, Copy, Debug, PartialEq, Eq)] enum NoteKind { Sticky, DotCard }`; `struct NoteSpec { pub bounds: Bounds<Pixels>, pub palette_ix: usize, pub kind: NoteKind, pub label: String }`; `struct NoteField { pub seed: u64, pub count: usize }` with:
  - `cols() -> usize` = `((count as f32).sqrt().ceil() as usize).max(1)`
  - `rows() -> usize` = `(count + cols - 1) / cols`
  - `step() -> f32` = `(NOTE_W + NOTE_GAP).as_f32()`
  - `span() -> Bounds<Pixels>` = origin `(0,0)`, size `(px(cols as f32 * step + NOTE_GAP.as_f32()), px(rows as f32 * step + NOTE_GAP.as_f32()))`
  - `spec(ix) -> Option<NoteSpec>`: `None` when `ix >= count`; else `col = ix % cols`, `row = ix / cols`, `h = splitmix64(seed ^ splitmix64(ix as u64 + 1))`, `rx = (h as u32) as f32 / u32::MAX as f32`, `ry = ((h >> 32) as u32) as f32 / u32::MAX as f32`, origin `px(col as f32 * step + NOTE_GAP.as_f32() + rx * NOTE_GAP.as_f32())` / same for y with `ry`, size `(NOTE_W, NOTE_H)`, `palette_ix = ((h >> 48) as usize) % PALETTE_LEN`, `kind = if ix % 4 == 3 { DotCard } else { Sticky }`, `label = if Sticky { format!("Note {ix}") } else { String::new() }`.
  - `fn dot_offsets() -> [Point<Pixels>; 6]` = xs `{12., 65., 118.}` × ys `{12., 65.}` (3×2 grid; rightmost dot edge `118+10=128 < 140`, bottom `65+10=75 < 90`).
- Tests (`content.test.rs`, plain `#[test]`): same seed+ix → identical `spec` (compare `f32::to_bits` of each bounds field, `palette_ix`, `kind`); different seed → different bounds for ix 0; `cols`/`rows` invariants for counts 1, 5_000, 20_000 (`cols*rows >= count && (cols-1)*rows < count`); every `spec(ix).bounds` lies inside `span()`; `palette_ix < PALETTE_LEN`; `spec(count)` is `None`; `kind == DotCard` exactly for `ix % 4 == 3`; every `dot_offsets()` entry + `DOT_SIZE` stays inside `(NOTE_W, NOTE_H)`.
- Done-check: `cargo test -p gpui-starter --lib infinite_canvas::content` passes.

### Step 2.2: Document build

- Files: `content.rs` (extend), `content.test.rs`.
- Build: `fn note_palette(cx: &App) -> [Hsla; PALETTE_LEN]` = `[t.accent, t.primary, t.secondary, t.info, t.success]` via `gpui_kit::component::ActiveTheme` (raw roles; note colors are generated data, exempt from the token rule per 01's Design Compliance); `fn build_field(state: &mut CanvasState, field: &NoteField, cx: &mut Context<CanvasState>) -> (LayerId, LayerId)`: read `palette`, `border = cx.theme().border`, `text_color = cx.theme().foreground.opacity(0.9)` before the edit; then one `state.edit_document(cx, |doc| { doc.clear(); let bodies = doc.add_layer(); let labels = doc.add_layer(); for ix in 0..field.count { let s = field.spec(ix).unwrap(); doc.add_shape(bodies, CanvasShape::rect(s.bounds, ShapeStyle::both(palette[s.palette_ix].opacity(0.85), StrokeStyle { width: px(1.), color: border, dash: None }))); match s.kind { NoteKind::Sticky => { doc.add_shape(labels, CanvasShape::text(s.bounds.origin + point(px(8.), px(6.)), s.label.clone(), px(12.), text_color)); } NoteKind::DotCard => { for off in content::dot_offsets() { doc.add_shape(bodies, CanvasShape::rect(Bounds { origin: s.bounds.origin + off, size: size(DOT_SIZE, DOT_SIZE) }, ShapeStyle::filled(palette[s.palette_ix]))); } } } } (bodies, labels) })`; finally `state.set_grid(CanvasGrid { visible: true, spacing: px(64.) }, cx)` and return the layer ids.
- Tests (`#[gpui_kit::test]`, `cx: &TestAppContext`): `let state = cx.update(|cx| cx.new(|_| CanvasState::new()));` with a `cx.subscribe` counting helper entity (01 Step 3.1 pattern); assert exactly one `DocumentChanged` for the whole build and `document_revision()` +1; shape counts for `NoteField { seed: 1, count: 100 }`: bodies `100 + 6*25 = 250`, labels `75`; `document().layers().len() == 2`; rebuild with a new seed emits exactly one more `DocumentChanged`.
- Done-check: `cargo test -p gpui-starter --lib infinite_canvas::content` passes with the new tests.

## Phase 3: Page zoom clamps and ExploreTool

### Step 3.1: Clamp math

- Files: `src/features/pages/infinite_canvas/explore_tool.rs`, `explore_tool.test.rs`; `mod.rs` declares the module + test wiring.
- Build: `pub(crate) const PAGE_MIN_ZOOM: f32 = 0.1;`, `PAGE_MAX_ZOOM: f32 = 8.0;`, `LABEL_LOD_ZOOM: f32 = 0.5;`, `LINE_PX: f32 = 40.;`; `pub(crate) fn clamp_zoom(z: f32) -> f32 { z.clamp(PAGE_MIN_ZOOM, PAGE_MAX_ZOOM) }`; `pub(crate) fn page_factor(zoom: f32, factor: f32) -> f32` → `let desired = clamp_zoom(zoom * factor); if (desired - zoom).abs() <= f32::EPSILON { 1.0 } else { desired / zoom }`; `fn lines(delta: &ScrollDelta) -> Point<f32>` matching `Lines(p) => *p`, `Pixels(p) => point(p.x.as_f32() / LINE_PX, p.y.as_f32() / LINE_PX)` (same 40 px/line convention as 01's router).
- Tests: `page_factor` mid-range returns `factor` unchanged; `zoom=8.0, factor=2.0` → `1.0` (no-op at ceiling); `zoom=0.1, factor=0.5` → `1.0`; `zoom=4.0, factor=4.0` → `2.0` (lands exactly on 8.0); a product crossing a bound lands exactly on the bound; `lines` maps `Lines(point(0., 2.))` → `(0., 2.)` and `Pixels(point(40., 20.))` → `(1., 0.5)`.
- Done-check: `cargo test -p gpui-starter --lib infinite_canvas::explore_tool` passes.

### Step 3.2: `ExploreTool`

- Files: same.
- Build: `pub(crate) struct ExploreTool { dragging: bool, last_screen: Option<Point<Pixels>> }` + `new()`; `impl CanvasTool for ExploreTool`: `id() -> "explore"`, `cursor() -> CursorStyle::OpenHand`; `pointer_down` (left button) sets `dragging`/`last_screen` (middle never reaches tools — router intercepts, 01 Step 7.1); `pointer_move` while dragging calls `state.pan_by(ev.screen_position - last, cx)` and updates `last` (screen-space delta, 01 PanTool semantics); `pointer_up` clears both; `scroll` returns `true` always and: if `event.modifiers.secondary()` (`GPUI/platform/keystroke.rs:483`) → `let f = page_factor(state.zoom(), (-lines(&event.delta).y * 0.1).exp());` then `state.zoom_at(event.screen_position, f, cx)` only when `f != 1.0`; else plain-wheel pan `state.pan_by(point(px(-d.x * LINE_PX), px(-d.y * LINE_PX)), cx)` with `(d.x, d.y) = lines(&event.delta)`. Consuming every scroll keeps the engine's 0.05–32 default clamp out of the page.
- Tests (`#[gpui_kit::test]`, dispatched through `state.dispatch_tool_event` exactly as 01 Step 4.2 drives its tools): drag sequence (down, move +20/+10, up) pans by `(20., 10.)`; secondary+wheel with `Lines(point(0., 2.))` zooms in with the anchor's doc point unchanged (`transform().to_doc(anchor)` before vs after within `px(0.01)`); at `zoom == 8.0` the same scroll is a no-op (zoom and pan unchanged); plain `Pixels(point(10., 5.))` wheel pans by `(-10., -5.)`; tool id and cursor.
- Done-check: `cargo test -p gpui-starter --lib infinite_canvas::explore_tool` passes.

## Phase 4: Viewport tracking, space-pan, pinch, LOD

### Step 4.1: Page state and space-pan transitions

- Files: `src/features/pages/infinite_canvas/viewport.rs`, `viewport.test.rs`, `mod.rs` (page fields: `state: Entity<CanvasState>`, `field: NoteField`, `layers: (LayerId, LayerId)`, `palette: [Hsla; 5]`, `viewport: Bounds<Pixels>`, `minimap_bounds: Bounds<Pixels>`, `space_pan: bool`, `preset_ix: usize`, `subscriptions: Vec<Subscription>`, `focus_handle: FocusHandle`).
- Build: `pub(crate) fn local_viewport(viewport: Bounds<Pixels>) -> Bounds<Pixels>` → `Bounds { origin: Point::default(), size: viewport.size }` (single seam for the window↔element-local conversion); page methods in `mod.rs`: `fn set_space_pan(&mut self, active: bool, cx: &mut Context<Self>)`, no-op when unchanged; `true` → `self.state.update(cx, |s, cx| s.set_builtin_tool(BuiltinTool::Pan, cx))`, `false` → `self.state.update(cx, |s, cx| s.set_tool(Box::new(ExploreTool::new()), cx))`; `cx.notify()` on change; `fn reconcile_lod(&mut self, cx: &mut Context<Self>)`: `let desired = self.state.read(cx).zoom() >= LABEL_LOD_ZOOM;` if `self.state.read(cx).document().layer(self.layers.1).is_some_and(|l| l.is_visible() != desired)` then `self.state.update(cx, |s, cx| s.set_layer_visible(self.layers.1, desired, cx))` (guard prevents the notify loop); `fn store_viewport(&mut self, bounds: Bounds<Pixels>)` and `fn store_minimap_bounds(&mut self, bounds: Bounds<Pixels>)`, each change-guarded plain field writes, no notify (rationale in Architecture; popup_menu precedent). Page `new()` builds the default field (`NoteField { seed: 0x06C0FFEE, count: 5_000 }`), stores palette/layers via `build_field`, sets `ExploreTool`, creates `focus_handle: cx.focus_handle()`, initializes `preset_ix: DEFAULT_PRESET_IX`, and subscribes `cx.observe(&self.state, |this, _, cx| { this.reconcile_lod(cx); cx.notify(); })` into `subscriptions` (three-parameter closure, `GPUI/app/context.rs:63-70`). This step also defines `pub(crate) const PRESETS: [usize; 3] = [1_000, 5_000, 20_000];` and `const DEFAULT_PRESET_IX: usize = 1;` in `mod.rs` (used by `new()` here and by `regenerate` in Step 6.2).
- Tests (`#[gpui_kit::test]`): `set_space_pan(true)` → `active_tool_id() == "pan"` and one `ToolChanged`; `false` → `"explore"`; double-`true` is a single transition (no second `ToolChanged`); LOD: `set_zoom(0.4)` then `reconcile_lod` hides the labels layer (`document().layer(labels).is_visible() == false`); `set_zoom(1.0)` + reconcile shows it; `reconcile_lod` twice at the same zoom performs no second mutation (subscription event count stays flat across the second call); `store_viewport` with equal bounds leaves the field untouched.
- Done-check: `cargo test -p gpui-starter --lib infinite_canvas::viewport` passes.

### Step 4.2: Interaction probe element

- Files: same.
- Build: `pub(crate) fn interaction_probe(page: Entity<InfiniteCanvasPage>, state: Entity<CanvasState>) -> impl IntoElement`: a `canvas()` element (`gpui_kit::canvas`, `brush.rs:4-20` import precedent) `.absolute().top_0().left_0().size_full()`, no hitbox, prepaint returns `()`. Paint registers, per frame (all listeners re-registered every paint, cleared next frame — `GPUI/window.rs:5294-5333`):
  - `window.on_key_event::<KeyDownEvent>`: bubble phase; guard `page.read(cx).focus_handle.contains_focused(window, cx) || window.focused(cx).is_none()`; if `keystroke.key == "space" && !is_held` → `page.update(cx, |p, cx| p.set_space_pan(true, cx))`.
  - `window.on_key_event::<KeyUpEvent>`: `keystroke.key == "space"` → `set_space_pan(false)`.
  - `window.on_mouse_event::<PinchEvent>`: `let viewport = page.read(cx).viewport;` if `viewport.contains(&event.position)` → `let anchor = event.position - viewport.origin;` `let factor = page_factor(state.read(cx).zoom(), 1.0 + event.delta);` (`delta` is the per-event magnification increment, macOS emit at `gpui-pre-macos-0.3.8/src/events.rs:253`); if `factor != 1.0` → `state.update(cx, |s, cx| s.zoom_at(anchor, factor, cx))`.
  - `window.on_mouse_event::<MouseDownEvent>`: if `viewport.contains(&event.position)` and not `page.read(cx).minimap_bounds.contains(&event.position)` (the minimap's `BlockMouse` hitbox sits above but this window-level listener still fires — exclude its rect explicitly) → `let handle = page.read(cx).focus_handle.clone();` then `handle.focus(window, cx)` (`GPUI/window.rs:610`; the clone-first shape exists because the entity read borrow is alive while `focus` takes `&mut App`).
  All closures capture `Entity` clones only.
- Tests: pure pieces only: `local_viewport` strips origin keeps size; the pinch factor path via a direct `state.update` call mirroring the listener body (clamped at the 8.0 ceiling, anchor-stable); focus-scope and space handling are exercised at the method level in 4.1 plus manual QA.
- Done-check: `cargo test -p gpui-starter --lib infinite_canvas::viewport` passes; `cargo check -p gpui-starter` clean.

### Step 4.3: Page render composition + integration test

- Files: `mod.rs` (render), `viewport.test.rs` (integration).
- Build: `impl Render for InfiniteCanvasPage`:

  ```text
  v_flex().id("infinite-canvas").size_full().min_h_0()
  ├── hud: v_flex().p_2().gap_1().border_b_1().border_color(theme.border).flex_shrink_0()
  │        [Phase 6 replaces a placeholder h_flex().gap_2() with the real controls]
  └── area: div().id("canvas-area").flex_1().min_h_0().relative()
              .track_focus(&self.focus_handle)                     [render.rs:225-227 precedent]
              .on_prepaint({ let page = cx.entity(); move |bounds, _, cx| {
                  page.update(cx, |this, _| this.store_viewport(bounds)); }})
              .child(Canvas::new("infinite-canvas", self.state.clone()))   [plan 01; fills parent]
              .child(interaction_probe(self_handle, self.state.clone()))   [absolute overlay, no hitbox]
              [.child(minimap(...)) lands in Step 5.2 — Phase 4 omits it, the tree compiles]
  ```

  `on_prepaint` is `gpui_kit::component::ElementExt` (`GB/element_ext.rs:14-26`); its callback is a plain closure, not a listener — capture `cx.entity()` and update it (`popup_menu.rs:1505` shape). The `Canvas` child must come before the probe child so the engine hitbox and its listeners register first; children prepend order also guarantees the viewport store runs in prepaint before the probe/minimap paint.
- Tests (`#[gpui_kit::test]`, async): a `CanvasHost` view rendering `h_flex().size_full().child(div().id("lead").w_16()).child(page)` (a leading strip forces a non-zero canvas-area origin, the window↔local conversion seam); `cx.add_window(...)`; `let cx = VisualTestContext::from_window(handle.into(), &cx);` draw one frame; assert `page.read(cx).viewport.size.width > px(0.)` and `viewport.origin.x >= px(64.)` (the strip); `cx.simulate_window_resize(handle.into(), size(px(800.), px(600.)))` (`test_context.rs:461`), draw, assert the size updated.
- Done-check: `cargo test -p gpui-starter --lib infinite_canvas` passes.

## Phase 5: Minimap

### Step 5.1: Projection math

- Files: `src/features/pages/infinite_canvas/minimap.rs`, `minimap.test.rs`; `mod.rs` declares the module + test wiring.
- Build (pure, `pub(crate)`, all in window space so click positions need no translation): `struct MiniProjection { scale: f32, offset: Point<Pixels> }`; `fn fit_projection(field: Bounds<Pixels>, minimap: Bounds<Pixels>) -> MiniProjection`: `scale = (minimap.size.width.as_f32() / field.size.width.as_f32()).min(minimap.size.height.as_f32() / field.size.height.as_f32())`; `offset = minimap.origin + (minimap.size - field.size * scale) * 0.5` (component-wise `Pixels * f32`, then subtraction; centers the field with half the leftover per axis); `fn project_doc_to_minimap(p: Point<Pixels>, pr: &MiniProjection) -> Point<Pixels>` = `pr.offset + p * pr.scale`; `fn unproject_minimap_to_doc(p: Point<Pixels>, pr: &MiniProjection) -> Point<Pixels>` = `px((p.x - pr.offset.x).as_f32() / pr.scale)` per axis (`Pixels / f32` does not exist — divide through `as_f32()`).
- Tests: known field/minimap rects → exact `scale`/`offset` (compare `f32::to_bits`); roundtrip `unproject(project(p)) == p` within `px(0.001)` for sample points; a portrait field centers with leftover split evenly.
- Done-check: `cargo test -p gpui-starter --lib infinite_canvas::minimap` passes.

### Step 5.2: Minimap element

- Files: `minimap.rs` (extend), `minimap.test.rs`.
- Build: `pub(crate) fn minimap(page: Entity<InfiniteCanvasPage>, state: Entity<CanvasState>) -> impl IntoElement`: wrapper `div().id("canvas-minimap").absolute().bottom_4().right_4().w_40().h_24()` (160×96 px at default rem; `h_30` does not exist — the macro suffix set is verified in Research Digest, so the height helper must come from it) `.border_1().border_color(theme.border).rounded(theme.radius_lg).bg(theme.popover).overflow_hidden().a11y(Role::Image, crate::i18n::localize("canvas_minimap_label"))` (repo `A11yExt` pattern, `notifications.rs:48-54`) with `.on_prepaint({ let p = page.clone(); move |bounds, _, cx| p.update(cx, |this, _| this.store_minimap_bounds(bounds)); })` as its FIRST child, then a `canvas()` child:
  - prepaint: `let hitbox = window.insert_hitbox(bounds, HitboxBehavior::BlockMouse); (bounds, hitbox)` — `BlockMouse` keeps canvas input alive outside the minimap and dead beneath it.
  - paint: read `field`, `palette`, `minimap_bounds` from `page`, `transform` from `state`; `let pr = fit_projection(field.span(), minimap_bounds);` dots: `let step = (field.count + MINIMAP_MAX_DOTS - 1) / MINIMAP_MAX_DOTS` (`pub(crate) const MINIMAP_MAX_DOTS: usize = 20_000`), for each `ix in (0..field.count).step_by(step)` with a `spec`: center = `project_doc_to_minimap(spec.bounds.center, &pr)` (`GPUI/geometry.rs:1001`), `window.paint_quad(fill(Bounds { origin: center - point(px(1.), px(1.)), size: size(px(2.), px(2.)) }, palette[spec.palette_ix]))`; viewport rect: visible doc rect = `transform.to_doc_bounds(local_viewport(page.read(cx).viewport))` clamped to the field span via `Bounds::intersect` (`geometry.rs:1143`), projected through `pr`, then four 1-px `fill` quads in `cx.theme().primary` (top/bottom/left/right edges), each clipped to the minimap bounds by `Bounds::intersect` before painting.
  - click-to-jump: `window.on_mouse_event::<MouseDownEvent>` (bubble): `if !hitbox.is_hovered(window) { return; }` then `cx.stop_propagation();` `let doc = unproject_minimap_to_doc(event.position, &pr);` and inside `state.update(cx, |s, cx| ...)`: `let local_center = Point::new(viewport.size.width * 0.5, viewport.size.height * 0.5);` `let target_pan = local_center - doc * s.zoom();` `let delta = target_pan - s.transform().pan;` `s.pan_by(delta, cx);` (window-space click → doc via `pr.offset`; element-local pan via the local center — the two spaces meet exactly here and nowhere else).
- Tests (integration, `#[gpui_kit::test]` + `VisualTestContext`, host from Step 4.3): mount, draw; `simulate_click` at `page.read(cx).minimap_bounds.center()`; assert `transform().to_doc(local_center_of_viewport)` is within `px(1.)` of `field.span().center()`; second probe: pick `let probe_doc = span.origin + point(span.size.width, span.size.height) * 0.25;`, `let click = project_doc_to_minimap(probe_doc, &fit_projection(span, minimap_bounds));` (compute `minimap_bounds` from the page read), `simulate_click(click)`, assert `to_doc(local_center)` within `px(1.)` of `probe_doc`. The interior-point probe avoids the letterbox: the projection centers a square field in a 160×96 minimap, so ~16 px of margin pad the wide axis and a corner click never unprojects near the field origin. This is also the coordinate-space regression: both probes fail if any window↔local conversion is off by the canvas-area origin.
- Done-check: `cargo test -p gpui-starter --lib infinite_canvas::minimap` passes.

## Phase 6: HUD, operations, perf instrumentation

### Step 6.1: Perf module

- Files: `src/features/pages/infinite_canvas/perf.rs`, `perf.test.rs`; `mod.rs` declares the module + test wiring.
- Build (mirrors `src/shell/root/frame_time.rs:1-25`): `pub(crate) const PAGE_SLOW_RENDER_THRESHOLD_US: u64 = 2_000;`, `static LAST_PAGE_RENDER_US: AtomicU64`, `static LAST_GEN_US: AtomicU64`; `pub(crate) fn store_page_render_us(u64)`, `pub fn last_page_render_us() -> u64`, `pub(crate) fn store_gen_us(u64)`, `pub fn last_gen_us() -> u64`, `pub(crate) fn is_slow_page_render(u64) -> bool` — all `Ordering::Relaxed`, same shape as `frame_time.rs`.
- Tests: store/read roundtrip for both atomics; `is_slow_page_render` boundary at exactly the threshold (false) and threshold+1 (true).
- Done-check: `cargo test -p gpui-starter --lib infinite_canvas::perf` passes.

### Step 6.2: Page operations

- Files: `mod.rs`.
- Build (methods on `InfiniteCanvasPage`, each ending in at most one `cx.notify()`):
  - `fn zoom_view(&mut self, factor: f32, cx)`: guard `self.viewport.size.width <= px(0.)`; `let anchor = Point::new(self.viewport.size.width * 0.5, self.viewport.size.height * 0.5);` (element-local center); `let f = page_factor(self.state.read(cx).zoom(), factor);` `if f != 1.0 { self.state.update(cx, |s, cx| s.zoom_at(anchor, f, cx)); }`.
  - `fn fit_view(&mut self, cx)`: guard `self.viewport.size.width <= px(0.)`; `let span = self.field.span(); let viewport = self.viewport;` then `self.state.update(cx, |s, cx| { s.zoom_to_fit(span, Size { width: viewport.size.width, height: viewport.size.height }, cx); let z = clamp_zoom(s.zoom()); if z != s.zoom() { s.set_zoom(z, cx); let local_center = Point::new(viewport.size.width * 0.5, viewport.size.height * 0.5); let target_pan = local_center - span.center() * s.zoom(); let delta = target_pan - s.transform().pan; if delta.x != px(0.) || delta.y != px(0.) { s.pan_by(delta, cx); } } })`. The recenter is the common case, not an edge: the default 5,000-note field spans `71·180+40 = 12,820` doc px per side (Step 2.1 math), so fit zoom is `0.9·min(W,H)/12,820`, below the 0.1 floor unless the viewport is ≥ ~1,424 px on its short side — ordinary windows clamp, `set_zoom` keeps pan (01 contract), and without the recenter the span center lands at `doc_center·(z_fit/0.1)`, roughly half the viewport center. The 20k preset (span ≈ 25,600 px) cannot fit above the floor at all; there the clamped+centered view is the defined behavior.
  - `fn reset_view(&mut self, cx)`: `self.state.update(cx, |s, cx| s.reset_view(cx))`.
  - `fn toggle_grid(&mut self, cx)`: read `state.grid()`, `set_grid(CanvasGrid { visible: !g.visible, spacing: g.spacing }, cx)`.
  - `fn regenerate(&mut self, count: usize, cx)`: `let started = crate::platform::clock::Instant::now();` (never `std::time`, `src/platform/clock.rs:1-8`); `let field = NoteField { seed: self.field.seed.wrapping_add(0x9E3779B9), count };` `self.palette = note_palette(cx);` `self.layers = self.state.update(cx, |s, cx| build_field(s, &field, cx));` `self.field = field;` `self.preset_ix = PRESETS.iter().position(|&c| c == count).unwrap_or(self.preset_ix);` `let us = started.elapsed().as_micros() as u64;` `perf::store_gen_us(us);` `tracing::info!(target: "gpui_starter::canvas_page", count, gen_us = us, "canvas field regenerated");` `cx.notify();`. (`PRESETS`/`DEFAULT_PRESET_IX` were defined in Step 4.1.)
- Tests (`#[gpui_kit::test]`): `zoom_view` at the ceiling is a no-op (zoom unchanged); `fit_view` in two regimes after a manual `store_viewport` (then recompute `local_center` from the stored bounds): (a) small field `NoteField { seed: 1, count: 4 }` (span 400×400) on an 800×600 viewport — fit zoom `0.9·600/400 = 1.35` lands inside page bounds unclamped, and `|transform().to_doc(local_center) - span.center()|` is within `px(1.)`; (b) the default 5,000-note field (span 12,820×12,820) on the same viewport — fit clamps to `zoom == PAGE_MIN_ZOOM` and the same centering assertion holds (the recenter path); `regenerate` keeps body shape count at `count + 6 * (count / 4)` and label count at `count - count / 4`, changes seed-dependent bounds (compare `field.spec(0)` before/after), and stores `last_gen_us() > 0`.
- Done-check: `cargo test -p gpui-starter --lib infinite_canvas` passes with the new tests.

### Step 6.3: HUD render + timing

- Files: `mod.rs`.
- Build: wrap the render body with `let started = crate::platform::clock::Instant::now();` … at the end `let us = started.elapsed().as_micros() as u64; perf::store_page_render_us(us); if perf::is_slow_page_render(us) { tracing::warn!(target: "gpui_starter::canvas_page", render_us = us, "slow canvas page render"); }` (mirrors `render.rs:270-283`). The HUD replaces the Phase 4 placeholder:

  ```text
  hud = v_flex().p_2().gap_1().border_b_1().border_color(theme.border).flex_shrink_0()
  ├── row: h_flex().w_full().gap_2().items_center().justify_between()
  │   ├── left: div().id("canvas-controls").a11y(Role::Group, localize("canvas_zoom_level"))
  │   │        .h_flex().gap_1()
  │   │   icon_button("canvas-zoom-out", Minus, canvas_zoom_out)     → zoom_view(1.0 / 1.25)
  │   │   div().id("canvas-zoom-level").w_12().text_center().text_sm()
  │   │       .child(format!("{}%", (zoom * 100.).round()))
  │   │   icon_button("canvas-zoom-in", Plus, canvas_zoom_in)        → zoom_view(1.25)
  │   │   icon_button("canvas-fit", Maximize, canvas_zoom_to_fit)    → fit_view
  │   │   icon_button("canvas-reset", Map, canvas_reset_view)        → reset_view
  │   │   icon_button("canvas-grid", LayoutDashboard, canvas_toggle_grid) → toggle_grid, .toggled(grid.visible)
  │   └── right: h_flex().gap_1()
  │       three outline compact Buttons "1k"/"5k"/"20k", .toggled(ix == preset_ix), → regenerate(PRESETS[ix])
  │       icon_button("canvas-regen", RefreshCw, canvas_regenerate)  → regenerate(PRESETS[preset_ix])
  │       div().text_xs().text_color(theme.muted_foreground)
  │           .child(localize_with_args("canvas_items", &[("count", field.count.to_string())]))
  └── hint: div().text_xs().text_color(theme.muted_foreground).child(localize("canvas_pan_hint"))
  ```

  `icon_button` = local helper: `Button::new(id).ghost().icon(name).tooltip(crate::i18n::localize(key)).accessibility_label(crate::i18n::localize(key))` — `IconName` converts into `ButtonIcon` directly (`GC/button/button.rs:397`; repo helper `src/gallery/sections/buttons/toolbar.rs:82-86`); icon-only buttons need the tooltip and the explicit accessible label. `Role::Group` is verified in the kit's accesskit 0.24.1 (`accesskit-0.24.1/src/lib.rs:152`). Under `#[cfg(debug_assertions)]`, append to the hint row: `format!("Frame: {:.2}ms · Page: {:.2}ms · Gen: {:.2}ms", crate::root::last_frame_time_us() as f64 / 1000., perf::last_page_render_us() as f64 / 1000., perf::last_gen_us() as f64 / 1000.)` — untranslated literal, same class as the status bar's `"Frame: {ms:.2}ms"` (`status_bar.rs:238`).
- Tests: render smoke only (mount + draw via the Step 4.3 host; no panic, viewport non-zero); HUD behavior is covered by the Step 6.2 method tests plus manual QA.
- Done-check: `cargo clippy -p gpui-starter --all-features` reports no new warnings; `cargo test -p gpui-starter --lib infinite_canvas` passes.

## Phase 7: Freeze and full gates

### Step 7.1: Module seam freeze

- Files: `src/features/pages/infinite_canvas/mod.rs`.
- Build: final `mod` decls, `pub use` surface matching the Interface Contract, removal-recipe header verbatim.
- Tests: none new.
- Done-check: `grep -rn "use gpui::" src/features/pages/infinite_canvas/` returns nothing (qa_migration rule, seams §9).

### Step 7.2: Full verification sweep

- Files: none.
- Build: run the full Verification section in order, including the perf measurement steps and the manual QA checklist.
- Tests: all existing suites plus the new ones.
- Done-check: every gate command exits 0; the budget table and manual checklist are confirmed or waived item-by-item with notes.

## Platform & Wasm Gating

- No `cfg` gates in the feature module. Every API used is platform-neutral and compiles on wasm32: `canvas()`, `paint_quad`/`fill`, `insert_hitbox`, `on_key_event`/`on_mouse_event` (including `PinchEvent` as a type), `on_prepaint`, kit `Button`, plan 01 engine.
- Pinch behavior per platform (facts in Research Digest): macOS trackpad (`NSEventTypeMagnify`), Linux X11 (XI2 gesture) and Wayland (pinch protocol), Windows precision touchpads (DirectManipulation) deliver `PinchEvent`; zoom-to-cursor works. Wasm: no producer and no touch forwarding in `gpui-pre-web-0.3.8`, so the pinch listener never fires; the page is not hidden or degraded (wheel and secondary+wheel cover zoom; space/middle-drag cover pan). This is "input device absent", not a broken capability — the same class as a mouse without a middle button.
- No capability-registry entry: nothing initializes, fails, or degrades at runtime; the only platform variance (pinch availability) is not runtime-queryable (`Platform::gestures()` unreachable from app code — Research Digest), so a registry row could only restate compile-time knowledge. Matches 01's stance and seams §4 (entries for features with real platform state).
- No native-only dependencies; `[target.'cfg(not(target_family = "wasm"))'.dependencies]` untouched. `Cargo.toml`/`Cargo.lock` unchanged.
- Wasm clock rule: all timing goes through `crate::platform::clock::Instant`; no `std::time` anywhere in the module (seams §6).
- `just wasm-check` (`cargo +nightly check --target wasm32-unknown-unknown -p gpui-starter --lib`, `justfile:24-25`) must pass: the lib gains the new module tree.

## i18n

Catalog facade: `crate::i18n::localize` / `localize_with_args` (seams §5). Key sets identical in both files (qa_localization lockstep). Generated note labels (`Note {ix}`) are demo data painted into the canvas, not interface copy — same exemption as document content in 01. Debug perf readouts reuse the status bar's untranslated `"Frame: …"` precedent (`status_bar.rs:238`). Preset labels `1k`/`5k`/`20k` are locale-neutral numerals.

| key | en | zh-CN |
|---|---|---|
| `nav_infinite_canvas` | Infinite Canvas | 无限画布 |
| `canvas_zoom_in` | Zoom in | 放大 |
| `canvas_zoom_out` | Zoom out | 缩小 |
| `canvas_zoom_level` | Zoom level | 缩放级别 |
| `canvas_zoom_to_fit` | Zoom to fit | 缩放至适应 |
| `canvas_reset_view` | Reset view | 重置视图 |
| `canvas_toggle_grid` | Grid | 网格 |
| `canvas_regenerate` | Regenerate | 重新生成 |
| `canvas_items` | %{count} items | %{count} 个项目 |
| `canvas_minimap_label` | Minimap; click to move the viewport | 小地图；点击移动视口 |
| `canvas_pan_hint` | Hold Space and drag to pan; Ctrl/Cmd + scroll to zoom. | 按住空格并拖动可平移；Ctrl/Cmd 加滚轮可缩放。 |

`title()` stays the English label "Infinite Canvas" for logs and non-localized surfaces; the sidebar shows `nav_infinite_canvas` via `localized_title()` (`sidebar.rs:34-49`). `localize(self.title_key())` is invisible to the qa scanner (seams §5 blind spot): the key must exist in both catalogs, enforced here by the table and by Step 1.1's lockstep guard.

## Design Compliance

- Tokens: every chrome color from `cx.theme()` — `border` (HUD rule, minimap frame), `popover` (minimap surface, the shared popup-family surface), `muted_foreground` (hint, stats), `primary` (minimap viewport rect), `background` via the engine's grid painter. No raw hex/`rgb`/`hsla` constructors in the module. Note fills and dots come from theme roles (`accent/primary/secondary/info/success`), tinted with `opacity(0.85)` for fills — generated content colors, the raster/data exemption 01 also uses.
- Fixed-pixel exceptions, each on the guide's audited list: 1-px minimap rect edges (hairline), 2-px minimap dots (raster-like data pixels), 1-px note borders (engine stroke). Minimap frame uses `w_40`/`h_24` rem helpers (the closest tokens in the macro's suffix set to the intended 160×120 ratio; `h_30` does not exist — Research Digest), not `px()`; its window-space rect is measured, never hardcoded, so interface zoom cannot desync the click target from the painted surface. Note geometry (`140×90+40` doc px, 10-px dots) is document data, not layout.
- Layout/density: compact toolbar (`.ghost()`/`.outline()` `.compact()` buttons, `gap_1`, `p_2`) per the "compact for toolbars and data-dense screens" tier; canvas area takes `flex_1().min_h_0()` with the HUD `flex_shrink_0()`; the scroll-owner rule does not apply (no scroll container: the canvas owns its own pan).
- Components: only kit `Button` (ghost/outline/compact/toggled) — no custom clickable divs; the minimap is a `Role::Image` with a label because no semantic component covers a viewport overview.
- Interaction states: zoom/grid/preset buttons use kit `toggled` for persistent state (`GC/button/button.rs:497`); icon-only buttons carry tooltips and explicit `accessibility_label`; hover/pressed/focus-visible come from the kit; canvas cursor is `OpenHand` via the tool (`set_cursor_style` through plan 01); zoom % is a live readout next to its buttons.
- Keyboard/a11y: every HUD action is a real button (tab-reachable, Enter/Space activatable); the hint documents the space-drag path; the focus-scope guard keeps Space activation on a focused button while routing Space to panning otherwise; the minimap is click-only by spec and its `Role::Image` label says so; viewport movement stays keyboard-reachable through the HUD (fit/reset). Canvas content itself is not in the accessibility tree (plan 01 v1 limitation, inherited and documented in Risks).
- Motion: none added; nothing animates, so the reduced-motion rule is trivially satisfied.
- Zoom/rem: canvas zoom is document zoom and never touches `rem_size`; HUD uses rem helpers so interface zoom keeps working independently (design guides, "Zoom, base font, and rem").

## Verification

Gates, in order (first three are the repo's mandatory set, AGENTS.md):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

Feature-specific automated coverage: `cargo test -p gpui-starter --lib infinite_canvas` (generator determinism, kinds, span and dot containment, single-batch rebuild with exact shape counts, clamp math, ExploreTool dispatch with anchor stability, space-pan tool swap, LOD guard, notify-free bounds stores, viewport tracking + window resize, minimap projection roundtrip + click-to-jump + coordinate-space regression, perf atomics) and `cargo test -p gpui-starter --test e2e_navigation --test qa_localization` (page count, key lockstep).

Perf budget (60 fps = 16.67 ms/frame):

| slice | budget | measured by |
|---|---|---|
| whole frame under interaction | ≤ 16.67 ms | status bar `Frame:` readout (debug + `show_frame_time`), `frame_time.rs` pattern |
| `AppRoot::render` | ≤ 4 ms | existing `SLOW_FRAME_THRESHOLD_US` warn (`frame_time.rs:5`, `render.rs:275-283`) |
| `InfiniteCanvasPage::render` | ≤ 2 ms | `perf::last_page_render_us()` HUD readout + warn at `PAGE_SLOW_RENDER_THRESHOLD_US` |
| field generation (regenerate click) | ≤ 150 ms one-shot, off steady-state frames | `perf::last_gen_us()` + `gpui_starter::canvas_page` trace |
| visible paint at 20k items | covered by the frame budget | engine culling (plan 01); no separate page budget |

Measurement steps (manual, debug build):

1. Settings → enable "Show Frame Time" (`show_frame_time`, `config_store.rs:88-89`); the status bar shows root render ms.
2. Open Infinite Canvas; load the 20k preset; the HUD debug readout shows Frame/Page/Gen ms.
3. For each interaction — idle, wheel pan, secondary+wheel zoom swept across the full 0.1–8 range, space-drag, middle-drag, fit, regenerate, minimap click — record Frame and Page numbers; all must stay within budget (a regenerate-click frame may exceed the frame budget; the next frame must not).
4. `RUST_LOG` run: check `gpui_starter::canvas_page` traces for `gen_us` and the absence of `slow canvas page render` warns during steady interaction.

Manual QA checklist:

1. Wheel pans without bound in every direction; secondary+wheel zooms at the cursor between 0.1x and 8x and hard-stops at both ends; the % readout tracks.
2. Space held turns the cursor to the pan hand and left-drag pans; releasing Space restores the explore tool; Space while a HUD button is focused activates the button, not panning.
3. Pinch zooms at the cursor on a trackpad (macOS/Linux/Windows precision touchpad); on the web build pinch is inert and nothing else regresses.
4. 20k preset: pan and zoom stay within budget; zooming below 0.5x drops note labels (LOD) and the grid step coarsens (engine LOD); zooming back restores both; every 4th card is a dot-grid card whose dots scale with zoom.
5. Minimap shows the full field as dots plus the live viewport rect; clicking anywhere recenters the canvas there; the rect tracks every pan/zoom.
6. Zoom-to-fit frames the whole field with margin when it fits above the 0.1x floor; at the 20k preset (span ≈ 25,600 doc px) the floor clips and the view centers the field at 0.1x; reset returns to 1.0x at origin; the grid toggle hides/shows the background grid.
7. Light and dark themes both render the HUD, minimap, and notes legibly.
8. Deep link `gpui-starter://infinite-canvas` opens the page; the page survives an app restart when persisted as `active_route`.

Acceptance criteria: all four gate commands pass; the automated suites above pass; the budget table is satisfied per the measurement steps; the manual checklist is confirmed (or waived item-by-item with a note naming what was checked and how).

## Risks & Open Questions

Ranked risks:

1. **Plan 01 not yet implemented.** Every canvas dependency is contract text, not compiled code (`src/ui/` holds only `widgets/`, verified this session). API drift in 01's landing (e.g., `set_layer_visible` notify semantics, `zoom_to_fit` signature, router space handling) breaks Phases 2–5 mechanically. Mitigation: phases run only after 01 lands green; failures surface as compile errors at the exact contract item. The coordinate-space ambiguity this plan originally flagged is resolved in 01's current revision (Risk 1 there); this plan keeps the `local_viewport`/projection seams and the minimap click regression test so a router regression fails a test instead of shipping a visual offset.
2. **Observer-loop hazard in `reconcile_lod`.** `set_layer_visible` notifies per call (01 contract); an unguarded call inside the state observer is a permanent redraw loop. Mitigation: change guard specified and unit-tested (Step 4.1, "no second mutation" assertion).
3. **Page-count coupling with plan 05.** Both plans append a `Page` variant and bump the same e2e literal. Whichever lands second must bump from the then-current value; the plan text says "+1 from the current literal" rather than a hardcoded number. Digit keys are unaffected (indices ≥ 10 never bind).
4. **20k-item minimap cost.** Up to 20k `paint_quad` calls per frame for dots (`MINIMAP_MAX_DOTS` decimation caps it). Budget: covered by the frame readout. Tried analytically: quad emission is O(1) each, µs-scale for 20k on desktop; not benchmarked here (no build was run).
5. **Space-pan not integration-testable.** `simulate_keystrokes` emits complete down+up sequences (`test_context.rs:587`), so a held-Space drag cannot be simulated. Mitigation: handler-level unit tests (Step 4.1) + manual QA item 2. Same limitation plan 01 records for the wheel.
6. **Text at extreme zoom.** Note labels at 8x are shaped at 96 px by the engine each frame without a shaped-line cache (01 risk 3, inherited). LOD hides labels only below 0.5x; at 8x few labels are visible (culling), but if QA shows shaping pressure, the fix is 01's deferred shaped-line cache, not this page.
7. **Dot-grid cards inflate shape count.** 20k preset builds `20k + 30k = 50k` bodies + 15k labels = 65k shapes in one batch. Storage is plain `Vec` (01 scene model); the gen budget (≤ 150 ms) and the Step 2.2 count tests police it. If generation exceeds budget, drop dots to 4 per card (2×2) — a one-constant change in `dot_offsets`.

Open questions (each with what was tried):

- **Pinch on wasm**; tried grepping `gpui-pre-web-0.3.8/src/platform.rs` for `Pinch`/`TouchEvent`/touch forwarding: no producer and no forwarded touches, and core synthesis needs `TouchEvent`s (`GPUI/window.rs:5668-5678`). Recorded as "never fires on wasm"; if a future gpui-pre adds web pinch, the handler needs zero changes.
- **Runtime pinch capability query**; tried `pub fn platform` / a gestures accessor on `App` (`GPUI/app.rs`): not exposed; `Platform::gestures()` (`GPUI/platform.rs:440-443`) is unreachable from app code. Capability-registry entry omitted; a row would restate compile-time knowledge.
- **No gates were run during planning** (doc-only scope, same as the survey and plans 01/05). Verification states the exact commands for the implementer.
