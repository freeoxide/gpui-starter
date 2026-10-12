# 10 Floating Canvas Windows Plan

## Goal

A "Floating Canvas" gallery section hosts three canvas mini-apps in floating
utility windows built on plans 01 and 02: a detachable mini-paint window whose
`Entity<CanvasState>` is rendered simultaneously in the main window and the
float (edits in either surface appear in both), any number of live sparkline
HUD windows driven by one shared ticking model entity, and a floating
shader-toy panel painting an animated procedural field. On wasm the same three
apps open as in-window overlay panes through plan 02's overlay path; no
platform branch exists in this feature's code.

## Dependencies

- `docs/plans/01-canvas-foundation.md` (implemented first; not landed yet —
  `src/ui/mod.rs:1` still reads `pub mod widgets;` only). Interface Contract
  items consumed: `Canvas::new(id, Entity<CanvasState>)` (`01:322`) plus its
  documented observe obligation (`01:209-210`, `:336`);
  `CanvasState::{new, set_builtin_tool, active_tool_id, edit_document,
  add_layer, add_shape, remove_shape, clear, set_grid, document,
  document_revision, transform, zoom, pan_by}` (`01:217-264`);
  `CanvasShape::path` constructor and pub-field `StrokeStyle` (`01:126-161`);
  `BrushConfig`, `BuiltinTool::{Brush, Pan}`, `CanvasGrid` (`01:199-316`);
  `LayerId`/`ShapeId` (`01:103-107`); pub(crate) `dispatch_tool_event`
  (`01:330-331`; test-only drive is legal crate-wide, rule documented in
  `05-canvas-showcase.md:14` with the drive shape at `:227`).
- `docs/plans/02-floating-window-toolkit.md` (implemented first). Interface
  Contract items consumed: `FloatingWindowId(pub SharedString)` (`02:289`),
  `FloatLevel::{Floating, AlwaysOnTop}` (`02:293`), `Placement9` (`02:302`),
  `FloatAnchor::Window` (`02:309`), `FloatingSpec` builder set `new/title/
  level/anchor/focus/resizable/movable/min_size` (`02:317-328`),
  `FloatingHandle` (`02:330`), `FloatingWindowEntry` pub fields `id`, `title`,
  `level`, `bounds`, `handle` (`02:336-345`), `spawn_floating`, `close_floating`,
  `respawn_floating`, `floating_snapshot` (`02:347-361`), and the
  `DismissFloating` Esc action (`02:394`). `floating::init` is already wired
  by plan 02 into `app::init` (`02` Seam Touches #2); this plan never re-inits.
- `docs/plans/07-canvas-shaders.md` and `docs/plans/11-wgpu-embed.md`: both on
  disk and read this session. Consumed ONLY by the explicitly-marked optional
  stretch (Phase 6); core Phases 1-5 reference neither.
- Repo baseline: `crate::app::window::root_window`
  (`src/app/window.rs:20-22`), gallery registry contract
  (`src/gallery/registry.rs:1-15`).

## Research Digest

Verified against
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` (short names `GPUI`
= `gpui-pre-0.3.8/src`, `KIT` = `gpui-kit-0.7.1/src`, `GC` =
`gpui-component-0.7.1/src`) and repo files. Survey docs re-verified where
load-bearing; everything below was checked against source this session.

Cross-window entity sharing (the mechanism the whole feature rests on):

- `App::notify(entity_id)` invalidates every window whose `tracked_entities`
  currently contains the entity, not just the current window
  (`GPUI/app.rs:2931-2960`: it filters `window_invalidators_by_entity`
  through `tracked_entities` per window id, then calls
  `invalidator.invalidate_view` for each; windows with no live invalidator
  fall back to a pending-notification effect). One `Entity<CanvasState>`
  mounted in the main window and in a floating window therefore repaints both
  from any `cx.notify()`, whichever window originated the mutation.
- `Entity<T>::update<R, C: AppContext>(&self, cx, f)` runs from any context,
  `&mut App` included (`GPUI/app/entity_map.rs:497-505`); `App` implements
  `AppContext` (`GPUI/app.rs:3104`). Panels and the section mutate the shared
  entities through plain `entity.update`.
- View entities are window-agnostic: plan 02's factory receives
  `(&mut Window, &mut App)` of the window the content is mounted in (native:
  the new window inside `gpui_kit::open_window`, `KIT/src/lib.rs:149-164`;
  wasm: the root window, plan 02 Step 4.2), so one factory closure serves both
  paths unchanged.
- Test-visible side of the same mechanism: `App::refresh_windows` only marks
  windows dirty (`GPUI/app.rs:1237-1239`, `apply_refresh_effect` at
  `:2011-2018`); headless test windows have no frame loop, so tests assert on
  entity-observer fan-out (which flushes at every update boundary,
  `GPUI/app.rs:1874-1877`), not on window re-render counts.

Live data (HUD tick producer):

- `Context<T>::spawn` takes `AsyncFnOnce(WeakEntity<T>, &mut AsyncApp)`
  (`GPUI/app/context.rs:230-244`); `WeakEntity::update` returns `Err` once
  the entity is released — the loop terminator
  (`GPUI/app/entity_map.rs:796-812`). Exact repo call shape:
  `cx.spawn(async move |this, cx| { cx.background_executor().timer(d).await;
  this.update(cx, |this, cx| ...) })` at
  `src/gallery/sections/messaging/scroller.rs:269-296`.
- `AsyncApp::background_executor() -> &BackgroundExecutor`
  (`GPUI/app/async_context.rs:157-159`; `AsyncApp: AppContext` at `:34`);
  `BackgroundExecutor::timer(Duration) -> Task<()>`
  (`GPUI/executor.rs:182-188`).
- Wasm: the web dispatcher's clock is `web_time::Instant`
  (`gpui-pre-web-0.3.8/src/dispatcher.rs:9`, `Instant::now()` at `:330`) and
  timers are browser `setTimeout` (`dispatcher.rs:278-286`), so the producer
  loop compiles and runs on wasm. The repo rule bans `std::time::Instant` in
  boot/render (seams survey §6); the producer uses only `timer()` durations
  and the model's tick counter, no wall clock.
- Tests drive the producer deterministically: `TestAppContext::executor()`
  returns the test `BackgroundExecutor` (`GPUI/app/test_context.rs:254-256`),
  `advance_clock(Duration)` fires fake timers (`GPUI/executor.rs:196-200`,
  test-support), `run_until_parked()` drains
  (`GPUI/app/test_context.rs:563-566`).

Animation (shader panel) and one-off painting:

- `Window::request_animation_frame()` redraws that window next frame and,
  from inside a view, notifies that view; its doc tells decorative-motion
  callers to check `App::reduce_motion()` and skip the request when set
  (`GPUI/window.rs:2656-2676`); `App::reduce_motion()` at
  `GPUI/app.rs:1222-1231`. In-repo precedent:
  `src/gallery/sections/charts/fps.rs:166-177` calls it at the top of
  `render` and uses a frame counter as the clock because render runs once per
  frame (`fps.rs:36-39`, counter at `:177`). `Window::simulate_next_frame`
  exists for tests (`GPUI/window.rs:2678-2690`, test-support).
- `canvas(prepaint, paint)` builds a `Canvas<T>` element for one-off drawing;
  the paint closure receives `(Bounds<Pixels>, T, &mut Window, &mut App)`
  (`GPUI/elements/canvas.rs:10-22`), and `Canvas<T>` implements `Styled`
  (`:84-88`) — the fps demo chains `.absolute().size_full()`
  (`fps.rs:133-135`). Distinct from plan 01's engine element: plan 01's
  `ui::canvas::Canvas` fills its parent by construction (request_layout with
  `Size::full()`, `01` Step 7.1) and sibling plans mount it bare inside a
  sizing div (plan 07: `div().relative().flex_1().min_h_0().child(
  Canvas::new("shader-canvas", ...))`, `07:889`). This plan mounts the engine
  element the same bare way; only the gpui `canvas()` closure element gets
  style chains.
- `window.paint_quad(quad)` at `GPUI/window.rs:4550`; `fill(bounds,
  background)` free constructor at `:7694`. The plot grid paints lines the
  same way (`gpui-base-0.7.1/src/plot/grid.rs:83`, survey §a).
- Shader-toy output is a quad grid, not a per-frame `RenderImage`:
  `RenderImage::new` allocates a fresh monotonic `ImageId`
  (`GPUI/assets.rs:59-70`) and the sprite atlas `get_or_insert_with` is
  insert-only (`GPUI/platform.rs:1749-1767`; eviction is the manual
  `AtlasState::remove`, called by no frame path). One new `RenderImage` per
  animation frame would grow the atlas without bound. The escape hatch —
  `window.drop_image(Arc<RenderImage>)` (`GPUI/window.rs:5037`) — exists and
  is exactly the drain plan 11 mandates for its dependents (`11` Interface
  Contract consumption notes); the core phases avoid `RenderImage` entirely
  and the stretch (Phase 6) inherits the drain obligation.

Controls:

- `Toggle` / `ToggleGroup` from `gpui_kit::component::button`:
  `Toggle::{new, label, icon, checked, on_click(&bool)}`
  (`GC/button/toggle.rs:48-115`), `ToggleGroup::{new, child, children,
  on_click(&Vec<bool>), segmented}` (`:233-287`), both `Sizable`
  (`GC/sizing.rs:190-215`, so `.small()` exists) and `Disableable`. Repo
  precedent including the listener shape:
  `src/gallery/sections/controls/toggle.rs:5`, `:65`, `:75`, `:100`,
  `:117` (`ToggleGroup::new(...).on_click(cx.listener(|this, values:
  &Vec<bool>, _, cx| ...))`).
- `Button::{new, label}` (`GC/button/button.rs:371`), `.ghost()` (`:82`,
  via `ButtonVariants as _` — import pattern
  `src/gallery/sections/editor/html_render.rs:6`), `.outline()` (`:332`),
  `Disableable::disabled(bool)` (`:524-525`), `Sizable` (`:562`).
- `Context::listener<E: ?Sized>` is generic over the event type
  (`GPUI/app/context.rs:246-256`), so `&Vec<bool>` group callbacks and
  `&ClickEvent` button callbacks both go through `cx.listener`.
- Theme: `cx.theme()` returns `&Theme` (`GC/theme/mod.rs:44-52`), `Theme`
  derefs to `ThemeColor` (`:220`) whose fields include `border` (`theme_color.rs:69`),
  `danger` (`:149`), `muted_foreground` (`:197`), `popover` (`:199`),
  `primary` (`:203`), `warning` (`:303`).
- `SharedString` implements `PartialEq<&'a str>`
  (`gpui-pre-shared-string-0.3.8/gpui_shared_string.rs:103-107`) and
  `From<&str>` (`:117`), so `entry.id.0 == MINI_PAINT_ID` and
  `FloatingWindowId(MINI_PAINT_ID.into())` both compile.
- `Entity<V: Render>` converts to `AnyView` (`GPUI/view.rs:29-38`) — the
  `new_view` factory returns.
- `Hsla` has manual `PartialEq`/`PartialOrd` (`GPUI/color.rs:374-384`); the
  free fn `hsla(h, s, l, a)` returns `Hsla` (used on data colors at
  `fps.rs:279-291`).
- Icons (svg filenames verified under `gpui-kit-assets-0.7.1/assets/icons/`
  this session): `brush.svg`, `palette.svg`, `activity.svg`, `eraser.svg`,
  `aperture.svg`, `play.svg`, `pause.svg`, `app-window.svg` all exist.
  `IconName` variants are generated from those filenames (survey Open
  Questions); used only where a label alone would read worse.

Layout and section seams:

- Gallery dir-section precedent (`src/gallery/sections/theme/mod.rs:11-14`):
  a directory under `sections/` whose `mod.rs` exposes
  `pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window,
  cx: &mut App)`; the group `mod.rs` adds `mod x;` plus one register line.
  The overlays group today holds eight registered sections plus the shared
  `demo` helper (`src/gallery/sections/overlays/mod.rs:6-14, 19-28`); plan 02
  adds `floating` there the same way; this plan adds a sibling
  `floating_canvas/` directory. No other plan edits the group's `mod.rs`
  (checked 03/04/09 seam tables: they cite overlays files but edit the
  sidebar/page/menubar seams, not the gallery).
- `GallerySection::new(id: &'static str, title, description, view: impl
  Into<AnyView>)` and `id()` are `pub(crate)`
  (`src/gallery/registry.rs:30-48`); ids/titles must be unique gallery-wide —
  `floating-canvas` does not collide with plan 02's `floating`.
- Section copy is literal English in every existing gallery section
  (`fps.rs:219-221`, plan 02's i18n note citing `sheet.rs`); no locale keys
  are added.
- Kit facade rule: no `use gpui::` anywhere; import style follows
  `src/ui/widgets/virtual_list.rs:4-6` and `fps.rs:6-8` (enforced by
  `tests/qa_migration.rs:386-405`).
- `_subscriptions: Vec<Subscription>` field pattern for stored observers:
  `src/features/pages/form_page.rs:116`, `notifications.rs:8`.
- `rems()` for frame heights (`fps.rs:190`); `tracing::warn!` with
  `target: "gpui_starter::gallery"` on failures
  (`src/gallery/sections/markdown/editor.rs:807-811`).
- `AppError` has no generic-message variant (`src/foundation/errors.rs:14-33`
  lists PathInitialization/StateRead/StateParse/StateWrite/InvalidDeepLink/Io),
  so spawn failures log via `tracing::warn!` instead of `events::emit_error`.

Crate picks: none. No `Cargo.toml` change, no lock change, no new deps; the
feature uses `gpui-kit` plus std only. It builds no `RenderImage`, so plan
05's future `image` table entry is irrelevant here.

## Architecture

Entities and state ownership:

- `FloatingCanvasSection` (`Render` entity, one per gallery build) owns the
  two shared models and the spawn bookkeeping:
  `paint_canvas: Entity<CanvasState>` (one mini-paint document),
  `metrics: Entity<HudMetrics>` (one live sample stream), `_subscriptions`
  holding `cx.observe(&paint_canvas, ..)` and `cx.observe(&metrics, ..)`,
  both `|_, _, cx| cx.notify()`. It renders the embedded mini-paint preview,
  the live metrics readout, and the spawn/close controls.
- `MiniPaintPanel` (`Render` view, mounted only inside a float): toolbar
  (color `ToggleGroup`, width `ToggleGroup`, `Clear` button) over
  `Canvas::new("mini-paint-float", canvas)` inside `div().flex_1().min_h_0()`.
  Holds the same `Entity<CanvasState>` clone; observes it per the plan 01
  mount obligation. Brush changes go through
  `canvas.update(.., set_builtin_tool(Brush(cfg)))`; the tool swap is visible
  to the embedded preview because it is engine state, not panel state.
- `HudMetrics` (model entity, no render): `samples: Vec<f32>` (capped ring,
  `HUD_SAMPLE_CAP`), `tick: u64`. `new` spawns the producer task
  (`cx.spawn`, weak handle, `timer(HUD_TICK)` loop, `tick()` + `cx.notify()`
  per iteration, `break` on `Err`). The section holds the strong
  `Entity<HudMetrics>` for the app lifetime (gallery sections are built
  eagerly, `src/gallery/registry.rs:74-79`), so the 4 Hz producer runs while
  the app runs — same shape as the fps gallery scene's permanent 60 fps
  request_animation_frame loop (`fps.rs:170`); see Risks.
- `HudPanel` (`Render` view, one per HUD float): owns its own display-side
  `Entity<CanvasState>` (grid off, `Pan` tool) and observes `metrics`; each
  observation runs `refresh`, which swaps one `CanvasShape::Path` in the
  panel's canvas document from the current sample window. The path is display
  state per window; the samples are the shared truth.
- `ShaderPanel` (`Render` view): self-contained. `frame: usize` clock,
  `playing: bool`, `cell: Pixels`. Render requests the next animation frame
  when `playing && !cx.reduce_motion()`, then paints the field through a
  gpui `canvas()` closure as a grid of `fill` quads colored by
  `shader_hsv(u, v, t)`. No shared state; each spawn is an independent toy.

Events and data flow:

- Mini-paint: pointer event in either window -> engine router (plan 01) ->
  `BrushTool` commits a `CanvasShape::Path` via `edit_document` ->
  `cx.notify()` on `CanvasState` -> `App::notify` invalidates both tracking
  windows -> both mounts repaint. Toolbar changes flow the same way through
  `set_builtin_tool`.
- HUD: producer timer -> `HudMetrics::tick` (push `sample_value(tick)`,
  `cx.notify()`) -> every `HudPanel` observer and the section readout
  refresh. N HUD windows means N observers of one entity.
- Spawn: section button -> build `FloatingSpec` -> `spawn_floating(spec,
  factory, cx)` with a factory capturing the shared entity clone. Detach on
  an already-open id calls `respawn_floating(&id, cx)`, plan 02's documented
  close-and-reopen refresh path; the registry's retained factory re-captures
  the same `Entity<CanvasState>`, so the document survives the respawn
  (entity identity, not view identity, carries the state; plan 02's
  builder-retention note).
- Close: shell close button / Esc (plan 02 `DismissFloating`) or the
  section's "Close Floating Canvases" button calling `close_floating` per
  tracked handle.

Module tree:

```text
src/gallery/sections/overlays/floating_canvas/
├── mod.rs        register() + FloatingCanvasSection (embedded preview, controls, status)
├── mod.test.rs
├── paint.rs      MINI_PAINT_ID, new_paint_canvas, MiniPaintPanel + factory
├── paint.test.rs
├── hud.rs        HudMetrics + producer + sample_value, HudPanel, spawn_hud spec
├── hud.test.rs
├── shader.rs     shader_hsv + grid math, ShaderPanel, SHADER_ID
└── shader.test.rs
```

## Interface Contract

Leaf demo feature; nothing downstream consumes it. Complete public surface:

```rust
// src/gallery/sections/overlays/floating_canvas/mod.rs
use gpui_kit::{App, Window};
use crate::gallery::registry::GallerySection;

/// Pushes the single "Floating Canvas" gallery section. Called once from the
/// overlays group's register().
pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App);
```

`pub(crate)` surface (crate-internal, exercised by sibling tests):

```rust
pub(crate) const MINI_PAINT_ID: &str = "canvas-mini-paint";
pub(crate) const HUD_ID_PREFIX: &str = "canvas-hud-";
pub(crate) const SHADER_ID: &str = "canvas-shader-toy";
pub(crate) const HUD_TICK: Duration;      // 250 ms
pub(crate) const HUD_SAMPLE_CAP: usize;   // 120

pub(crate) struct FloatingCanvasSection { /* private */ }
impl FloatingCanvasSection {
    pub(crate) fn view(window: &mut Window, cx: &mut App) -> Entity<Self>;
    pub(crate) fn detach_paint(&mut self, cx: &mut Context<Self>);       // spawn or respawn
    pub(crate) fn spawn_hud(&mut self, cx: &mut Context<Self>);          // cascading id
    pub(crate) fn open_shader(&mut self, cx: &mut Context<Self>);        // spawn or respawn
    pub(crate) fn close_ours(&mut self, cx: &mut Context<Self>) -> usize;
}

pub(crate) fn new_paint_canvas(cx: &mut App) -> Entity<CanvasState>;
pub(crate) struct MiniPaintPanel { /* private */ }
impl MiniPaintPanel {
    pub(crate) fn new_view(canvas: Entity<CanvasState>, window: &mut Window, cx: &mut App) -> AnyView;
    pub(crate) fn set_brush(&mut self, config: BrushConfig, cx: &mut Context<Self>);
}

pub(crate) struct HudMetrics { /* private */ }
impl HudMetrics {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self;   // spawns producer
    pub(crate) fn tick(&mut self, cx: &mut Context<Self>);
    pub(crate) fn samples(&self) -> &[f32];
    pub(crate) fn current(&self) -> f32;
    pub(crate) fn tick_count(&self) -> u64;
}
pub(crate) fn sample_value(tick: u64) -> f32;            // pure, [0.08, 0.92]
pub(crate) fn sparkline_points(samples: &[f32], bounds: Bounds<Pixels>) -> Vec<Point<Pixels>>; // pure

pub(crate) struct ShaderPanel { /* private */ }
impl ShaderPanel {
    pub(crate) fn new_view(window: &mut Window, cx: &mut App) -> AnyView;
}
pub(crate) fn shader_hsv(u: f32, v: f32, t: f32) -> Hsla;  // pure
pub(crate) fn grid_dims(bounds: Bounds<Pixels>, cell: Pixels) -> (usize, usize); // pure, cell-count clamped
pub(crate) fn grid_cell_bounds(bounds: Bounds<Pixels>, cell: Pixels, gx: usize, gy: usize) -> Bounds<Pixels>; // pure
```

Behavioral guarantees: `spawn_hud` never reuses an id present in
`floating_snapshot`; `detach_paint`/`open_shader` are idempotent per id
(respawn path); `close_ours` closes only ids this section spawned
(`MINI_PAINT_ID`, `HUD_ID_PREFIX*`, `SHADER_ID`); spawn failures log
(`tracing::warn!`, target `gpui_starter::gallery`) and never panic.

## Module Layout

| File | Purpose |
|---|---|
| `src/gallery/sections/overlays/floating_canvas/mod.rs` | Removal-recipe header, `register`, `FloatingCanvasSection` (preview mount, controls, status line, id bookkeeping) |
| `src/gallery/sections/overlays/floating_canvas/paint.rs` | Mini-paint: window id, `new_paint_canvas`, `MiniPaintPanel` (toolbar + engine mount), factory |
| `src/gallery/sections/overlays/floating_canvas/hud.rs` | `HudMetrics` model + producer, `sample_value`, `sparkline_points`, `HudPanel`, `spawn_hud` spec builder |
| `src/gallery/sections/overlays/floating_canvas/shader.rs` | `shader_hsv`, grid math, `ShaderPanel` (frame-clock animation + quad-grid paint), window id |
| `src/gallery/sections/overlays/floating_canvas/*.test.rs` | Sibling test modules, wired `#[cfg(test)] #[path = "..."] mod ..._test;` per repo convention |

## Seam Touches

One edit outside the new module:

| # | File | Change | Why minimal |
|---|---|---|---|
| 1 | `src/gallery/sections/overlays/mod.rs` | `mod floating_canvas;` inserted after `mod dialog;` (after plan 02's `mod floating;` if it has landed — alphabetical), and `floating_canvas::register(sections, window, cx);` inserted after `dialog::register(...)` in `register` | The gallery's documented two-line section hook (`src/gallery/registry.rs:1-15`) |

Deliberate non-touches: no `src/ui/` edit (plans 01/02 modules are consumed,
not modified), no `Cargo.toml`/`Cargo.lock` change, no `locales/` keys
(gallery copy is literal English by precedent), no sidebar/route/page-count
changes (gallery is one existing page), no capability-registry entries
(platform variance is plan 02's `floating_*` entries; this feature adds none),
no `app::init` change.

### Removal Recipe

`mod.rs` opens with a header mirroring `src/gallery/mod.rs:2-7`:

```rust
//! Floating-canvas gallery section: canvas mini-apps in floating windows.
//! Removal = delete this directory and revert its single seam touch: the
//! `mod floating_canvas;` and `floating_canvas::register` lines in
//! src/gallery/sections/overlays/mod.rs.
```

Ordered removal steps:

1. Revert `src/gallery/sections/overlays/mod.rs` (drop `mod floating_canvas;`
   and the register call).
2. `rm -r src/gallery/sections/overlays/floating_canvas/`.
3. Floats still open at runtime are plan 02 windows; closing them or
   restarting the app clears them. Nothing persists.
4. Run the gates in Verification; they must pass with zero references
   (`grep -rn floating_canvas src/` returns nothing).

## Implementation Phases

## Phase 1: Section scaffold

### Step 1.1: Module dir and register hook

- Files: `src/gallery/sections/overlays/floating_canvas/mod.rs` (new),
  `src/gallery/sections/overlays/mod.rs` (edit).
- Build: `mod.rs` with the removal-recipe header, `mod hud; mod paint; mod
  shader;` declarations backed by three stub files (each a one-line
  `// placeholder` comment so the crate compiles; real content lands in
  Phases 2-4), the `register` fn pushing `GallerySection::new(
  "floating-canvas", "Floating Canvas", "Canvas mini-apps in floating
  utility windows: shared-state mini paint, live sparkline HUDs, a shader
  toy.", FloatingCanvasSection::view(window, cx))`, and a minimal
  `FloatingCanvasSection { _subscriptions: Vec<Subscription> }` (`Render`,
  `view(window, cx)` = `cx.new(|cx| Self::new(window, cx))`, render =
  `v_flex().w_full().gap_4().p_4()` with a `text_sm` muted "Under
  construction" child, removed in Phase 2). Group `mod.rs` edit per Seam
  Touches. Imports: `use gpui_kit::{prelude::?, *};` facade style
  (`fps.rs:6-8`).
- Tests: none.
- Done-check: `cargo check -p gpui-starter` passes;
  `grep -n "floating_canvas" src/gallery/sections/overlays/mod.rs` prints both
  hook lines.

### Step 1.2: Test wiring and register smoke

- Files: `src/gallery/sections/overlays/floating_canvas/mod.test.rs` (new),
  wiring line at the bottom of `mod.rs`
  (`#[cfg(test)] #[path = "mod.test.rs"] mod mod_test;`).
- Build: one `#[gpui_kit::test]` async test with a minimal in-test stub view
  (`struct StubView;` + `impl Render` returning `div()`), then:
  `let handle = cx.add_window(|window, cx| StubView);` (build closure shape
  per `GPUI/app/test_context.rs:279-296`), `let cx =
  VisualTestContext::from_window(handle.into(), &cx);`, then `cx.update(
  |window, cx| { let mut sections = vec![];
  super::register(&mut sections, window, cx); assert_eq!(sections.len(), 1);
  assert_eq!(sections[0].id(), "floating-canvas"); });`. `GallerySection` and
  `id()` are `pub(crate)` (`src/gallery/registry.rs:30-48`), reachable
  crate-internally. Do not go through `registry::sections` (it constructs
  every section; heavy and brittle here).
- Tests: the one above.
- Done-check: `cargo test -p gpui-starter --lib floating_canvas` green.

## Phase 2: Mini-paint (shared-state detachable window)

### Step 2.1: Canvas factory and panel

- Files: `src/gallery/sections/overlays/floating_canvas/paint.rs`,
  `paint.test.rs` (new).
- Build:
  - `pub(crate) const MINI_PAINT_ID: &str = "canvas-mini-paint";`
  - `new_paint_canvas(cx: &mut App) -> Entity<CanvasState>`:
    `cx.new(|cx| { let mut s = CanvasState::new(); s.add_layer(cx);
    s.set_builtin_tool(BuiltinTool::Brush(BrushConfig::default()), cx); s })`
    (grid stays at plan 01's visible default).
  - `MiniPaintPanel { canvas: Entity<CanvasState>, brush: BrushConfig,
    _subscriptions: Vec<Subscription> }`; `new(canvas, window, cx)` stores
    `cx.observe(&canvas, |_, _, cx| cx.notify())` (plan 01 mount
    obligation); `new_view(canvas, window, cx) -> AnyView` =
    `cx.new(|cx| Self::new(canvas, window, cx)).into()`.
  - `set_brush(&mut self, config: BrushConfig, cx)`: mirror `self.brush =
    config;` then `self.canvas.update(cx, |s, cx|
    s.set_builtin_tool(BuiltinTool::Brush(config), cx)); cx.notify();`.
  - Render: `v_flex().size_full().min_h_0()`: a compact toolbar
    `h_flex().gap_2().p_2().border_b_1().border_color(cx.theme().border)`
    holding a color `ToggleGroup::new("paint-color").small().segmented()`
    with `Toggle` children "Ink" (black), "Accent" (`cx.theme().primary`),
    "Danger" (`cx.theme().danger`), "Warn" (`cx.theme().warning`), `.checked`
    derived from `self.brush.color` equality, `on_click(cx.listener(|this,
    values: &Vec<bool>, _, cx| { let ix = values.iter().position(|b| *b);
    if let Some(ix) = ix { this.set_brush(config_for(ix), cx); } }))`; a
    width `ToggleGroup::new("paint-width").small().segmented()` with "Fine"
    (`px(2.)`), "Medium" (`px(4.)`), "Broad" (`px(10.)`), checked from
    `self.brush.width`; a ghost small `Button::new("paint-clear").label(
    "Clear").on_click(cx.listener(|this, _, _, cx|
    this.canvas.update(cx, |s, cx| s.clear(cx))))`. Below it
    `div().flex_1().min_h_0().child(Canvas::new("mini-paint-float",
    self.canvas.clone()))` — engine `Canvas` fills its parent, no style chain
    (plan 07 mount idiom, `07:889`). `config_for(ix)` is a private fn
    mapping the palette index to `BrushConfig` (uses the same four colors;
    the currently selected width is kept).
  - Palette colors are document data (the paint is the content), exempt from
    the no-raw-color rule like the fps scene palette (`fps.rs:279-291`,
    backdrop precedent comment `fps.rs:187-193`); chrome (borders) stays on
    theme tokens.
- Tests (`paint.test.rs`, `#[gpui_kit::test]`):
  - `new_paint_canvas` yields `active_tool_id() == "brush"` and
    `document().layers().len() == 1`.
  - Brush stroke through the tool protocol: build the pointer event and
    tool context by hand (plan 05's drive shape, `05:227`) —
    `canvas.update(cx, |s, cx| { let tool_ctx = ToolContext { viewport:
    Bounds { origin: Point::default(), size: size(px(480.), px(360.)) },
    transform: s.transform(), hit: None }; let ev = CanvasPointerEvent {
    screen_position: point(px(10.), px(10.)), doc_position: point(px(10.),
    px(10.)), button: MouseButton::Left, click_count: 1, modifiers:
    Modifiers::default() }; s.dispatch_tool_event(cx, |t, s, cx|
    t.pointer_down(&ev, &tool_ctx, s, cx)); })` plus two `pointer_move`
    calls (> 1 px apart in screen space) and one `pointer_up`; assert
    `document().shapes().count()` grew by exactly one and the new shape is a
    `CanvasShape::Path`.
  - Shared-entity observer fan-out (the headless stand-in for the
    cross-window repaint proof): define a test host `struct ObsHost {
    canvas: Entity<CanvasState>, observed: usize, _subscriptions:
    Vec<Subscription> }` whose `new` stores `cx.observe(&canvas, |this, _,
    cx| { this.observed += 1; cx.notify(); })`; mount one host in each of
    two `cx.add_window`s against the SAME entity; then
    `cx.update(|cx| canvas.update(cx, |s, cx| s.pan_by(point(px(5.),
    px(0.)), cx)))` (observers flush at the update boundary,
    `GPUI/app.rs:1874-1877`); assert both hosts' `observed` advanced by 1
    and `canvas.read(cx).pan() == point(px(5.), px(0.))`. The
    window-invalidator half of the mechanism is source-verified
    (`GPUI/app.rs:2931-2960`) and covered by manual QA step 2.
- Done-check: `cargo test -p gpui-starter --lib floating_canvas::paint`
  green.

### Step 2.2: Detach, respawn, embedded preview

- Files: `mod.rs`, `paint.rs`, `mod.test.rs`.
- Build in `mod.rs`:
  - `FloatingCanvasSection` gains `paint_canvas: Entity<CanvasState>` (from
    `new_paint_canvas`), `_subscriptions` extended with the observe, and
    `detach_paint(&mut self, cx: &mut Context<Self>)`:
    if `floating_snapshot(cx).iter().any(|e| e.id.0 == MINI_PAINT_ID)` —
    `FloatingWindowEntry.id` is a pub field (`02:336-345`), and
    `SharedString == &str` compiles (`gpui_shared_string.rs:103-107`) —
    then `respawn_floating(&FloatingWindowId(MINI_PAINT_ID.into()), cx)`;
    else build the spec (below) and `spawn_floating(spec, { let canvas =
    self.paint_canvas.clone(); move |window, cx|
    MiniPaintPanel::new_view(canvas.clone(), window, cx) }, cx)`;
    `if let Err(e) = ... { tracing::warn!(target:
    "gpui_starter::gallery", error = %e, "mini-paint float failed to
    open"); }`; end with `cx.notify()`.
  - Spec: `FloatingSpec::new(FloatingWindowId(MINI_PAINT_ID.into()),
    size(px(480.), px(360.))).title("Mini Paint")
    .level(FloatLevel::Floating).anchor(window_anchor(cx,
    Placement9::TopRight, point(px(16.), px(16.))))
    .focus(false).resizable(true).movable(true)
    .min_size(size(px(320.), px(240.)))`.
  - Private helper `fn window_anchor(cx: &App, placement: Placement9,
    offset: Point<Pixels>) -> FloatAnchor`: `root_window(cx)`
    (`src/app/window.rs:20-22`) maps to `FloatAnchor::Window { window,
    placement, offset }`; `None` falls back to `FloatAnchor::Screen {
    display: None, placement, offset }` so the section still works headless
    (tests, no main window).
  - Section render row 1: level heading text ("Mini paint"), the embedded
    preview `div().relative().w_full().h(rems(16.)).border_1()
    .border_color(cx.theme().border).rounded(cx.theme().radius)
    .overflow_hidden().child(Canvas::new("mini-paint-embedded",
    self.paint_canvas.clone()))`, a default
    `Button::new("paint-detach").label("Detach…")` wired via `cx.listener`
    to `detach_paint`, and a muted `text_sm` status
    `format!("{} strokes", self.paint_canvas.read(cx).document().shapes()
    .count())`.
- Tests (`mod.test.rs`): `detach_paint` on a fresh section ->
  `floating_snapshot(cx)` contains an entry with `id.0 ==
  "canvas-mini-paint"`; calling `detach_paint` again leaves exactly one such
  entry (respawn path); `close_floating(&entry.handle, cx)` returns true and
  a second call returns false (plan 02 Step 3.3 precedent); after close, the
  section's `paint_canvas` document still holds previously drawn shapes
  (draw one via the Step 2.1 drive shape before closing; state survived the
  window).
- Done-check: `cargo test -p gpui-starter --lib floating_canvas` green.

## Phase 3: Live sparkline HUD windows

### Step 3.1: Metrics model and producer

- Files: `hud.rs`, `hud.test.rs` (new).
- Build:
  - Consts `HUD_TICK: Duration = Duration::from_millis(250)`,
    `HUD_SAMPLE_CAP: usize = 120`, `HUD_ID_PREFIX: &str = "canvas-hud-"`.
  - `sample_value(tick: u64) -> f32`:
    `(0.55 + 0.28 * (t * 0.9).sin() + 0.14 * (t * 2.3 + 1.7).sin())
    .clamp(0.08, 0.92)` with `t = tick as f32 / 4.` (seconds at 4 Hz).
    Pure; amplitude range is `[0.13, 0.97]` before the clamp, so the clamp
    is a real bound, not a dead branch.
  - `HudMetrics { samples: Vec<f32>, tick: u64 }`; `new(cx: &mut
    Context<Self>)` builds the empty state and spawns the producer
    (scroller.rs:269-296 shape):
    `cx.spawn(async move |this, cx| { loop {
    cx.background_executor().timer(HUD_TICK).await;
    if this.update(cx, |m, cx| m.tick(cx)).is_err() { break; } }
    }).detach();`
  - `tick(&mut self, cx)`: `self.tick += 1;
    self.samples.push(sample_value(self.tick)); if self.samples.len() >
    HUD_SAMPLE_CAP { self.samples.drain(..1); }` then one `cx.notify()`.
    Readers `samples`, `current` (last sample, `0.` when empty),
    `tick_count`.
- Tests: `tick` growth and cap (`len() <= HUD_SAMPLE_CAP` after 200 ticks;
  first element advanced past the earliest sample); `sample_value` within
  `[0.08, 0.92]` for ticks `0..1000` and strictly different across ticks
  `0..10`; producer through the fake clock: create the entity inside a test
  window, `cx.executor().advance_clock(Duration::from_millis(250 * 8));
  cx.run_until_parked();` then `entity.read(&cx).samples().len() == 8`
  (executor/advance/run_until_parked at `GPUI/app/test_context.rs:254-256`,
  `GPUI/executor.rs:196-200`, `test_context.rs:563-566`).
- Done-check: `cargo test -p gpui-starter --lib floating_canvas::hud` green.

### Step 3.2: Sparkline mapping and HUD panel

- Files: `hud.rs`, `hud.test.rs`.
- Build:
  - `sparkline_points(samples: &[f32], bounds: Bounds<Pixels>) ->
    Vec<Point<Pixels>>`: empty input -> empty; else `x = origin.x + i as f32
    / (len - 1).max(1) as f32 * width`; `y = origin.y + (1. - v) * height`
    (samples are already in `[0.08, 0.92]`, fixed mapping, no min/max pass).
  - `HudPanel { metrics: Entity<HudMetrics>, canvas: Entity<CanvasState>,
    path_id: Option<ShapeId>, _subscriptions: Vec<Subscription> }`;
    `new(metrics, window, cx)`: `canvas = cx.new(|cx| { let mut s =
    CanvasState::new(); s.set_grid(CanvasGrid { visible: false, spacing:
    px(16.) }, cx); s.set_builtin_tool(BuiltinTool::Pan, cx);
    s.add_layer(cx); s })`; store `cx.observe(&metrics, |this, _, cx|
    this.refresh(cx))`; `new_view` like `MiniPaintPanel`.
  - `refresh(&mut self, cx: &mut Context<Self>)`: `let samples =
    self.metrics.read(cx).samples().to_vec(); let stroke = StrokeStyle {
    width: px(2.), color: cx.theme().primary, dash: None }; let old =
    self.path_id; self.canvas.update(cx, |s, cx| { if let Some(old) = old {
    s.remove_shape(old, cx); } let layer =
    s.document().top_layer().unwrap_or_else(|| s.add_layer(cx));
    self.path_id = s.add_shape(layer, CanvasShape::path(sparkline_points(
    &samples, HUD_DOC_BOUNDS), stroke.clone()), cx); }); cx.notify();`
    with `const HUD_DOC_BOUNDS: Bounds<Pixels>` = origin `(0, 0)`, size
    `(256, 96)`, authored against the spawn size minus shell chrome. The
    single remove+add swap keeps exactly one shape alive per refresh.
  - Render: `v_flex().size_full()`: header `h_flex().justify_between().px_2()
    .py_1().border_b_1().border_color(cx.theme().border)` with muted
    `text_sm` "Load" and foreground `text_sm` `format!("{:.2}",
    self.metrics.read(cx).current())`; then
    `div().flex_1().min_h_0().child(Canvas::new("hud-spark",
    self.canvas.clone()))`.
  - `spawn_hud` spec (used by the section): size (260, 150), title "HUD",
    `level(FloatLevel::AlwaysOnTop)`, anchor `window_anchor(cx,
    Placement9::BottomRight, point(px(-24. * n), px(-24. * n)))` cascading
    for the n-th live HUD, `focus(false)`, `resizable(false)`, `movable(
    true)` (Windows ignores move for the PopUp kind, plan 02 platform
    matrix), `min_size(size(px(200.), px(110.)))`.
- Tests: `sparkline_points` geometry (empty input; first point at bounds
  origin; last at the right edge; y within bounds; monotone x; length equals
  sample length); `HudPanel::refresh` behavior — construct a panel entity
  against a seeded metrics entity (8 manual `tick`s), call `refresh` twice
  through `cx.update`, assert the panel canvas document holds exactly one
  shape, it is a `Path`, and its points length equals the sample count
  (remove-then-add leaves no stale shape).
- Done-check: `cargo test -p gpui-starter --lib floating_canvas::hud` green.

### Step 3.3: Section row and multi-window spawn

- Files: `mod.rs`, `mod.test.rs`.
- Build: the section gains `metrics: Entity<HudMetrics>` (created in `new`
  via `cx.new(|cx| HudMetrics::new(cx))`) plus its observe; `spawn_hud(&mut
  self, cx)`: pick the first `i` (from 0) whose
  `format!("{HUD_ID_PREFIX}{i}")` is absent from `floating_snapshot`; build
  the spec with cascade `n = i`; factory `{ let metrics = self.metrics.
  clone(); move |window, cx| HudPanel::new_view(metrics.clone(), window,
  cx) }`; log-on-Err as in Step 2.2; `cx.notify()`. Render row 2: heading
  "Live sparkline HUD", muted status `format!("load {:.2} · tick {} · {}
  HUD windows open", metrics.current(), metrics.tick_count(), hud_count)`
  (`hud_count` = snapshot entries whose `id.0.starts_with(HUD_ID_PREFIX)`),
  default `Button::new("hud-add").label("Add HUD…")` via `cx.listener`.
- Tests: two `spawn_hud` calls produce two distinct snapshot ids
  (`canvas-hud-0`, `canvas-hud-1`); after `advance_clock(250ms * 3)` +
  `run_until_parked`, the shared metrics entity shows 3 samples (the panels
  live inside plan 02 windows; their refresh path is already covered by the
  Step 3.2 observer test); `close_floating` on one HUD handle leaves the
  other in the snapshot.
- Done-check: `cargo test -p gpui-starter --lib floating_canvas` green.

## Phase 4: Floating shader-toy panel

### Step 4.1: Pure shader and grid math

- Files: `shader.rs`, `shader.test.rs` (new).
- Build:
  - `shader_hsv(u: f32, v: f32, t: f32) -> Hsla`:
    `hsla(((u + t * 0.05) % 1.0).max(0.), 0.85, (0.5 + ((u * 6. + t).sin()
    * 0.25 + (v * 4. - t * 1.7).sin() * 0.25)).clamp(0.08, 0.92), 1.)`
    (`hsla()` is the facade free fn; `Hsla: PartialEq` at
    `GPUI/color.rs:374-384`).
  - `grid_dims(bounds, cell) -> (cols, rows)`: `cols = ceil(w / cell)`,
    `rows = ceil(h / cell)` (through `as_f32()`, `Pixels / f32` does not
    exist); while `cols * rows > MAX_QUAD_CELLS` (private const `4096`),
    double `cell` and recompute; floor both at 1.
  - `grid_cell_bounds(bounds, cell, gx, gy)`: origin at `bounds.origin +
    (gx * cell, gy * cell)`, size `cell x cell` with the trailing edge
    clamped so the cell ends at `bounds.right()`/`bounds.bottom()` and never
    exceeds it (last column/row absorbs the remainder).
- Tests: `shader_hsv` outputs hue in `[0, 1)`, saturation at `0.85`,
  lightness in `[0.08, 0.92]`, alpha at `1.` for a sweep of `(u, v, t)`
  samples including large `t`; `shader_hsv(0.5, 0.5, 0.) !=
  shader_hsv(0.5, 0.5, 5.)`; `grid_dims` on a 400x280 bounds with `px(6.)`
  = `(67, 47)`; `grid_dims` on 400x280 with `px(1.)` = `(50, 35)` (cap
  doubling: 1 -> 112000, 2 -> 28000, 4 -> 7000, 8 -> 1750); `cell (0,0)`
  origin equals `bounds.origin`; last column of a 100-wide bounds with
  `px(30.)` ends exactly at `bounds.right()`; cells tile with no gap on a
  non-multiple width.
- Done-check: `cargo test -p gpui-starter --lib floating_canvas::shader`
  green.

### Step 4.2: Panel and spawn wiring

- Files: `shader.rs`, `mod.rs`, `shader.test.rs` (new), `mod.test.rs`.
- Build:
  - `pub(crate) const SHADER_ID: &str = "canvas-shader-toy";`
  - `ShaderPanel { frame: usize, playing: bool, cell: Pixels }`
    (`playing: true`, `cell: px(6.)`).
  - Render: `if self.playing && !cx.reduce_motion() {
    window.request_animation_frame(); }` (the API's own decorative-motion
    contract, `GPUI/window.rs:2656-2676`; fps.rs:170 precedent) then
    `self.frame += 1; let t = self.frame as f32 / 60.;` (fps.rs clock,
    `fps.rs:36-39`, `:177`). Layout `v_flex().size_full()`: controls row
    `h_flex().gap_2().p_2().border_b_1().border_color(cx.theme().border)`:
    a small default `Button::new("shader-play").label(if self.playing {
    "Pause" } else { "Play" })` toggling `playing` + `cx.notify()`; a small
    `ToggleGroup::new("shader-cell").segmented()` with "Fine" (`px(4.)`),
    "Balanced" (`px(6.)`), "Coarse" (`px(10.)`) checked against
    `self.cell`, `on_click(cx.listener(...))` setting `self.cell` +
    `cx.notify()`. Below: `div().relative().flex_1().min_h_0().
    overflow_hidden().child(canvas(|_bounds, _, _| (), move |bounds, _,
    window, _| { let (cols, rows) = grid_dims(bounds, cell); for gy in
    0..rows { for gx in 0..cols { let cb = grid_cell_bounds(bounds, cell,
    gx, gy); let c = shader_hsv((gx as f32 + 0.5) / cols as f32, (gy as
    f32 + 0.5) / rows as f32, t); window.paint_quad(fill(cb, c)); } } })
    .absolute().size_full())` — the gpui `canvas()` element is `Styled`
    (`GPUI/elements/canvas.rs:84-88`), mounted inside a `relative()` wrapper
    exactly like `fps.rs:101-135`; the panel-owned `cell`/`t` are moved
    into the paint closure each render (fresh closure per frame, fps.rs
    pattern).
  - Section: `open_shader(&mut self, cx)` mirroring `detach_paint` with
    `SHADER_ID`, factory `move |window, cx| ShaderPanel::new_view(window,
    cx)`; spec size (400, 300), title "Shader Toy",
    `FloatLevel::Floating`, `window_anchor(cx, Placement9::Right,
    point(px(24.), px(24.)))`, `focus(false)`, `resizable(true)`,
    `min_size(size(px(280.), px(200.)))`. Render row 3: heading "Shader
    toy", default `Button::new("shader-open").label("Open Shader Panel…")`.
- Tests: `open_shader` puts `canvas-shader-toy` in the snapshot; a second
  call keeps exactly one such entry (respawn); `ShaderPanel` needs no UI
  test (paint-only view; the pure fns in Step 4.1 carry the coverage).
- Done-check: `cargo test -p gpui-starter --lib floating_canvas` green.

## Phase 5: Status assembly, cleanup control, gates

### Step 5.1: Close-ours control and copy pass

- Files: `mod.rs`.
- Build: `close_ours(&mut self, cx) -> usize`: collect snapshot handles
  whose `id.0 == MINI_PAINT_ID || id.0 == SHADER_ID ||
  id.0.starts_with(HUD_ID_PREFIX)`; `close_floating` each, count trues,
  `cx.notify()`. Render: final row with a ghost small
  `Button::new("fc-close-all").label("Close Floating Canvases")
  .disabled(ours_open == 0)` (`Disableable`, `GC/button/button.rs:524`).
  Copy pass: literal English, sentence case, ellipsis only on the three
  window-opening buttons ("Detach…", "Add HUD…", "Open Shader Panel…"),
  none on immediate actions ("Clear", "Pause", "Play", "Close Floating
  Canvases"), per design guides "Buttons and confirmation dialogs" and
  "Capitalization, punctuation, and symbols".
- Tests (`mod.test.rs`): with mini-paint + one HUD + shader open (and plan
  02's own demo float absent), `close_ours` returns 3 and the snapshot
  retains zero `canvas-*` ids; `close_ours` on nothing returns 0.
- Done-check: `cargo test -p gpui-starter --lib floating_canvas` green.

### Step 5.2: Full gates

- Files: none new.
- Commands, in order (AGENTS.md):
  1. `cargo fmt --all -- --check`
  2. `cargo clippy --workspace --all-features`
  3. `cargo test --workspace --all-features`
  4. `just wasm-check`
- Also: `grep -rn "use gpui::"
  src/gallery/sections/overlays/floating_canvas/` returns nothing (facade
  rule); `git diff --stat` shows only `src/gallery/sections/overlays/mod.rs`
  and the new directory.
- Done-check: all four gates pass; both greps clean.

## Phase 6: Optional Stretch — GPU shader toy (requires plans 07/11; NOT core)

Explicitly marked stretch. Every item below consumes outputs of plans 07
(`07-canvas-shaders.md`) or 11 (`11-wgpu-embed.md`), both on disk and read
this session; NEITHER is implemented yet, and core Phases 1-5 never reference
these items. Nothing here is required for the feature to ship.

### Step 6.1: GPU shader-toy float (needs plan 11)

- Blocked until: plan 11 lands (`src/features/wgpu/` exists).
- Contract items consumed (all verified in `11:293-370`):
  `crate::features::wgpu::WgpuEngine::{new, device, queue,
  read_texture_bytes}` (native-only, `#[cfg(not(target_family = "wasm"))]`),
  `features::wgpu::{rgba_to_render_image, FpsTracker}`,
  `CAPABILITY_WGPU` (`"wgpu_embed"`), and 11's consumption note: "Every
  displayed frame must eventually reach `window.drop_image`".
- Build: a fourth section button "Open GPU Shader Panel…" spawning a float
  whose panel builds one WGSL fragment pipeline on `WgpuEngine::device()`,
  renders into an `Rgba8Unorm | RENDER_ATTACHMENT | COPY_SRC` texture,
  reads back with `read_texture_bytes` on a background thread, wraps via
  `rgba_to_render_image`, and blits through `CanvasShape::Image` into a
  plan 01 canvas (or `img()`); per-frame `window.drop_image` on the
  previous frame's `Arc<RenderImage>` mirrors 11's `page.rs` ticker
  (`window.drop_image` at `GPUI/window.rs:5037` — the atlas-eviction duty
  this plan's core avoids by not building `RenderImage`s).
- Platform: native only. Wasm keeps the CPU quad-grid panel from Phase 4;
  the button is `#[cfg(not(target_family = "wasm"))]` and the wasm
  degradation is reported by 11's `wgpu_embed` capability entry (supported:
  false, reason), not by a new one.
- Done-check (once unblocked): gates pass; on wasm the section renders
  exactly as Phase 5 left it.

### Step 6.2: Ripple-tool shader float (needs plan 07's documented extraction)

- Blocked until: plan 07 extracts its wgpu bridge. 07's own Interface
  Contract states the wgpu runtime is private to `ShaderCanvasPage` and "if
  a later feature wants the wgpu bridge, it should extract `runtime.rs`
  rather than import through the page" (`07:329-332`); 07 also anticipates
  the shared-runtime consolidation with plan 11 (`07:1251`). Nothing
  importable exists today; this step must not import through the page.
- Build once unblocked (via the extracted bridge or layered on Step 6.1's
  engine): a shader float hosting 07's mouse-reactive ripple demo — WGSL
  uniforms driven by a `CanvasTool` subclass forwarding `CanvasPointerEvent`
  positions (07's `RippleTool` pattern, `07` tool.rs) — reusing 07's
  `ShaderParams` uniform encoding if it becomes part of the extracted
  surface.
- Done-check (once unblocked): the float animates and reacts to in-window
  pointer drags on native; wasm shows the capability-reason panel.

## Platform & Wasm Gating

- No `cfg` gate anywhere in `floating_canvas/` (Phases 1-5). Every API used
  is platform-neutral: engine paint calls (plan 01, wasm-safe),
  `paint_quad` grid, `request_animation_frame`,
  `background_executor().timer` (web dispatcher `setTimeout`,
  `gpui-pre-web-0.3.8/src/dispatcher.rs:278-286`), `spawn_floating` (plan 02
  routes native vs overlay internally).
- Stretch Phase 6 is the only gated code: Step 6.1 is
  `#[cfg(not(target_family = "wasm"))]` end to end (plan 11's engine is
  native-only by contract); Step 6.2 inherits 07's gating (native runtime,
  wasm capability-reason panel).
- Wasm degradation path (inherited from plan 02, restated for QA): all three
  apps open as in-window overlay panes anchored at the same placements;
  mini-paint state sharing degenerates to same-window two-mount sharing
  (still one `Entity<CanvasState>`, both mounts repaint); HUDs and the
  shader panel run identically inside panes; the `floating_windows`
  capability shows degraded with the overlay reason (plan 02 capability
  table).
- Platform behavior matrix for the three windows (inherits plan 02's
  verified matrix):

| Behavior | macOS | Windows | X11 | Wayland | wasm |
| --- | --- | --- | --- | --- | --- |
| Mini paint / shader (Floating) | NSFloating panel, resizable, non-stealing | normal window, not topmost, `SW_SHOWNOACTIVATE` | transient-for | xdg parent | overlay pane |
| HUD (AlwaysOnTop) | NSPopUp level, all spaces, non-activating, not resizable | WS_EX_TOPMOST tool window, not draggable/resizable (style 0) | NOTIFICATION type, WM-dependent stacking | parent + kind level | overlay pane, no OS topmost |
| Detach respawn | close + reopen, canvas entity survives | same | same | same | registry bounds update, pane rebuilt |
| Esc close | shell action (plan 02) | same | same | same | same |

- Capability-registry entries: none added. The `floating_windows`,
  `floating_always_on_top`, `notch_geometry` entries from plan 02 already
  report every platform variance this feature encounters; a duplicate entry
  would be noise on the diagnostics page. Stretch Step 6.1 rides plan 11's
  `wgpu_embed` entry.
- Native-only deps: none. `Cargo.toml` and `Cargo.lock` are untouched;
  nothing joins the
  `[target.'cfg(not(target_family = "wasm"))'.dependencies]` table
  (`Cargo.toml:310-353`). Stretch 6.1 would consume plan 11's already-gated
  wgpu dep, not add one.
- `just wasm-check` (`cargo +nightly check --target wasm32-unknown-unknown
  -p gpui-starter --lib`, `justfile:24-25`) must pass: the gallery (and this
  section) compiles into the lib on wasm like every existing section.

## i18n

No catalog keys. Gallery section copy is literal English by repo precedent
(`src/gallery/sections/charts/fps.rs:219-221`, plan 02's i18n note citing
`sheet.rs`); the float shell's close-button label is plan 02's existing
`floating_close` key, not this plan's. Capability reasons are plan 02's
English literals.

| key | en | zh-CN |
|---|---|---|
| (none) | | |

## Design Compliance

- Components: spawn actions are kit `Button`s (default variant for ordinary
  visible actions, ghost for the quiet close-all) with hover/pressed/focus
  states from the component; brush color/width and shader cell size are
  `ToggleGroup::segmented().small()` (compact tier, "toolbars and repeated
  professional data" density), persistent checked state per "Selection is
  part of the information model". No custom clickable divs.
- Tokens: chrome (toolbar borders, headings, status text) reads
  `cx.theme().border/foreground/muted_foreground` (fields at
  `gpui-component-0.7.1/src/theme/theme_color.rs:69,197,203`); the HUD
  sparkline stroke is `cx.theme().primary` re-read on every refresh;
  preview frames use `theme().radius` + `border_1()`. Document colors
  (brush palette, shader field) are data, the guides' raster/data
  exemption, with the fps.rs precedent (`fps.rs:187-193`, `:279-291`)
  commented at the definition.
- Fixed-pixel exceptions, all on the guides' audited list: `px(2./4./10.)`
  brush widths and `px(4./6./10.)` shader cells are raster/data geometry,
  not layout; `FloatingSpec` sizes and `HUD_DOC_BOUNDS` are platform window
  geometry; layout otherwise uses `gap_2/p_2/p_4/rems()/flex_1/min_h_0`
  scale helpers.
- Interface language: sentence-case labels, result verbs ("Clear", "Pause");
  ellipsis exactly on the three window-opening buttons per "Append it to
  every Button ... that opens a ... separate window"; no terminal periods
  on control labels; status lines are fragments without periods.
- Motion: the shader panel is the only animated surface; it requests frames
  only while `playing && !cx.reduce_motion()` (the API's own documented
  contract, `GPUI/window.rs:2656-2676`), so reduced-motion users get a
  static field and an explicit Play control. HUD sample updates are data
  refresh at 4 Hz, not decorative transition, so they continue under
  reduced motion.
- A11y: all controls are kit components with real roles and labels; no
  icon-only buttons exist in this feature. Engine canvases paint no a11y
  nodes (plan 01 v1 limitation, inherited and documented there); the
  surrounding chrome carries the text state (stroke count, load value, open
  counts), and HUD values are deliberately not `a11y_live` (4 Hz
  announcements would flood assistive tech). Keyboard path: every button is
  focusable in the float shell (plan 02 focus tracking); Esc closes floats.
- Layout: each float is a "utility window" shell (one task, short action
  path, design guides "Choose a stable shell"); the section page keeps the
  gallery's single scroll owner, appending three titled rows plus the
  cleanup row.

## Verification

Repo gates (AGENTS.md, run in Phase 5.2 exactly as named):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

Feature-specific automated coverage (wired in the phases):
`cargo test -p gpui-starter --lib floating_canvas` covers the register
smoke, mini-paint tool/config/shared-entity observer fan-out,
detach/respawn/close lifecycle with state survival, metrics tick/cap/
determinism, producer via fake clock, sparkline geometry, panel refresh
swap, multi-HUD ids, shader field and grid math, and the close-ours sweep.

Manual QA (macOS dev flow `bash scripts/macos-dev-app.sh`,
CONTRIBUTING.md:32-38; Linux/Windows hosts where available):

1. Gallery -> Overlays -> "Floating Canvas": three rows render; embedded
   mini-paint draws with the brush; stroke count increments live.
2. "Detach…": float opens top-right of the main window without stealing
   focus; draw in the float, the embedded preview updates in the same
   frame; draw in the embedded preview, the float updates. Toolbar color
   and width changes apply to both surfaces; "Clear" empties both.
3. Close the float (X or Esc), "Detach…" again: window reopens with the
   previously drawn strokes (entity survived respawn).
4. "Add HUD…" twice: two always-on-top HUD windows at cascading offsets,
   both sparklines advancing at 4 Hz in lockstep, section "load/tick"
   readout advancing; HUDs stay above another app's window on macOS
   (all spaces) and Windows (topmost); on a WM that demotes them, they
   still update (documented X11 caveat).
5. "Open Shader Panel…": animated field; Pause freezes; Play resumes;
   cell-size toggle changes resolution; enabling the system
   reduced-motion setting freezes the field until Play is re-pressed.
6. "Close Floating Canvases": all three kinds close, button disables at
   zero; plan 02's own "Floating Windows" demo float (if open) survives.
7. Diagnostics page: `floating_windows` rows report per plan 02; no new
   rows from this feature.
8. Wasm (`just wasm-build` plus the repo web flow): all three apps open as
   in-window overlay panes at the same anchors; mini-paint preview and pane
   stay in sync; HUDs tick; shader animates; Esc and Close work.
9. Both light and dark themes: toolbar chrome, HUD stroke, status text
   legible; brush palette unaffected by theme (data).

Acceptance criteria: the four gates pass with only the Seam Touches files
changed; the automated suite above is green; manual items 1-6 and 9 confirmed
on a dev host (7-8 where the platform is available), or waived with a note
naming what was checked and how.

## Risks & Open Questions

Ranked:

1. **Depends on unimplemented plans.** Plans 01 and 02 are documents, not
   code, at planning time (`src/ui/mod.rs:1` carries only
   `pub mod widgets;`; `src/ui/canvas/` and `src/ui/floating/` do not exist).
   If either contract shifts (e.g. `spawn_floating` signature,
   `CanvasState` mutator names), this plan's phases must be reconciled
   against the landed code before Phase 1 starts.
2. **Windows AlwaysOnTop HUDs are not draggable** (PopUp style 0;
   `start_window_move` no-op, plan 02 risk 2). The cascade offset at spawn
   is the only positioning; users cannot drag HUDs on Windows. Documented
   in the platform matrix; accepted for a demo HUD.
3. **HUD fixed doc viewport.** The sparkline path is authored against
   `HUD_DOC_BOUNDS` (256x96), correct while the HUD keeps its spawn size
   (non-resizable by level on every platform; overlay panes use spec size).
   If a future plan makes HUDs resizable, the path must switch to
   bounds-driven geometry (fps.rs `on_prepaint` capture pattern,
   `fps.rs:180-200`).
4. **Metrics producer runs for the app lifetime.** Gallery sections are
   built eagerly when the page is constructed
   (`src/gallery/registry.rs:74-79`) and the page lives as long as the app,
   so the 4 Hz weak-entity tick loop never sees release. Cost is one timer
   wake plus a capped `Vec<f32>` push per 250 ms — the same shape as the
   fps gallery scene's permanent frame loop (`fps.rs:170`). If that ever
   matters, gate the loop on `floating_snapshot` HUD count inside `tick`.
5. **Producer timer on wasm is verified by source only.**
   `setTimeout`-backed timers (`gpui-pre-web-0.3.8/src/dispatcher.rs:278-286`)
   were read, not run; manual QA step 8 covers the runtime check.
6. **Two mounts of one canvas double the paint work** (embedded preview +
   float): per-window keyed caches in plan 01's engine mean the shape path
   is tessellated once per window per revision. Fine at demo scale; noted
   so nobody reads the cross-window notify as cross-window caching.
7. **Shader quad-grid resolution.** `MAX_QUAD_CELLS = 4096` bounds the
   per-frame quad count; at the default 6 px cells a 400x280 pane uses
   ~3.1k quads. The look is deliberately low-resolution; the per-frame
   `RenderImage` alternative needs the `drop_image` drain discipline
   (plan 11's obligation) and is the Phase 6 stretch, not core.
8. **Engine a11y gap inherited** (plan 01 risk 7): canvas contents are
   invisible to assistive tech; text chrome carries the state. No new
   exposure here.

Open questions (each with what was tried):

- **Whether stretch 6.2 can ever consume plan 07 directly: no, not as
  written.** 07's Interface Contract keeps the wgpu bridge private to
  `ShaderCanvasPage` and directs later features to an extraction
  (`07:329-332`), anticipating consolidation with plan 11 (`07:1251`).
  Tried: read 07's full contract and module layout this session; nothing
  outside `ShaderCanvasPage::new` is exported. Step 6.2 is therefore gated
  on that extraction, not on guessed details.
- **Plan 11's engine is read from its contract only** (`11:293-370`); no
  implementation exists to compile against yet (`src/features/` has no
  `wgpu/` directory — checked with `ls` this session). Step 6.1 is gated on
  11 landing.
- **Kit `test-support` module** (element snapshots) remains disabled in this
  repo (survey Open Questions; `gpui-kit-0.7.1/Cargo.toml` features read
  during the survey). Tests here use `#[gpui_kit::test]`,
  `VisualTestContext`, `advance_clock`, and `run_until_parked`, all verified
  on disk this session; no snapshot tooling is assumed.
- **Window re-render counts are not asserted headless.** Test windows have
  no frame loop and `refresh_windows` only marks dirty
  (`GPUI/app.rs:1237-1239`, `:2011-2018`); tried to find a synchronous
  redraw driver in `GPUI/app/test_context.rs` (its `draw` helper renders a
  one-off element, not the window). The cross-window test therefore asserts
  entity-observer fan-out; the invalidator half is source-verified and
  covered by manual QA step 2.
- **No gates were run during planning** (doc-only scope, same as the survey
  and plans 01/02). Verification lists the exact commands for the
  implementer.
