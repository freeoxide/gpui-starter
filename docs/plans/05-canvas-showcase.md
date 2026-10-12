# 05 Canvas Showcase Plan

## Goal

A "Canvas Showcase" sidebar page at `src/features/pages/canvas_showcase/` mounts six isolated demos on one page, switched by a tab bar: shape editing with selection and resize handles, a Bézier path editor with draggable control points, a node graph with draggable nodes, Bézier edges, and rubber-band selection, an animated particle field, text-on-canvas, and image-on-canvas. Every demo is a self-contained entity view owning its own `Entity<CanvasState>` from plan 01's engine; no demo shares state with another. Pan/zoom/middle-drag work in every demo through the engine's router defaults, and the particle animation runs on the real `Window::request_animation_frame` API verified in the gpui-pre 0.3.8 sources.

## Dependencies

Plan 01 (`01-canvas-foundation.md`); its Interface Contract must be implemented and landed first. Items consumed, by contract section (line numbers refer to the contract code block in `01-canvas-foundation.md`, re-verified against that file this session):

- Mount + lifecycle: `Canvas::new(id, state)` (contract `:322`), the `cx.observe(&state, |_, _, cx| cx.notify())` requirement (`:209-210`, consumption notes `:336`), `EventEmitter<CanvasEvent>` (`:214`).
- State mutators: `edit_document` (`:240`), `add_layer`/`add_shape` (`:241-242`), `remove_shape` (`:243`), `clear` (`:245`); document mutator `CanvasDocument::update_shape` (`:195`).
- Selection and hit testing: `selected_shape_ids` (`:248`), `hit_test` (`:249`), `select_shape` (`:250`), `clear_selection` (`:251`).
- View state and tools: `set_tool` (`:254`), `set_builtin_tool` (`:255`), `set_tool_overlay` (`:260`), `reset_view` (`:230`), `set_grid` (`:234`), readers `pan`/`zoom`/`transform` (`:221-223`), `document` (`:237`), `is_pointer_active` (`:263`).
- Scene types: `CanvasShape` + all five constructors and `doc_bounds`/`translate` (`:149-168`), `ShapeStyle` constructors (`:139-145`), `StrokeStyle` (`:128-133`), `CanvasGrid` (`:201-202`), `CanvasDocument` readers (`:185-190`), `LayerId`/`ShapeId` (`:104-107`).
- Tool protocol: `CanvasTool` trait with `pointer_down`/`pointer_move`/`pointer_up` default bodies (`:294-304`), `CanvasPointerEvent` (`:268-274`), `ToolContext` incl. `hit` (`:288`), `BuiltinTool::Pan` (`:312`), and the pub(crate) `dispatch_tool_event` test seam (`:330-331`, usage pattern in plan 01 Step 4.2; inherent methods on a re-exported pub type are callable crate-wide).
- Engine router rules consumed as-is: middle-button drag pans regardless of tool, wheel pans, ctrl/cmd+wheel zooms at the cursor, listeners re-registered every frame (contract consumption notes `:338`).

Deliberately NOT consumed: `zoom_to_fit` (no fit-to-content UI), `CanvasEvent` variants beyond observation, `BuiltinTool::Brush` (no freehand drawing here), `MIN_ZOOM`/`MAX_ZOOM`/`HIT_TOLERANCE` directly (the router already applies them).

## Research Digest

Verified this session against `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` (short names `GPUI` = `gpui-pre-0.3.8/src`, `GC` = `gpui-component-0.7.1/src`, `GB` = `gpui-base-0.7.1/src`, `KIT` = `gpui-kit-0.7.1/src`, `KITASSETS` = `gpui-kit-assets-0.7.1`) and the repo.

Kit import paths (load-bearing for every file this plan creates):

- The kit root glob re-exports only GPUI: `pub use ::gpui::*;` (`KIT/lib.rs:97`). `gpui-pre` has no `v_flex`/`h_flex` (grep across `GPUI/gpui.rs` and `GPUI/styled.rs`: no hits), so `use gpui_kit::*;` alone does not bring them in.
- `h_flex`/`v_flex` are defined in gpui-base (`GB/styled.rs:38` and `:46`), re-exported by gpui-component (`GC/styled.rs:7` `pub use gpui_base::{..., h_flex, v_flex};`), and exposed as `gpui_kit::component::{h_flex, v_flex}` via `pub use styled::*;` (`GC/lib.rs:115`). Every layout helper and component (`Button`, `Tab`, `TabBar`, `IconName`, `ActiveTheme`) lives behind `gpui_kit::component::...`.
- Repo convention is the two-line pair, component imports first: `use gpui_kit::component::{button::Button, h_flex, v_flex};` + `use gpui_kit::{prelude::*, *};` (`src/features/pages/notifications.rs:1-2`, `src/gallery/sections/tabs/tab_bar.rs:3-13`, `src/gallery/page.rs:4-10`). `ActiveTheme` is imported as `ActiveTheme as _` from `gpui_kit::component` (`src/gallery/page.rs:4`).

Animation frame API (the spec's explicit verification ask):

- `Window::request_animation_frame(&self)` is the real API (`GPUI/window.rs:2670-2676`): it captures `current_view()` (`GPUI/window.rs:5213`) and schedules `on_next_frame(|_, cx| cx.notify(entity))`. Doc: "If called from within a view, it will notify that view on the next frame. Otherwise, it will refresh the entire window" (`GPUI/window.rs:2664-2669`). The doc instructs decorative-animation callers to check `App::reduce_motion` and skip the frame request when set.
- `Window::on_next_frame(callback: impl FnOnce(&mut Window, &mut App) + 'static)` pushes into `next_frame_callbacks` and wakes the platform frame source (`GPUI/window.rs:2650-2658`).
- In-repo precedent, the pattern this plan copies exactly: `HilbertScene::render` (`src/gallery/sections/charts/fps.rs:166-177`) calls `window.request_animation_frame()` at the top of `render`, advances scene state inside render (`self.tilt.x += ...`, `self.frame += 1`), with the comment "The scene has to keep asking for frames to stay animated; each request notifies this view, so render runs once per frame". The same file uses a fixed frame-rate constant instead of a wall clock because "std::time::Instant has no wasm32 implementation" (`fps.rs:36-39`, `FRAMES_PER_SECOND: f32 = 60.`). Kit precedents agree: `SpeechWaveform::render` requests frames when animating (`GC/speech/waveform.rs:88-91`); the `img` element requests frames in paint for multi-frame images, gated on `window.is_window_active()` (`GPUI/window.rs:2855`) and `!cx.reduce_motion()` (`GPUI/app.rs:1222`) (`GPUI/elements/img.rs:381-387`).
- Wasm: the frame request reaches the browser. `on_next_frame` calls `platform_window.schedule_frame()` + `invalidator.wake_platform()`; the web window's `wake_frame_loop` drives `browser_window.request_animation_frame(func)` (`gpui-pre-web-0.3.8/src/window.rs:479-489`, the rAF call at `:485`). The animation loop is therefore unchanged on wasm.
- Tests may drive frames with `window.simulate_next_frame(cx) -> usize` (`GPUI/window.rs:2680-2690`, `test-support`), available through the dev-dependency `gpui` with `test-support` (`Cargo.toml:216`).

Point and Pixels arithmetic (every sketch in the phases below obeys these):

- `point<T>(x: T, y: T) -> Point<T>` is a `const fn` (`GPUI/geometry.rs:111`).
- `Pixels` derives `Add/AddAssign/Sub/SubAssign/Neg/Div/DivAssign` for `Pixels` operands only (`GPUI/geometry.rs:2670-2692`; `Div` is `Pixels / Pixels -> f32`, `:2694-2698`) and has `Mul<f32>` (`:2716-2723`). There is no `Pixels + f32` and no `Pixels / f32`: mixed arithmetic goes through `px(a.as_f32() ± b)` and `px(a.as_f32() / s)`.
- `Point<Pixels>` derives `Add/Sub` for `Point<Pixels>` (`GPUI/geometry.rs:64-79`), so `origin + point(px(8.), px(8.))` compiles while `origin + (8., 8.)` does not.

Raster image construction (image-on-canvas):

- `RenderImage::new(data: impl Into<SmallVec<[Frame; 1]>>)` (`GPUI/assets.rs:61-70`); `Frame` is `image::Frame` (`GPUI/assets.rs:4`); `RenderImage` data must be BGRA (`GPUI/assets.rs:41-42`). Exported through the kit glob (`GPUI/gpui.rs:105` `pub use assets::*`).
- `image::Frame::new(buffer: RgbaImage)` (`image-0.25.10/src/animation.rs:88`); `ImageBuffer::from_raw(width, height, container)` wraps bytes without reordering (`image-0.25.10/src/images/buffer.rs:680`); `RgbaImage = ImageBuffer<Rgba<u8>, Vec<u8>>` (`buffer.rs:1798`). `ImageBuffer::from_raw_bgra` exists but swaps BGR into RGB order (`buffer.rs:1495-1502`), the wrong direction for `RenderImage`; the demo authors BGRA byte order directly and uses plain `from_raw`.
- `Vec<Frame>` converts into `SmallVec<[Frame; 1]>` via `impl<A: Array> From<Vec<A::Item>> for SmallVec<A>` (`smallvec-1.15.2/src/lib.rs:2063`), so `RenderImage::new(vec![frame])` compiles without naming smallvec.
- gpui does not re-export the `image` crate (grep `pub use image` across `GPUI/`: no hit) and `Frame` is imported privately in `assets.rs`, so constructing a `RenderImage` from app code requires a direct `image` dependency. The repo already depends on `image 0.25` png-only on Linux and Windows (`Cargo.toml:266, 278`), cfg-gated at the use site (`src/app/assets.rs:30-36`), and the Cargo.lock root package block already lists `image`. Moving the identical spec into the common `[dependencies]` table adds it to macOS/wasm builds without adding a lock package; the pure-Rust core + png codec compiles on wasm32.

Geometry and kit chrome:

- `Bounds::from_corners(top_left, bottom_right)` (`GPUI/geometry.rs:821`), `Bounds::intersects(&other) -> bool` (`:965`), `intersect(&self, &Self) -> Self` (`:1145-1149`), `bottom_right()` (`:1368`), `contains(&point)` (`:1468`). No `Bounds::from_center` exists (checked by grep), so handle squares are built manually.
- `TabBar::new(id)`, `.selected_index(usize)`, `.on_click(closure taking &usize)`, `.child(Tab::new().label(..))`; repo usage `src/gallery/sections/tabs/tab_bar.rs:160-192`; `Tab` builders `label`/`disabled` at `GC/tab/tab.rs:483, 547`.
- `Button` builders: `outline` (`GC/button/button.rs:332`), `label` (`:371`), `accessibility_label` (`:391`), `icon(impl Into<ButtonIcon>)` (`:397`), `tooltip` (`:403`), `on_click` (`:447`); `ghost()`/`xsmall()` in-repo at `tab_bar.rs:181`; `.small()` in-repo across gallery sections.
- Theme fields exist for every color this plan uses: `primary`, `accent`, `secondary`, `group_box`, `popover`, `muted`, `muted_foreground`, `foreground`, `border` (`GC/theme/theme_color.rs:61-251`); `ActiveTheme` gives `cx.theme()` on `App` (in-repo everywhere, e.g. `notifications.rs:120`). `Hsla::opacity(&self, factor)` (`GPUI/color.rs:637`).
- Icons: the component enum the app registers is generated ONLY from `KITASSETS/default-icons.txt` (default 106; `KITASSETS/build.rs:20-33` generation rule, consumed via `COMP/icon.rs:12-38`), and the app's native asset source embeds exactly that set — full-catalog names (`PenTool`, `MousePointer2`, `RotateCcw`, `Sparkles`, …) compile through the `gpui_kit::assets` enum but render BLANK on native because their bytes are not registered (only wasm's CDN source serves them). Verified defaults used here: `Palette` (page icon — palette-as-drawing stand-in), `RotateCw` (reset; the counterclockwise variant is full-catalog only), `Play`, `Pause`. `cargo check` remains the final arbiter of generated spellings.
- Repo a11y pattern for headings and live regions: `src/features/pages/notifications.rs:48-53` (`.a11y(Role::Heading, title)` + `.a11y_live(accesskit::Live::Polite)` + `.aria_level(1)`, via `crate::accessibility::A11yExt` imported as `_` at `notifications.rs:4`).
- i18n facade: `crate::i18n::localize("key")` and `localize_with_args("key", &[("count", v)])` (`src/services/i18n.rs:3-13`); missing keys render as the key itself, and indirect calls (`title_key()`) are invisible to `tests/qa_localization.rs` (survey §5 blind spot), so this plan adds its own key-existence test (Phase 7).

Repo constraints (survey §1, §9, re-verified where cited):

- Page seam: 15 touch points enumerated in survey §1. Context-free page constructors are precedented (`AboutPage::new()` takes no args, `src/shell/root/app_root/state.rs:55`); this page follows that shape with `new(cx)` since nothing it builds needs `Window`. Sibling canvas plans append their own variants after `Page::Gallery`: plan 06 declares host `infinite-canvas` (`06-infinite-canvas.md:166`, re-verified this session), plan 07 `shader-canvas` (`07-canvas-shaders.md:452`), plan 08 `multiuser-canvas` (`08-multiuser-canvas.md:266-267`). Plan 06 claims the `canvas_*` i18n key prefix (`06-infinite-canvas.md:362-371`, keys in its i18n section). This plan therefore uses the `showcase_*` prefix and host `canvas-showcase`: no collisions.
- Kit facade only; no `use gpui::` in src/ or tests/ (`tests/qa_migration.rs:386-405`).
- Cargo.lock must be staged together with any manifest change (`tests/qa_migration.rs:167-186`).
- `#[gpui_kit::test]` + `TestAppContext`/`VisualTestContext` available in-crate (`src/state/config_store.test.rs:280-281`; plan 01 Step 7.2).
- No gates were run during planning (doc-only scope); Verification states the exact commands.

Crate picks: none new. `gpui-kit 0.7.1` (pinned facade) and workspace `image 0.25` (existing, png-only; moved to the common dependency table in Phase 6). No `rand` dependency: particle seeding uses an in-module xorshift32.

## Architecture

Entities and ownership (coding-guides "state ownership": each state in the narrowest owner that keeps it correct):

- `CanvasShowcasePage` (entity, `Render`) owns `demos: Vec<(CanvasDemo, AnyView)>` and `active_ix: usize`. It builds all six demo entities eagerly at construction (gallery precedent, `src/gallery/registry.rs:62-64`) and renders exactly one. Tab switching changes `active_ix`; unrendered demos keep their state but stop animating, because the particle tick lives in the demo's own render and a demo that is not rendered requests no frames.
- Each demo is an entity (`ShapesDemo`, `BezierDemo`, `NodeGraphDemo`, `ParticleFieldDemo`, `TextDemo`, `ImageDemo`), each owning one `Entity<CanvasState>` plus the mandatory `cx.observe(&state, |_, _, cx| cx.notify())` subscription from plan 01's contract. Isolation is structural: no demo references another's state, entity, or module beyond the shared pure helpers in `geometry.rs`.
- Demo models: the shapes/bezier/node-graph tools own their edit models (handle ids, control points, node/edge shape-id maps) because tools are the only code that runs on pointer events; the particle view owns its particle vector because it ticks from render; text/image demos have no model beyond their seeded document.
- Reset = build a fresh tool. Tool constructors (`ShapeEditTool::seed(state, cx) -> Self`, etc.) clear the document and re-seed shapes, then the view calls `state.set_tool(Box::new(tool), cx)` inside `state.update`. This is also how the view's Reset button works. No tool reach-in from the view is ever needed, and none is possible: tools are boxed inside `CanvasState`, and the view holds only the entity.
- Chrome is document data. Resize handles, Bézier control points, tangent guides, node cards, labels, and edges are ordinary `CanvasShape`s in dedicated top layers of their demo documents. This keeps every pixel and every hit test on plan 01's existing paths (engine painters, engine `hit_test` topmost-first, engine selection outline); the feature adds zero painting code and zero parallel input plumbing. Handle z-priority falls out of layer order: the engine's `hit_test` iterates layers reversed, so handle shapes always win over content shapes (plan 01 contract `hit_test`, Step 3.2).
- Events and repaint: every mutation goes through plan 01 mutators (one `cx.notify()` per coherent change); the demo view's observe turns that into its own repaint. The particle demo adds the only continuous loop: in `render`, when `playing && window.is_window_active() && !cx.reduce_motion()`, advance the model by one fixed step, write positions through one `edit_document`, then `window.request_animation_frame()`. That is the exact `fps.rs` mechanism.
- Data flow summary: platform pointer event → engine router (plan 01 `element.rs`) → `dispatch_tool_event` → demo tool mutates the document via `CanvasState` mutators → notify → demo view observe → repaint → engine paints. Particles: render tick → `edit_document` → same notify path; frame pacing via the `request_animation_frame` chain.

Module tree:

```text
src/features/pages/canvas_showcase/
├── mod.rs         page entity: CanvasDemo enum, demo registry, header + TabBar layout
├── geometry.rs    pub(super) pure helpers: cubic sampling, handle layout, resize math
├── shape_edit.rs  ShapesDemo view + ShapeEditTool (select/move/resize)
├── bezier.rs      BezierDemo view + BezierEditTool (draggable control points)
├── node_graph.rs  NodeGraphDemo view + NodeGraphTool (drag, rubber band, edges)
├── particles.rs   ParticleFieldDemo view + tick model + animation loop
├── text.rs        TextDemo view (shaped-text seeds, PanTool)
├── image.rs       ImageDemo view + BGRA swatch generator (RenderImage)
└── *.test.rs      sibling test modules per repo convention
```

## Interface Contract

Leaf feature: nothing else consumes it. The entire public surface is:

```rust
// src/features/pages/canvas_showcase/mod.rs (module is private; one re-export)
pub struct CanvasShowcasePage { /* fields private */ }
impl CanvasShowcasePage {
    /// Page view; `AppRoot` constructs it once per window. Nothing in the
    /// page needs `Window`, so the constructor takes only the context
    /// (context-free-page precedent: `AboutPage::new()`, state.rs:55).
    pub fn new(cx: &mut Context<Self>) -> Self;
}
// impl Render for CanvasShowcasePage: renders header + TabBar + active demo view.

// Re-exported from src/features/pages/mod.rs:
//   mod canvas_showcase;
//   pub use canvas_showcase::CanvasShowcasePage;
```

No events, no commands, no config fields, no capability entries. Everything under `canvas_showcase/` besides `CanvasShowcasePage` is private to the crate (`pub(super)` at most, for the `geometry.rs` helpers shared between demo modules).

## Module Layout

| File | Purpose |
|---|---|
| `src/features/pages/canvas_showcase/mod.rs` | Page seam: module header with removal recipe, `CanvasDemo` enum (order = tab order), demo registry construction, page render (title, TabBar, active demo), shared demo-shell layout helper |
| `src/features/pages/canvas_showcase/geometry.rs` | Pure doc-space math: `sample_cubic_bezier`, `square_at`, `handle_positions`, `resize_bounds`, `scale_points`, `rect_from_corners` |
| `src/features/pages/canvas_showcase/shape_edit.rs` | `ShapesDemo` entity + `ShapeEditTool`: selection, multi-move, 8-handle resize with min-size clamp |
| `src/features/pages/canvas_showcase/bezier.rs` | `BezierDemo` entity + `BezierEditTool`: 4 draggable points, tangent guides, re-sampled curve |
| `src/features/pages/canvas_showcase/node_graph.rs` | `NodeGraphDemo` entity + `NodeGraphTool`: node drag, Bézier edges, rubber-band selection, shift-additive selection |
| `src/features/pages/canvas_showcase/particles.rs` | `ParticleFieldDemo` entity: deterministic particle model, fixed-step tick, `request_animation_frame` loop, play/pause |
| `src/features/pages/canvas_showcase/text.rs` | `TextDemo` entity: six shaped-text seeds over one backing rect, PanTool |
| `src/features/pages/canvas_showcase/image.rs` | `ImageDemo` entity: BGRA checker swatch → `RenderImage` → two `CanvasShape::Image` placements |
| `src/features/pages/canvas_showcase/*.test.rs` | Sibling `#[cfg(test)] #[path = ...]` test modules for `geometry`, `shape_edit`, `bezier`, `node_graph`, `particles`, `image`, `mod` |

## Seam Touches

Every edit outside the new directory (survey §1 rows, adapted):

| # | File | Change | Why minimal |
|---|---|---|---|
| 1 | `src/features/pages/mod.rs` | `mod canvas_showcase;` + `pub use canvas_showcase::CanvasShowcasePage;` | Rust requires the decl; the re-export feeds `crate::views` (`src/lib.rs:45`) |
| 2 | `src/shell/sidebar.rs` | `CanvasShowcase` variant in `Page` (after `Gallery`); arms in `title()` (`"Canvas Showcase"`), `title_key()` (`nav_canvas_showcase`), `host()` (`canvas-showcase`), `icon()` (`IconName::Palette`); `all()` entry appended last | Exhaustive matches force the arms; the digit-key binding filter is `if i < 9` (`state.rs:148`), so 0-based index 9 onward (the 10th page, Gallery today) binds no digit key and appending after it rebinds nothing |
| 3 | `src/shell/route.rs` | `Page::CanvasShowcase.host(),` appended to `VALID_HOSTS` (`route.rs:10-21`) | Deep-link validation derives from the same host string |
| 4 | `src/shell/root/app_root/state.rs` | `canvas_showcase_page: Entity<CanvasShowcasePage>` field; construction `cx.new(|cx| CanvasShowcasePage::new(cx))`; import; `unchecked_active_page_view` arm | Same rows as every page (e.g. `state.rs:31, 56, 222`); context-only constructor matches the `AboutPage` row at `state.rs:55` |
| 5 | `tests/e2e_navigation.rs` | Count literal `10` → `11` (or current + 1 if a sibling plan landed first); add `Page::CanvasShowcase` to the `expected` array; update the count comment | Count gate stays honest; +1 keeps it mechanical regardless of sibling landing order (same policy plan 06 states at `06-infinite-canvas.md:160`) |
| 6 | `locales/en.yml` + `locales/zh-CN.yml` | `nav_canvas_showcase` in the `nav_*` block + the `showcase_*` block from the i18n section, identical key sets, same order in both files | Key lockstep enforced by `tests/qa_localization.rs:188-198` |
| 7 | `Cargo.toml` | Phase 6 only: `image = { version = "0.25", default-features = false, features = ["png"] }` in the common `[dependencies]` table (identical spec to `Cargo.toml:266, 278`) | The image demo needs `image::Frame` on every platform; the target tables already resolve this exact package+features, so the lock should not change. If `cargo` rewrites `Cargo.lock`, stage it (survey §9) |

Deliberate non-touches: no `CommandId` entry (no command-palette precedent for showcase pages; `src/services/commands.rs` lists none for Gallery/Query pages), no capability-registry entry (no platform variance), no `src/lib.rs` edit (automatic through `views`), no engine edits (everything runs through plan 01's published contract).

### Removal Recipe

`mod.rs` opens with (mirrors `src/gallery/mod.rs:1-7`):

```rust
//! Removable canvas showcase page. `CanvasShowcasePage` is the only public
//! surface; everything else stays private to the crate. Removal = delete this
//! directory and revert its seam touches: the `mod`/`pub use` pair in
//! features/pages/mod.rs, the Page::CanvasShowcase arms and all() entry in
//! shell/sidebar.rs, the VALID_HOSTS entry in shell/route.rs, the AppRoot
//! wiring in shell/root/app_root/state.rs, the page count in
//! tests/e2e_navigation.rs, the nav_canvas_showcase/showcase_* keys in both
//! locale files, and the common-table image line in Cargo.toml.
```

Ordered removal steps:

1. Revert `src/features/pages/mod.rs` (drop the `mod`/`pub use` pair).
2. Revert `src/shell/sidebar.rs` (variant, four match arms, `all()` entry).
3. Revert `src/shell/route.rs` (`VALID_HOSTS` entry).
4. Revert `src/shell/root/app_root/state.rs` (field, construction, import, view arm).
5. Revert `tests/e2e_navigation.rs` (count literal, `expected` entry, comment).
6. Remove `nav_canvas_showcase` and every `showcase_*` key from `locales/en.yml` and `locales/zh-CN.yml`.
7. Remove the common-table `image` line from `Cargo.toml` and stage `Cargo.lock` if `cargo` rewrote it.
8. `rm -r src/features/pages/canvas_showcase/`.
9. Run the gates in Verification; `grep -rn "canvas_showcase\|CanvasShowcase" src/ tests/ locales/` must return nothing.

## Implementation Phases

## Phase 1: Page seam scaffold

### Step 1.1: Locale keys

- Files: `locales/en.yml`, `locales/zh-CN.yml`.
- Build: `nav_canvas_showcase` in the `nav_*` block (after `nav_gallery`, `en.yml:62`); append the `showcase_*` block (every key from the i18n section below) after the `nav_*` block in both files, same order. Values exactly as in the i18n section; double-quoted scalars, one nesting level max (grammar `tests/qa_localization.rs:20-71`).
- Tests: none yet (Phase 7 adds the key-existence guard).
- Done-check: `cargo test -p gpui-starter --test qa_localization` passes.

### Step 1.2: Sidebar, route, e2e seams

- Files: `src/shell/sidebar.rs`, `src/shell/route.rs`, `tests/e2e_navigation.rs`.
- Build: `Page::CanvasShowcase` variant after `Gallery` (`sidebar.rs:15`); `title()` → `"Canvas Showcase"`; `title_key()` → `"nav_canvas_showcase"`; `host()` → `"canvas-showcase"`; `icon()` → `IconName::Palette` (verified in the default 106; `pen-tool.svg` exists in the full catalog but its bytes are not registered natively — Research Digest icons bullet); `all()` entry appended after `Page::Gallery` (`sidebar.rs:103`). `route.rs`: `Page::CanvasShowcase.host(),` appended to `VALID_HOSTS` (`route.rs:20`). `e2e_navigation.rs`: count literal 10 → 11, add `Page::CanvasShowcase` to `expected`, extend `test_route_parsing` cases with `("gpui-starter://canvas-showcase", AppRoute::Page(Page::CanvasShowcase))`, fix the count comment.
- Tests: the two edited tests cover this step.
- Done-check: `cargo test -p gpui-starter --test e2e_navigation` passes.

### Step 1.3: Stub page + AppRoot wiring

- Files: `src/features/pages/canvas_showcase/mod.rs` (new), `src/features/pages/mod.rs`, `src/shell/root/app_root/state.rs`.
- Build: `mod.rs` with the removal-recipe header (Seam Touches) and a stub `CanvasShowcasePage { }` with `pub fn new(_cx: &mut Context<Self>) -> Self { Self { } }` (the underscore prefix keeps clippy's `unused_variables` quiet until Phase 2 uses the context) and a `Render` impl returning `v_flex().size_full().p_6().child(crate::i18n::localize("nav_canvas_showcase"))` (same placeholder shape plan 06 uses for its page scaffold). Imports are the repo two-line pair; `v_flex` is NOT in the kit root glob, it comes from the component path (Research Digest, import chains):
  ```rust
  use gpui_kit::component::v_flex;
  use gpui_kit::{prelude::*, *};
  ```
  `pages/mod.rs`: `mod canvas_showcase;` + `pub use canvas_showcase::CanvasShowcasePage;`. `state.rs`: field `canvas_showcase_page: Entity<CanvasShowcasePage>` after `gallery_page`, construction `cx.new(|cx| CanvasShowcasePage::new(cx))` after the `gallery_page` line, import in the `crate::views` list, arm `Page::CanvasShowcase => self.canvas_showcase_page.clone().into()` in `unchecked_active_page_view` (`state.rs:222`).
- Tests: none beyond the existing e2e (the route case from 1.2 plus `AppRoot` construction covers the wiring).
- Done-check: `cargo check -p gpui-starter` passes (proving the import pair compiles); `cargo run` shows the stub page and sidebar entry (manual, optional at this step).

## Phase 2: Demo scaffold + shape editing

### Step 2.1: Pure geometry helpers

- Files: `src/features/pages/canvas_showcase/geometry.rs` (new), `geometry.test.rs` (wired `#[cfg(test)] #[path = "geometry.test.rs"] mod geometry_test;`).
- Build (all `pub(super)`, pure, doc-space `Pixels` math; `Point<Pixels> * f32` works, `Pixels / f32` and `Pixels + f32` do not, so mixed arithmetic goes through `px(a.as_f32() ... )` and offsets through `point(px(a), px(b))`):
  - `const HANDLE_SIDE: Pixels = px(8.);` and `const MIN_SHAPE_SIZE: Pixels = px(8.);`
  - `square_at(center: Point<Pixels>, side: Pixels) -> Bounds<Pixels>` with origin `center - point(px(side.as_f32() / 2.), px(side.as_f32() / 2.))` (manual; no `from_center` in the crate).
  - `handle_positions(b: Bounds<Pixels>) -> [Point<Pixels>; 8]`: TL, TM, TR, MR, BR, BM, BL, ML (corners then edge midpoints, exactly that index order).
  - `resize_bounds(start: Bounds<Pixels>, handle_ix: usize, pointer: Point<Pixels>) -> Bounds<Pixels>`: set the edges the handle controls (0 TL→left+top, 1 TM→top, 2 TR→right+top, 3 MR→right, 4 BR→right+bottom, 5 BM→bottom, 6 BL→left+bottom, 7 ML→left) to the pointer, then clamp: if width < `MIN_SHAPE_SIZE`, restore the moved left/right edge so width equals the minimum (the anchored edge wins; no flipping); same for height with the moved top/bottom edge.
  - `scale_points(points: &[Point<Pixels>], old: Bounds<Pixels>, new: Bounds<Pixels>) -> Vec<Point<Pixels>>`: per-axis affine `new.origin + p'` where each axis maps through `px(new.origin.axis.as_f32() + (p.axis - old.origin.axis).as_f32() * (new_size / old_size))` in f32; if either old dimension is zero, translate by `new.origin - old.origin` instead.
  - `rect_from_corners(a: Point<Pixels>, b: Point<Pixels>) -> Bounds<Pixels>`: min per axis → origin, max-min → size (normalizes reversed drag corners; `Bounds::from_corners` (`GPUI/geometry.rs:821`) assumes ordered inputs).
- Tests: `handle_positions` exact values for bounds `(10,20)-(110,70)`; `resize_bounds` for TL, BR, and MR from a known rect to a known pointer; a shrink past the minimum clamping to 8x8; a cross-over drag that anchors instead of flipping; `scale_points` maps corners onto corners and a midpoint proportionally; `square_at` centers on the input; `rect_from_corners` normalizes reversed corners.
- Done-check: `cargo test -p gpui-starter --lib canvas_showcase::geometry` passes.

### Step 2.2: Demo shell + ShapesDemo view

- Files: `mod.rs` (extend), `shape_edit.rs` (new), `shape_edit.test.rs`.
- Build in `mod.rs`. Import block (repo pair; `Tab`/`TabBar` nest inside the component group exactly as `src/gallery/page.rs:4-9` nests `sidebar::{...}`):
  ```rust
  use gpui_kit::component::{
      ActiveTheme as _, IconName, tab::{Tab, TabBar}, v_flex,
  };
  use gpui_kit::{prelude::*, *};

  use crate::accessibility::A11yExt as _;
  ```
  Every demo module (`shape_edit.rs`, `bezier.rs`, `node_graph.rs`, `particles.rs`, `text.rs`, `image.rs`) uses this same convention, trimmed to what it names:
  ```rust
  use gpui_kit::component::{ActiveTheme as _, button::Button, h_flex, v_flex};
  use gpui_kit::{prelude::*, *};

  use crate::ui::canvas::*;
  ```
  plus `use crate::accessibility::A11yExt as _;` only in modules that render an `a11y_live` readout (`shape_edit.rs`, `node_graph.rs`). `crate::ui::canvas::*` is safe: plan 01's `mod.rs` re-exports every contract item (`01-canvas-foundation.md:95`), and none of those names collide with the kit glob.
  - `#[derive(Clone, Copy, Debug, PartialEq, Eq)] enum CanvasDemo { Shapes, Bezier, Nodes, Particles, Text, Image }` with `const ALL: [Self; 6]` in tab order; `tab_key(self) -> &'static str` → `showcase_tab_*`; `title_key`/`desc_key` → `showcase_<demo>_title`/`showcase_<demo>_desc`; `icon(self) -> IconName` → `MousePointer2`, `Spline`, `Workflow`, `Sparkles`, `Type`, `Image` (all svg-verified).
  - `render_demo_shell(id: &str, title: String, desc: String, toolbar: Vec<AnyElement>, canvas: impl IntoElement, cx: &App) -> Div`: `v_flex().size_full()` → header `v_flex().p_4().gap_1().border_b_1().border_color(cx.theme().border)` with title div (`.id(SharedString::from(format!("{id}-title")))`, `.a11y(Role::Heading, title.clone())`, `.aria_level(2)`, `.text_lg().font_weight(FontWeight::SEMIBOLD)`), desc div (`.text_color(cx.theme().muted_foreground)`), toolbar `h_flex().gap_2().mt_2()` with the given children; content `div().flex_1().min_h_0().p_4()` wrapping `div().size_full().border_1().rounded_lg().border_color(cx.theme().border).overflow_hidden().child(canvas)`.
  - `CanvasShowcasePage::new(cx)`: build `demos` (Shapes entry this phase; later phases append), `active_ix: 0`. `render`: `v_flex().size_full()` → page header `v_flex().p_4().pb_0().gap_1()` with h1 (`.a11y(Role::Heading, localized title)`, `.aria_level(1)`, `.text_xl().font_weight(FontWeight::BOLD)`, text `localize("nav_canvas_showcase")`) and desc (`localize("showcase_page_desc")`, muted) → `TabBar::new("showcase-tabs").selected_index(self.active_ix).on_click(cx.listener(|this, ix: &usize, _, cx| { if *ix < this.demos.len() { this.active_ix = *ix; cx.notify(); } })).children(self.demos.iter().map(|(kind, _)| Tab::new().label(crate::i18n::localize(kind.tab_key()))))` → active demo `v_flex().flex_1().min_h_0().child(self.demos[self.active_ix].1.clone())`. Remove the Phase 1 stub render; `new` now uses `cx`, so drop the underscore.
- Build in `shape_edit.rs`: `ShapesDemo { state: Entity<CanvasState>, _subscription: Subscription }`; `new(cx: &mut Context<Self>) -> Self`: `let state = cx.new(|_| CanvasState::new());` then `state.update(cx, |s, cx| { let tool = ShapeEditTool::seed(s, cx); s.set_tool(Box::new(tool), cx); });` then observe. `ShapeEditTool::seed(state: &mut CanvasState, cx: &mut Context<CanvasState>) -> Self`: `state.edit_document(cx, |d| d.clear());` two layers via `state.add_layer(cx)` (`content_layer`, then `handles_layer`); `state.set_grid(CanvasGrid { visible: false, spacing: px(16.) }, cx);` seed into content (colors from `cx.theme()`; `Context<CanvasState>` derefs to `App`. These are document data colors, the design-guides raster/data exemption, still theme-derived):
  - rect `(80,80)-(260,180)`: `ShapeStyle::both(theme.primary.opacity(0.30), StrokeStyle { width: px(1.), color: theme.foreground.opacity(0.7), dash: None })`
  - ellipse `(320,100)-(480,220)`: `ShapeStyle::filled(theme.accent.opacity(0.35))`
  - rect `(140,280)-(300,380)`: `ShapeStyle::filled(theme.secondary.opacity(0.35))`
  - path `[(100,430),(160,390),(220,450),(280,410)]`: `CanvasShape::path(...)` with `StrokeStyle { width: px(2.), color: theme.foreground, dash: None }`
  Toolbar: ghost small `Button::new("shapes-reset").icon(IconName::RotateCw).label(localize("showcase_reset")).on_click(...)` that rebuilds via a fresh `ShapeEditTool::seed` + `set_tool`; muted readout `localize_with_args("showcase_selected_count", &[("count", n.to_string())])` with `n = state.read(cx).selected_shape_ids().len()`, wrapped in a div with `.a11y_live(accesskit::Live::Polite)` (notifications.rs:50-53 pattern). Render: `render_demo_shell("shapes", ...)` with `Canvas::new("shapes-canvas", self.state.clone())`.
- Tests (`shape_edit.test.rs`, `#[gpui_kit::test]`): `let demo = cx.add_window(|_, cx| cx.new(|cx| ShapesDemo::new(cx)));` then through the demo's state entity assert: exactly 4 content shapes, 2 layers, grid invisible, active tool id `"shape-edit"` (the tool's `id()`).
- Done-check: `cargo test -p gpui-starter --lib canvas_showcase` passes; `cargo run` shows the page with one tab and the seeded shapes.

### Step 2.3: ShapeEditTool behavior

- Files: `shape_edit.rs` (extend), `shape_edit.test.rs` (extend).
- Build: `ShapeEditTool` fields: `content_layer: LayerId`, `handles_layer: LayerId`, `handle_ids: Option<[ShapeId; 8]>`, `move_drag: Option<MoveDrag>`, `resize_drag: Option<ResizeDrag>` (private structs: `MoveDrag { snapshot: Vec<(ShapeId, CanvasShape)>, origin: Point<Pixels> }`, `ResizeDrag { handle_ix: usize, start_bounds: Bounds<Pixels>, shape_id: ShapeId }`). `id() -> "shape-edit"`, `cursor() -> CursorStyle::Arrow`.
  - `sync_handles(&mut self, state: &mut CanvasState, cx)`: exactly one selected shape → ensure the 8 handle shapes exist in `handles_layer` (add once, store ids) and `update_shape` each to `CanvasShape::rect(square_at(pos, HANDLE_SIDE), ShapeStyle::filled(cx.theme().foreground))` with positions from `handle_positions(shape.doc_bounds())`; any other selection count → remove all 8 via `remove_shape` and set `handle_ids = None`.
  - `pointer_down(ev, ctx, state, cx)`: if `ctx.hit` is `Some(id)` and `self.handle_ids` contains it → start `resize_drag` (shape's current `doc_bounds()` via `state.document()`); else if `ctx.hit` is `Some(id)` (a content shape) → `state.select_shape(id, ev.modifiers.shift, cx)` unless `id` is already in `state.selected_shape_ids()`, snapshot every selected shape (`state.document().shape(sid).cloned()`), start `move_drag` with `origin = ev.doc_position`; no hit and not shift → `state.clear_selection(cx)`. Finish with `sync_handles`.
  - `pointer_move(ev, ctx, state, cx)`: resize → `state.edit_document(cx, |d| d.update_shape(id, |s| match s { CanvasShape::Rect { bounds, .. } | CanvasShape::Ellipse { bounds, .. } => *bounds = resize_bounds(start, ix, ev.doc_position), CanvasShape::Path { points, .. } => *points = scale_points(points, start, resize_bounds(start, ix, ev.doc_position)), _ => {} }))` then `sync_handles`; move → `delta = ev.doc_position - origin`, one `edit_document` restoring each snapshot clone then `CanvasShape::translate(delta)` via `update_shape`, then `sync_handles`; neither (hover) → `sync_handles` only when the handle set disagrees with the current selection (keeps handles correct after any selection change).
  - `pointer_up`: clear both drag fields, `sync_handles`. `scroll`: default `false` (engine pan/zoom defaults apply).
- Tests (drive via `entity.update(cx, |s, cx| s.dispatch_tool_event(cx, |tool, s, cx| tool.pointer_down(&ev, &ctx, s, cx)))` with hand-built `CanvasPointerEvent { screen_position, doc_position, button: MouseButton::Left, click_count: 1, modifiers: Modifiers::default() }` and `ToolContext { viewport: Bounds { origin: Point::default(), size: size(px(600.), px(500.)) }, transform: CanvasTransform::IDENTITY, hit: <precomputed via state.hit_test(doc_pos, px(4.))> }`):
  - click inside the rect selects it and creates exactly 8 handle shapes in the handles layer at the expected positions;
  - shift-click the ellipse → 2 selected, `handle_ids == None` (multi-selection shows no handles);
  - move-drag the rect by `(30, 10)` translates its `doc_bounds` origin by exactly that delta;
  - resize-drag handle 0 (TL) up-left shrinks the rect per `resize_bounds`; dragging TL past the bottom-right corner clamps to `MIN_SHAPE_SIZE` without flipping;
  - click empty space clears selection and removes the handle shapes;
  - a subscription counter records exactly one `DocumentChanged`/`SelectionChanged` per logical change (plan 01 Step 3.1 pattern).
- Done-check: `cargo test -p gpui-starter --lib canvas_showcase::shape_edit` passes.

## Phase 3: Bézier path editor

### Step 3.1: Cubic sampling helper

- Files: `geometry.rs` (extend), `geometry.test.rs` (extend).
- Build: `pub(super) fn sample_cubic_bezier(p0: Point<Pixels>, c1: Point<Pixels>, c2: Point<Pixels>, p1: Point<Pixels>, segments: usize) -> Vec<Point<Pixels>>`: `segments + 1` points, `t = i as f32 / segments as f32`, standard cubic `B(t) = (1-t)^3 p0 + 3(1-t)^2 t c1 + 3(1-t) t^2 c2 + t^3 p1` per axis in f32.
- Tests: endpoints exact at t=0/1 for a fixed input; an interior point matches the formula recomputed by hand; point count is `segments + 1`; `segments = 1` yields the two endpoints.
- Done-check: `cargo test -p gpui-starter --lib canvas_showcase::geometry` passes.

### Step 3.2: BezierDemo + BezierEditTool

- Files: `bezier.rs` (new), `bezier.test.rs`, `mod.rs` (append `(CanvasDemo::Bezier, view)` to the registry).
- Build: `BezierDemo { state, _subscription }` with `new(cx: &mut Context<Self>)` constructed exactly like `ShapesDemo` (fresh state, tool seeds, observe; import block per Step 2.2 without `A11yExt`). `BezierEditTool` fields: `guides_layer`, `curve_layer`, `handles_layer: LayerId`; `points: [Point<Pixels>; 4]` (P0, C1, C2, P1) seeded `(120,260), (200,80), (360,440), (460,260)`; `handle_ids: [ShapeId; 4]`, `curve_id: ShapeId`, `guide_ids: [ShapeId; 2]`; `dragging: Option<usize>`; `id() -> "bezier-edit"`, `cursor() -> CursorStyle::Crosshair`.
  - `seed(state, cx)`: clear, grid off, three layers (guides bottom, curve, handles top); curve = `CanvasShape::path(sample_cubic_bezier(p0, c1, c2, p1, 24), StrokeStyle { width: px(2.), color: theme.foreground, dash: None })`; guides = `CanvasShape::path(vec![p0, c1])` and `vec![c2, p1]`, stroke `theme.muted_foreground.opacity(0.6)`, width `px(1.)`; handles = anchors P0/P1 as `CanvasShape::ellipse(square_at(p, px(12.)), ShapeStyle::filled(theme.primary))`, controls C1/C2 as `ellipse(square_at(p, px(10.)), ShapeStyle::both(theme.background, StrokeStyle { width: px(1.), color: theme.muted_foreground, dash: None }))`.
  - `pointer_down`: `ctx.hit` in `handle_ids` → `dragging = Some(ix)`; else no-op (curve and guides are not draggable).
  - `pointer_move` while dragging: `points[ix] = ev.doc_position`; one `edit_document` updating the handle shape's bounds (`square_at(new_pos, side)`), the affected guide's points (`[p0, c1]` or `[c2, p1]`), and the curve's points (`sample_cubic_bezier` re-run at 24 segments). No `select_shape` use; engine selection is unused in this demo.
  - `pointer_up`: `dragging = None`. Toolbar: reset (fresh `seed` + `set_tool`), ghost small `RotateCcw`. View render: `render_demo_shell("bezier", ...)` + `Canvas::new("bezier-canvas", state)`.
- Tests: seed produces 1 curve shape with 25 points, 2 two-point guides, and 4 handle ellipses across 3 layers (bottom-to-top order guides, curve, handles); drag C1 (index 1) to a new position updates the curve's sampled points to exactly `sample_cubic_bezier` recomputed with the new control point, moves guide 0's second point, and leaves guide 1 untouched; one `DocumentChanged` per move; pointer_down on the curve (not a handle) mutates nothing.
- Done-check: `cargo test -p gpui-starter --lib canvas_showcase::bezier` passes.

## Phase 4: Node graph

### Step 4.1: Node graph seeding and edge recompute

- Files: `node_graph.rs` (new), `node_graph.test.rs`.
- Build: `const NODE_W: Pixels = px(120.);` `const NODE_H: Pixels = px(56.);` `NodeGraphTool` fields: `edges_layer`, `nodes_layer: LayerId`; `nodes: Vec<NodeCell>` with `NodeCell { title: &'static str, bounds: Bounds<Pixels>, rect_id: ShapeId, label_id: ShapeId }` seeded `Input (100,120)`, `Filter (320,100)`, `Transform (330,260)`, `Output (560,170)`; `edges: Vec<EdgeCell>` with `EdgeCell { from: usize, to: usize, path_id: ShapeId }` seeded `(0,1), (0,2), (1,3), (2,3)`; `node_drag: Option<{ node_ix: usize, origin: Point<Pixels> }>`; `band: Option<{ start: Point<Pixels>, current: Point<Pixels> }>`; `id() -> "node-graph"`, `cursor() -> CursorStyle::Arrow`.
  - `fn edge_points(from: &NodeCell, to: &NodeCell) -> Vec<Point<Pixels>>`: `s = from.bounds` right-center, `t = to.bounds` left-center, `off = ((t.x - s.x).as_f32() / 2.).clamp(40., 120.)`, `sample_cubic_bezier(s, point(px(s.x.as_f32() + off), s.y), point(px(t.x.as_f32() - off), t.y), t, 20)` (the `point(px(..), ..)` wrappers exist because `Pixels + f32` does not compile; `point()` at `GPUI/geometry.rs:111`, `Pixels` impls at `:2670-2692`, `:2716-2723`).
  - Node shapes: rect `ShapeStyle::both(theme.group_box, StrokeStyle { width: px(1.), color: theme.border, dash: None })`; label `CanvasShape::text(bounds.origin + point(px(8.), px(8.)), title, px(14.), theme.foreground)` (`Point<Pixels> + Point<Pixels>` is the derived `Add`, `GPUI/geometry.rs:64-79`; a bare tuple does not implement it). Edge shapes: `CanvasShape::path(edge_points(...), StrokeStyle { width: px(1.5), color: theme.muted_foreground, dash: None })`.
  - `seed(state, cx)`: clear, grid off, two layers (edges bottom, nodes top); per node add rect then label to `nodes_layer`; per edge add path to `edges_layer`.
- Tests: seed yields 4 rect + 4 text shapes in the nodes layer and 4 path shapes (21 points each) in the edges layer; `edge_points` first/last points equal the source right-center and target left-center ports.
- Done-check: `cargo test -p gpui-starter --lib canvas_showcase::node_graph` passes.

### Step 4.2: NodeGraphTool interaction + view

- Files: `node_graph.rs` (extend), `node_graph.test.rs` (extend), `mod.rs` (registry entry).
- Build:
  - `pointer_down`: resolve `ctx.hit` against `rect_id`/`label_id` (either maps to the node) → node ix: if shift → `state.select_shape(rect_id, true, cx)`; else if `rect_id` not already in `state.selected_shape_ids()` → `state.select_shape(rect_id, false, cx)`; start `node_drag { node_ix, origin: ev.doc_position }`. No hit → `band = Some { start: ev.doc_position, current: ev.doc_position }`, `state.set_tool_overlay(Some(CanvasShape::rect(rect_from_corners(start, start), ShapeStyle::filled(cx.theme().primary.opacity(0.08)))), cx)`, and if not shift → `state.clear_selection(cx)`.
  - `pointer_move`: node drag → `delta = ev.doc_position - origin`; one `edit_document` translating the node's rect bounds and label position by delta and re-sampling every edge whose `from`/`to` is that node (`edge_points` + `update_shape` on the path's points). Band → update `current`, refresh the overlay rect to `rect_from_corners(start, current)` with the same style. Selected nodes get the engine's 1-px primary outline automatically (plan 01 paint order: selection outline last).
  - `pointer_up`: band → `let band_rect = rect_from_corners(start, current);` for every node whose `bounds.intersects(&band_rect)` (`GPUI/geometry.rs:965`) call `state.select_shape(rect_id, ev.modifiers.shift, cx)`; `state.set_tool_overlay(None, cx)`; clear both fields. Toolbar: reset (fresh seed), selected-count readout (`state.selected_shape_ids().len()`; only rect ids are ever selected, so the count equals selected nodes) with `a11y_live`. View: `NodeGraphDemo { state, _subscription }` with `new(cx: &mut Context<Self>)`; `render_demo_shell("nodes", ...)` + `Canvas::new("nodes-canvas", state)`.
- Tests: dragging `Input` by `(40, 20)` moves its rect and label by that delta and re-samples both its outgoing edges (first point of edge 0 equals the new right-center port); dragging one node leaves other nodes' shapes untouched; rubber band from `(90,90)` to `(450,200)` selects exactly `Input` + `Filter` (their rect ids in `selected_shape_ids()`, seed order); shift+band adds to an existing selection; band on empty space without shift clears selection; `tool_overlay()` is `None` after pointer up; one `DocumentChanged` per drag move.
- Done-check: `cargo test -p gpui-starter --lib canvas_showcase::node_graph` passes.

## Phase 5: Particle field

### Step 5.1: Particle model + tick

- Files: `particles.rs` (new), `particles.test.rs`.
- Build: `const FIELD_W: f32 = 800.;` `const FIELD_H: f32 = 500.;` `const PARTICLE_COUNT: usize = 48;` `struct Particle { pos: Point<Pixels>, vel: Point<Pixels>, diameter: Pixels }`. `fn seed_particles() -> Vec<Particle>`: xorshift32 (`state ^= state << 13; state ^= state >> 17; state ^= state << 5;` on a `u32` starting at `0x9E37_79B9`), per particle `x = 20 + (next() % 760)`, `y = 20 + (next() % 460)`, speed `0.5 + (next() % 30) as f32 * 0.05` per axis with sign from `next() & 1`, `diameter = px(6. + (next() % 9) as f32)`. `fn tick(particles: &mut [Particle])`: `pos += vel`; per axis with `r = diameter.as_f32() / 2.`: if `pos < r` → `pos = r`, `vel = |vel|`; if `pos > field - r` → clamp, `vel = -|vel|` (f32 math throughout; positions rebuilt with `px()` where stored back into `Point<Pixels>`).
- Tests: `seed_particles` is deterministic (two calls produce equal vectors) and every particle starts strictly inside the field; `tick` moves each particle by exactly `vel`; bounce at both walls reflects the sign and clamps position; a particle aimed at a wall never escapes across 600 sequential ticks.
- Done-check: `cargo test -p gpui-starter --lib canvas_showcase::particles` passes.

### Step 5.2: ParticleFieldDemo view + animation loop

- Files: `particles.rs` (extend), `mod.rs` (registry entry).
- Build: `ParticleFieldDemo { state: Entity<CanvasState>, particles: Vec<Particle>, ids: Vec<ShapeId>, playing: bool, _subscription: Subscription }`. `new(cx: &mut Context<Self>)`: fresh `CanvasState`; `state.update(cx, |s, cx| { s.set_builtin_tool(BuiltinTool::Pan, cx); s.set_grid(CanvasGrid { visible: false, spacing: px(16.) }, cx); let layer = s.add_layer(cx); ... })` with one layer, then one `edit_document` adding an ellipse per particle, fill cycling `[theme.primary, theme.accent, theme.secondary]` by `i % 3`, each `.opacity(0.85)`; store the `ShapeId`s parallel to `particles`. `playing` starts `true`. `Render`:
  ```rust
  if self.playing && window.is_window_active() && !cx.reduce_motion() {
      tick(&mut self.particles);
      let positions: Vec<_> = self.particles.iter().map(|p| square_at(p.pos, p.diameter)).collect();
      let ids = self.ids.clone();
      self.state.update(cx, |s, cx| s.edit_document(cx, |d| {
          for (id, bounds) in ids.iter().zip(&positions) {
              d.update_shape(*id, |sh| {
                  if let CanvasShape::Ellipse { bounds: b, .. } = sh { *b = *bounds; }
              });
          }
      }));
      window.request_animation_frame();
  }
  ```
  (the `fps.rs` mechanism: request at top of the interaction, advance state in render, fixed step, no clock; `src/gallery/sections/charts/fps.rs:166-177, 36-39`; gates per `GPUI/window.rs:2670` doc and `GPUI/elements/img.rs:381-387`). Toolbar: play/pause toggle `Button::new("particles-toggle").ghost().small().icon(if self.playing { IconName::Pause } else { IconName::Play }).label(localize(if self.playing { "showcase_pause" } else { "showcase_play" }))`; reset re-runs `seed_particles()`, then one `edit_document` rewrites all bounds, leaving `playing` untouched. View render: `render_demo_shell("particles", ...)` + `Canvas::new("particles-canvas", state)`.
- Tests: `#[gpui_kit::test]` + `cx.add_window(|_, cx| cx.new(|cx| ParticleFieldDemo::new(cx)))` render smoke with `playing = false` draws without panic and leaves positions unchanged across two renders; the tick math itself is unit-covered in 5.1; the live loop is manual QA (`VisualTestContext` has no automatic frame pump; `window.simulate_next_frame` (`GPUI/window.rs:2680-2690`) is available to the implementer for an optional driven test, but was not verified end-to-end here).
- Done-check: `cargo test -p gpui-starter --lib canvas_showcase::particles` passes; manual QA confirms motion.

## Phase 6: Text and image demos

### Step 6.1: TextDemo

- Files: `text.rs` (new), `mod.rs` (registry entry).
- Build: `TextDemo { state, _subscription }`; `new(cx: &mut Context<Self>)`: fresh state, `set_builtin_tool(BuiltinTool::Pan, cx)`, grid off (clean typography surface), one layer, seeds (doc space): backing rect origin `(64,88)` size `(500,52)` `ShapeStyle::filled(theme.muted.opacity(0.5))`; texts, `"The quick brown fox jumps over"` at `(80,100)` `px(32.)` `theme.foreground`; `"Scale, pan, and zoom"` at `(80,170)` `px(20.)` `theme.muted_foreground`; `"Canvas text stays crisp at every zoom level"` at `(80,210)` `px(14.)` `theme.foreground`; `"0123456789 !@#$%"` at `(80,260)` `px(28.)` `theme.primary`; `"aligned baseline sample"` at `(80,310)` `px(16.)` `theme.muted_foreground`. Latin-only content on purpose: CJK glyph coverage depends on platform fonts (wasm embeds its own set, `src/app/assets.rs:48-56`) and no in-repo precedent paints CJK through `shape_line`; demo strings are data, not UI copy (i18n exemption). Toolbar: reset view (`showcase_reset_view`, `RotateCcw`) → `state.update(cx, |s, cx| s.reset_view(cx))`. Render: `render_demo_shell("text", ...)` + `Canvas::new("text-canvas", state)`.
- Tests: seed produces exactly 6 text shapes + 1 rect; every text shape carries its seeded font size and color (read back via `document()`).
- Done-check: `cargo test -p gpui-starter --lib canvas_showcase::text` passes.

### Step 6.2: BGRA swatch generator + Cargo.toml

- Files: `Cargo.toml`, `image.rs` (new), `image.test.rs`.
- Build: `Cargo.toml`: add `image = { version = "0.25", default-features = false, features = ["png"] }` to the common `[dependencies]` table (identical spec to `Cargo.toml:266, 278`; the lock's root block already lists `image`, so stage `Cargo.lock` only if cargo rewrites it). `image.rs`: `const SWATCH: u32 = 96;` `const CELL: u32 = 12;` `fn checker_swatch() -> image::RgbaImage`: `Vec<u8>` of `96*96*4` BGRA bytes, cell parity `(x / CELL + y / CELL) % 2` selecting `(B:60, G:120, R:230, A:255)` or `(B:170, G:90, R:40, A:255)`; wrap with `image::ImageBuffer::from_raw(SWATCH, SWATCH, bytes).expect("exact-size buffer")` (plain `from_raw` wraps verbatim, `image-0.25.10/src/images/buffer.rs:680`; `from_raw_bgra` is the swapping variant, `buffer.rs:1495-1502`; do not use). `fn render_image() -> Arc<RenderImage>`: `RenderImage::new(vec![image::Frame::new(checker_swatch())])` (`Frame::new` at `image-0.25.10/src/animation.rs:88`; `Vec` → `SmallVec` via `smallvec-1.15.2/src/lib.rs:2063`). Import block per Step 2.2 convention plus `use std::sync::Arc;` (`Arc` is also in the kit glob; the explicit `std` import is unnecessary if the glob supplies it, use whichever rustfmt keeps).
- Tests: buffer length `96*96*4`; bytes at pixel `(0,0)` are `B=60, G=120, R=230, A=255` (blue first: BGRA order); pixel `(12,0)` is the other color; `render_image().size(0)` is `96x96` (`RenderImage::size`, `GPUI/assets.rs:81-88`); two `render_image()` calls produce distinct ids (`PartialEq` is id-based, `GPUI/assets.rs:51-58`).
- Done-check: `cargo check -p gpui-starter` passes; `git status` shows `Cargo.toml` (and `Cargo.lock` only if cargo rewrote it).

### Step 6.3: ImageDemo view

- Files: `image.rs` (extend), `mod.rs` (registry entry).
- Build: `ImageDemo { state, image: Arc<RenderImage>, _subscription }`; `new(cx: &mut Context<Self>)`: `let image = render_image();` once, fresh state, `set_builtin_tool(Pan)`, grid off, one layer, seeds: `CanvasShape::image(Bounds` origin `(160,140)` size `(192,192)`, `image.clone())` (2x display), `CanvasShape::image(Bounds` origin `(400,140)` size `(96,96)`, `image.clone())` (1x), plus two stroke-only frame rects at the same bounds (`ShapeStyle::stroked(StrokeStyle { width: px(1.), color: theme.border, dash: None })`). Both placements share one `Arc`; the engine's atlas keys by `(ImageId, frame_index)`, so the second paint is a cache hit (plan 01 Research Digest, `GPUI/window.rs:4919-4960`). Toolbar: reset view. Render: `render_demo_shell("image", ...)` + `Canvas::new("image-canvas", state)`.
- Tests: seed produces 2 image shapes + 2 frame rects; both image shapes carry `RenderImage`s with the same id; document read-back via `document()`.
- Done-check: `cargo test -p gpui-starter --lib canvas_showcase::image` passes.

## Phase 7: Finalize

### Step 7.1: i18n key guard

- Files: `mod.rs` (extend), `mod.test.rs` (new sibling test module).
- Build: a test listing every key this feature localizes: `nav_canvas_showcase`, `showcase_page_desc`, all `tab_key`/`title_key`/`desc_key` outputs derived from `CanvasDemo::ALL` (so the list cannot drift), plus `showcase_reset`, `showcase_reset_view`, `showcase_play`, `showcase_pause`, `showcase_selected_count`. Assert `crate::i18n::localize(key) != key` for each (missing keys return the key itself, `src/services/i18n.rs:2`; the qa scanner cannot see these indirect calls, survey §5). Also assert `Page::CanvasShowcase.title_key() == "nav_canvas_showcase"` resolves.
- Tests: the guard itself.
- Done-check: `cargo test -p gpui-starter --lib canvas_showcase` passes.

### Step 7.2: Module header, gates, manual QA

- Files: `mod.rs` (final header), whole feature.
- Build: confirm the removal-recipe header matches the final seam list (Seam Touches); run the full gate set in Verification; walk the manual QA checklist.
- Tests: full suite.
- Done-check: all four gates pass; `grep -rn "use gpui::" src/features/pages/canvas_showcase/` returns nothing.

## Platform & Wasm Gating

- No `cfg` gates anywhere in the feature. Every demo runs identically on macOS, Windows, Linux, and wasm; no platform is hidden or degraded: engine paint APIs are platform-neutral (plan 01's Platform section), and the animation loop reaches the browser's rAF on wasm (`gpui-pre-web-0.3.8/src/window.rs:479-489`) and the platform frame scheduler natively (`GPUI/window.rs:2650-2658`).
- The only manifest change is Phase 6's `image` line in the common `[dependencies]` table. Nothing goes under `[target.'cfg(not(target_family = "wasm"))'.dependencies]`: `image 0.25` png-only is pure Rust and already locks at 0.25.10; wasm builds gain the dependency without any new native code. If `cargo` rewrites `Cargo.lock`, stage it together with the manifest change (`tests/qa_migration.rs:167-186`).
- No capability-registry entry: the feature has no platform variance, no init step, and no failure mode to report as supported/degraded/hidden (survey §4; registry entries exist for features with real platform state).
- Wasm constraints honored: no `std::time` clocks anywhere. The particle tick is a fixed per-frame step, so the module needs no clock at all (stricter than the `crate::platform::clock` rule, survey §6; same rationale as `fps.rs:36-39`). No threads, sockets, or file IO; repaints flow through `cx.notify()` and `request_animation_frame`, both wasm-safe.
- `just wasm-check` (`cargo +nightly check --target wasm32-unknown-unknown -p gpui-starter --lib`, `justfile:24-25`) must pass: the lib gains the new module and the common-table `image` dependency.

## i18n

Catalog facade `crate::i18n::localize` / `localize_with_args` (survey §5). Keys added to both `locales/en.yml` and `locales/zh-CN.yml` with identical key sets (lockstep test), `showcase_` prefix to avoid plan 06's claimed `canvas_*` keys. Strings painted into canvases (pangrams, node titles, swatch pixels) are demo content data, not interface copy; the same exemption plan 01 states and plan 06 applies to its note labels.

| key | en | zh-CN |
|---|---|---|
| `nav_canvas_showcase` | Canvas Showcase | 画布演示 |
| `showcase_page_desc` | Six isolated demos built on the shared canvas engine. | 基于共享画布引擎的六个独立演示。 |
| `showcase_tab_shapes` | Shapes | 图形 |
| `showcase_tab_bezier` | Bézier | 贝塞尔 |
| `showcase_tab_nodes` | Nodes | 节点 |
| `showcase_tab_particles` | Particles | 粒子 |
| `showcase_tab_text` | Text | 文本 |
| `showcase_tab_image` | Image | 图像 |
| `showcase_shapes_title` | Shape editing | 图形编辑 |
| `showcase_shapes_desc` | Select a shape, drag to move it, drag a handle to resize. | 选择图形后拖动移动，拖动手柄调整大小。 |
| `showcase_bezier_title` | Bézier path editor | 贝塞尔路径编辑器 |
| `showcase_bezier_desc` | Drag the anchors and control points to shape the curve. | 拖动锚点与控制点调整曲线形状。 |
| `showcase_nodes_title` | Node graph | 节点图 |
| `showcase_nodes_desc` | Drag nodes, shift-click to add to the selection, drag empty space to rubber-band select. | 拖动节点，Shift 点击追加选择，在空白处拖动进行框选。 |
| `showcase_particles_title` | Particle field | 粒子场 |
| `showcase_particles_desc` | An animated field bouncing inside the canvas bounds. | 在画布边界内弹跳的粒子动画。 |
| `showcase_text_title` | Text on canvas | 画布文本 |
| `showcase_text_desc` | Shaped text painted through the engine; it scales with zoom. | 由引擎排版的文本；随缩放一起变化。 |
| `showcase_image_title` | Image on canvas | 画布图像 |
| `showcase_image_desc` | A raster image built from a generated BGRA buffer, drawn at two scales. | 由生成的 BGRA 缓冲构建的位图，以两种尺寸绘制。 |
| `showcase_reset` | Reset | 重置 |
| `showcase_reset_view` | Reset view | 重置视图 |
| `showcase_play` | Play | 播放 |
| `showcase_pause` | Pause | 暂停 |
| `showcase_selected_count` | %{count} selected | 已选 %{count} 个 |

`Page::title()` stays the English "Canvas Showcase" for logs and non-localized surfaces; the sidebar label comes from `nav_canvas_showcase` via `localized_title()` (`sidebar.rs:34-49`). Sentence-case English, no terminal periods on short labels, full-width Chinese sentence punctuation, `%{count}` placeholder matching the `localize_with_args` contract (`src/services/i18n.rs:7-13`).

## Design Compliance

- Tokens: every chrome color reads from `cx.theme()`: `border` (canvas frame, header divider), `muted_foreground` (descriptions, edges, guides), `foreground` (titles, curve, handles), `primary` (engine selection outline, rubber-band fill at `opacity(0.08)`, anchor handles), `group_box` (node cards), `muted` (text backing rect), `background` (control-point fill). No raw hex, `rgb()`, or `hsla()` in the module (Color and themes). Document-content colors (shape fills, particle fills, swatch bytes) are data colors, the guides' raster/data exemption also stated by plan 01, but they remain theme-derived wherever a role exists.
- Fixed pixels: chrome uses rem-based helpers only (`p_4`, `gap_1`, `gap_2`, `mt_2`, `text_lg`, `text_xl`, `border_1`, `rounded_lg`, `size_full`, `flex_1`, `min_h_0`). Doc-space constants (`HANDLE_SIDE px(8.)`, `MIN_SHAPE_SIZE px(8.)`, node 120x56, field 800x500, seeded font sizes, stroke widths) are document geometry, the canvas equivalent of raster dimensions, the same class plan 01 uses for grid spacing.
- Density: compact toolbar tier with ghost `.small()` buttons (icon + label) and `gap_2` rows (Density tiers: "compact for toolbars and data-dense screens"); page and demo headers use medium density (`p_4`, `text_lg` titles, muted one-line descriptions).
- Hierarchy: page h1 → TabBar (kit-owned navigation with persistent selected state) → demo title (level 2) + one-line description → the canvas as the single focal surface. No cards-in-cards: the canvas frame is one hairline border and headers use a bottom hairline, not boxes (Hierarchy; Surfaces and elevation).
- Interaction states: selection is persistent (engine 1-px primary outline, distinct from hover per the Interaction states table); the rubber band is a live overlay during the drag; play/pause flips label + icon into a stable visible state; tool cursors come from the engine (`Arrow`, `Crosshair`, `OpenHand` for pan); no hover-only affordances, since every command is a labeled button or a direct manipulation.
- Motion: the particle loop follows the `request_animation_frame` doc contract: decorative animation gated on `App::reduce_motion` (`GPUI/window.rs:2670` doc; `GPUI/app.rs:1222`) and `window.is_window_active()` (`GPUI/elements/img.rs:381-387`), matching the repo's own `fps.rs` loop. Pausing or reduced motion leaves a legible static field; no animation is required to understand any demo (Motion).
- A11y: page title `Role::Heading` level 1, demo titles level 2, selected-count readouts `a11y_live` Polite (repo pattern `notifications.rs:48-53`); TabBar and Buttons carry the kit's focus, keyboard, and role contracts; icon usage is always icon + label (no icon-only buttons). Canvas contents remain invisible to AT, plan 01's documented engine limitation; the per-demo descriptions carry the semantics instead. Keyboard path: tab switching and every toolbar action are keyboard-operable through kit components.
- Layout: `v_flex` page with the canvas region as `flex_1().min_h_0()`; no scroll containers, because the canvas owns its own pan and the scroll-owner rules do not apply (Responsive desktop windows: "scroll only the region whose content actually overflows"). Shared content inset `p_4` across header and canvas padding keeps one leading edge (Alignment details).

## Verification

Gates (repo-mandatory set plus wasm, in order):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

Targeted during development: `cargo test -p gpui-starter --lib canvas_showcase` after each phase; `cargo test -p gpui-starter --test e2e_navigation --test qa_localization` after Phase 1.

Manual QA (run the app, both light and dark themes, on the Canvas Showcase page):

1. Sidebar shows "Canvas Showcase" (zh: 画布演示) with the pen-tool icon; deep link `gpui-starter://canvas-showcase` opens the page; `#/canvas-showcase` works in the web build.
2. Shapes: click selects (primary outline + 8 handles); shift-click adds; drag moves; each of the 8 handles resizes in its direction; shrinking past the minimum stops at 8x8 without flipping; clicking empty space deselects.
3. Bézier: dragging each of the 4 points reshapes the curve live; tangent guides track the control points; anchors and controls are visually distinct.
4. Nodes: dragging a node moves its label with it and re-curves every connected edge; dragging on empty space shows the band overlay and selects intersected nodes on release; shift-drag adds; selection outlines match.
5. Particles: Play animates at frame rate; Pause freezes; Reset reseeds; switching tabs away stops the animation and returning resumes if still playing; enabling the system reduce-motion preference freezes the field; particles bounce inside the 800x500 bounds without escaping.
6. Text: pan and zoom; text scales with zoom and stays clipped to the canvas frame; the backing rect stays behind the first line.
7. Image: both placements of the one swatch render at 1x and 2x; zooming in shows atlas-scaled smoothing; frames hug the images.
8. Every demo: middle-drag pans regardless of tool; plain wheel pans; ctrl/cmd+wheel zooms at the cursor; window resizing keeps painting inside the canvas frame.
9. Tab switching preserves each demo's state (selection, node positions, curve shape, playing state).
10. Both themes render every demo legibly (handles visible on both; edges and guides distinct from backgrounds).

Acceptance criteria: the four gates pass; the targeted `canvas_showcase` suite passes; every manual item is confirmed or waived with a note naming what was checked and how.

## Risks & Open Questions

Ranked risks:

1. **Plan 01 must land first.** Every phase consumes its Interface Contract; the contract is on disk and line-stable by its own Dependencies note, but nothing is implemented yet (repo carries only `docs/plans/`, `git status`). If 01's implementation drifts from the contract, these phases break at compile time, which is the intended failure mode rather than silent misbehavior.
2. **Render-phase mutation in the particle loop.** Tick + `entity.update` inside `render` deviates from the "render is declarative" guidance, but it is the repo's own shipped animation mechanism (`fps.rs:166-177` advances scene state in render under a top-of-render frame request; `GC/speech/waveform.rs:88-91` requests from render). The guides' rule targets *unconditional* notification loops; this loop is gated three ways (`playing`, `is_window_active`, `reduce_motion`) and stops entirely when the tab is hidden. A `cx.spawn` timer loop was rejected: timers drift from vsync and the wasm story for executor timers was not verified this session.
3. **Engine `StrokeStyle::dash` painting is unverified.** Plan 01's painter description (Step 6.2) never applies `dash_array` even though the field and its cache-key hashing exist. This plan uses only solid strokes for guides, edges, and frames so it cannot depend on dash behavior; if 01 implements dashes, dashed tangent guides are a polish upgrade, not a blocker.
4. **Per-frame document churn in the particle demo.** 48 `update_shape` calls per frame re-hash and re-tessellate 48 ellipses; plan 01's cache is content-keyed, so a moving particle is always a cache miss by design. Fine at 48; if QA shows jank, `PARTICLE_COUNT` is one constant to lower, with no structural change.
5. **Handle-hit priority depends on engine hit-test order.** Correct behavior needs topmost-layer-first resolution, which the contract fixes (`hit_test` iterates layers reversed, plan 01 Step 3.2). If 01 regresses that order, handles stop winning over shapes, caught immediately by the Step 2.3 tests.
6. **Icon variant reachability — resolved.** The draft picked `PenTool`/`MousePointer2`/`RotateCcw` after verifying only the svg filenames in the full catalog; the app registers the default 106, so those compile only via the assets enum and render blank on native. Revision uses verified defaults (`Palette`, `RotateCw`, `Play`, `Pause`). The enum is still generated at build time; `cargo check` in Step 1.2 is the arbiter for any future swap.
7. **Cargo.lock churn from the common-table `image` line.** The root lock block already lists `image` (verified by grep), so no change is expected, but cargo may rewrite the file; the lock-consistency test (`tests/qa_migration.rs:167-186`) fails the build if a manifest change ships without its lock, so stage whatever cargo produces.
8. **Sibling-plan drift in this doc's cross-references.** Plans 06 and 07 were revised by other sessions during this planning pass (plan 07's sidebar declaration moved from line 321 to 452 between two reads this session); the citations here were re-verified against the current files, but a further sibling revision could stale them again. The substantive claims (hosts, key prefixes) are unaffected.

Open questions (each with what was tried):

- **Does the engine's `Path` painter honor `StrokeStyle::dash`?** Tried: reading plan 01 Steps 5.1/6.2. The dash field is hashed into cache keys but the painter description only calls `PathBuilder::stroke(width)`. Unresolved against implementation; the plan avoids dash entirely.
- **Can `VisualTestContext` pump the animation loop automatically?** Tried: reading `GPUI/app/test_context.rs:894-940` (survey section 7), which lists mouse/text simulators only; `window.simulate_next_frame` (`GPUI/window.rs:2680-2690`) runs queued next-frame callbacks, but driving a continuous render-tick-render cycle through it was not verified end-to-end here. Animation is therefore manual QA plus deterministic unit ticks, with `simulate_next_frame` noted as an implementer option.
- **CJK text on canvas.** Omitted deliberately: glyph coverage depends on platform fonts (wasm embeds its own set, `src/app/assets.rs:48-56`), and no in-repo precedent paints CJK through `shape_line`. If wanted, add one seeded line after checking both platforms' font coverage.
- **Whether `cargo` rewrites `Cargo.lock` for the common-table `image` move.** Tried: grepping the lock's `gpui-starter` block (already lists `image`) and reading the qa_migration lock test (it only guards *removed* packages). Expected no-op; resolved mechanically at Phase 6 by staging whatever `cargo check` produces.
- **Review notes intentionally not applied:** none; every reviewer note was applied (import pair, arithmetic wrappers, citation corrections, digit-key boundary, constructor parameters, em-dash reduction).
