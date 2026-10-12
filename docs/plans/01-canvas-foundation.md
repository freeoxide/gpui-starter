# 01 Canvas Foundation Plan

## Goal

A shared canvas engine lives under `src/ui/canvas/` as a reusable widget library with no page of its own. Any view can mount `Canvas::new(id, state)` backed by an `Entity<CanvasState>` and get a custom GPUI element that paints layers of shapes, paths, text, and images through the real window paint APIs (`paint_quad`, `paint_path`, `paint_image`, shaped text). The engine provides document-to-screen transforms (pan offset, zoom scale), pointer routing to pluggable tools (pan, select, brush, plus a `CanvasTool` trait for new tools), hit testing, a background grid, and a shape-path cache so pure pans do not re-tessellate.

## Dependencies

None. This is the foundation plan. Dependents on disk consuming the Interface Contract below (read this session): `05-canvas-showcase.md`, `06-infinite-canvas.md`, `07-canvas-shaders.md`, `08-multiuser-canvas.md`, `10-floating-canvas.md`. Plan 08 cites line numbers into the contract's code block, so that block stays line-stable; revisions to this doc edit prose around it, not its layout. Plan 14 is named as a consumer by the feature spec but no file exists (`ls docs/plans/` shows 01–11 plus `_survey/`). Survey input: `docs/plans/_survey/seams.md` (sections 2, 6, 7, 8, 9) and `docs/plans/_survey/ecosystem.md` (sections a, d, e); every load-bearing claim was re-verified against the crate sources this session.

## Research Digest

Verified against `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` (short names `GPUI` = `gpui-pre-0.3.8/src`, `KIT` = `gpui-kit-0.7.1/src`, `GB` = `gpui-base-0.7.1/src`) and the repo.

Element lifecycle:

- `Element` trait: `request_layout`, `prepaint`, `paint` with associated types `RequestLayoutState`/`PrepaintState: 'static`; `id()` returns `Option<ElementId>` and enables keyed element state (`GPUI/element.rs:53-106`).
- Full-container layout idiom: `Style { size: Size::full(), ..Default::default() }` passed to `window.request_layout(style, None, cx)` (`GB/plot/element.rs:73-85`; `Size::full()` at `GPUI/geometry.rs:677`).
- Hitboxes are inserted in prepaint: `window.insert_hitbox(bounds, HitboxBehavior) -> Hitbox`, prepaint-phase asserted (`GPUI/window.rs:5157-5168`). `HitboxBehavior::BlockMouse` makes hitboxes behind report `is_hovered() == false` and `should_handle_scroll() == false` (`GPUI/window.rs:888-910` doc block).
- Mouse listeners are registered in paint: `window.on_mouse_event::<E: MouseEvent>(|&E, DispatchPhase, &mut Window, &mut App|)`; listeners are cleared when the next frame renders, so they must be re-registered every paint (`GPUI/window.rs:5288-5305`).
- `DispatchPhase::{Bubble, Capture}` with `.bubble()` predicate (`GPUI/window.rs:92-113`). `cx.stop_propagation()` on `App` (`GPUI/app.rs:2525`).
- Real custom-paint example: `PlotElement<P>` implements `Element` with `(Option<Hitbox>, Vec<AnyElement>, Option<AnyElement>)` prepaint state, hitbox in prepaint, hover via `window.mouse_position()` and `hitbox.is_hovered(window)`, repaint via `cx.notify(view)` with `window.current_view()` (`GB/plot/element.rs:57-233`).

Paint primitives (paint-phase only, enforced by `debug_assert_paint`):

- `window.paint_quad(quad: PaintQuad)` (`GPUI/window.rs:4550`); free constructors `quad(...)` (`:7675`) and `fill(bounds, background)` (`:7694`). The plot grid paints lines as `window.paint_quad(fill(line_bounds(start, end), stroke))` (`GB/plot/grid.rs:83`).
- `window.paint_path(path: Path<Pixels>, color: impl Into<Background>)`; `From<Hsla> for Background` exists (`GPUI/window.rs:4621`; `GPUI/color.rs:950-958`).
- `PathBuilder`: `stroke(width)`, `fill()`, `move_to`, `line_to`, `curve_to(to, ctrl)` (quadratic), `cubic_bezier_to`, `close`, `dash_array(&[Pixels])`, `build() -> Result<Path<Pixels>>` (`GPUI/path_builder.rs:88-98, 125-152, 199, 244`). `Path<Pixels>` vertices are plain data that can be cloned and translated (`GB/plot/path_cache.rs:46-53`).
- `window.paint_image(bounds, image_bounds, corner_radii, data: Arc<RenderImage>, frame_index, grayscale) -> Result<()>`; tiles through the sprite atlas keyed by `(ImageId, frame_index)` (`GPUI/window.rs:4919-4960`). `RenderImage` is exported through the kit glob (`pub use assets::*`, `GPUI/gpui.rs:105`; re-exported by `KIT/lib.rs:97`).
- Text: `window.text_system().shape_line(text: SharedString, font_size: Pixels, runs: &[TextRun], force_width: Option<Pixels>) -> ShapedLine` (`GPUI/window.rs:2402`; `GPUI/text_system.rs:638`), then `ShapedLine::paint(origin, line_height, align: TextAlign, align_width, window, cx)` (`GPUI/text_system/line.rs:108-118`). `TextRun { len, font, color: Hsla, background_color, underline, strikethrough }` (`GPUI/text_system.rs:1228-1241`); `Font::default()` (`:1311`). There is no `paint_text` on `Window` in this snapshot (checked by grep; survey section a agrees).
- Clipping: `window.with_content_mask(Some(ContentMask { bounds }), f)` intersects with the current mask (`GPUI/window.rs:4029-4044`).

Caching pattern (the invalidation model this plan reuses):

- `PathCache::get(key, origin, build)` builds once per `u64` key and clones-plus-translates the stored path to each frame origin; `ShapeKey` hashes projected points and stroke parameters to f32 bits (`GB/plot/path_cache.rs:25-53, 109-131`).
- Cross-frame state for a value-like element lives in window keyed element state: `window.use_keyed_state(key, cx, init) -> Entity<S>` (`GPUI/window.rs:4204`) or `window.with_element_state(global_id, f)` (`GPUI/window.rs:4247`).

Events and entities:

- `Entity<T>::read(&self, cx: &App) -> &T` and `Entity<T>::update(&self, cx, f: FnOnce(&mut T, &mut Context<T>) -> R)` (`GPUI/app/entity_map.rs:485, 497`).
- Entity events: `impl EventEmitter<E> for T {}` (`GPUI/gpui.rs:346`), `cx.emit(event)` on `Context<T> where T: EventEmitter<E>` (`GPUI/app/context.rs:758-766`).
- `Context<T>` derefs to `App`, never to `T` (`GPUI/app/context.rs:26-38`); `Context::entity() -> Entity<T>` (`:47-52`), `Context::notify(&mut self)` (`:221`), `impl AppContext for Context<'_, T>` (`:777`). Re-entering the same entity from inside its lease panics: `lease_erased` calls `double_lease_panic("update", ...)` ("cannot update T while it is already being updated") when the entity is already leased (`GPUI/app/entity_map.rs:160-165`, panic text at `:231-234`). So a tool running inside `Entity::<CanvasState>::update` must never call `cx.entity().update(cx, ...)` on that same entity; it needs `&mut CanvasState` handed to it (the dispatch design below).
- `IntoElement` (singular) converts a value into an `Element`; `Element: 'static + IntoElement` is the supertrait (`GPUI/element.rs:53, 147-159`). There is no `IntoElements` trait in this snapshot: `grep -rn "IntoElements" gpui-pre-0.3.8/src gpui-kit-0.7.1/src` returns zero hits (run this session). A custom element type therefore needs both `impl Element` and `impl IntoElement for Itself { type Element = Self; }` (AnyElement does the same, `GPUI/element.rs:717-727`). `Element::source_location()` has no default body and must be implemented (`GPUI/element.rs:71`); the plot returns `None` (`GB/plot/element.rs:69-71`).
- Mouse event structs and fields: `MouseDownEvent { button, position, modifiers, click_count, first_mouse }` (`GPUI/interactive.rs:148-163`), `MouseUpEvent { button, position, modifiers, click_count }` (`:185-197`), `MouseMoveEvent { position, pressed_button, modifiers }` with `dragging()` (`:494-518`), `ScrollWheelEvent { position, delta: ScrollDelta, modifiers, touch_phase }` with `ScrollDelta::{Pixels(Point<Pixels>), Lines(Point<f32>)}` (`:522-565`), `MouseButton::{Left, Right, Middle, Navigate}` (`:453-466`). `Modifiers` is `Copy + Default` (`GPUI/platform/keystroke.rs:447-448`); its boolean fields are exactly `control`, `alt`, `shift`, `platform`, `function` (`:450-464`) — there is no `meta` or `ctrl` field, so "ctrl/cmd held" is `modifiers.control || modifiers.platform`.
- Cursor: `CursorStyle::{Arrow, IBeam, Crosshair, OpenHand, ClosedHand, PointingHand, ...}` (`GPUI/platform.rs:2828-2870`); `window.set_cursor_style(style, &hitbox)` (`GPUI/window.rs:3993`).

Arithmetic (matters because `Pixels.0` is `pub(crate)`, `GPUI/geometry.rs:2689`):

- `Point<T>` derives `Add, AddAssign, Sub, SubAssign, Neg` (`GPUI/geometry.rs:64-79`); `Point<Pixels> * f32` works elementwise (`impl<T, Rhs> Mul<Rhs> for Point<T>` at `GPUI/geometry.rs:236`), but `Point<Pixels> / f32` does not, because `Pixels` only has `Mul<f32>` (`:2719`) and `Pixels / Pixels -> f32` (`:2691`). Divide through `px(value.as_f32() / zoom)`; `as_f32()` at `:2802`. `px` itself is a `const fn` (`:3748`), so `pub const HIT_TOLERANCE: Pixels = px(4.)` compiles.
- `Hsla` fields `h, s, l, a` are public; derives are `Default, Copy, Clone, Debug` (`GPUI/color.rs:332-334`), and equality exists through manual `PartialEq`/`PartialOrd`/`Ord`/`Eq` impls built on `total_cmp` (`GPUI/color.rs:374-398`). `Serialize` is absent, so scene types are not serde-serializable as-is. `Hsla::opacity(&self, factor) -> Self` (`:637`; `Rgba::opacity` is the lookalike at `:126`). `CanvasShape` and `CanvasLayer` derive `Clone, Debug` only — a deliberate choice, since `RenderImage` carries no derive attribute (`GPUI/assets.rs:41-48`) even though manual `PartialEq` (`:51`) and `Debug` (`:109`) impls exist, so `PartialEq` on shapes would compile but serves no consumer. Of the `Hsla`-bearing leaf types, `StrokeStyle`, `BrushConfig`, and `CanvasTransform` derive `PartialEq` while `ShapeStyle` stays `Clone, Debug, Default` — the Interface Contract block below is authoritative for every derive. `ScrollDelta` is `Clone, Copy, Debug` without `PartialEq` (`GPUI/interactive.rs:553-554`), so `CanvasScrollEvent` derives no `PartialEq`; `MouseButton` (`:452`) and `Modifiers` (`GPUI/platform/keystroke.rs:447`) both carry `Copy + PartialEq`, so `CanvasPointerEvent` may.
- Theme access from paint's `&mut App`: `use gpui_kit::component::ActiveTheme;` then `cx.theme()` (`gpui-component-0.7.1/src/theme/mod.rs:44-50`).

Tests:

- `#[gpui_kit::test]` with `cx: &TestAppContext` is available in-crate (`src/state/config_store.test.rs:280-281`; survey section 7). `TestAppContext::add_window(build) -> WindowHandle<V>` (`GPUI/app/test_context.rs:279-296`), `VisualTestContext::from_window(any_handle, &cx)` derefs to `TestAppContext` and wraps `update` (`:823-849`), `simulate_mouse_move/down/up` and `simulate_click` dispatch real events into the drawn frame (`:894-940`). The kit's own `gpui_kit::test` module needs the kit `test-support` feature, which this repo does not enable (survey Open Questions); the gpui-level harness above needs nothing new.

Repo constraints (from `docs/plans/_survey/seams.md`, re-checked where cited):

- No `use gpui::` in `src/` or `tests/`; kit facade only, enforced by `tests/qa_migration.rs:386-405`. Widget module pattern: `src/ui/widgets/virtual_list.rs:1-6` (`use gpui_kit::{prelude::*, *};`), sibling test wiring `#[cfg(test)] #[path = "x.test.rs"] mod x_test;` (`:90-92`).
- No Cargo dependency changes are needed for this feature: it uses only `gpui-kit` plus `serde` (already workspace deps, `Cargo.toml:21, 25`).

Crate picks: none new. Everything is built on `gpui-kit 0.7.1` (pinned, facade rule) and workspace `serde 1`.

## Architecture

Ownership model (matches the `Entity<InputState>` pattern from the coding guides; `src/ui/widgets/virtual_list.rs` is the module pattern):

- `CanvasState` is the only retained state, an `Entity<CanvasState>` owned by the mounting view. It holds the document, view transform, grid settings, selection, the boxed active tool, the tool overlay, a document revision counter, and two `pub(crate)` router flags (`middle_pan`, `pointer_active`).
- `Canvas` is a value-like element built each render (`Canvas::new(id, entity)`). Its `CanvasElement` implements `Element`: request_layout fills the parent, prepaint inserts one `BlockMouse` hitbox, paint draws the frame from `state.read(cx)` and re-registers mouse listeners.
- Redraw flow: every `CanvasState` mutator finishes with one `cx.notify()` (dirtying the entity) plus a `cx.emit(CanvasEvent::...)` when the change is semantic. The mounting view must `cx.observe(&state, |_, _, cx| cx.notify())`; that observe is part of the public contract and is documented on `Canvas::new`. Nothing else triggers repaints; there is no animation loop in v1.
- Pointer flow: platform event -> `window.on_mouse_event` listener (registered in paint, bubble phase) -> router in `element.rs`. The router first converts the window-space event position to viewport space (`position - bounds.origin`; element bounds arrive in window space, and mouse event positions are window-relative, `GPUI/interactive.rs:522-524`). It resolves `BlockMouse` hitbox hover, then either drives the router-level middle-button pan or forwards a `CanvasPointerEvent`/`CanvasScrollEvent` to the active tool. All coordinates downstream of the router are viewport space (element-local, origin = element top-left): `CanvasTransform` outputs, `CanvasPointerEvent.screen_position`, `ToolContext.viewport` (origin `(0,0)`, element size), and `zoom_at` anchors; painters add `bounds.origin` back exactly once when calling `paint_quad`/`paint_path`/glyph/image APIs, which take window-space geometry. Tool dispatch runs inside `Entity::<CanvasState>::update` through `CanvasState::dispatch_tool_event` (pub(crate)): it swaps the tool out with `mem::replace` so the tool can receive `&mut CanvasState` and `&mut Context<CanvasState>` as separate arguments, then restores it. This shape is forced by the entity map: `Context<CanvasState>` derefs to `App` (`GPUI/app/context.rs:26-38`), and re-leasing the same entity from inside its own update panics (`GPUI/app/entity_map.rs:160-165, 231-234`). Tools call the ordinary `CanvasState` mutators (`state.pan_by(d, cx)`, `state.select_shape(id, add, cx)`, ...); the notify/observe path produces the next frame.
- Invalidation: GPUI repaints whole windows; the engine's cost control is (1) a per-shape tessellation cache keyed by a hash of geometry+style+zoom, reused across pans via clone-and-translate (the plot's `PathCache` mechanism), and (2) viewport culling: shapes whose transformed `doc_bounds` do not intersect the element bounds are skipped. The document revision and zoom are the cache keys' inputs; pan never invalidates.

Module tree:

```text
src/ui/canvas/
├── mod.rs         pub seam: header comment + module decls + re-exports
├── geometry.rs    CanvasTransform, doc/screen conversion, segment distance
├── scene.rs       CanvasDocument, CanvasLayer, CanvasShape, ShapeStyle, ids, hit geometry
├── state.rs       CanvasState entity, CanvasEvent, CanvasGrid, zoom clamp consts
├── tool.rs        CanvasTool trait, ToolContext, pointer/scroll event types, built-in tools
├── cache.rs       ShapeCache + CanvasCaches (window keyed state), key hashing
├── paint.rs       grid, shape, overlay, selection painters (thin over pure helpers)
└── element.rs     Canvas, CanvasElement: Element impl + pointer router
```

Each module except `mod.rs` gets a sibling `*.test.rs`.

## Interface Contract

Exact pub surface. Consumers (plans 05, 06, 07, 08, 10, 14) import from `gpui_starter::ui::canvas`. Everything is `#[non_exhaustive]` where it may grow; record-like structs keep pub fields, behavioral structs keep private fields.

```rust
// src/ui/canvas/mod.rs — re-exports every item below.
use gpui_kit::*;

pub const MIN_ZOOM: f32 = 0.05;
pub const MAX_ZOOM: f32 = 32.0;
/// Minimum hit-test tolerance in screen pixels; doc-space tolerance = this / zoom.
pub const HIT_TOLERANCE: Pixels = px(4.);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct LayerId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ShapeId(pub u64);

/// Document coordinates are logical pixels at zoom 1.0; screen is viewport space (element-local): screen = doc * zoom + pan.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasTransform { pub pan: Point<Pixels>, pub zoom: f32 }

impl CanvasTransform {
    pub const IDENTITY: Self;                                  // pan (0,0), zoom 1.0
    pub fn new(pan: Point<Pixels>, zoom: f32) -> Self;
    pub fn to_screen(&self, doc: Point<Pixels>) -> Point<Pixels>;   // doc * zoom + pan
    pub fn to_doc(&self, screen: Point<Pixels>) -> Point<Pixels>;   // px((screen - pan).axis / zoom)
    pub fn to_screen_bounds(&self, doc: Bounds<Pixels>) -> Bounds<Pixels>;
    pub fn to_doc_bounds(&self, screen: Bounds<Pixels>) -> Bounds<Pixels>;
    pub fn scale_length(&self, doc_len: Pixels) -> Pixels;          // doc_len * zoom
}

/// Distance from `point` to segment `a`..`b`, in the input units. Pure.
pub fn segment_distance(point: Point<Pixels>, a: Point<Pixels>, b: Point<Pixels>) -> Pixels;

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct StrokeStyle {
    pub width: Pixels,        // document-space stroke width, scales with zoom
    pub color: Hsla,
    pub dash: Option<Vec<Pixels>>,
}
impl Default for StrokeStyle;                                    // width px(1.), black, no dash

/// Fill and/or stroke. Private fields; use the constructors.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct ShapeStyle { /* fill: Option<Hsla>, stroke: Option<StrokeStyle> */ }
impl ShapeStyle {
    pub fn filled(fill: Hsla) -> Self;
    pub fn stroked(stroke: StrokeStyle) -> Self;
    pub fn both(fill: Hsla, stroke: StrokeStyle) -> Self;
    pub fn fill(&self) -> Option<Hsla>;
    pub fn stroke(&self) -> Option<&StrokeStyle>;
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum CanvasShape {
    Rect { bounds: Bounds<Pixels>, style: ShapeStyle },
    Ellipse { bounds: Bounds<Pixels>, style: ShapeStyle },
    Path { points: Vec<Point<Pixels>>, style: StrokeStyle },
    Text { position: Point<Pixels>, text: SharedString, font_size: Pixels, color: Hsla },
    Image { bounds: Bounds<Pixels>, image: Arc<RenderImage> },
}

impl CanvasShape {
    pub fn rect(bounds: Bounds<Pixels>, style: ShapeStyle) -> Self;
    pub fn ellipse(bounds: Bounds<Pixels>, style: ShapeStyle) -> Self;
    pub fn path(points: Vec<Point<Pixels>>, style: StrokeStyle) -> Self;
    pub fn text(position: Point<Pixels>, text: impl Into<SharedString>, font_size: Pixels, color: Hsla) -> Self;
    pub fn image(bounds: Bounds<Pixels>, image: Arc<RenderImage>) -> Self;
    /// Conservative document-space bounds. Text uses an explicit approximation:
    /// height = font_size * 1.25, width = font_size * 0.6 * char count.
    pub fn doc_bounds(&self) -> Bounds<Pixels>;
    pub fn translate(&mut self, delta: Point<Pixels>);
    pub fn hit_test(&self, doc_point: Point<Pixels>, tolerance: Pixels) -> bool;
}

/// A paint order: layers[0] paints first (bottom). Private fields.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct CanvasLayer { /* id: LayerId, visible: bool, shapes: Vec<(ShapeId, CanvasShape)> */ }
impl CanvasLayer {
    pub fn id(&self) -> LayerId;
    pub fn is_visible(&self) -> bool;
    pub fn shapes(&self) -> &[(ShapeId, CanvasShape)];
}

#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct CanvasDocument { /* layers: Vec<CanvasLayer>, next_id: u64 */ }
impl CanvasDocument {
    pub fn new() -> Self;
    pub fn layers(&self) -> &[CanvasLayer];
    pub fn layer(&self, id: LayerId) -> Option<&CanvasLayer>;
    pub fn top_layer(&self) -> Option<LayerId>;                       // last layer
    pub fn shape(&self, id: ShapeId) -> Option<&CanvasShape>;
    /// Bottom-to-top iterator across visible and hidden layers.
    pub fn shapes(&self) -> impl Iterator<Item = (LayerId, ShapeId, &CanvasShape)> + '_;
    pub fn add_layer(&mut self) -> LayerId;
    pub fn add_shape(&mut self, layer: LayerId, shape: CanvasShape) -> Option<ShapeId>;
    pub fn remove_shape(&mut self, id: ShapeId) -> Option<CanvasShape>;
    pub fn set_layer_visible(&mut self, id: LayerId, visible: bool);
    pub fn update_shape<R>(&mut self, id: ShapeId, f: impl FnOnce(&mut CanvasShape) -> R) -> Option<R>;
    pub fn clear(&mut self);
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CanvasGrid { pub visible: bool, pub spacing: Pixels }   // spacing in doc px
impl Default for CanvasGrid;                                        // visible, px(16.)

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum CanvasEvent { DocumentChanged, SelectionChanged, ViewTransformed, ToolChanged }

/// Retained canvas state. Mount with `cx.new(|_| CanvasState::new())`, render with
/// `Canvas::new(id, entity)`, and observe: `cx.observe(&state, |_, _, cx| cx.notify())`.
/// All mutators take `&mut Context<Self>` and end with exactly one `cx.notify()`.
/// No `#[derive(Default)]`: the `tool: Box<dyn CanvasTool>` field has no `Default`
/// bound, so `Default` is implemented by hand as `Self::new()` (PanTool active).
pub struct CanvasState { /* fields private; router flags are pub(crate) */ }
impl EventEmitter<CanvasEvent> for CanvasState {}
impl Default for CanvasState;   // -> Self::new()

impl CanvasState {
    pub fn new() -> Self;

    // View transform
    pub fn pan(&self) -> Point<Pixels>;
    pub fn zoom(&self) -> f32;
    pub fn transform(&self) -> CanvasTransform;
    pub fn pan_by(&mut self, delta: Point<Pixels>, cx: &mut Context<Self>);
    /// Zoom by `factor` keeping `anchor_screen` fixed; clamps to [MIN_ZOOM, MAX_ZOOM].
    pub fn zoom_at(&mut self, anchor_screen: Point<Pixels>, factor: f32, cx: &mut Context<Self>);
    pub fn set_zoom(&mut self, zoom: f32, cx: &mut Context<Self>);   // clamped, pan kept
    /// Fits doc bounds into viewport with a 0.9 margin; empty/zero-size bounds reset the view.
    pub fn zoom_to_fit(&mut self, doc_bounds: Bounds<Pixels>, viewport: Size<Pixels>, cx: &mut Context<Self>);
    pub fn reset_view(&mut self, cx: &mut Context<Self>);            // pan (0,0), zoom 1.0

    // Grid
    pub fn grid(&self) -> CanvasGrid;
    pub fn set_grid(&mut self, grid: CanvasGrid, cx: &mut Context<Self>);

    // Document
    pub fn document(&self) -> &CanvasDocument;
    pub fn document_revision(&self) -> u64;
    /// Runs `edit`, then bumps the revision, emits `DocumentChanged`, notifies once.
    pub fn edit_document<R>(&mut self, cx: &mut Context<Self>, edit: impl FnOnce(&mut CanvasDocument) -> R) -> R;
    pub fn add_layer(&mut self, cx: &mut Context<Self>) -> LayerId;
    pub fn add_shape(&mut self, layer: LayerId, shape: CanvasShape, cx: &mut Context<Self>) -> Option<ShapeId>;
    pub fn remove_shape(&mut self, id: ShapeId, cx: &mut Context<Self>) -> bool;
    pub fn set_layer_visible(&mut self, layer: LayerId, visible: bool, cx: &mut Context<Self>);
    pub fn clear(&mut self, cx: &mut Context<Self>);

    // Selection and hit testing (topmost first; hidden layers skipped)
    pub fn selected_shape_ids(&self) -> &[ShapeId];
    pub fn hit_test(&self, doc_point: Point<Pixels>, tolerance: Pixels) -> Option<ShapeId>;
    pub fn select_shape(&mut self, id: ShapeId, additive: bool, cx: &mut Context<Self>);
    pub fn clear_selection(&mut self, cx: &mut Context<Self>);

    // Tools
    pub fn set_tool(&mut self, tool: Box<dyn CanvasTool>, cx: &mut Context<Self>);   // clears overlay
    pub fn set_builtin_tool(&mut self, tool: BuiltinTool, cx: &mut Context<Self>);
    pub fn active_tool_id(&self) -> &str;
    pub fn active_tool_cursor(&self) -> CursorStyle;
    /// Transient shape painted above the document, below the selection outline.
    /// Not hit-testable, not part of the document; cleared by `set_tool`.
    pub fn set_tool_overlay(&mut self, overlay: Option<CanvasShape>, cx: &mut Context<Self>);
    pub fn tool_overlay(&self) -> Option<&CanvasShape>;
    /// True between a forwarded pointer down and its pointer up (drag in progress).
    pub fn is_pointer_active(&self) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CanvasPointerEvent {
    pub screen_position: Point<Pixels>,
    pub doc_position: Point<Pixels>,
    pub button: MouseButton,
    pub click_count: usize,
    pub modifiers: Modifiers,
}

#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct CanvasScrollEvent {
    pub screen_position: Point<Pixels>,
    pub doc_position: Point<Pixels>,
    pub delta: ScrollDelta,
    pub modifiers: Modifiers,
}

/// Router-computed snapshot handed to tools each event. `viewport` is viewport space: origin (0,0), element size.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct ToolContext { pub viewport: Bounds<Pixels>, pub transform: CanvasTransform, pub hit: Option<ShapeId> }

/// Pluggable tool. `state` is the live canvas state: call the ordinary mutators
/// on it (`state.pan_by(d, cx)`, ...). Never re-enter the entity with
/// `cx.entity().update(cx, ..)` while dispatched; the lease is already held and
/// the entity map panics (`GPUI/app/entity_map.rs:160-165, 231-234`).
pub trait CanvasTool: 'static {
    /// Stable identifier ("pan", "select", "brush", ...). Shown by diagnostics only, never UI copy.
    fn id(&self) -> &'static str;
    fn cursor(&self) -> CursorStyle { CursorStyle::Arrow }
    fn pointer_down(&mut self, _: &CanvasPointerEvent, _: &ToolContext, _: &mut CanvasState, _: &mut Context<CanvasState>) {}
    fn pointer_move(&mut self, _: &CanvasPointerEvent, _: &ToolContext, _: &mut CanvasState, _: &mut Context<CanvasState>) {}
    fn pointer_up(&mut self, _: &CanvasPointerEvent, _: &ToolContext, _: &mut CanvasState, _: &mut Context<CanvasState>) {}
    /// Return true when consumed; false falls through to the engine default
    /// (wheel pans, ctrl/cmd+wheel zooms at the cursor).
    fn scroll(&mut self, _: &CanvasScrollEvent, _: &ToolContext, _: &mut CanvasState, _: &mut Context<CanvasState>) -> bool { false }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BrushConfig { pub color: Hsla, pub width: Pixels }   // Default: black, px(3.)

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum BuiltinTool { Pan, Select, Brush(BrushConfig) }

pub struct PanTool;    // id "pan", cursor OpenHand; left-drag pans (tool), middle-drag pans (router); wheel pans, ctrl/cmd+wheel falls through to engine zoom
pub struct SelectTool; // id "select", cursor Arrow; click selects, drag moves selection
pub struct BrushTool;  // id "brush", cursor Crosshair; drag draws a smoothed path, up commits

/// Mounts the canvas. `id` keys the element state (path caches) and must be
/// unique among siblings. Fills its parent (size full).
pub struct Canvas { /* private */ }
impl Canvas {
    pub fn new(id: impl Into<ElementId>, state: Entity<CanvasState>) -> Self;
}
impl IntoElement for Canvas { type Element = CanvasElement; /* into_element() constructs it */ }
pub struct CanvasElement { /* private; build via `Canvas` */ }
impl IntoElement for CanvasElement { type Element = Self; /* required by the Element supertrait */ }

// pub(crate) router surface (state.rs / tool.rs, not exported past ui::canvas):
//   CanvasState { pub(crate) middle_pan: Option<Point<Pixels>>, pub(crate) pointer_active: bool }
//   impl CanvasState { pub(crate) fn dispatch_tool_event<R>(...) }  // tool swap dispatch, see below
//   pub(crate) struct NoopTool;                                      // id "inactive", placeholder during dispatch
```

Consumption notes that are part of the contract:

- Redraw requires the mounting view to `cx.observe(&state, |_, _, cx| cx.notify())`; mutations from outside a window (tasks, sockets) use `entity.update(cx, |s, cx| s.pan_by(.., cx))` and the same observe drives the frame.
- Tool dispatch: the router calls `state.update(cx, |s, cx| s.dispatch_tool_event(cx, |tool, s, cx| tool.pointer_down(&ev, &tool_ctx, s, cx)))`. `dispatch_tool_event` swaps the tool out with `mem::replace(&mut self.tool, Box::new(NoopTool))`, runs the callback with disjoint `tool`/`state`/`cx`, then restores the tool only if `self.tool.id()` is still `"inactive"` (a `set_tool` call inside the callback intentionally wins). Custom tools are exercised in tests the same way, inside `entity.update`.
- Router rules (fixed by this plan): middle-button drag always pans regardless of tool; the active tool receives left/right events; wheel without ctrl/meta pans, with ctrl/meta zooms at the cursor by `factor = (-delta_lines * 0.1).exp()` where `delta_lines` is `Lines(y)` or `Pixels(y)/40`; all handled events call `cx.stop_propagation()`. Coordinate spaces are pinned once at the router: events arrive in window space, are converted by `position - bounds.origin`, and everything downstream (`to_doc`, `screen_position`, `zoom_at` anchors, `ToolContext`) is viewport space; painters add `bounds.origin` back at the single `paint_*` call seam.
- Paint order per frame: background quad, grid, layers bottom-to-top (culled), tool overlay, selection outline, then listeners and `set_cursor_style`.

## Module Layout

| File | Purpose |
|---|---|
| `src/ui/canvas/mod.rs` | Module seam: header comment with removal recipe, `mod` decls, `pub use` re-exports |
| `src/ui/canvas/geometry.rs` | `CanvasTransform`, `segment_distance`, coordinate conversions |
| `src/ui/canvas/scene.rs` | `CanvasDocument`/`CanvasLayer`/`CanvasShape`/`ShapeStyle`/`StrokeStyle`/ids, per-variant bounds, translate, hit geometry |
| `src/ui/canvas/state.rs` | `CanvasState` entity, `CanvasEvent`, `CanvasGrid`, zoom clamping, mutators, selection, hit-test dispatch |
| `src/ui/canvas/tool.rs` | `CanvasTool`, `ToolContext`, event types, `BuiltinTool`, `NoopTool` (pub(crate)), `PanTool`, `SelectTool`, `BrushTool`, `BrushConfig` |
| `src/ui/canvas/cache.rs` | `ShapeCache`/`CanvasCaches` keyed by geometry+style+zoom hash; clone-and-translate reuse |
| `src/ui/canvas/paint.rs` | Painters (grid, shapes, overlay, selection) plus pure helpers (grid step LOD, smoothed path segments, text bounds) |
| `src/ui/canvas/element.rs` | `Canvas`/`CanvasElement`: `Element` impl, `BlockMouse` hitbox, pointer router, cursor |
| `src/ui/canvas/*.test.rs` | Sibling test modules per repo convention (`#[cfg(test)] #[path = ...]`) |

## Seam Touches

One edit outside the new module:

- `src/ui/mod.rs` (currently a single line, `pub mod widgets;`, `src/ui/mod.rs:1`): add `pub mod canvas;`. Minimal because the canvas is a library consumed through `gpui_starter::ui::canvas`, needs no page, route, command, locale key, or capability entry.

Deliberate non-touches (so reviewers can hold the line): no `src/features/pages/` entry, no `src/shell/sidebar.rs` variant, no gallery section, no `locales/` keys, no `Cargo.toml` change, no capability-registry entry (nothing platform-specific to report; see Platform & Wasm Gating).

### Removal Recipe

Mirror of the `src/gallery/mod.rs:1-7` header. `mod.rs` opens with:

```rust
//! Removable shared canvas widget library. `ui::canvas` is the only public
//! surface; features mount it as a child element, nothing routes to it.
//! Removal = delete this directory and revert its single seam touch: the
//! `pub mod canvas;` line in src/ui/mod.rs.
```

Ordered removal steps:

1. Revert `src/ui/mod.rs` to `pub mod widgets;` only.
2. `rm -r src/ui/canvas/`.
3. Grep `ui::canvas` across `src/` and `tests/`: hits remaining are dependent features (plans 05+); removal of the foundation is only complete when those are removed or stubbed first.
4. Run the gates in Verification; they must pass with zero references left.

## Implementation Phases

## Phase 1: Module scaffold and transforms

### Step 1.1: Scaffold the module

- Files: `src/ui/canvas/mod.rs` (header comment, `mod geometry;` for now), `src/ui/mod.rs` (+ `pub mod canvas;`).
- Build: `mod.rs` re-exports `geometry::*` items for now; later phases extend the re-export list.
- Tests: none.
- Done-check: `cargo check -p gpui-starter` passes and `grep -n "pub mod canvas" src/ui/mod.rs` prints the new line.

### Step 1.2: `CanvasTransform` and pure geometry

- Files: `src/ui/canvas/geometry.rs`, `src/ui/canvas/geometry.test.rs`.
- Build: `CanvasTransform { pan, zoom }` with `IDENTITY`, `new`, `to_screen` (`doc * zoom + pan`, using `Point<Pixels> * f32`, `GPUI/geometry.rs:2719`), `to_doc` (`px((screen - pan).x.as_f32() / zoom)` per axis, because `Pixels / f32` does not exist), `to_screen_bounds`, `to_doc_bounds`, `scale_length`; free fn `segment_distance(point, a, b)` (project onto the segment, clamp `t` to `0..=1`, return euclidean distance).
- Tests (`geometry.test.rs`): identity roundtrip `to_doc(to_screen(p)) == p` for sample points; `to_screen((10,20))` with pan `(5,7)` zoom `2.` is `(25., 47.)`; bounds conversion maps and inverses; `segment_distance` of the point onto the segment midpoint and past both ends.
- Done-check: `cargo test -p gpui-starter --lib canvas::geometry` passes.

## Phase 2: Scene model

### Step 2.1: Shape types and geometry ops

- Files: `src/ui/canvas/scene.rs`, `src/ui/canvas/scene.test.rs`.
- Build: `StrokeStyle`, `ShapeStyle` (private fields + constructors + readers), `CanvasShape` enum with the five variants and constructors; `doc_bounds` (Rect/Ellipse/Image: the bounds; Path: min/max over points expanded by half the stroke width; Text: the documented approximation `height = font_size * 1.25`, `width = font_size * 0.6 * chars.count()`); `translate` (move bounds/points/position by delta); `hit_test` (Rect/Image: bounds contains, expanded by tolerance; Ellipse: inside the ellipse expanded by tolerance on both radii, `(dx/(rx+tol))^2 + (dy/(ry+tol))^2 <= 1.`; Path: any `segment_distance <= tolerance` plus, for a single point, the ellipse test on a dot of diameter `width`; Text: bounds contains).
- Tests: per-variant `doc_bounds` values for fixed inputs; translate of each variant; hit accept/reject at the boundary and just outside tolerance for rect, ellipse, and a two-point path; single-point path hit.
- Done-check: `cargo test -p gpui-starter --lib canvas::scene` passes.

### Step 2.2: Layers and document

- Files: same.
- Build: `LayerId`/`ShapeId` newtypes (pub tuple field, serde derives); `CanvasLayer` (private `id`, `visible`, `shapes: Vec<(ShapeId, CanvasShape)>`) with readers; `CanvasDocument` with `new`, readers (`layers`, `layer`, `top_layer`, `shape`, `shapes` iterator), mutators (`add_layer`, `add_shape` returning `None` for an unknown layer, `remove_shape`, `set_layer_visible`, `update_shape`, `clear`). IDs come from a private `next_id` counter, monotonic across removals.
- Tests: `add_layer`/`add_shape` allocate increasing ids and survive `remove_shape` (no reuse); `add_shape` to an unknown layer returns `None`; `shapes()` yields bottom-to-top including hidden layers; `update_shape` on a missing id returns `None`; `clear` empties layers but keeps the id counter monotonic.
- Done-check: `cargo test -p gpui-starter --lib canvas::scene` passes with the new tests.

## Phase 3: `CanvasState` entity

### Step 3.1: State core, grid, revisions

- Files: `src/ui/canvas/mod.rs` (+ `mod tool;`), `src/ui/canvas/tool.rs` (seed, see Build), `src/ui/canvas/state.rs`, `src/ui/canvas/state.test.rs`.
- Build: `CanvasState` struct (private fields: `document`, `transform`, `grid`, `selection: Vec<ShapeId>`, `doc_revision: u64`, `tool: Box<dyn CanvasTool>`, `overlay: Option<CanvasShape>`; plus two `pub(crate)` fields for the router: `middle_pan: Option<Point<Pixels>>`, `pointer_active: bool`); `CanvasEvent`; `impl EventEmitter<CanvasEvent> for CanvasState`; `new()` with `PanTool` active and a hand-written `impl Default for CanvasState { fn default() -> Self { Self::new() } }` (a derive cannot work: `Box<dyn CanvasTool>` is not `Default`); consts `MIN_ZOOM`, `MAX_ZOOM`, `HIT_TOLERANCE`; readers (`pan`, `zoom`, `transform`, `grid`, `document`, `document_revision`, `selected_shape_ids`, `tool_overlay`, `is_pointer_active`); `set_grid`; `edit_document` (run closure, `doc_revision += 1`, emit `DocumentChanged`, `cx.notify()` once) and the delegating mutators (`add_layer`, `add_shape`, `remove_shape`, `set_layer_visible`, `clear`), each also notifying once; the pub(crate) dispatch helper (lives here, used by the element router and by tool tests):

  ```rust
  impl CanvasState {
      /// Runs `f` with the active tool, the state, and the context as disjoint
      /// borrows, by temporarily swapping the tool out of the entity.
      pub(crate) fn dispatch_tool_event<R>(
          &mut self,
          cx: &mut Context<Self>,
          f: impl FnOnce(&mut dyn CanvasTool, &mut CanvasState, &mut Context<CanvasState>) -> R,
      ) -> R {
          let mut tool = std::mem::replace(&mut self.tool, Box::new(NoopTool));
          let result = f(tool.as_mut(), self, cx);
          // A set_tool call inside f intentionally replaces the tool.
          if self.tool.id() == "inactive" {
              self.tool = tool;
          }
          result
      }
  }
  ```

  `NoopTool` (pub(crate), in `tool.rs`) implements `CanvasTool` with `id() -> "inactive"` and default no-op bodies. The swap exists because `Context<CanvasState>` derefs to `App`, not to the state (`GPUI/app/context.rs:26-38`), and re-leasing the entity from inside its own update panics (`GPUI/app/entity_map.rs:160-165, 231-234`). This step also seeds the rest of `tool.rs`: the `CanvasTool` trait exactly as the Interface Contract defines it (default no-op bodies, `scroll -> false`) plus a `PanTool` struct using those default bodies — the state owns `Box<dyn CanvasTool>` and `new()` activates `PanTool`, so the seed is required for this phase to compile; Step 4.2 fills `PanTool`'s drag/scroll behavior.
- Tests (`#[gpui_kit::test]`, `cx.update` + `cx.new`): `edit_document` bumps the revision exactly once per call and emits `DocumentChanged` (captured with `cx.subscribe` inside the window view or a helper entity); `add_shape` emits `DocumentChanged`; mutators keep a subscription count so a single call yields one event.
- Done-check: `cargo test -p gpui-starter --lib canvas::state` passes.

### Step 3.2: View transforms and selection

- Files: same.
- Build: `pan_by` (pan += delta, emit `ViewTransformed`, notify); `zoom_at` (clamp `zoom * factor` to `[MIN_ZOOM, MAX_ZOOM]`; `pan = anchor - (anchor - pan) * (new_zoom / zoom)`; anchor is viewport space; emit, notify); `set_zoom` (clamp, keep pan); `zoom_to_fit` (if either doc dimension or viewport dimension is `<= 0`, delegate to `reset_view`; else `zoom = clamp(0.9 * min(vw/dw, vh/dh))` computed with `as_f32()`, `pan = viewport_center - doc_center * zoom` where `viewport_center = Point { x: px(viewport.width.as_f32() / 2.), y: px(viewport.height.as_f32() / 2.) }` in viewport space); `reset_view`; `hit_test` (tolerance in doc units; iterate `document.layers()` reversed, skip `!is_visible`, shapes reversed, first `CanvasShape::hit_test` wins); `select_shape` (replace selection unless `additive`, no-op when already the exact same set, emit `SelectionChanged` only on change); `clear_selection`.
- Tests: `zoom_at` keeps the anchor fixed (transform the anchor through the new transform and assert equality within `px(0.01)`); clamping at both ends; `zoom_to_fit` on a known rect yields a zoom matching the formula and centers the doc bounds; `hit_test` prefers the top layer and skips hidden layers; `select_shape` additive and dedupe semantics; no event when selection does not change.
- Done-check: `cargo test -p gpui-starter --lib canvas::state` passes.

## Phase 4: Tool protocol and built-in tools

### Step 4.1: Protocol types

- Files: `src/ui/canvas/tool.rs` (event types, `ToolContext`, `BuiltinTool`, `BrushConfig`), `src/ui/canvas/state.rs` (`set_tool`, `set_builtin_tool`, `active_tool_id`, `active_tool_cursor`), `src/ui/canvas/tool.test.rs`.
- Build: `CanvasPointerEvent`, `CanvasScrollEvent`, `ToolContext` (the `CanvasTool` trait, `NoopTool`, and the seeded `PanTool` struct came in Step 3.1; verify the trait matches the contract's signatures, including the `state: &mut CanvasState` parameter on every event method and default `scroll -> false`); `BuiltinTool`, `BrushConfig` (Default: black, `px(3.)`); `CanvasState::set_tool` (swap, clear overlay, emit `ToolChanged`, notify), `set_builtin_tool`, `active_tool_id`, `active_tool_cursor`.
- Tests: `set_tool` emits `ToolChanged` and resets the overlay to `None`; `active_tool_id` reflects the swap; `set_builtin_tool(Brush(..))` yields id `"brush"`.
- Done-check: `cargo test -p gpui-starter --lib canvas::tool` passes.

### Step 4.2: Pan, select, brush

- Files: same (tool.rs only; the `PanTool` struct was seeded in Step 3.1 — this step fills its behavior and adds `SelectTool` and `BrushTool`).
- Build: all tool bodies operate on the `state: &mut CanvasState` parameter with the `cx: &mut Context<CanvasState>` parameter (never `cx.entity().update` on CanvasState; the entity lease is held by the dispatcher). Tool-internal drag state lives in the tool's own fields.
- `PanTool` (fields: `dragging: bool`, `last_screen: Option<Point<Pixels>>`): `pointer_down` left sets both (middle never reaches tools — the router intercepts it, Step 7.1); `pointer_move` while dragging calls `state.pan_by(position - last, cx)` in screen space and updates `last`; `pointer_up` clears; `scroll` returns `false` without consuming when `event.modifiers.control || event.modifiers.platform`, so the engine's ctrl/cmd+wheel zoom default applies (`Modifiers` fields at `GPUI/platform/keystroke.rs:450-464`; no `meta` field exists), otherwise calls `state.pan_by(negated delta, cx)` (lines multiplied by 40 px) and returns true; cursor `OpenHand`.
- `SelectTool` (fields: `move_origin: Option<Point<Pixels>>` doc, `snapshot: Vec<(ShapeId, CanvasShape)>`): `pointer_down` left: `hit` -> if not already selected, `state.select_shape(hit, modifiers.shift, cx)`; snapshot every selected shape (`state.document().shape(id).cloned()` plus its doc_bounds origin) and set `move_origin`; no hit -> `state.clear_selection(cx)` unless shift; `pointer_move` while `move_origin` is set: `delta = doc - move_origin`, `state.edit_document(cx, |d| ...)` restoring each snapshot clone then `translate(delta)` via `update_shape`; `pointer_up` clears both fields; cursor `Arrow`.
- `BrushTool` (fields: `config: BrushConfig`, `points: Vec<Point<Pixels>>`, `stroking: bool`): `pointer_down` left starts (`points = vec![doc]`, `state.set_tool_overlay(Some(path_shape), cx)`); `pointer_move` while stroking appends the doc point only when the screen-space distance from the last exceeds `px(1.)` (compare through `ToolContext::transform.to_screen`) and refreshes the overlay to `CanvasShape::path(points.clone(), StrokeStyle { width: config.width, color: config.color, dash: None })` via `state.set_tool_overlay(.., cx)`; `pointer_up` commits: `let layer = state.document().top_layer().unwrap_or_else(|| state.add_layer(cx));` then `state.add_shape(layer, CanvasShape::path(points.clone(), style), cx)`, `state.set_tool_overlay(None, cx)`, clear strobing; cursor `Crosshair`.
- Tests (`#[gpui_kit::test]` with a `CanvasState` entity; drive tools with `entity.update(cx, |s, cx| s.dispatch_tool_event(cx, |tool, s, cx| tool.pointer_down(&ev, &ctx, s, cx)))` and matching move/up calls): pan drag sequence (down, two moves, up) moves pan by the total delta; select click on a known rect selects it; select drag translates the shape by the drag delta exactly once per move; brush down/move/up leaves exactly one `CanvasShape::Path` in the document with the expected point count, no overlay, and one `DocumentChanged` per commit; a tool callback that calls `state.set_tool(.., cx)` wins over the restored tool (`dispatch_tool_event` leaves the new one in place); middle-button behavior is router-level and not tested here.
- Done-check: `cargo test -p gpui-starter --lib canvas::tool` passes.

## Phase 5: Caches

### Step 5.1: Shape cache and keyed state

- Files: `src/ui/canvas/cache.rs`, `src/ui/canvas/cache.test.rs`.
- Build: `ShapeCache { key: Option<u64>, path: Option<Path<Pixels>> }` with `get(key, origin, build) -> Option<Path<Pixels>>` cloning and translating the stored path's vertices and bounds origin to `origin` (copy the `translated` technique from `GB/plot/path_cache.rs:46-53`); `CanvasCaches { slots: Vec<ShapeCache> }` with `slot(ix) -> &mut ShapeCache` (resize with default); `CanvasCaches::for_paint(key: impl Into<ElementId>, window, cx) -> Entity<Self>` via `window.use_keyed_state` (`GPUI/window.rs:4204`); key hashing `fn shape_key(shape: &CanvasShape, zoom: f32) -> u64` with `std::collections::hash_map::DefaultHasher` over the discriminant, every coordinate and length as `f32::to_bits`, the dash lengths, and `zoom.to_bits()`. Not part of the Interface Contract (`pub(crate)`).
- Tests: build-callback runs once per distinct key and not on origin change; returned path vertices differ from the stored ones by exactly the origin delta; key changes when zoom, a point, a stroke width, or the dash array changes (same assertions pattern as `GB/plot/path_cache.rs:146-196`).
- Done-check: `cargo test -p gpui-starter --lib canvas::cache` passes.

## Phase 6: Painters

### Step 6.1: Pure helpers

- Files: `src/ui/canvas/paint.rs`, `src/ui/canvas/paint.test.rs`.
- Build (all pure, `pub(crate)`): `grid_step(spacing: Pixels, zoom: f32) -> Pixels` doubling while `spacing * zoom < px(32.)` and halving while `>= px(64.)`; `smoothed_segments(points: &[Point<Pixels>]) -> Vec<(Point<Pixels>, Point<Pixels>)>` returning `(to, ctrl)` pairs for `curve_to` (move_to `p0`; for i in `1..len-1`: `curve_to(midpoint(p[i], p[i+1]), p[i])`; final segment is a plain `line_to(p[len-1])` and is not part of the return value); `text_bounds(position, text, font_size)` matching the `doc_bounds` approximation so hit and paint agree.
- Tests: `grid_step(px(16.), 1.)` is 32 (16 < 32 doubles once, 32 stops); `grid_step(px(16.), 0.25)` is 128 (doubles until `step * zoom` reaches 32); `grid_step(px(16.), 4.)` is 8 (64 triggers the halving branch once); for zooms `0.05, 0.1, 0.25, 0.5, 1., 2., 4., 8., 32.` the result always satisfies `32. <= step * zoom < 64.`; smoothed segments for a fixed 3-point input produce the expected midpoints.
- Done-check: `cargo test -p gpui-starter --lib canvas::paint` passes.

### Step 6.2: Painters

- Files: same (no new tests; thin wrappers over Phase 6.1 and the verified paint APIs).
- Build (`pub(crate)` fns taking `&mut Window`/`&mut App`): all math is viewport space; every `paint_*` call adds `bounds.origin` exactly once. `paint_grid(bounds, grid, transform, window, cx)` — background `fill(bounds, cx.theme().background)`, then for x and y separately: `let local = Bounds { origin: Point::default(), size: bounds.size };` visible doc range from `transform.to_doc_bounds(local)`, iterate multiples of the LOD step inside it, viewport line coordinate computed per axis (`px((doc.x * zoom + pan.x).as_f32().round())` for vertical lines, same for `.y` on horizontal ones — `Point<Pixels>` has no `as_f32()`), painted at `bounds.origin + line`, minor lines `cx.theme().border.opacity(0.4)`, every 4th line `.opacity(0.8)`, each a `fill` quad one logical pixel thick spanning `bounds` (`GB/plot/grid.rs:83` pattern). `paint_shape(shape, transform, bounds, cache: &mut ShapeCache, window, cx)` — cull when `transform.to_screen_bounds(shape.doc_bounds()).intersect(&local)` is empty (`GPUI/geometry.rs:1143`; `local` as above); Rect fill: `paint_quad(fill(screen_bounds + bounds.origin, hsla))` (add `bounds.origin` to the bounds' origin); Rect/Ellipse stroke and Path stroke: build a `PathBuilder::stroke(screen_width)` where `screen_width = style.width * zoom`, in doc-scaled coordinates relative to the shape's `doc_bounds().origin` (points multiplied by zoom), then `cache.get(shape_key, bounds.origin + to_screen(doc_bounds.origin), build)` — the translate step (Phase 5) lands the path in window space — and `window.paint_path(path, color)`; Ellipse fill: `PathBuilder::fill()` outline path through the same cache, `paint_path(path, fill_color)`; Path single point: filled ellipse of diameter `width * zoom` at `bounds.origin + to_screen(point)`; Text: `window.text_system().shape_line(text, font_size * zoom, &[TextRun { len: text.len(), font: Font::default(), color, background_color: None, underline: None, strikethrough: None }], None)` then `shaped_line.paint(bounds.origin + to_screen(position), font_size * zoom * 1.25, TextAlign::Left, None, window, cx)` and log-and-skip on `Err` (`TextRun` fields at `GPUI/text_system.rs:1228-1241`, `Font::default()` at `:1311`, `TextAlign::Left` at `GPUI/style.rs:426`); Image: `window.paint_image(screen_bounds + bounds.origin, same, Corners::default(), image.clone(), 0, false)`, log-and-skip on `Err`. `paint_overlay` and `paint_selection` (selection: 1-px screen-space stroke rect around each selected shape's transformed `doc_bounds` at `bounds.origin +` its viewport position, color `cx.theme().primary`).
- Done-check: `cargo clippy -p gpui-starter --all-features` reports no warnings for the new module.

## Phase 7: Element and integration

### Step 7.1: `CanvasElement`

- Files: `src/ui/canvas/element.rs`, extend `mod.rs` re-exports.
- Build: `Canvas { id: ElementId, state: Entity<CanvasState> }` with `new`; `CanvasElement` holding the same; `impl IntoElement for Canvas { type Element = CanvasElement }` and `impl IntoElement for CanvasElement { type Element = Self }` (the `Element` supertrait requires it, `GPUI/element.rs:53, 147-159`); `impl Element for CanvasElement` with `RequestLayoutState = ()`, `PrepaintState = Hitbox`, `id() -> Some(self.id.clone())`, `source_location() -> None` (required, no default body, `GPUI/element.rs:71`; plot precedent `GB/plot/element.rs:69-71`); `request_layout` returns `window.request_layout(Style { size: Size::full(), ..Default::default() }, None, cx)` (`GB/plot/element.rs:73-85` idiom); `prepaint` returns `window.insert_hitbox(bounds, HitboxBehavior::BlockMouse)`; `paint`: read state (`state.read(cx)`), `window.with_content_mask(Some(ContentMask { bounds }), ...)` around all painting, `paint_grid`, for each visible layer and shape `paint_shape` with `CanvasCaches::for_paint((self.id.clone(), "caches"), window, cx)` slot per paint index (enumerate; index shifts when the shape list changes, which only happens on document edits that change the key anyway), `paint_overlay`, `paint_selection`, `window.set_cursor_style(state.active_tool_cursor(), &hitbox)`, then register listeners (bubble phase, guarded by `phase.bubble()`). Every handler begins with `let position = e.position - bounds.origin;` (window -> viewport space; the `hitbox.is_hovered(window)` / `should_handle_scroll(window)` checks keep the raw window event). The router reads/writes the `pub(crate)` fields `middle_pan`/`pointer_active` and calls `dispatch_tool_event`; doc tolerance is `px(HIT_TOLERANCE.as_f32() / zoom)` (`Pixels / f32` does not exist; `as_f32` at `GPUI/geometry.rs:2802`):

  - `MouseDownEvent`: if `!hitbox.is_hovered(window)` return; `cx.stop_propagation()`; if `button == Middle` -> `state.update(cx, |s, _| s.middle_pan = Some(position))`; else `state.update(cx, |s, cx| { let doc = s.transform().to_doc(position); let event = CanvasPointerEvent { screen_position: position, doc_position: doc, button: e.button, click_count: e.click_count, modifiers: e.modifiers }; let tool_ctx = ToolContext { viewport: Bounds { origin: Point::default(), size: bounds.size }, transform: s.transform(), hit: s.hit_test(doc, px(HIT_TOLERANCE.as_f32() / s.zoom())) }; s.pointer_active = true; s.dispatch_tool_event(cx, |tool, s, cx| tool.pointer_down(&event, &tool_ctx, s, cx)); })`.
  - `MouseMoveEvent`: if `middle_pan` is Some (read via `state.read`), `pan_by(position - last)` and update `middle_pan` inside `state.update`, stop propagation; else if `hitbox.is_hovered(window) || state.is_pointer_active()`, forward `pointer_move` with the same dispatch shape (tools gate themselves on their own pressed state).
  - `MouseUpEvent`: if middle released and `middle_pan` was Some, clear it, stop propagation; else if `is_pointer_active()`, forward `pointer_up` through `dispatch_tool_event`, clear `pointer_active`, stop propagation.
  - `ScrollWheelEvent`: if `!hitbox.should_handle_scroll(window)` return; build `CanvasScrollEvent`; forward to `tool.scroll` through `dispatch_tool_event`; if it returns false, apply the engine default (exact formulas in the Interface Contract) with `state.update`; then `cx.stop_propagation()`.

  All closures clone the `Hitbox`, the `Entity<CanvasState>`, and capture `bounds: Bounds<Pixels>` (Copy); nothing borrows the element.
- Tests: none yet (Phase 7.2).
- Done-check: `cargo check -p gpui-starter` passes.

### Step 7.2: Integration tests

- Files: `src/ui/canvas/element.test.rs`, wired from `element.rs`.
- Build (`#[gpui_kit::test]` async): a `CanvasHost` view holding `Entity<CanvasState>` with the observe wiring from the contract; `cx.add_window(|window, cx| CanvasHost::new(window, cx))`; `let cx = VisualTestContext::from_window(handle.into(), &cx);` then:
  - pan test: `set_builtin_tool(Pan)`, `simulate_mouse_down(mid-window, Left, Modifiers::default())`, `simulate_mouse_move(+20,+10, Some(Left), default)`, `simulate_mouse_up`; assert `pan() == (20., 10.)` (deltas are origin-independent).
  - select-and-move test: identity transform, seed a rect at doc bounds `(100., 100.)..(200., 200.)`; the host renders `Canvas` as the root child so the element origin is `(0,0)` and window-space click position equals the viewport position `transform().to_screen(doc_center)`; `set_builtin_tool(Select)`, click inside the rect, assert `selected_shape_ids() == &[id]`; drag by `(30., 0.)`, assert the shape's `doc_bounds()` origin moved by `(30., 0.)`.
  - brush test: `set_builtin_tool(Brush(default))`, down/move/up across three positions spaced more than 1 px apart in screen space (BrushTool drops sub-pixel moves, so tighter spacing would not produce 3 points), assert one `CanvasShape::Path` with 3 points exists and `tool_overlay().is_none()`.
  - wheel default is not simulable with this harness (`TestAppContext` exposes no scroll simulation; checked `GPUI/app/test_context.rs:894-940`); assert the formula through a direct `zoom_at` call and cover wheel handling by review against the router code.
- Done-check: `cargo test -p gpui-starter --lib canvas` passes; full gates in Verification pass.

### Step 7.3: Docs and re-export freeze

- Files: `src/ui/canvas/mod.rs`.
- Build: final re-export list matching the Interface Contract exactly; doc comment on `Canvas::new` stating the observe requirement and unique-id precondition; module header per the Removal Recipe.
- Tests: `tests/qa_migration.rs` (existing) must stay green, proving no `use gpui::` crept in.
- Done-check: `cargo test --workspace --all-features` passes and `grep -rn "use gpui::" src/ui/canvas/` returns nothing.

## Platform & Wasm Gating

- No `cfg` gates anywhere in `src/ui/canvas/`. Every API used is platform-neutral: `paint_quad`/`paint_path` (lyon CPU tessellation) and `paint_image` (sprite atlas) run on the wgpu backends including WebGL (`gpui-pre-wgpu-0.3.8/src/shaders_webgl.wgsl` exists on disk, checked with `ls`; window rendering is per-platform code the engine never touches).
- No capability-registry entry: the engine has no platform variance, no init step, and no failure mode to report as degraded. A registry entry would be noise on the diagnostics page (seams survey section 4 lists entries for features with real platform state).
- No native-only dependencies; nothing goes under the `[target.'cfg(not(target_family = "wasm"))'.dependencies]` table. `Cargo.toml` is untouched, so `Cargo.lock` stays unchanged (seams survey section 9, lock check).
- Wasm constraints respected: no `std::time` clocks in the engine (no timing code at all); no file, socket, or OS-thread access; the repaint path is purely `cx.notify()`, no `request_animation_frame` (would spin the frame loop; the engine is event-driven).
- `just wasm-check` (`cargo +nightly check --target wasm32-unknown-unknown -p gpui-starter --lib`, `justfile:24-25`) must pass since the lib gains the new module.

## i18n

No user-facing strings originate in this feature. The engine renders documents (user data) and interaction chrome only; tool ids (`"pan"`, `"select"`, `"brush"`) are stable identifiers for diagnostics, never displayed copy. Labels for canvas surfaces are owned by the mounting feature (plans 05+), which adds its own `locales/en.yml` and `locales/zh-CN.yml` keys in lockstep (seams survey section 5).

| key | en | zh-CN |
|---|---|---|
| (none) | | |

## Design Compliance

- Tokens: background from `cx.theme().background`, grid and structural lines from `cx.theme().border` with `opacity()` factors, selection from `cx.theme().primary`. No raw hex, `rgb()`, or `hsla()` constructors in the module (design guides, Color and themes; `Hsla::opacity` at `GPUI/color.rs:637`). Stroke colors inside `CanvasShape` are data colors (the document's content, like raster data), which the guides exempt.
- Fixed-pixel exceptions, each on the guide's audited list: 1-px grid and selection lines (one-device-pixel hairline class), `HIT_TOLERANCE = px(4.)` (minimum hit-test tolerance), zoom clamp constants (geometry, not presentation). Grid spacing default `px(16.)` is document-space data, not layout.
- Density and components: the engine ships no controls; dependents compose chrome from kit components. The canvas itself fills its parent (`Size::full()`), the PlotElement idiom.
- Interaction states: cursor reflects the active tool via `set_cursor_style` (`Crosshair` brush, `OpenHand` pan, `Arrow` select); selection is a persistent visible state distinct from hover; drag feedback is the live tool overlay. Pointer capture semantics (events continue off-bounds until release) match desktop expectation for drags.
- A11y: v1 paints no accessibility node (`a11y_role()` defaults to `None`, `GPUI/element.rs:114-116`). The canvas is a custom-drawn surface whose content is not expressible in the tree without per-shape nodes, which dependents' overlays should provide (labeled toolbars, selection readouts). This is a documented limitation in Risks, not a silent omission; keyboard paths are owned by dependents' controls.
- Zoom/rem interplay: canvas zoom is document zoom, not interface zoom; it never touches `rem_size`. The only cached geometry keyed to rendering is the shape cache, keyed by content and canvas zoom, so rem changes cannot stale it.

## Verification

Gates (run in this order; the first three are the repo's mandatory set, AGENTS.md):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

Feature-specific automated coverage (all wired in the phases above):

- `cargo test -p gpui-starter --lib canvas` — transform math, scene hit testing, id allocation, state events/revision, tool behaviors, cache reuse, grid LOD, element pointer integration (real window via `VisualTestContext`).

Manual QA (optional, on a scratch working tree, never committed): temporarily mount `Canvas::new("smoke", state)` with a seeded document (one rect, one ellipse, one path, one text, one image) inside any existing page view, `cargo run`, then check:

1. Grid renders, adapts step density while zooming, and stays aligned with pan.
2. Ctrl/cmd+wheel zooms at the cursor; plain wheel pans; middle-drag pans under every tool.
3. Brush draws a smoothed stroke that lands in the document on release; the in-progress overlay tracks the cursor.
4. Select highlights with a 1-px primary outline and drag moves shapes; shift-click adds to the selection.
5. Resizing the window keeps content clipped to the canvas bounds; shapes disappear at the edge (culling), no painting outside.
6. Both light and dark themes render legibly (grid visible on both).

Acceptance criteria: gates pass; the automated integration tests exercise pan, select, move, and brush through real dispatched pointer events; the manual checklist is confirmed on the scratch tree or waived with a note naming which items were checked and how.

## Risks & Open Questions

Ranked:

1. **Contract churn breaks dependents.** Plans 05, 06, 07, 08, and 10 are on disk and consume this Interface Contract verbatim (read this session); 08 cites line numbers into the contract's code block, so that block must stay line-stable — revisions extend prose comments in place, never insert or delete lines inside the block. If a dependent needs, for example, rotation transforms, exact text metrics, or layer reordering, it must extend this contract in a follow-up edit (same in-place rule) rather than work around it. Plan 06 flagged a coordinate-space ambiguity in the original Step 7.1 router text (window-space positions passed to `to_doc` verbatim would offset every anchor by the element origin); this revision resolves it by pinning viewport-space semantics end to end (Architecture, contract comments, router rules bullet, Steps 3.2/6.2/7.1), matching 06's stated assumption, so 06's own origin conversions stay valid.
2. **Text hit test is an approximation** (`0.6 * font_size` per char): exact glyph widths exist only inside the paint phase via `shape_line`; no measure API is reachable from pure state code (tried: grep of `GPUI/text_system.rs` and `Window`; only `shape_line`/`layout_line` paths exist, both paint-side). Selection accuracy for short labels is coarse; acceptable for v1 hit testing, documented on `doc_bounds`.
3. **Per-frame text shaping cost.** Text shapes call `shape_line` every frame with no cache (unlike tessellated paths). Fine at document scale; if dependents paint hundreds of live text shapes, add a shaped-line cache keyed like `ShapeCache`. Deferred deliberately.
4. **Scroll-wheel handling is not integration-tested.** The test harness exposes no scroll simulation (`GPUI/app/test_context.rs:894-940` lists mouse/text/action helpers only; tried grep for `simulate` there). Wheel behavior is verified by code review plus the `zoom_at` unit test of the same formula.
5. **`BlockMouse` hitbox blocks tooltips/hover on elements behind the canvas** by design (`GPUI/window.rs:888-910`). If a dependent stacks overlays inside canvas bounds, those overlays must paint above and insert their own hitboxes; flagged so it surprises no one.
6. **Single `f32` hash collisions in cache keys** could paint a stale path: probability is negligible and the plot layer ships the same scheme (`GB/plot/path_cache.rs:109-131`); the key includes a revision-derived rehash through the document content itself, so an actual collision requires identical 64-bit hashes of different geometry.
7. **No a11y node in v1** (see Design Compliance). Screen readers see nothing of canvas content until dependents add labeled surfaces. Tracked as the follow-up with the clearest user impact.

Open questions (each with what was tried):

- **Plan 14 absent.** The feature spec names plans 05, 06, 07, 08, 10, 14 as contract consumers; `ls docs/plans/` shows 01–11 plus `_survey/` and no 14 file. 05/06/07/08/10 were read this session and their consumed items all exist in the contract above. Whatever 14 is, it inherits the same contract; nothing here blocks on it.
- **Whether dependents persist scenes.** `Hsla` has no `Serialize` (`GPUI/color.rs:332`), so scene types are not serde-serializable as-is. If a dependent needs persistence it must map to a serializable color form; tried deriving and confirmed the derive is unavailable on `Hsla`-bearing structs. Not needed for this plan's scope.
- **Kit test-support module** (`ElementSnapshot` et al.) stays unavailable; tried survey section 7 plus reading `gpui-kit-0.7.1/Cargo.toml` features as cited there. The plan uses gpui's own `TestAppContext`/`VisualTestContext`, verified on disk at `GPUI/app/test_context.rs:279, 823-849, 894-940`.
- **No gates were run during planning** (doc-only scope, same as the survey). The Verification section states the exact commands for the implementer.
