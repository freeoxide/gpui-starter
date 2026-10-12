# 14 Image Editor Plan

## Goal

A sidebar page, `Image Editor`, opens an image through the existing
`desktop_actions` file picker and edits it on plan 01's canvas engine: pan/zoom
view, a base-image layer plus an annotation layer, brush and eraser with size
and color, rect/ellipse/arrow shapes, a crop rect with Apply, rotate 90°, and
grayscale/blur/brightness filters computed with the `image` crate. Every edit is
undoable, and `Save As…` bakes filters and annotations into a new file through
`desktop_actions::save_file`. Edits touch only the shapes they change, so plan
01's revision/culling/shape-cache strategy keeps repaints localized.

## Dependencies

- **01-canvas-foundation** (`docs/plans/01-canvas-foundation.md`, Interface
  Contract read in full this session). Consumed items:
  - `Canvas::new(id, Entity<CanvasState>)` mount + the observe contract
    (`cx.observe(&state, |_, _, cx| cx.notify())`).
  - `CanvasState`: `new`, `edit_document`, `add_layer`, `add_shape`,
    `set_tool`, `active_tool_id`, `transform`, `zoom_to_fit`,
    `set_zoom`, `pan_by`, `hit_test`, `set_tool_overlay`; and through
    `document()`/`edit_document` (per 01's Interface Contract):
    `CanvasDocument::update_shape`, `CanvasDocument::remove_shape`,
    `CanvasDocument::top_layer`, `document_revision` — the document-scoped
    methods live on `CanvasDocument`, reached via the state's
    `document()`/`edit_document`, not on `CanvasState` itself.
  - `CanvasTool` trait (custom tools), `ToolContext`, `CanvasPointerEvent`,
    `NoopTool` dispatch semantics (the `dispatch_tool_event` swap contract).
  - Deliberately NOT consumed: `set_builtin_tool`/`BuiltinTool`/`BrushConfig`
    (01's builtin `BrushTool` commits through its own `state.add_shape` with
    no commit hook, so it cannot record editor history; this plan ships an
    editor-local `BrushTool` in `tools.rs`, Step 5.1), `is_pointer_active`
    (tools track their own drag state), `selected_shape_ids` (no selection UI
    in v1), `CanvasScrollEvent` (tools take the default `scroll -> false`).
  - `CanvasShape::{image, path, rect, ellipse}` constructors, `doc_bounds`,
    `translate`, `hit_test`; `ShapeStyle`/`StrokeStyle`; `LayerId`/`ShapeId`;
    `CanvasEvent`; `HIT_TOLERANCE`.
  - Consumption note: the router's middle-drag pan and wheel/ctrl-wheel zoom
    come free (01 contract "Router rules" bullet).
- `src/services/desktop_actions.rs` (in-repo, already shipped): `pick_file`
  (`desktop_actions.rs:193-201`), `save_file` (`:213-221`),
  `DesktopActionsSnapshot::picker_available` (`:42`, set from
  `!cfg!(target_family = "wasm")` at `:83`).
- `src/services/undo_stack.rs`: examined and **not** reused (justification in
  Architecture).
- `image` crate 0.25.10 — already in `Cargo.lock:4471-4473` via gpui-pre;
  currently declared only in the Linux (`Cargo.toml:266`) and Windows
  (`Cargo.toml:278`) target tables. This plan adds it to the common table.

## Research Digest

Verified against crate sources on disk
(`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`; short names
`GPUI` = `gpui-pre-0.3.8/src`, `GB` = `gpui-base-0.7.1/src`,
`GC` = `gpui-component-0.7.1/src`, `IMG` = `image-0.25.10/src`) and the repo.

Image pipeline (CPU buffer → screen):

- `RenderImage::new(data: impl Into<SmallVec<[Frame; 1]>>)` (`GPUI/assets.rs:61-70`);
  the type doc states data is **BGRA** (`:40`). `image::Frame` is imported
  privately there (`:3`), so it is NOT re-exported through the gpui/gpui_kit
  glob — the editor imports `image::Frame` from its own `image` dep. Same
  version in the lock (0.25.10, single copy), so the types unify.
- `impl<A: Array> From<Vec<A::Item>> for SmallVec<A>` exists
  (`smallvec-1.15.2/src/lib.rs:2063`), so
  `RenderImage::new(vec![Frame::new(buf)])` compiles without naming `SmallVec`.
- `Frame::new(buffer: RgbaImage)` (`IMG/animation.rs:88`).
- `ImageBuffer<Rgba<u8>>` (aka `RgbaImage`): `from_raw(w, h, Container)` returns
  `Option<Self>` (`IMG/images/buffer.rs:680`); **`from_raw_bgra`** swaps BGR→RGB
  on construction and **`into_raw_bgra`** returns the buffer as BGRA bytes
  (`:1497-1511`). RGBA→BGRA is therefore one call:
  `let bgra = rgba.into_raw_bgra();` then
  `RgbaImage::from_raw(w, h, bgra)` for the `Frame` (channel mislabel is fine;
  `RenderImage` reads raw bytes, and `paint_image` tiles through the sprite
  atlas, `GPUI/window.rs:4919` per plan 01).

image crate ops (all on `DynamicImage`, `IMG/images/dynimage.rs`):
`to_rgba8() -> RgbaImage` `:304`, `crop_imm(x: u32, y: u32, w: u32, h: u32)` `:508`,
`grayscale()` `:831`, `blur(sigma: f32)` `:980`, `brighten(value: i32)` `:1072`,
`rotate90()` (clockwise) `:1116`, `write_to<W: Write + Seek>(&self, W, ImageFormat)` `:1371`.
`ImageReader::open(path) -> io::Result<Self>` (`IMG/io/image_reader_type.rs:344`)
+ `decode() -> ImageResult<DynamicImage>` (`:311`); in-memory decode is
`image::load_from_memory(buffer: &[u8]) -> ImageResult<DynamicImage>`
(`IMG/images/dynimage.rs:1676-1680`, itself a `Cursor` wrapper over
`ImageReader::with_guessed_format().decode()`). Features: `default =
["rayon", "default-formats"]` (`IMG/../Cargo.toml:73-76`); `png = ["dep:png"]`,
`jpeg = ["dep:zune-core", "dep:zune-jpeg"]` — both pure Rust, no C, so the
crate compiles on wasm32-unknown-unknown with `default-features = false` (same
shape the repo already uses for the Linux/Windows icon-decode lines).

Async/off-thread decode and filter work:

- `Context<T>::spawn(AsyncFnOnce(WeakEntity<T>, &mut AsyncApp))`
  (`GPUI/app/context.rs:230-238`); `AsyncApp::background_spawn(future) -> Task<R>`
  trait method (`GPUI/app/async_context.rs:131`). In-repo precedent:
  `src/gallery/sections/tables/data_table.rs:1100-1110` (spawn → background_spawn
  → `this.update(cx, ..)`).

Kit component APIs (verified signatures):

- `SliderState::new()` then `.min(f32)`, `.max(f32)`, `.step(f32)` builders;
  `set_value(impl Into<SliderValue>, &mut Window, &mut Context<Self>)`;
  `value() -> SliderValue`; `SliderEvent::{Change(SliderValue),
  Release(SliderValue)}`; `SliderValue::Single(f32)`
  (`GB/slider.rs:170-260, 273-290, 15-35`). Component side:
  `Slider::new(&Entity<SliderState>)` (`GC/slider.rs:96`), re-exporting the
  state types (`GC/slider.rs:4`).
- `ColorPickerState::new(&mut Window, &mut Context<Self>)`; `value() ->
  Option<Hsla>`; `set_value(impl Into<Hsla>, ..)` without emitting;
  `ColorPickerEvent::Change(Option<Hsla>)` (`GB/color_picker.rs:194, 262,
  287, 92-95`). Component side: `ColorPicker::new(&Entity<ColorPickerState>)`,
  `.label(..)` (`GC/color_picker.rs:73, 110` area).
- `Switch::new(id)`, `.checked(bool)`, `.label(..)`, `.on_click(Fn(&bool, &mut
  Window, &mut App))` (controlled value; owner writes it back) (`GC/switch.rs:56, 62, 79-84`).
- `Button`: `outline()` `GC/button/button.rs:332`, `label()` `:371`, `icon()`
  `:397`, `on_click()` `:447`, `accessibility_label()` `:391`, `tooltip()` `:403`;
  impls `Disableable` `:524`, `Selectable` `:542`, `Sizable` `:562`.
- Viewport discovery for `zoom_to_fit`: `gpui_kit::base::ElementExt as _` +
  `.on_prepaint(move |bounds, _, cx| ..)` resolves an element's bounds during
  prepaint via a nested `canvas()` child (`GB/element_ext.rs:16-27`). In-repo
  precedents: `src/gallery/sections/charts/fps.rs:196`,
  `src/gallery/sections/menus/native_menu.rs:189`; plan 06 relies on the same
  technique (`docs/plans/06-infinite-canvas.md:82`).
- Icons: `image.svg`, `brush.svg`, `eraser.svg`, `square.svg`, `circle.svg`,
  `move-up-right.svg`, `crop.svg`, `rotate-cw.svg`, `undo-2.svg`, `redo-2.svg`,
  `save.svg`, `folder-open.svg` all exist under
  `gpui-kit-assets-0.7.1/assets/icons/` (checked with `ls` this session;
  kebab→PascalCase rule per seams survey Open Questions).

Misc gpui facts:

- Border-side methods are macro-generated from the physical prefix list only —
  `border`, `border_t`, `border_b`, `border_r`, `border_l`, `border_x`,
  `border_y` (`gpui-pre-macros-0.3.8/src/styles.rs:1278-1330`,
  `border_prefixes()`; registry-wide `grep -rn 'border_e\|border_s'` over
  gpui-pre-0.3.8/src returns only unrelated `border_style` hits, run this
  session). This snapshot has no logical start/end sides: the tool rail uses
  `border_r_1()`, the filter panel `border_l_1()`.
- `impl From<Hsla> for Rgba` (`GPUI/color.rs:194`) — brush color → export RGBA.
- `Bounds::intersects(&other) -> bool` (`GPUI/geometry.rs:965`),
  `intersect(&other) -> Self` (`:1143`).
- `cx.listener` page callbacks: `src/features/pages/home.rs:67`
  (`.on_click(cx.listener(|this, event, window, cx| ..))` — the context is the
  4th param and is `&mut Context<Self>`), `error_playground/sections.rs:74`.
  Passing that `cx` to a `&mut App` function such as `pick_file` compiles via
  deref coercion: `Context<T>: Deref/DerefMut<Target = App>`
  (`GPUI/app/context.rs:26-37`).
- Page/error patterns: `notifications.rs:12-25` (page ctor), a11y heading with
  `a11y_live` (`:50-53`); `error_surface::report` for surfaced failures
  (`desktop_actions.rs:414-426`); `window.push_notification(msg, cx)` precedent
  `src/gallery/sections/tables/tree.rs:90` (not used — inline save status is
  chosen instead, see Design Compliance).

Undo decision input: `src/services/undo_stack.rs` is an **app-global**
(`impl Global for UndoState`, `undo_stack.rs:28`) holding one closed enum
`UndoKind::ThemeMode` (`:16-18`) applied through
`crate::app::set_theme_mode_with_record` (`:110-124`). Record/apply is global
with an `applying` re-entrancy guard (`:72-108`). A separate `undo = "0.52.0"`
crate is declared at `Cargo.toml:206` but has zero call sites
(`grep -rn 'undo::\|use undo' src/ tests/` → no hits, run this session).

Plan 01 status: `src/ui/canvas/` does not exist yet (`ls src/ui/` → `mod.rs`,
`widgets/` only). This plan consumes the 01 contract verbatim and is blocked on
01's implementation landing first.

No cargo gate was run during planning (doc-only scope, same as the survey);
Verification lists the exact commands for the implementer.

## Architecture

Two entities plus a thin page:

- `Entity<CanvasState>` (plan 01 engine) owns the scene: layer 0 = base image
  (`CanvasShape::Image`, bounds `(0,0,w,h)` in doc px), layer 1 = annotations
  (`CanvasShape::Path/Rect/Ellipse`). Document space == image pixel space
  (1 doc px = 1 image px at zoom 1.0); every pixel op (rotate/crop) keeps that
  invariant by translating/rotating annotations to match.
- `Entity<ImageEditorDocument>` owns the data: current base pixels
  (`Arc<RgbaImage>`, unfiltered), `FilterSettings`, the derived display image
  (`Arc<RenderImage>` cache + staleness-guarded request counter), the source
  path, the pending crop rect, layer/shape ids, and the per-document history.
  It never touches a window.
- `ImageEditorPage` owns both entities, the tool rail state, the kit input
  states (sliders/color picker), the canvas-area viewport probe, and the
  save status. It observes both entities (`cx.observe`) and notifies itself —
  plan 01's redraw contract.

Event flow:

1. User gesture → kit control callback (`cx.listener`) or engine pointer event
   → editor tool (dispatched by 01's router inside
   `Entity::<CanvasState>::update`).
2. Tools mutate annotations through the `&mut CanvasState` param (ordinary
   engine mutators) and record history by updating the *different*
   `Entity<ImageEditorDocument>` from inside the dispatch
   (`self.doc.update(cx, ..)` — re-leasing a different entity is legal; only
   same-entity re-lease panics, `GPUI/app/entity_map.rs:160-165` per plan 01).
3. `ImageEditorDocument` emits `ImageEditorEvent` and notifies; the page
   observer applies follow-ups: `DisplayChanged` → `update_shape` the base
   image shape (one shape, one revision bump), `ImageOpened` → `zoom_to_fit`,
   `HistoryChanged`/`CropRectChanged` → toolbar state refresh.
4. Filter/pixel derivations run on the background executor
   (`Context::spawn` + `background_spawn`); completion is guarded by the
   document's `display_request` counter (stale results are dropped).

Undo/redo decision — **per-document stack, new `history.rs`**; reuse of
`src/services/undo_stack.rs` is rejected because (a) it is an app-global with
one closed app-level variant (`UndoKind::ThemeMode`, `undo_stack.rs:16-18`) —
image history needs per-document identity and megabyte pixel payloads that must
die with the document; (b) its apply path dispatches through app-level theme
helpers (`:110-124`), the wrong layer for document mutation; (c) interleaving
theme switches and image edits in one past/future list breaks both histories'
ordering. The unused `undo 0.52` crate is also not adopted (zero precedent call
sites; a snapshot-swap stack is ~80 lines and needs no history-tree features).
Undo model: **whole-state snapshots**. Every history entry stores `before` and
`after` snapshots of `{base: Arc<RgbaImage>, filters, annotations}`; apply =
swap fields + rebuild annotation layer + request display re-derivation.
Consecutive entries share `Arc` buffers, so cost is bounded by distinct bases
(see budget below).

Dirty-region discipline (the spec's "only damaged regions repaint"):
annotations edits add/remove/update single shapes through `edit_document`
(one revision bump, cache keys of untouched shapes unchanged — 01's
`shape_key` hashes content, so untouched entries keep their tessellation
cache); filter changes replace exactly the base `CanvasShape::Image`; pan/zoom
never bump the document revision; engine culling skips off-screen shapes
(01 Architecture, "Invalidation").

Module tree:

```text
src/features/pages/image_editor/
├── mod.rs         page seam: ImageEditorPage, render tree (toolbar, tool rail,
│                  canvas area + viewport probe + empty state, filter panel),
│                  removal-recipe header
├── document.rs    ImageEditorDocument entity + ImageEditorEvent + FilterSettings:
│                  open/rotate/crop/set_filters ops, display re-derivation task,
│                  history apply, snapshot capture
├── history.rs     EditOp, DocSnapshot, EditLabel, EditorHistory (pure), budget
├── tools.rs       BrushTool, EraserTool, RectTool, EllipseTool, ArrowTool,
│                  CropTool (impl CanvasTool), arrow_segments, bounds_from_corners
├── bake.rs        pure rasterizers (stamp_line, fill_circle, stroke_rect,
│                  stroke_ellipse) + apply_filters + compose_export
└── io.rs          native-only load_image(bytes) / save_image(path, img) [cfg not wasm]
```

Each code module gets a sibling `*.test.rs` wired
`#[cfg(test)] #[path = "..."] mod ..._test;`.

## Interface Contract

This is a leaf feature; nothing downstream consumes it. The public surface is
exactly what the page seam exports:

```rust
// src/features/pages/mod.rs
pub use image_editor::ImageEditorPage;

// ImageEditorPage
impl ImageEditorPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self;
}
impl Render for ImageEditorPage { /* toolbar + rail + canvas + filter panel */ }
```

Everything inside `image_editor/` is `pub(crate)` at most:

```rust
// document.rs
#[derive(Clone, Copy, Debug, PartialEq, Default)]  // Default: all off (false, 0., 0)
pub(crate) struct FilterSettings { /* grayscale: bool, blur_sigma: f32, brightness: i32 */ }pub(crate) enum ImageEditorEvent { ImageOpened, DisplayChanged, HistoryChanged, CropRectChanged }
pub(crate) struct ImageEditorDocument { /* private fields */ }
impl EventEmitter<ImageEditorEvent> for ImageEditorDocument {}
impl ImageEditorDocument {
    pub(crate) fn new() -> Self;                                        // no image, empty history
    pub(crate) fn is_loading(&self) -> bool;
    pub(crate) fn has_image(&self) -> bool;
    pub(crate) fn filters(&self) -> FilterSettings;
    pub(crate) fn crop_rect(&self) -> Option<Bounds<Pixels>>;           // doc px
    pub(crate) fn can_undo(&self) -> bool;
    pub(crate) fn can_redo(&self) -> bool;
    pub(crate) fn history_len(&self) -> usize;
    pub(crate) fn display(&self) -> Option<&Arc<RenderImage>>;
    pub(crate) fn base_size(&self) -> Option<(u32, u32)>;

    /// Decodes off-thread from raw bytes; on success resets the document
    /// (base, history, filters), inserts layer 0 + layer 1, requests the
    /// derived display, emits ImageOpened. Decode failure lands in
    /// `last_error` (page renders it inline).
    pub(crate) fn open(&mut self, bytes: Vec<u8>, canvas: &Entity<CanvasState>, cx: &mut Context<Self>);
    pub(crate) fn last_error(&self) -> Option<&str>;
    /// Records + applies a filter change (Filters entry).
    pub(crate) fn set_filters(&mut self, filters: FilterSettings, cx: &mut Context<Self>);
    /// Records + applies rotate90 (pixels + annotations).
    pub(crate) fn rotate90(&mut self, canvas: &Entity<CanvasState>, cx: &mut Context<Self>);
    /// Records + applies the pending crop_rect (pixels + annotations).
    /// No-op returning false when the rect is missing or under MIN_CROP_PX.
    pub(crate) fn apply_crop(&mut self, canvas: &Entity<CanvasState>, cx: &mut Context<Self>) -> bool;
    pub(crate) fn set_crop_rect(&mut self, rect: Option<Bounds<Pixels>>, cx: &mut Context<Self>); // emits CropRectChanged
    pub(crate) fn has_valid_crop(&self) -> bool;
    pub(crate) fn annotation_layer(&self) -> Option<LayerId>;
    /// Records a full-annotation-layer snapshot swap; used by tools on commit.
    /// `after` is captured by the CALLER from the `&mut CanvasState` dispatch
    /// param before entering `doc.update` — reading the canvas entity from
    /// inside its own update lease panics (`double_lease_panic("read", ...)`,
    /// `GPUI/app/entity_map.rs:151-157`), so this signature takes data, not
    /// the canvas handle.
    pub(crate) fn record_annotate(&mut self, label: EditLabel, before: AnnotationSnapshot, after: AnnotationSnapshot, cx: &mut Context<Self>);
    pub(crate) fn undo(&mut self, canvas: &Entity<CanvasState>, cx: &mut Context<Self>) -> bool;
    pub(crate) fn redo(&mut self, canvas: &Entity<CanvasState>, cx: &mut Context<Self>) -> bool;
    /// Test seam: apply a derived display synchronously (skips the executor).
    pub(crate) fn set_display_for_test(&mut self, display: Arc<RenderImage>, cx: &mut Context<Self>);
}

// history.rs
pub(crate) type AnnotationSnapshot = Vec<(ShapeId, CanvasShape)>;       // one full annotation layer
pub(crate) struct DocSnapshot { /* base: Arc<RgbaImage>, filters: FilterSettings, annotations: AnnotationSnapshot */ }
pub(crate) enum EditLabel { Stroke, Erase, Rotate, Crop, Filters }
pub(crate) struct EditOp { /* label: EditLabel, before: DocSnapshot, after: DocSnapshot */ }
pub(crate) struct EditorHistory { /* past, future: Vec<EditOp>, private */ }
impl EditorHistory {
    pub(crate) fn new() -> Self;
    pub(crate) fn record(&mut self, op: EditOp);     // clears future, enforces budget
    pub(crate) fn undo(&mut self) -> Option<&EditOp>; // moves to future
    pub(crate) fn redo(&mut self) -> Option<&EditOp>; // moves to past
    pub(crate) fn clear(&mut self);
    pub(crate) fn can_undo(&self) -> bool;
    pub(crate) fn can_redo(&self) -> bool;
    pub(crate) fn len(&self) -> usize;
}
pub(crate) const HISTORY_ENTRY_LIMIT: usize = 40;
pub(crate) const HISTORY_PIXEL_BUDGET: usize = 256 * 1024 * 1024; // distinct base buffers across entries

// tools.rs — all impl plan 01's `CanvasTool`; constructed by the page with
// (Entity<ImageEditorDocument>, Entity<CanvasState>, annotation LayerId,
// StrokeStyle) captured. The editor-local BrushTool replaces 01's builtin
// brush: same drag behavior, but its commit routes through record_annotate.
pub(crate) struct BrushTool;    // id "image-brush",   Crosshair
pub(crate) struct EraserTool;   // id "image-eraser",  Crosshair
pub(crate) struct RectTool;     // id "image-rect",    Crosshair
pub(crate) struct EllipseTool;  // id "image-ellipse", Crosshair
pub(crate) struct ArrowTool;    // id "image-arrow",   Crosshair
pub(crate) struct CropTool;     // id "image-crop",    Crosshair
/// Arrow shaft + head segments in doc px; shared by display shapes and bake.
pub(crate) fn arrow_segments(from: Point<Pixels>, to: Point<Pixels>) -> [(Point<Pixels>, Point<Pixels>); 3];
pub(crate) fn bounds_from_corners(a: Point<Pixels>, b: Point<Pixels>) -> Bounds<Pixels>;

// bake.rs — pure, no gpui types beyond geometry
pub(crate) fn apply_filters(base: &RgbaImage, filters: &FilterSettings) -> RgbaImage; // grayscale → brighten → blur
pub(crate) fn fill_circle(buf: &mut RgbaImage, center: (f32, f32), radius: f32, rgba: [u8; 4]);
pub(crate) fn stamp_line(buf: &mut RgbaImage, a: (f32, f32), b: (f32, f32), radius: f32, rgba: [u8; 4]);
pub(crate) fn stroke_ellipse(buf: &mut RgbaImage, center: (f32, f32), rx: f32, ry: f32, radius: f32, rgba: [u8; 4]);
pub(crate) fn rasterize_shape(buf: &mut RgbaImage, shape: &CanvasShape, rgba: [u8; 4], width: f32);
pub(crate) fn compose_export(base: &RgbaImage, filters: &FilterSettings, annotations: &[CanvasShape]) -> RgbaImage;

// io.rs — #[cfg(not(target_family = "wasm"))]
pub(crate) fn load_image(path: &Path) -> anyhow::Result<RgbaImage>;
pub(crate) fn save_image(path: &Path, img: &RgbaImage) -> anyhow::Result<()>;
```

## Module Layout

| File | Purpose |
|---|---|
| `src/features/pages/image_editor/mod.rs` | Page seam + removal header; render tree; toolbar/rail/filter-panel handlers; viewport probe; empty state; save status |
| `src/features/pages/image_editor/document.rs` | `ImageEditorDocument` entity: open/rotate/crop/set_filters, background display derivation with staleness guard, history apply, layer bookkeeping |
| `src/features/pages/image_editor/history.rs` | Snapshot-based `EditorHistory`, `EditOp`, `DocSnapshot`, budget enforcement (pure) |
| `src/features/pages/image_editor/tools.rs` | Editor tools implementing 01's `CanvasTool`; arrow geometry; corner→bounds helper |
| `src/features/pages/image_editor/bake.rs` | Filter pipeline + annotation rasterization + export compose (pure CPU) |
| `src/features/pages/image_editor/io.rs` | Native file decode/encode only |
| `src/features/pages/image_editor/*.test.rs` | Sibling test modules per repo convention |

## Seam Touches

All edits outside `src/features/pages/image_editor/` (survey §1 page seam):

| # | File | Change |
|---|---|---|
| 1 | `Cargo.toml` | Common `[dependencies]`: `image = { version = "0.25", default-features = false, features = ["png", "jpeg"] }`. Unifies with the existing Linux/Windows lines (`Cargo.toml:266,278`) to the union; macOS and wasm gain the dep. Regenerate and stage `Cargo.lock` (qa_migration checks the lock blob, `tests/qa_migration.rs:167-186`). |
| 2 | `src/features/pages/mod.rs` | `mod image_editor;` + `pub use image_editor::ImageEditorPage;` |
| 3 | `src/shell/sidebar.rs` | `Page::ImageEditor` variant + arms in `title` (`"Image Editor"`), `title_key` (`nav_image_editor`), `host` (`image-editor`), `icon` (`IconName::Frame` — verified default; the full-catalog `Image` renders blank on native because the app embeds only the default 106), and **append** to `all()` (position 11 — beyond the 9 digit bindings, so no key rebinding; survey §1 gotcha) |
| 4 | `src/shell/route.rs` | `VALID_HOSTS` entry `Page::ImageEditor.host(),` |
| 5 | `src/shell/root/app_root/state.rs` | `image_editor_page: Entity<ImageEditorPage>` field, construct in `AppRoot::new` (`cx.new(|cx| ImageEditorPage::new(window, cx))`), import, `unchecked_active_page_view` arm |
| 6 | `tests/e2e_navigation.rs` | Page count `10` → `11` (`:16-21`) + add `Page::ImageEditor` to the `expected` array |
| 7 | `locales/en.yml`, `locales/zh-CN.yml` | `nav_image_editor` + all `image_editor_*` keys from the i18n table, both files in lockstep |
| 8 | `src/app/init.rs` | Capability entry: `capabilities::set("image_editor", …)` — `supported_enabled` native, `degraded(localize("image_editor_no_picker"))` on wasm, placed AFTER `set_locale` (`init.rs:131`; the capabilities block at `:91-115` runs before the locale is applied, so a reason string captured there freezes in the default locale — set this one entry after `set_locale`, where localization is live) |

Deliberate non-touches: no command-palette entries (`services/commands.rs`), no
sub-routes, no gallery section, no new OS-specific deps, no tray/shortcut work.

### Removal Recipe

`src/features/pages/image_editor/mod.rs` opens with (mirroring
`src/gallery/mod.rs:1-7`):

```rust
//! Removable image editor page. `ImageEditorPage` is the only public surface
//! via `views::ImageEditorPage`; everything else stays private to the module.
//! Removal = delete this directory and revert its seam touches: the `image`
//! dependency in Cargo.toml (+ Cargo.lock), the `mod image_editor;`/`pub use`
//! lines in features/pages/mod.rs, the Page::ImageEditor arms and all() entry
//! in shell/sidebar.rs, the VALID_HOSTS entry in shell/route.rs, the AppRoot
//! wiring in shell/root/app_root/state.rs, the page count in
//! tests/e2e_navigation.rs, the image_editor capability in app/init.rs, and
//! the nav_image_editor + image_editor_* keys in locales/en.yml and
//! locales/zh-CN.yml.
```

Ordered steps:

1. Revert `src/features/pages/mod.rs` (drop `mod image_editor;` + `pub use`).
2. Revert `src/shell/root/app_root/state.rs` (field, ctor, import, view arm).
3. Revert `src/shell/sidebar.rs` (variant + five arms + `all()` entry) and
   `src/shell/route.rs` (`VALID_HOSTS`).
4. Revert `tests/e2e_navigation.rs` count to 10 and drop the expected entry.
5. Revert `src/app/init.rs` capability entry.
6. Delete the `image_editor_*`/`nav_image_editor` keys from both locale files.
7. Remove the `image` line from the common `[dependencies]` table and
   regenerate `Cargo.lock` (the Linux/Windows icon lines stay).
8. `rm -r src/features/pages/image_editor/`.
9. `grep -rn "image_editor\|ImageEditor" src/ tests/ locales/` → zero hits;
   run the Verification gates.

## Implementation Phases

## Phase 1: Dependency and page scaffold

### Step 1.1: `image` dependency

- Files: `Cargo.toml`, `Cargo.lock` (regenerated).
- Build: add `image = { version = "0.25", default-features = false, features = ["png", "jpeg"] }` to the common `[dependencies]` table (after `rust-embed = "8"`, `Cargo.toml:210`); stage the regenerated lock with the change.
- Tests: none.
- Done-check: `cargo check -p gpui-starter` passes and
  `grep -n 'features = \["png", "jpeg"\]' Cargo.toml` prints the new line.

### Step 1.2: Page module + all seams

- Files: `src/features/pages/image_editor/mod.rs` (header comment per Removal
  Recipe; `ImageEditorPage { subscriptions: Vec<Subscription> }`;
  `new(window, cx)` no-op subscriptions for now; `render` returns the empty
  state: `v_flex().min_h_full().p_6().gap_4()` with an
  `a11y(Role::Heading, …).aria_level(1)` title using
  `localize("image_editor_title")` — notifications.rs:44-53 pattern — and a
  muted hint line), plus seam touches 2–8 from Seam Touches.
- Build: page arms, route host, `AppRoot` wiring, locale keys (i18n table
  below), capability entry in `init.rs`
  (`#[cfg(target_family = "wasm")]` → `CapabilityStatus::degraded(localize("image_editor_no_picker"))`,
  else `supported_enabled()`; the entry is set after `set_locale` —
  `init.rs:131` — because the capabilities block at `:91-115` runs before the
  persisted locale is applied and would freeze the reason in the default
  locale), e2e count bump.
- Tests: `tests/e2e_navigation.rs` (existing, now expecting 11) and
  `tests/qa_localization.rs` (existing) must stay green.
- Done-check: `cargo test --test e2e_navigation --test qa_localization` passes
  and `just wasm-check` passes (page compiles on wasm).

## Phase 2: Pure model — history and bake

### Step 2.1: `history.rs`

- Files: `src/features/pages/image_editor/history.rs`, `history.test.rs`.
- Build: `AnnotationSnapshot` alias; `DocSnapshot { base: Arc<RgbaImage>,
  filters: FilterSettings, annotations: AnnotationSnapshot }` (pub(crate)
  fields — record-like, changes with every entry); `EditLabel`; `EditOp`;
  `EditorHistory` with `record` (push, clear future, then enforce budget:
  while `past.len() > HISTORY_ENTRY_LIMIT` or the summed bytes of distinct
  `Arc` base buffers (dedupe by `Arc::as_ptr`) exceed `HISTORY_PIXEL_BUDGET`,
  drop `past[0]`; byte size = `w * h * 4` via `RgbaImage` dims),
  `undo`/`redo` (pop-and-transfer returning the entry by value), `clear`,
  readers.
- Tests: record→undo→redo round-trip restores entries; recording after undo
  clears the future; budget: with a fake limit (build entries with small
  `RgbaImage`s and a `#[cfg(test)]`-visible budget override or a
  `record_with_budget(op, limit)` internal used by tests) the oldest entry is
  dropped when the pixel budget is exceeded; duplicate `Arc`s count once;
  `HISTORY_ENTRY_LIMIT` enforced.
- Done-check: `cargo test -p gpui-starter --lib image_editor::history` passes.

### Step 2.2: `bake.rs` filters + rasterizers

- Files: `src/features/pages/image_editor/bake.rs`, `bake.test.rs`.
- Build:
  - `apply_filters`: `let mut img = DynamicImage::ImageRgba8(base.clone());`
    then in fixed order `if filters.grayscale { img = img.grayscale(); }`,
    `if filters.brightness != 0 { img = img.brighten(filters.brightness); }`,
    `if filters.blur_sigma > 0. { img = img.blur(filters.blur_sigma); }`,
    `img.to_rgba8()` (IMG/images/dynimage.rs:831, 980, 1072).
  - `fill_circle`: iterate the circle's bounding box rows/columns
    (`center ± radius` clamped to buffer), set pixels whose center distance
    `<= radius` using source-over alpha blend
    (`out = src*sa + dst*(1-sa)` per channel, 8-bit).
  - `stamp_line`: step along a→b in increments of `radius.max(1.) / 2.` px,
    `fill_circle` at each step (round caps/joins for free).
  - `stroke_ellipse`: `n = ceil(TAU * rx.max(ry)).max(12)` parametric samples,
    `fill_circle` at each.
  - `rasterize_shape` (match on `CanvasShape`): `Path` → `stamp_line` over
    consecutive points, and a single `fill_circle` at the point when the path
    has exactly one point (a tap-dot: the engine paints 1-point paths as a
    filled dot, bake must match); `Rect` → 4 `stamp_line`s over the bounds
    edges; `Ellipse` → `stroke_ellipse`; `Text`/`Image` → skip (editor never
    creates them; debug_assert in tests).
  - `compose_export`: `apply_filters(base, filters)` then
    `rasterize_shape` for every annotation in layer order.
- Tests: `apply_filters` identity (all off) returns equal pixels; grayscale
  of a 2×1 red/green image makes each pixel R==G==B (sRGB luma weights
  2126/7152/722 per `image-0.25.10/src/color.rs:606-607` mean red ≈ 54 and
  green ≈ 182 — the two pixels are NOT equal; assert grayness per pixel, not
  pixel equality); `brighten(10)` raises
  every channel by 10 with saturation at 255; `blur(0.)` is identity; blur of
  a single white pixel in black spreads mass (non-zero neighbors);
  `fill_circle` radius 2 at (4,4) in 9×9 black sets exactly the 13 pixels with
  distance ≤ 2; `stamp_line` (0,0)→(8,0) radius 1 covers y∈0..=1 for x∈0..=8
  (allow off-by-one at ends); `stroke_ellipse` on a 9×9 buffer with rx=ry=4
  leaves the center untouched and sets the four axis extremes; a 1-point
  `Path` at (4,4) width 2 darkens exactly the pixel (4,4) and its radius-1
  neighborhood.
- Done-check: `cargo test -p gpui-starter --lib image_editor::bake` passes.

## Phase 3: Document entity

### Step 3.1: `document.rs` core

- Files: `src/features/pages/image_editor/document.rs`, `document.test.rs`;
  `mod.rs` gains `mod document; mod history; mod bake; mod tools; mod io;`
  decls as they land (tools/io in later phases).
- Build: `FilterSettings` (derives `Clone, Copy, Debug, PartialEq, Default` —
  equality gates `set_filters`, snapshots store it by value);
  `ImageEditorEvent`; `ImageEditorDocument` fields
  (`base: Option<Arc<RgbaImage>>, filters, display: Option<Arc<RenderImage>>,
  display_request: u64, source_path, crop_rect,
  history: EditorHistory, loading: bool, base_layer: Option<LayerId>,
  annotation_layer: Option<LayerId>, base_shape: Option<ShapeId>,
  _derive: Option<Task<()>>`); `EventEmitter` impl; `new()`; readers from the
  Interface Contract; `rgba_to_render(img: &RgbaImage) -> Arc<RenderImage>` —
  `let (w, h) = img.dimensions(); let bgra = img.clone().into_raw_bgra();`
  `RgbaImage::from_raw(w, h, bgra)` → `Frame::new` →
  `RenderImage::new(vec![frame])` (buffer.rs:1505-1511, assets.rs:61-70);
  `request_display(&mut self, cx)` — bumps `display_request`, spawns
  `cx.spawn(async move |this, cx| { let req = …; let base = …clone…; let f = …clone…;
  let derived = cx.background_spawn(async move { apply_filters(&base, &f) }).await;
  this.update(cx, |doc, cx| { if doc.display_request != req { return; }
  doc.display = Some(rgba_to_render(&derived));
  cx.emit(ImageEditorEvent::DisplayChanged); cx.notify(); }) })`, store the
  task in `_derive` (keeps it alive; a newer request supersedes by counter);
  `set_display_for_test` (skips spawn: build directly, emit, notify).
- Tests (`#[gpui_kit::test]`): `rgba_to_render` round-trips size
  (`display().size(0)` equals base dims, `RenderImage::size` at
  `GPUI/assets.rs:81`) and byte length (`as_bytes(0)` == w*h*4);
  `request_display` via `set_display_for_test` emits exactly one
  `DisplayChanged`.
- Done-check: `cargo test -p gpui-starter --lib image_editor::document` passes.

### Step 3.2: Open, set_filters, rotate, crop, undo/redo apply

- Files: same.
- Build (all methods take `&Entity<CanvasState>` where annotations change):
  - `open(bytes, canvas, cx)`: `loading = true; last_error = None; cx.notify()`;
    spawn `cx.spawn(async move |this, cx| { let decoded =
    cx.background_spawn(async move { image::load_from_memory(&bytes)
    .map(|d| d.to_rgba8()) }).await;
    this.update(cx, |doc, cx| { doc.loading = false; match decoded {
    Ok(img) => { /* apply, see below */ … cx.emit(ImageOpened); }
    Err(e) => { doc.last_error = Some(e.to_string()); cx.notify(); } } }) })`
    (`load_from_memory` verified at `IMG/images/dynimage.rs:1676-1680`).
    Apply on success: reset `filters` to
    default, `history.clear()`, `crop_rect = None`, `base = Some(Arc::new(img))`,
    then `canvas.update(cx, |s, cx| s.edit_document(cx, |d| { d.clear();
    base_layer = d.add_layer(); annotation_layer = d.add_layer();
    base_shape = d.add_shape(base_layer, CanvasShape::image(Bounds { origin:
    Point::default(), size: size(px(w as f32), px(h as f32)) }, placeholder) ) }))`
    with `placeholder = rgba_to_render(&unfiltered)` (display swaps in when the
    derive task lands), `request_display(cx)`, `cx.emit(ImageOpened)`.
  - `set_filters(next, cx)`: if `next == self.filters` return; snapshot
    before; set; snapshot after; `history.record(EditOp { label: Filters,
    before, after })`; `request_display(cx)`; emit `HistoryChanged`.
  - `rotate90(canvas, cx)`: snapshot before (annotations cloned from
    `canvas.read(cx).document()`); compute `after_base =
    DynamicImage::ImageRgba8((*base).clone()).rotate90().to_rgba8()`; rotate
    annotations: for each shape, rotate geometry by the clockwise mapping
    `(x, y) -> (H - y, x)` (old H = height; the continuous form — a pixel at
    index `(x, y)` lands at index `(H - 1 - y, x)` in the rotated buffer, and
    region bounds rotate without the off-by-one the discrete form would
    introduce) — `Path` points individually;
    `Rect`/`Ellipse` via the rotated corners' bounding box; swap `base`,
    rebuild annotation layer (remove old ids, add rotated shapes), swap
    filters unchanged; record entry labeled Rotate; `request_display`;
    emit `HistoryChanged`.
  - `apply_crop(canvas, cx) -> bool`: rect = `crop_rect` valid and ≥
    `MIN_CROP_PX` (named const `pub(crate) const MIN_CROP_PX: Pixels = px(8.);`
    declared in `document.rs`, minimum crop size in doc px per axis) else
    return false; integer
    rect = floor/ceil corners clamped to image bounds; snapshot before;
    `after_base = DynamicImage::ImageRgba8(base.clone()).crop_imm(x, y, w,
    h).to_rgba8()`; annotations: keep shapes whose `doc_bounds().intersect
    (&crop_bounds)` is non-empty (geometry.rs:1143), `translate(-origin)` each
    survivor (plan 01 `CanvasShape::translate`), drop the rest; rebuild
    annotation layer; `crop_rect = None`; record entry labeled Crop;
    `request_display`; emit `HistoryChanged` + `CropRectChanged`; true.
  - `undo`/`redo(canvas, cx)`: pop entry via history; apply the `before`
    (`undo`) or `after` (`redo`) snapshot: `base = snap.base.clone()`,
    `filters = snap.filters.clone()`, rebuild annotation layer from
    `snap.annotations`, `crop_rect = None` on pixel-op labels;
    `request_display(cx)` when `base` or `filters` changed vs current; emit
    `HistoryChanged` (+ `DisplayChanged` comes from the derive task);
    return whether anything was applied.
  - `record_annotate(label, before, after, cx)` (signature as the Interface
    Contract declares — `after` arrives as data, never read from the canvas
    entity here): `history.record(EditOp { label, before, after })`; emit
    `HistoryChanged`.
- Tests (`#[gpui_kit::test]`, canvas entity created with `cx.new(|_| CanvasState::new())`
  — note: 01 not implemented yet; these tests land in Phase 3 only if 01 is
  merged; otherwise the whole plan is blocked on 01, see Dependencies):
  encode a 16×8 RGBA image to PNG bytes in-test
  (`DynamicImage::ImageRgba8(..).write_to(&mut Cursor::new(&mut Vec::new()),
  ImageFormat::Png)` then `open(bytes, ..)`), assert two layers exist, base
  shape id set,
  `base_size() == (16, 8)`, one `ImageOpened`; `set_filters(grayscale)` records
  one entry, `can_undo()` true, undo restores `filters()` default;
  `rotate90` of a 16×8 base yields `base_size() == (8, 16)` and an annotation
  `Rect` at doc `(0,0)..(2,1)` maps to `(7,0)..(8,2)` (pixel `(x,y) →
  (H-1-y, x)` with H=8); `apply_crop` with rect `(2,1)..(10,7)` (8×6 ≥
  MIN_CROP_PX) yields base 8×6 and translates an annotation at `(3,2)` to
  `(1,1)`; `apply_crop` with a 4×2 rect under MIN_CROP_PX returns false (on a
  16×8 base, valid rects are distinguishable from under-minimum ones);
  undo after crop restores 16×8; eraser-style `record_annotate` with an empty
  `after` snapshot makes `can_redo` true and redo restores the shapes.
- Done-check: `cargo test -p gpui-starter --lib image_editor::document` passes.

## Phase 4: Canvas integration and open flow

### Step 4.1: `io.rs` (native)

- Files: `src/features/pages/image_editor/io.rs` — entire module behind
  `#[cfg(not(target_family = "wasm"))]`, `io.test.rs` likewise cfg'd.
- Build: `load_image(path) -> anyhow::Result<RgbaImage>` =
  `ImageReader::open(path)?.decode().map(|d| d.to_rgba8())` mapped to anyhow;
  `save_image(path, img)`: format from extension (`png` | `jpg`/`jpeg` →
  `ImageFormat::Jpeg`, else `ImageFormat::Png`);
  `DynamicImage::ImageRgba8(img.clone()).write_to(&mut std::fs::File::create(path)?, format)`
  (dynimage.rs:1371; `File` is `Write + Seek`).
- Tests (native-only, plain `#[test]` with `tempfile` — already a dep,
  `Cargo.toml:186`): round-trip save→load PNG of a 3×2 image preserves
  dimensions and pixels; `.jpg` extension encodes without error (dimensions
  only — JPEG is lossy).
- Done-check: `cargo test -p gpui-starter --lib image_editor::io` passes;
  `just wasm-check` still passes (module cfg'd out).

### Step 4.2: Page canvas mount, viewport probe, open flow

- Files: `src/features/pages/image_editor/mod.rs`.
- Build: page fields per Architecture (canvas + doc entities built in `new`
  with `cx.new(|_| CanvasState::new())`, `cx.new(|_| ImageEditorDocument::new())`;
  `viewport: Bounds<Pixels>` default zero; `pending_fit: bool`; tool state
  enum `EditorTool` default `Brush`; kit states:
  `brush_size: Entity<SliderState>` (min 1, max 100, step 1, value 4),
  `eraser_size` (same range, value 12), `blur` (min 0, max 20, step 0.5,
  value 0), `brightness` (min -100, max 100, step 1, value 0),
  `color: Entity<ColorPickerState>` (`ColorPickerState::new(window, cx)` then
  `set_value(default red, window, cx)` without emitting);
  `save_state: SaveState`; `_open_task: Option<Task<()>>`). Subscriptions:
  observe canvas (notify), observe doc (`match` event: `ImageOpened` →
  `pending_fit = true` then `try_fit(cx)`; `DisplayChanged` →
  `self.update_base_shape(cx)`; others → notify).
  - `try_fit`: if `pending_fit && viewport.size.width > px(0.)`:
    `canvas.update(cx, |s, cx| s.zoom_to_fit(Bounds { origin: Point::default(),
    size: size(px(w), px(h)) }, self.viewport.size, cx))`, `pending_fit = false`.
    Viewport is valid because the canvas area is mounted (and laid out) before
    any image can be opened — the Open button lives in the same frame.
  - `update_base_shape`: `doc.read → display + base_shape`; `canvas.update(..
    edit_document(update_shape(base_shape, |s| *s = CanvasShape::image(bounds,
    display.clone()))))` — single-shape swap, dirty-region rule.
  - Render: `v_flex().size_full()` → toolbar (`h_flex().border_b_1().border_color(
    cx.theme().border).px_4().py_2().gap_2()` — Open (folder-open icon), Save As…
    (label ends with `…`), Undo/Redo (icon buttons, `disabled(!can_undo())`),
    Rotate 90°, Apply crop `disabled(!crop_rect_valid)`) → `h_flex().flex_1()
    .min_h_0()` → tool rail (`v_flex().border_r_1().p_2().gap_1()`, six small
    icon Buttons `selected(active)` + `tooltip` + `accessibility_label`) →
    canvas area: `div().id("image-editor-canvas").relative().size_full()
    .child(Canvas::new("image-editor", self.canvas.clone()))` wrapped with
    `.on_prepaint(move |bounds, _, cx| page.update(cx, |p, _| p.store_viewport(bounds)))`
    (change-guarded plain write, no notify — `GB/element_ext.rs:16-27`,
    `native_menu.rs:189` precedent; clone an `Entity<ImageEditorPage>` handle
    into the closure) → when `!has_image`, an overlay empty state
    (`div().absolute().size_full().items_center().justify_center()` with muted
    hint `image_editor_empty_hint`); right filter panel (`v_flex().border_l_1()
    .w_64().p_4().gap_4()` — Switch grayscale, two Sliders with labels,
    all `disabled(!has_image || loading)`).
  - `store_viewport`: `if bounds != self.viewport { self.viewport = bounds; }`
    plus `try_fit` call when `pending_fit` (fit after a window resize while a
    load just finished — keep exact: call `try_fit(cx)` only when a field
    actually changed and `pending_fit` was set).
  - `on_open` (`cx.listener`): on wasm the button renders disabled
    (`snapshot.picker_available == false`, `desktop_actions.rs:83`), so the
    handler is native-only in practice; `pick_file(cx)` (the listener's
    context param is `&mut Context<Self>`; it coerces to `&mut App` for
    `pick_file` via `DerefMut`, `GPUI/app/context.rs:26-37`; `home.rs:67`
    callback shape); `Some(path)` →
    `this._open_task = Some(cx.spawn(async move |this, cx| { let bytes =
    cx.background_spawn(async move { std::fs::read(&path).map_err(|e|
    e.to_string()) }).await; match bytes { Ok(bytes) =>
    this.update(cx, |page, cx| page.doc.update(cx, |d, cx| d.open(bytes,
    &page.canvas, cx))), Err(msg) => /* report as below */ } }) })`. Read
    failures and decode failures (`doc.last_error()` on the next
    `HistoryChanged`-less notify) surface through
    `error_surface::report(localize_with_args("image_editor_open_failed",
    &[("error", msg)]), Warning, System, [Dismiss], cx)` — the
    `desktop_actions.rs:414-426` pattern; the page additionally renders
    `last_error` inline under the toolbar until the next Open.
- Tests: none new (covered by Phase 3 entity tests + manual QA); page render
  smoke via existing e2e lifecycle.
- Done-check: `cargo check -p gpui-starter` passes;
  `grep -n "on_prepaint" src/features/pages/image_editor/mod.rs` prints the
  probe wiring.

## Phase 5: Annotation tools

### Step 5.1: Geometry helpers + shape tools

- Files: `src/features/pages/image_editor/tools.rs`, `tools.test.rs`.
- Build:
  - `bounds_from_corners(a, b)`: min/max per axis.
  - `arrow_segments(from, to)`: shaft `(from, to)`; head length
    `hl = min(len * 0.2, px(32.))` with `hl = hl.max(px(8.))`; head angle 25°;
    returns `[(from, to), (to, to - dir*hl rotated +25°), (to, to - dir*hl rotated -25°)]`
    computed in f32 via `Pixels::as_f32` and rebuilt with `px`.
  - Shared tool shape: every editor tool holds `{ doc:
    Entity<ImageEditorDocument>, canvas: Entity<CanvasState>, layer: LayerId,
    stroke: StrokeStyle }` (constructed by the page from the current color and
    size) plus per-tool drag fields. The tool runs inside the canvas lease and
    receives `&mut CanvasState`, not the entity handle, so it captures both
    handles at construction; the commit path captures the `after` snapshot
    from the `&mut CanvasState` dispatch param BEFORE entering
    `doc.update` — a `canvas.read(cx)` there would hit the entity's own
    update lease and panic (`double_lease_panic("read", ...)`,
    `GPUI/app/entity_map.rs:151-157`), so the tool never reads the canvas
    entity while it is dispatched.
  - `BrushTool { .., points: Vec<Point<Pixels>>, stroking: bool, before:
    Option<AnnotationSnapshot> }` — editor-local replacement for 01's builtin
    brush (the builtin commits via its own `state.add_shape` with no hook, so
    it cannot record editor history; this is why `BuiltinTool` is not
    consumed). Behavior mirrors 01 Step 4.2's `BrushTool`: `pointer_down`
    left → `points = vec![doc_pos]`, `stroking = true`, `before` = layer
    snapshot, overlay = `CanvasShape::path(points.clone(), stroke.clone())`;
    `pointer_move` while stroking appends the doc point only when the
    screen-space distance from the last exceeds `px(1.)` (compare through
    `ToolContext::transform.to_screen`) and refreshes the overlay;
    `pointer_up` → clear overlay, `state.add_shape(layer,
    CanvasShape::path(std::mem::take(&mut self.points), stroke.clone()), cx)`,
    then `let after = AnnotationSnapshot::capture(state, layer);` (from the
    `&mut CanvasState` dispatch param, before any entity update), then
    `self.doc.update(cx, |d, cx| d.record_annotate(EditLabel::Stroke,
    before, after, cx))`, reset.
  - `RectTool { .., anchor: Option<Point<Pixels>>, before:
    Option<AnnotationSnapshot> }`: `pointer_down` left → anchor + before
    snapshot; `pointer_move` → `state.set_tool_overlay(Some(
    CanvasShape::rect(bounds_from_corners(anchor, doc_pos),
    ShapeStyle::stroked(stroke.clone()))), cx)`; `pointer_up` → clear overlay,
    `state.add_shape(layer, final_rect, cx)`, capture `after` from the
    dispatch param, `record_annotate(Stroke, before, after, cx)`.
    EllipseTool identical with `CanvasShape::ellipse` on the corner bounds.
    ArrowTool: commit 3 × `CanvasShape::path(vec![a, b], stroke)` from
    `arrow_segments(anchor, doc_pos)` (2-point paths render straight — 01
    Step 6.1's smoothing loop is empty for 2 points and the final segment is a
    plain `line_to`), overlay = the same three paths.
  - All tools: `cursor() = Crosshair`; ignore non-left buttons; `scroll`
    default (returns false → engine pan/zoom).
- Tests (drive via `dispatch_tool_event` exactly as 01 Step 4.2 tests do):
  brush down/move/up across three positions spaced more than 1 px apart in
  screen space leaves exactly one `CanvasShape::Path` with 3 points, no
  overlay, and one history entry — undo removes the stroke, redo restores it;
  brush tap (down+up, no move) commits a 1-point path dot; rect down/move/up
  leaves exactly one `CanvasShape::Rect` with the expected doc bounds and one
  history entry; ellipse likewise; arrow commits 3 `Path` shapes whose points
  match `arrow_segments` for the same endpoints; overlay cleared after every
  up; pure geometry: `arrow_segments((0,0),(10,0))` shaft is `((0,0),(10,0))`
  and both head segments start at `(10,0)` (doc px via `Pixels::as_f32`);
  `arrow_segments((0,0),(0,10))` rotates the heads onto the −x side;
  `bounds_from_corners` normalizes reversed corners.
- Done-check: `cargo test -p gpui-starter --lib image_editor::tools` passes.

### Step 5.2: Eraser and crop tools

- Files: same.
- Build:
  - `EraserTool { ..shared, radius: f32, before:
    Option<AnnotationSnapshot>, removed: bool }` (`radius` from the eraser
    size slider; the shared `stroke` is unused): `pointer_down`/
    `pointer_move` → tolerance = `px(radius)` doc units (zoom-independent
    eraser size in image px); walk `state.document().layer(layer)` shapes,
    `hit_test(doc_pos, tolerance)` → collect ids; remove via
    `state.remove_shape(id, cx)` as found (live feedback); `pointer_up` → if
    `removed`, capture `after` from the dispatch param, then
    `doc.update(cx, |d, cx| d.record_annotate(EditLabel::Erase,
    before, after, cx))`, reset. History stores full before/after
    snapshots, so no id bookkeeping.
  - `CropTool { ..shared, anchor: Option<Point<Pixels>> }` (its `stroke` is
    the crop outline: page sets `color: cx.theme().foreground` at construction
    — tools cannot reach `cx.theme()`, so the color is passed in as data):
    `pointer_down` anchor; `pointer_move` → overlay `CanvasShape::rect` with
    `StrokeStyle { width: px(1.), color, dash: Some(vec![px(4.), px(4.)]) }`;
    `pointer_up` → clear overlay, `doc.update(cx, |d, cx| d.set_crop_rect(
    Some(rect), cx))` (contract method: stores, emits `CropRectChanged`), does
    NOT record history (Apply does). Re-drag replaces the rect.
- Tests: eraser down over a stroke (path shape crossing the pointer) removes
  it and records one `Erase` entry; undo restores it; eraser miss records
  nothing (`history_len()` unchanged); crop drag sets `crop_rect()` and Apply
  path is covered by Phase 3 crop tests; overlay cleared after up.
- Done-check: `cargo test -p gpui-starter --lib image_editor::tools` passes.

### Step 5.3: Tool rail wiring

- Files: `mod.rs`.
- Build: `EditorTool` enum { Brush, Eraser, Rect, Ellipse, Arrow, Crop };
  `fn select_tool(&mut self, tool: EditorTool, cx)` builds the engine tool:
  every variant mounts through `set_tool(Box::new(XTool::new(doc.clone(),
  canvas.clone(), annotation_layer, stroke)))` — including Brush, which uses
  the editor-local `BrushTool` (Step 5.1), never `set_builtin_tool`; the
  eraser's `stroke` is irrelevant except for `radius`, passed from the eraser
  size slider. `annotation_layer` comes from the contract's reader; rail
  buttons call `cx.listener(select_tool)`; sliders /
  color picker subscriptions: `cx.subscribe(&brush_size, |this, _, ev, cx| if
  let SliderEvent::Release(v) = ev { this.brush_value = v; this.reselect_tool
  (cx) })` — reselect rebuilds the active tool with new params (tools are
  stateless across re-selects except drag state, which a mid-drag reselect
  drops; sliders can't be dragged while canvas pointer is down, so no loss);
  `ColorPickerEvent::Change(Some(color))` → same. Filter sliders:
  `Release` → `doc.update(set_filters(FilterSettings { grayscale: this.gray,
  blur_sigma: blur_val, brightness: bright_val as i32 }))`; grayscale Switch
  `on_click` → toggles + `set_filters` immediately.
- Tests: `select_tool(Brush)` → `active_tool_id() == "image-brush"`;
  `select_tool(Eraser)` → `"image-eraser"`; changing brush size slider (set
  value + emit Release via entity update) then reselect keeps id and updates
  config (observable through a subsequent stroke's stroke width in the
  committed shape).
- Done-check: `cargo test -p gpui-starter --lib image_editor` passes.

## Phase 6: Pixel ops and filters UI

### Step 6.1: Toolbar actions

- Files: `mod.rs`.
- Build: `on_rotate` → `doc.update(rotate90(&canvas))` (guard `has_image` via
  disabled render); `on_apply_crop` → `doc.update(apply_crop(&canvas))`;
  Undo/Redo buttons → `doc.update(undo/redo(&canvas))`; all disabled without
  an image; Apply crop additionally disabled while `crop_rect` missing or
  smaller than `MIN_CROP_PX` (doc exposes `has_valid_crop() -> bool`).
- Tests: covered via Phase 3; add one page-level `#[gpui_kit::test]` if the
  listener plumbing needs proof (rotate button handler test optional — skip;
  document decision).
- Done-check: `cargo check -p gpui-starter` passes; manual QA step 4 covers
  behavior.

## Phase 7: Export, status, gates

### Step 7.1: Save As flow

- Files: `mod.rs`, `bake.rs` (compose already built), `io.rs` (save built).
- Build: `on_save` listener: wasm → unreachable (button disabled with reason);
  native: `save_file(cx)` → `Some(path)` → `save_state = Busy`,
  `cx.spawn(async move |this, cx| { let img = cx.background_spawn(async move {
  compose_export(&base, &filters, &annotations) }).await;` — snapshot
  `base/filters/annotations` by cloning out of the entities before spawning;
  `let res = cx.background_spawn(async move { io::save_image(&path, &img)
  }).await; this.update(cx, |page, cx| { page.save_state = match res { Ok(())
  => Done(path file name), Err(e) => Failed(e.to_string()) }; cx.notify(); })
  })`. Status line next to Save: `Saving…` (with `Spinner::new().xsmall()`),
  `Done` text (muted, `a11y_live(Polite)`), or `Failed` (danger-colored text,
  same live region). `SaveState` cleared on next Open/Save click. Keyboard:
  standard Button activation.
- Tests: pure compose covered in Step 2.2; save covered in Step 4.1; add
  `compose_export` end-to-end: 8×8 white base, no filters, one black Path
  (0,0)→(7,7) width 2 → loaded pixels along the diagonal are darkened.
- Done-check: `cargo test -p gpui-starter --lib image_editor::bake` passes
  with the new test.

### Step 7.2: Docs, dead code sweep, full gates

- Files: `mod.rs` header finalized; `Cargo.toml` unchanged since Step 1.1.
- Build: verify the Interface Contract's pub surface (`ImageEditorPage` only
  via `views`); confirm no `use gpui::` in the new module
  (`tests/qa_migration.rs:386-405` enforces).
- Tests: full suite.
- Done-check: all four gates from Verification pass in order.

## Platform & Wasm Gating

- `src/features/pages/image_editor/io.rs` and its test are entirely behind
  `#[cfg(not(target_family = "wasm"))]` — std::fs decode/encode has no wasm
  caller anyway (pickers return None there).
- `image` sits in the **common** `[dependencies]` table (`default-features =
  false, features = ["png", "jpeg"]`) because the document model and bake
  functions must compile on wasm for `just wasm-check` (`cargo +nightly check
  --target wasm32-unknown-unknown -p gpui-starter --lib`, `justfile:24-25`);
  png/zune-jpeg are pure Rust. No native-only deps are introduced; the
  not-wasm table is untouched. Feature unification merges with the existing
  Linux/Windows `png` lines (`Cargo.toml:266,278`).
- Wasm degradation: page present in the sidebar (hiding it per-platform would
  make `Page::all()` platform-dependent and break the e2e count); Open and
  Save As render **disabled** with the reason surfaced via
  `tooltip(image_editor_open_unavailable)` and the capability entry
  `image_editor = degraded("no file picker on wasm")` on the diagnostics page
  (`desktop_actions` already reports `picker_available: false` on wasm,
  `desktop_actions.rs:83`; `pick_file`/`save_file` return `None` there,
  `:193-221`). Filter controls and tools render enabled but inert (no image
  can exist); the empty-state hint explains this.
- Native platforms (macOS/Linux/Windows): identical behavior; `rfd` dialogs
  are the existing blocking-modal precedent (`settings/dev_sections/runtime_sections.rs:57-79`).
- No `std::time` clocks anywhere in the module (wasm panic rule, survey §6);
  the only async timing is task completion, not clocks.

## i18n

Facade `crate::i18n::localize` / `localize_with_args` (survey §5). Keys land in
both `locales/en.yml` and `locales/zh-CN.yml` in lockstep
(`tests/qa_localization.rs:188-198`). Page prefix `image_editor_` per the
`notifications_*` convention.

| key | en | zh-CN |
|---|---|---|
| `nav_image_editor` | "Image Editor" | "图片编辑器" |
| `image_editor_title` | "Image Editor" | "图片编辑器" |
| `image_editor_empty_hint` | "Open an image to start editing." | "打开一张图片开始编辑。" |
| `image_editor_open` | "Open…" | "打开…" |
| `image_editor_open_unavailable` | "File dialogs are unavailable in the web build." | "网页版不支持文件对话框。" |
| `image_editor_save_as` | "Save As…" | "另存为…" |
| `image_editor_saving` | "Saving…" | "正在保存…" |
| `image_editor_saved` | "Saved" | "已保存" |
| `image_editor_save_failed` | "Couldn't save the image: %{error}" | "无法保存图片：%{error}" |
| `image_editor_open_failed` | "Couldn't open the image: %{error}" | "无法打开图片：%{error}" |
| `image_editor_undo` | "Undo" | "撤销" |
| `image_editor_redo` | "Redo" | "重做" |
| `image_editor_rotate` | "Rotate 90°" | "旋转 90°" |
| `image_editor_crop_apply` | "Apply crop" | "应用裁剪" |
| `image_editor_tool_brush` | "Brush" | "画笔" |
| `image_editor_tool_eraser` | "Eraser" | "橡皮擦" |
| `image_editor_tool_rect` | "Rectangle" | "矩形" |
| `image_editor_tool_ellipse` | "Ellipse" | "椭圆" |
| `image_editor_tool_arrow` | "Arrow" | "箭头" |
| `image_editor_tool_crop` | "Crop" | "裁剪" |
| `image_editor_brush_size` | "Brush size" | "画笔大小" |
| `image_editor_eraser_size` | "Eraser size" | "橡皮擦大小" |
| `image_editor_color` | "Color" | "颜色" |
| `image_editor_filter_grayscale` | "Grayscale" | "灰度" |
| `image_editor_filter_blur` | "Blur" | "模糊" |
| `image_editor_filter_brightness` | "Brightness" | "亮度" |
| `image_editor_filters` | "Filters" | "滤镜" |
| `image_editor_canvas_label` | "Image canvas" | "图片画布" |
| `image_editor_no_picker` | "no file picker in the web build" | "网页版没有文件选择器" |

`image_editor_no_picker` is the capability reason string (set in `init.rs`,
after `set_locale` so the captured string is already localized); not rendered
directly. `Open…`/`Save As…` take the single ellipsis character per the
design guides (commands that need more input before completing).

## Design Compliance

- **Shell/layout**: single-workspace variant — toolbar on top, narrow tool
  rail on the leading edge, filter panel as a trailing inspector (`w_64`,
  subordinate to the canvas; "Inspectors… should not cover the primary
  object", Proportion and layer hierarchy). Canvas area takes the surplus via
  `flex_1().min_h_0()`/`min_w_0()` (flex shrink rule from Coding Guides).
- **Density**: compact for toolbar and tool rail (small buttons, `p_2().gap_1()`
  rail, `py_2` toolbar — "compact / small: toolbars"); medium for the filter
  panel. Sizes via kit `.small()`/component defaults, no custom heights.
- **Tokens**: all chrome colors from `cx.theme()` (`border`, `background`,
  `foreground`, `muted_foreground`, `danger` for save failure); no raw
  hex/`rgb()`/`hsla()` constructors — annotation stroke colors are document
  **data colors** (the guide's raster/data exemption), and the crop overlay's
  color is captured from `cx.theme().foreground` at tool construction, not
  hardcoded.
- **Fixed-px exceptions** (audited list): the crop overlay `dash_array` 4/4
  (hairline-class marker), slider ranges (semantic control params, not
  layout), `MIN_CROP_PX` (document-space data minimum). Layout itself uses
  rem helpers (`p_4`, `gap_2`, `w_64`).
- **Interaction states**: active tool = persistent `selected` on its rail
  button (Button `Selectable`), distinct from hover; Undo/Redo/Apply-crop and
  all filter controls disable with real state (`can_undo`, `has_image`,
  `has_valid_crop`, `is_loading`), not hidden; Save shows Busy (spinner +
  disabled) / Done / Failed inline near the action ("show the result near
  the object that changed"); no success toast — the saved state is the inline
  line, chosen over `push_notification` because the dialog result (file on
  disk) is not visible in-app.
- **Buttons**: Open and Save As are labeled default buttons with icons (not
  primary — "a lone, frequent action is not automatically primary"); icon-only
  buttons only in the tool rail, each with `tooltip` + `accessibility_label`;
  `Save As…`/`Open…` carry the ellipsis (opens dialogs).
- **A11y**: page heading `Role::Heading` level 1 (`notifications.rs:44-53`
  pattern); save status is `a11y_live(Polite)`; sliders/switch/color picker
  carry labels; canvas content itself exposes no per-shape nodes in v1 —
  inherited plan 01 limitation, keyboard paths for every command run through
  labeled buttons.
- **Empty/loading/error states**: empty state names the next action (Open);
  decode/save errors render inline plus `error_surface::report`;
  `image_editor_open_unavailable` explains the wasm-disabled Open.
- **Copy**: sentence case, no terminal periods on control labels, verb-first
  commands, zh-CN full-width punctuation in sentences only.

## Verification

Gates (repo-mandatory set plus wasm, in order):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

Feature-targeted runs during development:

```sh
cargo test -p gpui-starter --lib image_editor
cargo test -p gpui-starter --test e2e_navigation --test qa_localization
```

`Cargo.lock` must be staged with the Step 1.1 manifest change
(`tests/qa_migration.rs:167-186`).

Manual QA (native build, scratch tree):

1. Sidebar shows Image Editor (position 11), deep link `gpui-starter://image-editor`
   routes to it, digit shortcuts for pages 1–9 unchanged.
2. Open a PNG and a JPEG: image fits the canvas; wheel zooms at cursor;
   ctrl/cmd+wheel zooms; middle-drag pans under every tool.
3. Brush draws smoothed strokes in the chosen color/size; eraser removes
   strokes it touches; rect/ellipse/arrow commit on release; arrowheads
   render straight.
4. Crop drag → dashed overlay → Apply crops pixels and shifts annotations;
   rotate 90° rotates pixels and annotations; both undo/redo correctly.
5. Filters: grayscale toggle, blur and brightness sliders apply on release
   (UI stays responsive; large images swap in when the background derive
   finishes); undo restores previous filter state.
6. Save As… writes a PNG whose pixels match the on-screen result (filters +
   annotations baked); status line shows Saved; saving to an unwritable path
   shows the inline failure.
7. Undo history survives 40+ edits (oldest dropped); memory stays bounded
   after a dozen rotates on a 4000×3000 image.
8. Wasm build (`just wasm-check` compiles; run the web harness if available):
   page renders empty state, Open disabled with the reason, no console errors.
9. Both light and dark themes: rail, panel, crop overlay legible.

Acceptance criteria: all four gates pass; the targeted lib tests pass; manual
items 2–6 confirmed on a scratch tree (7–9 optional with notes).

## Risks & Open Questions

Ranked risks:

1. **Blocked on plan 01 implementation.** `src/ui/canvas/` does not exist
   (`ls src/ui/` shows only `widgets/`); everything from Phase 3 onward
   compiles only after 01 lands. This plan consumes 01's contract verbatim;
   if 01's contract changes (it pledges line-stability for 08), re-verify the
   consumed items listed in Dependencies.
2. **Export fidelity vs display.** Annotations render on screen through 01's
   `PathBuilder` strokes and in exports through `bake.rs` circle-stamping
   (round caps). Corner radii, cap shape, and anti-aliasing differ slightly
   between the two rasterizers; a 1:1 pixel match is not promised. Mitigation:
   shared geometry (`arrow_segments`, same stroke widths); risk accepted,
   documented in Step 2.2.
3. **Blur cost on large images.** `blur` on a 48 MP image can take hundreds of
   ms; it runs on the background executor with a request counter, so the UI
   stays responsive and stale results drop. Worst case the display lags a few
   slider releases behind.
4. **History memory.** Snapshot entries share `Arc` bases, but every
   rotate/crop creates a new buffer; the 256 MiB distinct-buffer budget plus
   the 40-entry cap bounds it. A pathological rotate-heavy session still holds
   several full-size buffers simultaneously — that is the accepted trade for
   trivially correct undo.
5. **Blocking file dialogs.** `rfd`'s modal runs on the main thread
   (`desktop_actions.rs:193-221`); same behavior as the settings page
   precedent. No mitigation in scope.
6. **Annotation ids churn on every undo/redo** (snapshot rebuild reallocates
   them). Nothing in the editor persists ids; if a future feature keys on
   annotation identity, it must add stable ids before relying on them.
7. **Editor-local `BrushTool` duplicates 01's builtin brush drag logic**
   (sub-pixel drop threshold, overlay refresh) because the builtin commits
   with no hook the editor can record through. If 01 retunes its `BrushTool`,
   the editor copy drifts; accepted cost for undoable strokes.
8. **`undo = "0.52.0"` is an unused declared dependency**
   (`grep -rn 'undo::\|use undo' src/ tests/` → no hits, run this session).
   This plan does not adopt it; AGENTS.md de-bloat says unused deps are
   removed — flagging for a separate housekeeping commit rather than folding
   it into the feature.

Open questions (each with what was tried):

- **`IconName` PascalCase variants** (`Image`, `Brush`, `Eraser`, `Square`,
  `Circle`, `MoveUpRight`, `Crop`, `RotateCw`, `Undo2`, `Redo2`, `Save`,
  `FolderOpen`): svg filenames verified on disk; variant names follow the
  documented kebab→Pascal rule (survey Open Questions notes the enum is
  build-time generated; compiling is the only full check).
- **`ImageFormat` import path** (`image::ImageFormat::{Png, Jpeg}`): enum
  existence inferred from `write_to(.., format: ImageFormat)` signature at
  `IMG/images/dynimage.rs:1371`; variant names not individually grep-verified.
  Compile is the check.
- **Whether `rfd::FileDialog` needs format filters for Save As** — current
  `desktop_actions::save_file` sets none; extension-based format detection is
  the plan's answer. If users save extensionless files, the plan defaults to
  PNG (Step 4.1) — acceptable, could be improved by adding a filter to
  `desktop_actions` (out of scope, would touch a shared service).
- **Plan 01's exact paint behavior for 2-point paths** (straight `line_to`
  final segment) is asserted from 01's Step 6.1 text, not from code — the
  engine does not exist yet. If 01 implements smoothing differently, arrows
  render curved; fallback documented: build arrows as three 2-point paths and
  re-verify against 01's `smoothed_segments` implementation when it lands.
