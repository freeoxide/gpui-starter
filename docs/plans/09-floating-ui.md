# 09 Floating UI Plan

## Goal

A "Floating UI" sidebar page spawns three tool palettes (color picker,
asset/emoji, sticky note) as draggable always-on-top windows built on plan 02,
with the same palettes available as in-window draggable snap-to-edge panels
(the wasm/portable fallback). Palette position, surface, and visibility persist
into `AppConfig` through the existing `update_config` path and restore on
launch. A color picked in any palette window updates the main window live
through the app event queue.

## Dependencies

Plan 02 (`docs/plans/02-floating-window-toolkit.md`), implemented first. Items
consumed from its Interface Contract (section at `02:279-407`):

- `FloatingWindowId(pub SharedString)` (`02:290-291`), `FloatLevel::AlwaysOnTop`
  (`02:293-300`), `Placement9` (`02:302-307`), `FloatAnchor::{Window, Screen}`
  (`02:309-315`), `FloatingSpec::new(id, size)` with `.title()`, `.level()`,
  `.anchor()`, `.focus()`, `.resizable()`, `.movable()`, `.min_size()`
  builders (`02:317-328`).
- `spawn_floating(spec, build, cx) -> anyhow::Result<FloatingHandle>` with the
  `Fn(&mut Window, &mut App) -> AnyView` retained factory (`02:347-351`;
  retention rule `02:412-418`), `close_floating(&FloatingHandle, cx) -> bool`
  (`02:353-354`), `floating_snapshot(cx) -> Vec<FloatingWindowEntry>` ordered
  by id (`02:361`, ordering guarantee `02:425-426`), `FloatingHandle::{Native,
  Overlay}` and `FloatingWindowEntry { id, title, level, bounds, handle }`
  (`02:330-344`).
- `FloatingWindowRegistry` as an observable Global re-exported by
  `crate::ui::floating`: mutation through the module's set/remove helpers fires
  `observe_global` consumers (`02:227-231`, `02:405-406`).
- `probe(cx) -> FloatingPlatformInfo` field `native_windows`
  (`02:371-379, 391`).
- `floating::init(cx)` bootstrapping (`02:363-365`) and plan 02 Seam Touches
  #3/#4 (AppRoot `floating_host` field + last-child mount, `02:457-460`),
  which this plan's panel host stacks above. Plan 02's `anchor_rect`/resolve
  math (`02:503-524`) is the geometry contract `restore_anchor` relies on.

No other plan is consumed. Plan 03 also consumes plan 02; no conflict: 03 uses
its own notch id namespace, 09 uses the `floating-ui-*` prefix.

## Research Digest

All gpui/gpui-kit claims verified this session against
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` (short `GPUI/src`,
`KIT/src`, `GC/src`, `GB/src`, `KITMAC/src` for gpui-pre-0.3.8, gpui-kit-0.7.1,
gpui-component-0.7.1, gpui-base-0.7.1, gpui-pre-macros-0.3.8) and repo files.

Event queue (cross-window transport):

- Kinds and emit/drain: `src/runtime/events.rs:10-24` (enum),
  `:48-57` (`emit` → `cx.default_global::<AppEventQueue>().0.push`), `:71-81`
  (`drain`, read-only early return). `App::default_global` pushes a
  notify-observers effect on every call (`GPUI/src/app.rs:2293-2302`), so an
  emit in window B wakes observers app-wide.
- `App::set_global` also pushes `Effect::NotifyGlobalObservers`
  (`GPUI/src/app.rs:2305-2309`); `Context::observe_global` registers an
  app-level observer that fires regardless of which window mutated
  (`GPUI/src/app/context.rs:168-183`). That is the redraw-live mechanism for
  views in other windows.
- AppRoot owns the single exhaustive drain:
  `cx.observe_global::<events::AppEventQueue>` at
  `src/shell/root/app_root/state.rs:63-115` (`match event.kind` at `:65`).
  Verified the only exhaustive match on `AppEventKind` in the repo
  (`grep -rn 'match event.kind\|match .*AppEventKind' src/` → one hit). Every
  other AppEventKind user (`src/app/init.rs`, `tray/windows.rs`,
  `web/router.rs`, `single_instance.rs`, `web.rs`,
  `settings/dev_sections/event_sections.rs`, `diagnostics/mod.rs`,
  `title_bar.rs`, `services/commands.rs`) only constructs events. Adding one
  variant = variant + one match arm.
- Effects flush at every `App::update` boundary (`App::flush_effects`,
  `GPUI/src/app.rs:1873-1877`), the same boundary the config-store tests rely
  on (`src/state/config_store.test.rs:280-287`).

Config persistence:

- `AppConfig` pub fields are the pinned serde schema; additions need
  `#[serde(default)]` (`src/state/config_store.rs:63-65`). Write path:
  `crate::app_state::update_config(cx, |config| ...)` runs the closure
  synchronously, then persists after a 300 ms debounce
  (`config_store.rs:274-287`, `DEBOUNCE_MS` at `:28`); when the
  `AppState` global is absent it warns and no-ops (`config_store.rs:275-278`);
  borrowed read `with_config` (`:228-233`). `NotificationInboxItem` is the
  precedent for a feature-owned serde type inside `AppConfig`
  (`config_store.rs:18`).
- Schema pins must be edited with any field add: `tests/snapshot_tests.rs:26-52`
  (inline insta yaml of the default serialization) and the exhaustive struct
  literal in the round-trip test from `:57` on.
- `AppState` fields are private; out-of-module tests cannot construct it
  (`config_store.rs:48-59`); in-module tests use a `clean_state` helper
  (`config_store.test.rs:265-270`). Consequence: floating_ui tests must not
  attempt live config round-trips; they test pure `sanitize`/serde and leave
  the debounced write to `config_store.test.rs` coverage plus manual QA.
- Wasm: config save is a no-op, in-memory config stays authoritative
  (`config_store.rs:521-528`); persistence code compiles unchanged on wasm.

Window spawn and root window:

- `gpui_kit::open_window<V: Render>(options, cx, build)` hosts the kit
  `base::Root` (`KIT/src/lib.rs:149-164`); repo precedent
  `src/app/window.rs:95-98`.
- Root window handle: `RootWindow` Global (`src/app/window.rs:9-11`), set via
  `cx.set_global` in `note_root_window`, called from `create_new_window`
  (`src/app/window.rs:107`); read via `root_window(cx)`
  (`src/app/window.rs:20-22`). Observable as the boot signal for restore.
- `cx.primary_display()` / `PlatformDisplay::visible_bounds()` for
  restore-anchor math (`GPUI/src/app.rs:1487-1509`,
  `GPUI/src/platform.rs:566-593`, quoted through plan 02 `02:120-126`).

In-window panel drag (verified machinery):

- `on_drag<T, W>(value, constructor)`: constructor
  `Fn(&T, Point<Pixels>, &mut Window, &mut App) -> Entity<W>` builds the drag
  preview (`GPUI/src/elements/div.rs:615-630`). The drag starts only after a
  2px move threshold with the left button held (`DRAG_THRESHOLD` at
  `div.rs:52`, start logic `:2983-2996`). An entity rendering a bare `div()`
  is an invisible zero-size preview.
- `on_drag_move<T>` fires on the capture phase for every move event while a
  drag of type `T` is active, on every element that registered it
  (`div.rs:359-381`); the listener receives `&DragMoveEvent<T>` whose
  `event.position` is window-space and whose `drag(cx) -> &T` returns the
  payload (`div.rs:66-81`). Listener code must therefore key off
  `ev.drag(cx).id`, not off which header registered.
- Drop listeners fire on mouse-up only while hovered over the element's
  hitbox (`hitbox.is_hovered(window)` gate, `div.rs:2904-2933`); the drag
  session always ends on mouse-up (`cx.active_drag = None`,
  `GPUI/src/window.rs:5829-5839`). A pane that follows the cursor keeps the
  pointer inside itself, so `on_drop` on the pane is the release hook.
- Cursor affordances: `cursor_grab` / `cursor_grabbing` / `cursor_ew_resize`
  (`gpui-pre-macros-0.3.8/src/styles.rs:247-265`).
- `Window::viewport_size() -> Size<Pixels>` is the drawable area
  (`GPUI/src/window.rs:2810-2812`); `Window::bounds()` is global-coordinate
  space (`:2744-2746`). Panels clamp against `viewport_size`, never
  `bounds()`.

Palette content components:

- `Textarea::new(&Entity<TextareaState>)` (`GC/src/input/textarea.rs:70`,
  re-export `GC/src/input/mod.rs:58`; `TextareaState` re-export at
  `GC/src/input/mod.rs:38`). `TextareaState::new(window, cx)`
  (`GB/src/input/base/state.rs:10706-10709`), `set_value(impl
  Into<InputContent>, window, cx)` clears history and emits no event
  (`:947-951`), `value() -> SharedString` (`:1257`). Changes observable as
  `InputEvent::Change` (`GB/src/input/base/state.rs:126-131`) via
  `cx.subscribe`; repo precedent `src/gallery/page.rs:23-27`. `impl Styled
  for Textarea` exists (`textarea.rs:179`), so `.h_full()` applies.
- `rgba(hex: u32) -> Rgba` (`GPUI/src/color.rs:20-23`), re-exported through
  `gpui_kit::*` (`GPUI/src/gpui.rs:106`); `Rgba` has four pub f32 fields 0..1
  (`color.rs:39-48`); `.bg()` accepts `Rgba` via
  `impl From<Rgba> for Background` (`color.rs:959-961`). gpui has no hex
  formatter; parse/format is a local pure helper. Swatch colors are data,
  the design guides' audited color-literal exception ("Color and themes":
  raw colors are allowed "in audited data content whose color is itself the
  data").
- Kit `ColorPicker`/`ColorSelect` are trigger+popover fields
  (`GC/src/color_picker.rs:72-74, 544-546`), not an always-open grid; the
  palette grid is a new surface built from stateful divs with explicit a11y
  (see Design Compliance).
- Buttons: `Button::label` (`GC/src/button/button.rs:371`),
  `accessibility_label` (`:391`), `icon(impl Into<ButtonIcon>)` (`:397`;
  `Icon` converts through `ButtonIconVariant`'s blanket
  `impl<T: Into<Icon>> From<T>`, `GC/src/button/button_icon.rs:55-64`),
  `ghost` (`:82`), `disabled` (`GB/src/button.rs:59`); repo icon-button
  precedent `src/gallery/sections/buttons/button.rs:199`.
- ElementId tuple composition: `impl<T: Into<SharedString>>
  From<(ElementId, T)> for ElementId` (`GPUI/src/window.rs:7601-7605`):
  `Button::new(("prefix", slug))` with `slug: &'static str` compiles
  (first slot `&'static str → ElementId`, `:7552-7556`); `format!`-built
  `String` also works (`From<String>`, `:7536-7540`).
- a11y builders: `role` (`GPUI/src/elements/div.rs:1310`), `aria_label`
  (`:1332`), `aria_toggled(accesskit::Toggled)` (`:1406`; `Toggled`
  re-exported via `gpui_kit` from `GPUI/src/gpui.rs:99`), `aria_level`
  (`:1457`). There is NO `aria_pressed` (verified: full builder list on
  `StatefulInteractiveElement`, `src/services/accessibility.rs:11-16`
  inventory + grep of div.rs); selected swatch state uses `aria_toggled`.
  Repo `A11yExt::{a11y, a11y_live}` at `src/services/accessibility.rs:27,35`;
  `Role` and `accesskit::Live` come from `gpui_kit`
  (`src/services/accessibility.rs:17`).
- TWO `IconName` enums, and the distinction is load-bearing for this plan:
  `gpui_kit::component::IconName` (the import at `src/shell/sidebar.rs:1`,
  and the return type of `Page::icon()`) is generated from the 106-entry
  default list `gpui-kit-assets-0.7.1/default-icons.txt` via the
  `__component_icon_names!` macro (`gpui-component-0.7.1/src/icon.rs:17-45`,
  fed by `gpui-kit-assets-0.7.1/build.rs:73-102`, whose comment states "The
  component compatibility enum keeps exactly the original variants, while the
  shared assets enum above exposes the complete catalog"). Read the list in
  full this session: `palette` and `panel-left` are present; `app-window`,
  `layers`, `sticky-note`, and `x` are ABSENT (the close icon in this enum is
  `close` → `IconName::Close`; repo has 6 uses of `IconName::Close` and zero
  of X/AppWindow/Layers/StickyNote, grep run). The full 1818-icon catalog is
  `gpui_kit::assets::IconName` (`KIT/src/lib.rs:140-141`
  `pub use ::gpui_kit_assets as assets;`; `app-window.svg`, `layers.svg`,
  `sticky-note.svg`, `palette.svg`, `panel-left.svg`, `close.svg`, `x.svg`
  all exist in `gpui-kit-assets-0.7.1/assets/icons/`). `gpui_kit::assets` is
  the sanctioned facade path (`tests/qa_migration.rs:386-405` bans only the
  bare `gpui_kit_assets::` prefix); repo precedent
  `use gpui_kit::assets::IconName as AssetIconName;`
  (`src/gallery/sections/buttons/toolbar.rs:3`, used at `:162`). Both enums
  feed `Icon::new`/`Button::icon` (`impl<T: IconNamed> From<T> for Icon`,
  `gpui-component-0.7.1/src/icon.rs:56-58`; ButtonIcon blanket
  `gpui-component-0.7.1/src/button/button_icon.rs:55-64`). Consequence: the
  sidebar `icon()` arm must use a default-list variant (`Palette`); page and
  panel icons may use the assets enum where the default list has no fit.
- `Role::Pane` exists (`accesskit-0.24.1/src/lib.rs:92`, reached via
  `gpui_kit::accesskit` per `tests/qa_migration.rs:386-405`).
- `Icon::new(impl Into<Icon>)` (`GC/src/icon.rs:105`), `impl Sizable for Icon`
  (`GC/src/icon.rs:215`); `Badge` is a child-carrying RenderOnce
  (`GC/src/badge.rs:41-99`). `TextareaState` carries a fluent
  `placeholder(impl Into<SharedString>)` builder
  (`GB/src/input/base/state.rs:814`; repo precedent
  `InputState::new(window, cx).placeholder(...)` at
  `src/gallery/page.rs:22`).

Capability status shape: `CapabilityStatus` has FIVE pub fields
(`supported, enabled, degraded, reason, last_error`) and no `Default`
(`src/runtime/capabilities.rs:6-13`); helpers `supported_enabled()` /
`degraded()` / `error()` at `capabilities.rs:16-51`. Every struct literal must
name all five fields.

Tests: the test platform implements `open_window` headless
(`GPUI/src/platform/test/platform.rs:493-506`); `#[gpui_kit::test]` +
`TestAppContext` usable in-crate (`src/state/config_store.test.rs:280-281`);
sibling test wiring `#[cfg(test)] #[path = "<name>.test.rs"] mod <name>_test;`
(`src/ui/widgets/virtual_list.rs:90-92`). The localization scanner asserts
every string-literal `localize("...")` / `localize_with_args("...")` call
site resolves in BOTH catalogs (`tests/qa_localization.rs:146-190`), so each
key lands with (or before) the step that first renders it.

Crate picks: none new. No dependency changes; wasm and native tables
untouched, `Cargo.lock` untouched.

## Architecture

Entities and state ownership:

- `PaletteId` (enum `Color | Assets | Note`): value object; slugs
  `"color" | "assets" | "note"` serde-as-lowercase; plan 02 window id prefix
  `floating-ui-<slug>`.
- `PaletteSyncState` (App Global, `Default`): `{ color_hex: String,
  last_asset: Option<SharedString> }`. Written ONLY by `state::apply_change`
  (the AppRoot drain arm) and seeded once by `floating_ui::init`; read by the
  page and every palette view. Mutated via `cx.set_global` so all observers
  re-render (`GPUI/src/app.rs:2305-2309`).
- `PersistedPalette` (feature-owned serde struct) stored as
  `AppConfig.floating_palettes: Vec<PersistedPalette>` (`#[serde(default)]`).
  One entry per `PaletteId`: `{ id, open, surface ("window"|"panel"), x, y,
  width, height, color: Option<String>, note: Option<String> }`.
- `PaletteSession` (Entity, field of `AppRoot`, handle passed to the page and
  host): source of truth for which palette is open on which surface. Owns the
  plan 02 spawn/close calls, the `FloatingWindowRegistry` observation
  (external closes → persist `open: false`), boot restore, and all
  `update_config` writes for open/surface.
- `PalettePanelHost` (Entity, last child of AppRoot's root `v_flex`): renders
  open panels as absolutely positioned panes, owns panel geometry + drag +
  snap, persists geometry on drag release. Holds `Entity<PaletteSession>` and
  the three content view entities.
- Content views (Entity + Render, `views.rs`): `ColorPaletteView`,
  `AssetPaletteView`, `StickyNoteView`. Shared verbatim between native palette
  windows (plan 02 factory creates fresh instances in the new window's
  context) and panels (host embeds its instances). They never write
  `PaletteSyncState`; they only `events::emit` and observe.

Events and data flow:

1. Swatch/emoji click in any surface → `crate::events::emit(
   AppEventKind::PaletteChanged { palette, value }, cx)` (queue push,
   `src/runtime/events.rs:48-57`).
2. `emit`'s `default_global` notifies queue observers app-wide → AppRoot's
   observer in the main window drains (`state.rs:63-115`); the new arm calls
   `floating_ui::state::apply_change(&palette, &value, cx)`.
3. `apply_change` writes `PaletteSyncState` via `set_global` → observers of
   `PaletteSyncState` (page in the main window, palette views in every
   window) fire → each `cx.notify()` redraws its own window. Live
   cross-window sync through the event queue, one writer, no polling.
4. Sticky note typing: `InputEvent::Change` subscription → `update_config`
   (debounced). Note text is document state, not cross-window state; it does
   not use the event queue.

Native spawn flow: page button → `session.toggle_window(id, cx)` → open?
`close_floating` + persist closed : `spawn_floating(spec, factory, cx)` with
`FloatLevel::AlwaysOnTop`, anchor from the persisted origin (restore) or
`FloatAnchor::Window { root_window, TopRight, offset(16,16) }` (first open) →
registry entry appears → persist `{open: true, surface: "window"}`.

External close flow (Esc in FloatingShell, OS close button): plan 02's close
tracker removes the registry entry → session's
`observe_global::<FloatingWindowRegistry>` fires → id absent from
`floating_snapshot` → persist `{open: false}`.

Panel flow: page button → `session.set_panel_open(id, open, cx)` → persist →
session notify → host (observes session) shows/hides the pane. Drag: header
`on_drag(PanelDrag)` + `on_drag_move` (origin += cursor delta, clamped to
`window.viewport_size()`), release `on_drop` → `snap_origin` (horizontal snap
within 24px of either window edge) → `update_config` geometry.

Boot restore: `AppRoot::new` registers
`cx.observe_global::<crate::app::window::RootWindow>` (Step 5.3); the first
fire (main window recorded, `src/app/window.rs:13-15,107`) runs
`session.restore_windows(cx)` for persisted `open && surface == "window"`
palettes on native. `floating_ui::init(cx)` seeds `PaletteSyncState` and
writes the capability entry. Panel slots and geometry load from config during
`PaletteSession::new` / `PalettePanelHost::new` (pure reads; no side effects
in render).

Module tree:

```text
src/features/pages/floating_ui/
├── mod.rs             FloatingUiPage, feature init(cx), re-export seam
├── state.rs           PaletteId, presets (colors, emoji), PaletteSyncState, apply_change, hex helpers
├── state.test.rs
├── geometry.rs        snap_origin, clamp_origin, restore_anchor (pure)
├── geometry.test.rs
├── persistence.rs     PersistedPalette, slot load/save, sanitize
├── persistence.test.rs
├── session.rs         PaletteSession: open/surface truth, spawn/close, registry watch, restore
├── session.test.rs
├── panel_host.rs      PalettePanelHost: absolute pane layer, drag/snap, geometry persist
├── panel_host.test.rs
├── views.rs           ColorPaletteView, AssetPaletteView, StickyNoteView, palette_view_factory
└── views.test.rs
```

## Interface Contract

Public surface at `crate::features::pages::floating_ui` (crate-internal
feature module; only `FloatingUiPage` leaves it, via
`crate::views::FloatingUiPage`):

```rust
/// Stable palette identity; serde lower-case slug ("color" | "assets" | "note").
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaletteId { Color, Assets, Note }

impl PaletteId {
    pub const ALL: &'static [PaletteId];      // [Color, Assets, Note]
    pub fn slug(self) -> &'static str;
    pub fn from_slug(slug: &str) -> Option<Self>;
    /// Plan 02 window id, e.g. "floating-ui-color".
    pub fn window_id(self) -> crate::ui::floating::FloatingWindowId;
    /// Localized palette title (catalog key per id).
    pub fn localized_title(self) -> String;
    /// Default pane/window size per palette (logical px).
    pub fn default_size(self) -> gpui_kit::Size<gpui_kit::Pixels>;
}

/// Materialized cross-window palette state. Written only by [`apply_change`]
/// and seeded by [`init`]; read by observers in any window.
#[derive(Clone, Debug, Default)]
pub struct PaletteSyncState {
    pub color_hex: String,                 // "#RRGGBB"; seeded by init from the first preset
    pub last_asset: Option<gpui_kit::SharedString>,
}
impl gpui_kit::Global for PaletteSyncState {}

/// Interpret a drained `AppEventKind::PaletteChanged`; called from AppRoot only.
/// Unknown palette slug is a logged no-op.
pub fn apply_change(palette: &str, value: &str, cx: &mut gpui_kit::App);

/// Feature boot hook: seeds `PaletteSyncState` (first color preset) and
/// writes the `floating_ui_palettes` capability entry. Call once from
/// `app::init`, after `crate::ui::floating::init`. Boot restore of open
/// windows is AppRoot's RootWindow observer (Step 5.3), not this function:
/// an App-level observer here has no entity handle to call.
pub fn init(cx: &mut gpui_kit::App);

pub const CAPABILITY_FLOATING_UI: &str = "floating_ui_palettes";

/// Page view; constructed by AppRoot with the session handle.
pub struct FloatingUiPage { /* private */ }
impl FloatingUiPage {
    pub fn new(
        session: gpui_kit::Entity<PaletteSession>,
        cx: &mut gpui_kit::Context<Self>,
    ) -> Self;
}

/// Open/surface truth for the three palettes. AppRoot owns the entity; the
/// page and panel host hold handles.
pub struct PaletteSession { /* private */ }
impl PaletteSession {
    pub fn new(cx: &mut gpui_kit::Context<Self>) -> Self;
    pub fn is_open(&self, id: PaletteId) -> bool;
    pub fn surface(&self, id: PaletteId) -> Option<PaletteSurface>;
    /// Native window toggle. Returns Err(reason) on wasm or spawn failure;
    /// closing an open window always returns Ok.
    pub fn toggle_window(
        &mut self, id: PaletteId, cx: &mut gpui_kit::Context<Self>,
    ) -> anyhow::Result<()>;
    pub fn set_panel_open(&mut self, id: PaletteId, open: bool, cx: &mut gpui_kit::Context<Self>);
    /// Close+respawn a native window at the default anchor (position reset;
    /// the only move path on Windows, where PopUp windows cannot be dragged).
    pub fn reset_window_position(&mut self, id: PaletteId, cx: &mut gpui_kit::Context<Self>);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteSurface { Window, Panel }

/// In-window draggable panel layer; AppRoot renders it as the last child.
pub struct PalettePanelHost { /* private */ }
impl PalettePanelHost {
    pub fn new(
        session: gpui_kit::Entity<PaletteSession>,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::Context<Self>,
    ) -> Self;
}

/// Content factory handed to plan 02's `spawn_floating`; callable more than
/// once (respawn), creates a fresh view entity per call.
pub(crate) fn palette_view_factory(
    id: PaletteId,
) -> std::rc::Rc<dyn Fn(&mut gpui_kit::Window, &mut gpui_kit::App) -> gpui_kit::AnyView>;

// --- pure helpers (seam-test targets) ---
/// "#RRGGBB"/"#RRGGBBAA" -> Rgba; None on malformed input.
pub(crate) fn hex_to_rgba(hex: &str) -> Option<gpui_kit::Rgba>;
pub(crate) fn rgba_to_hex(color: gpui_kit::Rgba) -> String;      // "#RRGGBB"
/// Clamp a panel box fully inside the viewport; snap x to an edge when
/// within `SNAP_THRESHOLD` (left edge wins when both qualify).
pub(crate) const SNAP_THRESHOLD: gpui_kit::Pixels = gpui_kit::px(24.0);
pub(crate) fn clamp_origin(
    origin: gpui_kit::Point<gpui_kit::Pixels>,
    size: gpui_kit::Size<gpui_kit::Pixels>,
    viewport: gpui_kit::Size<gpui_kit::Pixels>,
) -> gpui_kit::Point<gpui_kit::Pixels>;
pub(crate) fn snap_origin(
    origin: gpui_kit::Point<gpui_kit::Pixels>,
    size: gpui_kit::Size<gpui_kit::Pixels>,
    viewport: gpui_kit::Size<gpui_kit::Pixels>,
) -> gpui_kit::Point<gpui_kit::Pixels>;
/// Screen-anchored spec offset that reproduces `origin` through plan 02's
/// resolve math (TopLeft + offset, clamped by plan 02 into the same visible
/// bounds). Caller resolves the primary display's visible bounds; when no
/// display resolves, fall back to the Window-anchored default instead.
pub(crate) fn restore_anchor(
    origin: gpui_kit::Point<gpui_kit::Pixels>,
    screen_visible: gpui_kit::Bounds<gpui_kit::Pixels>,
) -> crate::ui::floating::FloatAnchor;
```

Event kind added to `AppEventKind` (`src/runtime/events.rs`):
`PaletteChanged { palette: String, value: String }` (both `String`, matching
the existing `DeepLinkReceived(String)` shape; `palette` is a
`PaletteId::slug`, `value` is a hex string or emoji glyph).

Persisted schema added to `AppConfig` (`src/state/config_store.rs`):

```rust
#[serde(default)]
pub floating_palettes: Vec<crate::features::pages::floating_ui::persistence::PersistedPalette>,
```

Guarantees: `apply_change` with an unknown palette slug logs at warn and
changes nothing. `PaletteSyncState` readers before `init` see the derived
`Default` (empty hex) and the page swatch falls back to opaque black
(`rgba(0x000000FF)`). All persistence goes through
`crate::app_state::update_config`; no direct file writes. Wasm never calls
`spawn_floating` (`toggle_window` returns `Err` before reaching it), so plan
02's overlay path is not exercised by this feature.

## Module Layout

Every module wires its sibling test with the repo convention
`#[cfg(test)] #[path = "<name>.test.rs"] mod <name>_test;`
(`src/ui/widgets/virtual_list.rs:90-92`).

| File | Purpose |
| --- | --- |
| `src/features/pages/floating_ui/mod.rs` | `FloatingUiPage`, `init(cx)`, re-exports, `CAPABILITY_FLOATING_UI` |
| `src/features/pages/floating_ui/state.rs` | `PaletteId`, color/emoji presets, `PaletteSyncState`, `apply_change`, hex helpers |
| `src/features/pages/floating_ui/state.test.rs` | slug round-trip, hex round-trip, `apply_change` global write + observer fire |
| `src/features/pages/floating_ui/geometry.rs` | `clamp_origin`, `snap_origin`, `restore_anchor`, `SNAP_THRESHOLD` |
| `src/features/pages/floating_ui/geometry.test.rs` | snap/clamp/restore tables (plain `#[test]`) |
| `src/features/pages/floating_ui/persistence.rs` | `PersistedPalette`, slot load/save via `update_config`, `sanitize` |
| `src/features/pages/floating_ui/persistence.test.rs` | sanitize drops garbage; PersistedPalette serde round-trip (no App) |
| `src/features/pages/floating_ui/session.rs` | `PaletteSession`: open/surface truth, spawn/close/reset, registry watch, boot restore |
| `src/features/pages/floating_ui/session.test.rs` | spawn/close lifecycle, external-close handling, restore from seeded slots |
| `src/features/pages/floating_ui/panel_host.rs` | `PalettePanelHost`: out-of-flow absolute layer, pane chrome, drag/snap, geometry persist |
| `src/features/pages/floating_ui/panel_host.test.rs` | pane count follows session; empty host renders nothing; snap-commit math |
| `src/features/pages/floating_ui/views.rs` | Three palette content views + `palette_view_factory` |
| `src/features/pages/floating_ui/views.test.rs` | view construction smoke; emit helpers produce queue events |

## Seam Touches

| # | File | Change | Why minimal |
| --- | --- | --- | --- |
| 1 | `src/shell/sidebar.rs:4-16,19-31,36-49,57-69,77-89,92-105` | `Page::FloatingUi` variant + `title` ("Floating UI")/`title_key` (`nav_floating_ui`)/`host` ("floating-ui")/`icon` (`IconName::Palette`; `palette` is in the component default list, and no other page uses it)/`all()` entry APPENDED after `Gallery` | The page seam (survey `seams.md:9-31`). Appending avoids the digit-key rebinding footgun: `NavigateToPage(i)` binds only `i < 9` (`src/shell/root/app_root/state.rs:144-166`), and 10 pages already exist, so index 10 binds nothing and no existing binding moves. The icon must be a `gpui_kit::component::IconName` variant (the enum `sidebar.rs:1` imports); `AppWindow` does not exist in that enum (default-list fact, Research Digest) |
| 2 | `src/shell/route.rs:10-21` | `Page::FloatingUi.host(),` in `VALID_HOSTS` | One-line exhaustive list |
| 3 | `src/features/pages/mod.rs` | `mod floating_ui;` + `pub use floating_ui::FloatingUiPage;` | Two-line mount (`mod.rs:1-25` pattern) |
| 4 | `src/shell/root/app_root/state.rs:17-38` | Fields `floating_ui_session: Entity<PaletteSession>`, `floating_ui_host: Entity<PalettePanelHost>` | Entities must live in the main window's tree |
| 5 | `src/shell/root/app_root/state.rs:41-66` | Construct session, then host, then `FloatingUiPage::new(floating_ui_session.clone(), cx)`; import | Constructor block grows three lines |
| 6 | `src/shell/root/app_root/state.rs:211-224` | `Page::FloatingUi => self.floating_ui_page.clone().into()` arm + field | Exhaustive match forces it |
| 7 | `src/shell/root/app_root/state.rs:63-115` | Drain arm `AppEventKind::PaletteChanged { palette, value } => floating_ui::apply_change(&palette, &value, cx)` | Single-writer rule; one arm, no `cx.notify()` (observed state is not AppRoot's) |
| 8 | `src/shell/root/app_root/render.rs:287-291` | Append `.child(self.floating_ui_host.clone())` as the LAST child of the root `v_flex` (after plan 02's `floating_host` if present) | Panels must paint above everything (gpui stacking is tree order; no `z_index`, plan 02 `02:138-139`) |
| 9 | `src/runtime/events.rs:10-24` | `PaletteChanged { palette: String, value: String }` variant | One enum variant; only AppRoot matches exhaustively (verified) |
| 10 | `src/state/config_store.rs:66-90,115-135` | `#[serde(default)] pub floating_palettes: Vec<PersistedPalette>` + `Default` entry + import | Sanctioned additive-schema path (`config_store.rs:63-65`); `NotificationInboxItem` import precedent at `:18` |
| 11 | `src/app/init.rs:274` (after `crate::notifications::initialize(cx);`, i.e. after plan 02's `floating::init` line) | `crate::features::pages::floating_ui::init(cx);` | Single boot hook |
| 12 | `tests/e2e_navigation.rs:14-19,22-40` | Count `10` → `11`; add `Page::FloatingUi` to `expected` | Mechanical test updates |
| 13 | `tests/snapshot_tests.rs:26-52,57-` | Add `floating_palettes: []` to the inline snapshot; add the field (empty vec) to the round-trip literal | Schema pin |
| 14 | `locales/en.yml`, `locales/zh-CN.yml` | Keys in the i18n table below, both catalogs in lockstep | `tests/qa_localization.rs:188-198` |

### Removal Recipe

Mirrors `src/gallery/mod.rs:2-7`:

1. Delete `src/features/pages/floating_ui/` entirely.
2. Revert `src/features/pages/mod.rs` to the previous `mod`/`pub use` list.
3. Remove the `Page::FloatingUi` arms and `all()` entry from
   `src/shell/sidebar.rs` and the `VALID_HOSTS` entry in
   `src/shell/route.rs`.
4. Remove the `floating_ui_session`/`floating_ui_host` fields, constructor
   lines, the RootWindow restore observer, `FloatingUiPage` construction, the
   `unchecked_active_page_view` arm, and the `PaletteChanged` drain arm from
   `src/shell/root/app_root/state.rs`; remove the
   `.child(self.floating_ui_host.clone())` line from
   `src/shell/root/app_root/render.rs`.
5. Remove the `PaletteChanged` variant from `AppEventKind`
   (`src/runtime/events.rs`).
6. Remove the `floating_palettes` field from `AppConfig` + its `Default`
   entry (`src/state/config_store.rs`); restore the two snapshot fixtures
   (`tests/snapshot_tests.rs`). Old state files keep a `floating_palettes`
   array that serde then ignores (unknown fields are skipped; no migration).
7. Remove `crate::features::pages::floating_ui::init(cx);` from
   `src/app/init.rs`.
8. Remove every `floating_ui*` key from `locales/en.yml` and
   `locales/zh-CN.yml`; revert `tests/e2e_navigation.rs` to 10.
9. Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-features`,
   `cargo test --workspace --all-features`. Open palettes cannot survive the
   removal (their windows close with the process), so no runtime cleanup.

## Implementation Phases

## Phase 1: Domain types and pure geometry

### Step 1.1: `state.rs` core types

Files: `src/features/pages/floating_ui/state.rs`,
`src/features/pages/floating_ui/mod.rs` (new), `src/features/pages/mod.rs`,
`locales/en.yml`, `locales/zh-CN.yml`.

Build:
- `floating_ui/mod.rs`: `mod state;` + `pub use state::{PaletteId,
  PaletteSyncState};` + the sibling-test wiring line for `state` + a doc
  comment naming the removal seams. Later phases add their own `mod`/`pub
  use` lines when their modules exist (noted per step).
- `src/features/pages/mod.rs`: `mod floating_ui;` only; the
  `pub use floating_ui::FloatingUiPage;` line lands in Phase 4 when the type
  exists.
- `state.rs`: `PaletteId` (serde `rename_all = "lowercase"`), `slug`,
  `from_slug`, `ALL`, `window_id` (`format!("floating-ui-{}", slug)` into
  `FloatingWindowId`), `localized_title` mapping to the three
  `floating_ui_palette_*` catalog keys — the three keys land in THIS step,
  with the first call site (qa_localization is a static source scan over all
  of `src/`, so a `localize` literal shipped before its key fails the suite
  at every intermediate commit), `default_size`: Color `px(260.) x px(320.)`,
  Assets `px(300.) x px(260.)`, Note `px(260.) x px(220.)`.
- `PaletteSyncState { color_hex: String, last_asset: Option<SharedString> }`,
  `Default` (empty string / None), `impl Global`.
- `hex_to_rgba` / `rgba_to_hex`: parse the 6- or 8-digit tail after `#` via
  `u32::from_str_radix`; format rounds each channel
  `(c * 255.).round().clamp(0., 255.) as u8` and drops alpha below 1.0 to
  the `#RRGGBB` form.
- Presets: `COLOR_PRESETS: &[(&str, u32); 12]` (hex slug + `0xRRGGBBFF` data
  color; one set covering the hue wheel plus black/white; values are data,
  not theme tokens); `ASSET_PRESETS: &[&str]` (24 emoji glyphs, e.g.
  "🚀 🎨 📌 ✅ ⭐ 🔥 💡 🧪 📦 🗂️ 🔒 ⏱️ 📊 🧭 ✏️ 🗑️ 👀 🎯 ⚙️ 🔗 💬 🏷️ 📍 🧩").
- `apply_change(palette, value, cx)`: `from_slug` → None: `tracing::warn!`
  (target `gpui_starter::floating_ui`) and return; Color → `color_hex =
  value`, Assets → `last_asset = Some(value.into())`, Note → logged no-op.
  Then `cx.set_global(next)` where `next` is the current global (or Default)
  cloned with the field replaced.

Tests: none yet.
Done-check: `cargo check -p gpui-starter` passes; `cargo test -p gpui-starter --test qa_localization` passes (the three `floating_ui_palette_*` keys are in both catalogs from this step on).

### Step 1.2: `geometry.rs`

Files: `src/features/pages/floating_ui/geometry.rs`,
`src/features/pages/floating_ui/mod.rs` (grows `mod geometry;`).

Build (all pure, `gpui_kit::{Point, Size, Pixels, px, Bounds}` only):
- `SNAP_THRESHOLD: Pixels = px(24.)`.
- `clamp_origin(origin, size, viewport) -> Point<Pixels>`: x clamped into
  `[0, (viewport.width - size.width).max(0)]`, y likewise (a panel larger
  than the viewport pins to 0).
- `snap_origin(origin, size, viewport) -> Point<Pixels>`: clamp first; then
  if `x.abs() <= SNAP_THRESHOLD` → x = 0; else if
  `(x - (viewport.width - size.width)).abs() <= SNAP_THRESHOLD` → x = right
  edge; y unchanged beyond clamp. When both edges qualify (narrow viewport),
  the right-edge check is an `else if`, so the left edge wins: deterministic
  and covered by test.
- `restore_anchor(origin, screen_visible: Bounds<Pixels>) -> FloatAnchor`:
  `FloatAnchor::Screen { display: None, placement: Placement9::TopLeft,
  offset: origin - screen_visible.origin }`. Plan 02's `resolve_bounds`
  places TopLeft at the visible-bounds corner, applies the offset, then
  clamps into the same rect (`02:503-524`), so this reproduces `origin`
  exactly unless the display changed.

Tests (`geometry.test.rs`, plain `#[test]`): snap left at 0/10/24px; no snap
at 25px; snap right; both-edges case picks left; clamp at each edge;
oversized panel pins 0; `restore_anchor` offset reproduces origin for three
(origin, visible) pairs.
Done-check: `cargo test -p gpui-starter --lib floating_ui::geometry` green.

## Phase 2: Persistence

### Step 2.1: `persistence.rs` + `AppConfig` field

Files: `src/features/pages/floating_ui/persistence.rs`,
`src/features/pages/floating_ui/mod.rs`, `src/state/config_store.rs`.
`mod.rs` grows `mod persistence;` (types stay `pub(crate)`/in-module; nothing
re-exported yet).

Build:
- `PersistedPalette { id: String, open: bool, surface: String, x: f32, y:
  f32, width: f32, height: f32, color: Option<String>, note: Option<String>
  }`, `Clone/Debug/PartialEq/Serialize/Deserialize`.
- `sanitize(raw: &[PersistedPalette]) -> BTreeMap<PaletteId,
  PersistedPalette>`: keep known ids (one per id, first wins), finite coords,
  `0 < width/height <= 100_000` (matches `MAX_PLAUSIBLE_DIM`,
  `config_store.rs:39`), surface normalized to "window"/"panel" (unknown →
  "panel"), cap the input at 16 entries.
- `slot(id, cx) -> Option<PersistedPalette>`: `crate::app_state::with_config`
  read of `config.floating_palettes` (`config_store.rs:228-233`).
- `save_slot(entry, cx)`: `crate::app_state::update_config(cx, |config| {
  let v = &mut config.floating_palettes;
  v.retain(|p| p.id != entry.id);
  v.push(entry.clone());
  })`; this is the one writer shape for every later phase. When the `AppState` global is
  absent (unit tests) this warns and no-ops (`config_store.rs:275-278`), so
  tests that exercise it need no fixture.

`config_store.rs`: import the type (precedent: `NotificationInboxItem`
import at `:18`), add `#[serde(default)] pub floating_palettes:
Vec<crate::features::pages::floating_ui::persistence::PersistedPalette>,` to
`AppConfig` and `floating_palettes: Vec::new(),` to `Default`
(`config_store.rs:66-90,115-135`). No `normalized()` change: sanitize runs
at read (`slot`), keeping the config-store seam to two lines plus the import.

Tests (`persistence.test.rs`, plain `#[test]`; NO App global, because `AppState`
fields are private so an out-of-module test cannot seed it,
`config_store.rs:48-59`): `sanitize` drops unknown ids, non-finite coords,
non-positive/oversized dims, duplicates (first wins); normalizes surface;
caps at 16. `serde_json` round-trip of a fully populated `PersistedPalette`
verifies the serde shape (field names are the persisted schema). The live
`update_config` write path is already covered by
`src/state/config_store.test.rs` and by manual QA 5/7.
Done-check: `cargo test -p gpui-starter --lib floating_ui::persistence`
green; `cargo test -p gpui-starter --test snapshot_tests` fails only on the
two schema pins (fixed next step).

### Step 2.2: Schema pin updates

Files: `tests/snapshot_tests.rs`.

Build: add `floating_palettes: []` to the inline yaml snapshot
(`snapshot_tests.rs:26-52`) and `floating_palettes: Vec::new(),` to the
round-trip literal (`:57` on). If insta rejects the inline edit, accept-new
on that single test (`cargo insta review` or the env var); no other
snapshots change.

Tests: the two edited tests themselves.
Done-check: `cargo test -p gpui-starter --test snapshot_tests` green.

## Phase 3: Event pipeline

### Step 3.1: `AppEventKind::PaletteChanged` + drain arm

Files: `src/runtime/events.rs`, `src/shell/root/app_root/state.rs`.

Build:
- Variant `PaletteChanged { palette: String, value: String }` in
  `AppEventKind` (`events.rs:10-24`).
- AppRoot drain arm after `DiagnosticsChanged` (`state.rs:82`):
  `AppEventKind::PaletteChanged { palette, value } =>
  crate::features::pages::floating_ui::apply_change(&palette, &value, cx),`.
- No `cx.notify()` in the arm: the page and palette views observe
  `PaletteSyncState` directly, not AppRoot.

Tests (`state.test.rs`, `#[gpui_kit::test]`): seed the default global; call
`apply_change("color", "#FF0000", cx)` → `try_global::<PaletteSyncState>`
carries the value; `apply_change("bogus", ..)` leaves it unchanged; an
`observe_global::<PaletteSyncState>` subscription fired (flag pattern).
Done-check: `cargo test -p gpui-starter --lib floating_ui::state` green;
`grep -n "PaletteChanged" src/runtime/events.rs
src/shell/root/app_root/state.rs` shows both sites.

## Phase 4: Page shell and nav seam

### Step 4.1: `FloatingUiPage` skeleton + session stub

Files: `src/features/pages/floating_ui/mod.rs` (page lives here; it is the
module's public face), `src/features/pages/floating_ui/session.rs` (new),
`src/features/pages/mod.rs`.

Build:
- Minimal `session.rs`: `PaletteSurface { Window, Panel }` (+ `as_str`),
  `SessionSlot { open: bool, surface: PaletteSurface, origin:
  Point<Pixels>, window_origin: Point<Pixels>, size: Size<Pixels> }`,
  `PaletteSession { slots: BTreeMap<PaletteId, SessionSlot>, restored: bool,
  _subscriptions: Vec<Subscription> }`, `new(cx)` loading slots from
  `persistence::sanitize(&with_config(..).floating_palettes)` with defaults
  for missing ids (closed, Panel, origin `point(px(16.), px(16.))`, size
  `PaletteId::default_size`), plus `is_open`/`surface` readers. Spawn/close/
  registry logic arrives in Phase 5; this step needs the constructor and
  readers so the page and AppRoot compile.
- `mod.rs` grows `mod session;` + `pub use session::{PaletteSession,
  PaletteSurface};` and the page: `FloatingUiPage { session:
  Entity<PaletteSession>, _subscriptions: Vec<Subscription> }`;
  `new(session, cx)` stores the handle and subscribes
  `cx.observe_global::<PaletteSyncState>(|_, cx| cx.notify())`
  (`GPUI/src/app/context.rs:168-183`; no window needed, unlike the
  `observe_global_in` variant `notifications.rs:15` uses).
- `src/features/pages/mod.rs` gains `pub use floating_ui::FloatingUiPage;`.
- Locale keys land NOW, with the first rendered call sites, so a commit
  stopped at this step's done-check passes `tests/qa_localization`: add
  `nav_floating_ui`, `floating_ui_title`, `floating_ui_subtitle`,
  `floating_ui_persist_hint`, `floating_ui_sync_color`,
  `floating_ui_sync_asset` to BOTH `locales/en.yml` and `locales/zh-CN.yml`
  (values in the i18n table; quoted scalars after the `nav_*` block,
  `locales/en.yml:53-62`). Step 4.2 adds no further keys.
- Render (this phase, static): page heading patterned on
  `notifications.rs:37-59`: `v_flex().min_h_full().p_6().gap_4()`; heading
  div `.id("floating-ui-title").a11y(Role::Heading, title)
  .a11y_live(accesskit::Live::Polite).aria_level(1).text_xl()
  .font_weight(FontWeight::BOLD)`; subtitle `text_sm` muted
  (`floating_ui_subtitle`); a live-sync card: current color swatch
  `div().size_8().rounded(cx.theme().radius).bg(color)` where `color =
  hex_to_rgba(&sync.color_hex).unwrap_or_else(|| rgba(0x000000FF))`; parse
  failure falls back to opaque black, the same value class as the data it
  displays. Hex label `text_sm`, `last_asset` text, and a muted hint line
  (`floating_ui_persist_hint`).
- Palette control cards per `PaletteId::ALL` are Phase 8; this phase renders
  the sync card only.

Done-check: `cargo check -p gpui-starter` green (page type exists; not yet
routed); `cargo test -p gpui-starter --test qa_localization` green.

### Step 4.2: Sidebar/route seam

Files: `src/shell/sidebar.rs`, `src/shell/route.rs`,
`src/shell/root/app_root/state.rs`, `tests/e2e_navigation.rs`.

Build (Seam Touches rows 1, 2, 12 plus the AppRoot page wiring; the locale
keys from Seam row 14 landed in Step 4.1):
- `Page::FloatingUi` arms: title "Floating UI", `title_key`
  `nav_floating_ui`, host `"floating-ui"`, icon `IconName::Palette`
  (component default-list variant; `AppWindow` does not exist in that enum),
  `all()` entry appended after `Page::Gallery`.
- `VALID_HOSTS` entry appended.
- `AppRoot::new`: `let floating_ui_session = cx.new(|cx|
  PaletteSession::new(cx));` then `let floating_ui_page = cx.new(|cx|
  FloatingUiPage::new(floating_ui_session.clone(), cx));`; add the field,
  the import, and the `unchecked_active_page_view` arm (`Page::FloatingUi =>
  self.floating_ui_page.clone().into()`). The `floating_ui_host` field lands
  in Phase 7 with the host itself.
- `tests/e2e_navigation.rs`: 10 → 11 and add `Page::FloatingUi` to
  `expected`.

Tests: existing suites stay green.
Done-check: `cargo test -p gpui-starter --lib` green;
`cargo test -p gpui-starter --test e2e_navigation --test qa_localization`
green; sidebar shows the page.

## Phase 5: Session and native windows

### Step 5.1: `PaletteSession` core

Files: `src/features/pages/floating_ui/session.rs`, `mod.rs`,
`src/app/init.rs`.

Build (extends the Phase 4 stub):
- In `new(cx)`, after slot loading, subscribe
  `cx.observe_global::<crate::ui::floating::FloatingWindowRegistry>(|this, cx|
  this.sync_external_closes(cx))`: for each open Window slot whose
  `window_id` is absent from `floating_snapshot(cx)`, set `open = false` and
  `save_slot(open: false)`; `cx.notify()`. (Test runs without the
  `AppState` global: `save_slot` no-ops with a warn, so assertions target
  the slot state, not the file.)
- `toggle_window(id, cx)`: open Window → find the handle by id in
  `floating_snapshot` (entry fields `id`/`handle`, `02:338-344`),
  `close_floating(&handle, cx)` (`02:353-354`), slot closed, persist, notify.
  Closed/Panel → spawn:
  `#[cfg(target_family = "wasm")] return Err(anyhow!("native palette
  windows unavailable on wasm"));` else resolve the anchor: persisted
  `window_origin` plus `cx.primary_display().map(|d| d.visible_bounds())`
  → `restore_anchor`, else `FloatAnchor::Window { window:
  root_window(cx), placement: Placement9::TopRight, offset:
  point(px(16.), px(16.)) }`; when `root_window(cx)` is None (main window
  not yet recorded; only possible if a caller toggles before boot
  completes) fall back to `FloatAnchor::Screen { display: None,
  placement: Placement9::TopRight, offset: point(px(16.), px(16.)) }` so
  the spawn still resolves.
  Spec: `FloatingSpec::new(id.window_id(), slot.size)
  .title(id.localized_title()).level(FloatLevel::AlwaysOnTop)
  .anchor(anchor).focus(false).resizable(false).movable(true)
  .min_size(slot.size)`; build = `palette_view_factory(id)` (`views.rs`,
  Phase 6. Until then a temporary factory returning a placeholder `AnyView`
  (`div().p_4().child(title)`) keeps this phase compiling; replaced in
  Step 6.3); `spawn_floating(spec, build, cx)?`; slot `{open: true, surface:
  Window, window_origin: resolved}`, persist, notify. The persisted
  `window_origin` is the spawn-time resolved origin; plan 02 documents no
  move/position API (`02:74-78`), so OS-window drags after spawn are not
  trackable; `reset_window_position` is the documented re-anchor path.
- `reset_window_position(id, cx)`: if a window is open, close it; recompute
  the default anchor origin (main-window TopRight + 16,16 resolved against
  the current display) and write it into the persisted slot as the new
  restore origin; respawn via the `toggle_window` spawn path. No sentinel
  values: the persisted x/y always hold a usable origin.
- `set_panel_open(id, open, cx)`: slot `{open, surface: Panel}`, persist,
  notify. Opening a panel while a window is open closes the window first
  (the `toggle_window` close path): one surface per palette at a time.
- `restore_windows(cx)` (`pub(crate)`): if `restored` return; set flag; for
  each slot `open && Window` on native: run the `toggle_window` spawn path
  (it reads the persisted origin). Called from the AppRoot hook in Step 5.3.
- `init(cx)` in `mod.rs`: seed `PaletteSyncState` when absent
  (`try_global` → None: `set_global(PaletteSyncState { color_hex:
  rgba_to_hex(rgba(COLOR_PRESETS[0].1)), last_asset: None })`); capability
  entry `floating_ui_palettes`; native:
  `CapabilityStatus::supported_enabled()`; wasm: five-field literal
  `{ supported: false, enabled: true, degraded: true, reason: Some("native
  palette windows unavailable on wasm; in-window panels are the
  fallback".into()), last_error: None }` (`CapabilityStatus` has five pub
  fields and no `Default`, `capabilities.rs:6-13`). `init` does exactly
  this; boot restore is Step 5.3.
- Wire `crate::features::pages::floating_ui::init(cx);` into
  `src/app/init.rs` after `crate::notifications::initialize(cx);`
  (`init.rs:274`), i.e. after plan 02's `crate::ui::floating::init(cx);`
  line (plan 02 Seam #2).

Tests (`session.test.rs`, `#[gpui_kit::test]`; headless windows via the test
platform, `GPUI/src/platform/test/platform.rs:493-506`):
`toggle_window(Color)` → `floating_snapshot` contains `floating-ui-color`,
slot open+Window; `toggle_window` again → snapshot lacks the id, slot closed;
`set_panel_open` while a window is open → window closed first (snapshot
empty), slot Panel; `sync_external_closes`: spawn, then `close_floating`
directly (simulating plan 02's Esc path), then one further `cx.update`
boundary (registry observer flush, `GPUI/src/app.rs:1873-1877`) → slot
closed; `restore_windows` from a session whose slots were seeded with
`open: true, surface: "window"` spawns the window. Persistence side of these
flows is manual QA (Step 2.1 note).
Done-check: `cargo test -p gpui-starter --lib floating_ui::session` green.

### Step 5.2: Wasm refusal path

Files: `src/features/pages/floating_ui/session.rs`.

Build: the `#[cfg(target_family = "wasm")]` early return in `toggle_window`'s
spawn path (wasm never reaches `spawn_floating`; the `floating_snapshot`/
`close_floating` imports stay shared). `set_panel_open` uncfg'd. All session
persistence code shared (wasm config save is a no-op in the store,
`config_store.rs:521-528`).

Tests: compile-only on native; `just wasm-check` in Phase 9.
Done-check: `cargo check -p gpui-starter` green; wasm branch present.

### Step 5.3: Boot restore hook

Files: `src/shell/root/app_root/state.rs`.

Build: in `AppRoot::new`, after the session entity is created:
`cx.observe_global::<crate::app::window::RootWindow>(|this, cx| {
    this.floating_ui_session.update(cx, |s, cx| s.restore_windows(cx));
}).detach();`. The closure receives `&mut AppRoot` directly
(`Context::observe_global`, `GPUI/src/app/context.rs:168-183`), so
`this.floating_ui_session.update(cx, ...)` needs no extra plumbing.
`RootWindow` is set once by `create_new_window`
(`src/app/window.rs:13-15,107`); the `restored` flag makes re-fires no-ops.
Restore runs after the main window exists, so `root_window(cx)` anchors
resolve for the default-anchor path.

Tests: covered by the `restore_windows` test in Step 5.1.
Done-check: `grep -n "restore_windows" src/shell/root/app_root/state.rs`
shows the observer; `cargo test -p gpui-starter --lib` green.

## Phase 6: Palette content views

### Step 6.1: `StickyNoteView`

Files: `src/features/pages/floating_ui/views.rs` (new;
`floating_ui/mod.rs` grows `mod views;`), `locales/en.yml`,
`locales/zh-CN.yml`. Key landing with the first call site (the scanner
asserts every literal `localize("...")` resolves in both catalogs,
`tests/qa_localization.rs:146-190`): `floating_ui_note_placeholder` in
both files this step.

Build: `StickyNoteView { textarea: Entity<TextareaState>, _subscriptions:
Vec<Subscription> }`; `new(id: PaletteId, window, cx)`: build
`TextareaState::new(window, cx).placeholder(crate::i18n::localize(
"floating_ui_note_placeholder"))` (placeholder builder at
`GB/src/input/base/state.rs:814`, repo precedent `src/gallery/page.rs:22`),
seed the value from `persistence::slot(Note, cx).and_then(|p| p.note)
.unwrap_or_default()` via `set_value(.., window, cx)`
(`GB/src/input/base/state.rs:947-951`); subscribe
`cx.subscribe(&textarea, |_, _, ev: &InputEvent, cx| if matches!(ev,
InputEvent::Change) { persistence::save_slot(current_slot_with_note(textarea
.read(cx).value()), cx) })` (subscribe pattern `src/gallery/page.rs:23-27`;
the entry is rebuilt from the current slot read each event). Render:
`div().p_2().size_full().child(Textarea::new(&self.textarea).h_full())`
(`Textarea` is `Styled`, `textarea.rs:179`).

### Step 6.2: `ColorPaletteView` and `AssetPaletteView`

Files: `src/features/pages/floating_ui/views.rs`, `locales/en.yml`,
`locales/zh-CN.yml`. Keys landing with their first call sites this step:
`floating_ui_color_swatch`, `floating_ui_asset_pick` (both catalogs).

Build:
- `ColorPaletteView { _subscriptions }`; observes `PaletteSyncState`
  (`cx.observe_global::<PaletteSyncState>(|_, cx| cx.notify())`).
  Render: `v_flex().p_2().gap_2()`; grid `h_flex().flex_wrap().gap_1()` of
  swatches. For each `(slug, hex)` in `COLOR_PRESETS` compute `hex_str =
  rgba_to_hex(rgba(hex))` once, then:
  `div().id(ElementId::Name(SharedString::from(format!("swatch-{slug}"))))
  .size_6().rounded(cx.theme().radius).bg(rgba(hex))
  .when(selected, |this| this.border_1().border_color(cx.theme().primary))
  .hover(|this| this.border_1().border_color(cx.theme().border))
  .focus_visible(|this| this.border_1().border_color(cx.theme().primary))
  .a11y(Role::Button, format!("{} {hex_str}",
  crate::i18n::localize("floating_ui_color_swatch")))
  .aria_toggled(if selected { Toggled::True } else { Toggled::False })
  .on_click(...)` where `selected = sync.color_hex == hex_str` (`Toggled`
  comes from `gpui_kit`, `GPUI/src/gpui.rs:99`; there is no `aria_pressed`
  in this snapshot; verified). The click calls the factored
  `pub(crate) fn emit_color_change(hex: String, cx: &mut App)`:
  `events::emit(AppEventKind::PaletteChanged { palette: "color".into(),
  value: hex }, cx)` (factored so tests call it directly). Below the grid: a
  readout of `sync.color_hex`, `text_sm` muted.
  Component choice: the kit's color surface is a trigger+popover
  (`GC/src/color_picker.rs:72-74,544-546`), not an always-open grid; a
  stateful div with explicit a11y is deliberate, and swatch colors are data,
  the guides' audited color exception.
- `AssetPaletteView`: same shell; glyphs in `h_flex().flex_wrap().gap_1()` as
  `div().id(ElementId::Name(SharedString::from(format!("asset-{ix}"))))
  .p_2().rounded(cx.theme().radius).text_lg().child(glyph)` (ix = preset
  index; the preset list is a fixed compile-time constant, so index-derived
  ids are stable identity here); a11y label via
  `crate::i18n::localize_with_args("floating_ui_asset_pick", &[("asset",
  glyph)])`; `.on_click` calls the factored `emit_asset_change(glyph.
  to_string(), cx)`.

### Step 6.3: `palette_view_factory`

Files: `src/features/pages/floating_ui/views.rs`, `session.rs` (drop the
placeholder factory).

Build: `pub(crate) fn palette_view_factory(id) -> Rc<dyn Fn(&mut Window,
&mut App) -> AnyView>` matching per id: `cx.new(|cx|
ColorPaletteView::new(window, cx)).into()` etc. (`Entity<V: Render>` →
`AnyView` via `.into()`, `state.rs:213` precedent). `Fn` not `FnOnce`:
plan 02 retains the factory for respawn (`02:412-418`). Replace Step 5.1's
placeholder.

Tests (`views.test.rs`, `#[gpui_kit::test]`): `emit_color_change` pushes one
event; assert via `events::drain(cx)` returning a single `PaletteChanged`
with the right fields; `emit_asset_change` likewise; construct all three
views in a test window: `cx.update(|cx| cx.open_window(
WindowOptions::default(), |window, cx| cx.new(|cx|
StickyNoteView::new(PaletteId::Note, window, cx))))` and assert construction
does not panic (test platform opens windows headless,
`GPUI/src/platform/test/platform.rs:493-506`).
Done-check: `cargo test -p gpui-starter --lib floating_ui::views` green.

## Phase 7: Panel host (in-window draggable snap-to-edge panels)

### Step 7.1: Host layout

Files: `src/features/pages/floating_ui/panel_host.rs`,
`src/features/pages/floating_ui/mod.rs`,
`src/shell/root/app_root/state.rs`, `src/shell/root/app_root/render.rs`,
`locales/en.yml`, `locales/zh-CN.yml`. Key landing with its first call
site: `floating_ui_close` (both catalogs).

Build:
- `PanelGeom { origin: Point<Pixels>, size: Size<Pixels> }`;
  `PalettePanelHost { session: Entity<PaletteSession>, geoms:
  BTreeMap<PaletteId, PanelGeom>, content: BTreeMap<PaletteId, AnyView>,
  drag: Option<ActiveDrag>, _subscriptions: Vec<Subscription> }` where
  `ActiveDrag { id: PaletteId, last_cursor: Point<Pixels> }`.
  `new(session, window, cx)`: for each id, `PanelGeom { origin: persisted
  x/y (stored raw; clamped at drag/render time when a viewport exists),
  size: persisted or default_size }`; content entities
  `cx.new(|cx| <View>::new(id, window, cx))` per id; subscribes
  `cx.observe(&session, |_, _, cx| cx.notify())`.
- Render: root `div().absolute().top_0().left_0().size_full()`, the same
  out-of-flow pattern and rationale as plan 02's overlay host (`02:697-713`:
  an in-flow `size_full()` child would join the AppRoot column with default
  `flex_shrink 1.0` and steal `content_area`'s `flex_1()` height,
  `src/shell/root/app_root/render.rs:266-268`; absolute panes resolve against
  the closest positioned ancestor, so the host box must equal the window).
  Empty (no open panels) → childless div, no `.id()`, no hitbox, no a11y
  node (`02:710-713`).
- Per open panel slot (`session.is_open && surface == Panel`): pane
  `div().id(ElementId::Name(SharedString::from(format!(
  "palette-panel-{slug}"))))
  .absolute().top(px(origin.y)).left(px(origin.x))
  .w(px(size.width)).h(px(size.height))
  .a11y(Role::Pane, localized_title).bg(cx.theme().popover)
  .border_1().border_color(cx.theme().border).rounded(cx.theme().radius)
  .shadow_md()` (`shadow_md` at `gpui-pre-macros-0.3.8/src/styles.rs:441`;
  `px()` for measured/persisted geometry is the documented exception in the
  design guides' "Zoom, base font, and rem" rule; plan 02 overlay panes take
  it too, `02:688-691`). Header row (compact, 32px, `text_sm` muted title)
  with an icon-only close button mirroring plan 02's shell:
  `Button::new(("palette-panel-close", slug)).ghost()
  .icon(Icon::new(IconName::Close))
  .accessibility_label(crate::i18n::localize("floating_ui_close"))
  .on_click(...)` → `session.update(cx, |s, cx| s.set_panel_open(id, false,
  cx))`. `IconName::Close` is the component default-list close variant
  (`close` in `default-icons.txt`; `x` is absent from that enum, see Research
  Digest); `accessibility_label` at `GC/src/button/button.rs:391`; repo
  icon-button precedent `src/gallery/sections/buttons/button.rs:199`.
  Content child below the header.
- Module mount: `floating_ui/mod.rs` grows `mod panel_host;` + `pub use
  panel_host::PalettePanelHost;`.
- AppRoot: add the `floating_ui_host` field (Seam #4), construct it after
  the session (`cx.new(|cx| PalettePanelHost::new(floating_ui_session.
  clone(), window, cx))`), and render `.child(self.floating_ui_host.clone())`
  as the LAST child of the root `v_flex` (`render.rs:287-291`), after plan
  02's `floating_host` (Seam #8; tree order is the only stacking mechanism).

Done-check: `cargo check -p gpui-starter` green;
`grep -n "floating_ui_host" src/shell/root/app_root/render.rs` shows the
last child.

### Step 7.2: Drag and snap

Files: `src/features/pages/floating_ui/panel_host.rs`.

Build:
- `struct PanelDrag { id: PaletteId }` (payload type).
- Header strip: `.id(ElementId::Name(SharedString::from(format!(
  "palette-panel-header-{slug}"))))` (stateful; `on_drag`/`on_drop` are
  stateful-element APIs), `.cursor_grab()`
  (`gpui-pre-macros-0.3.8/src/styles.rs:247-251`), `.on_drag(PanelDrag {
  id }, move |_, _, _, cx| cx.new(|_| DragGhost))` where `struct DragGhost;
  impl Render for DragGhost { fn render(..) -> impl IntoElement { div() } }`
  is a zero-size invisible preview (`GPUI/src/elements/div.rs:615-630`; drag
  starts after the 2px threshold, `:52`, `:2983-2996`).
- Header `.on_drag_move(cx.listener(|this, ev: &DragMoveEvent<PanelDrag>,
  window, cx| { let pos = ev.event.position; let Some(active) =
  this.drag.as_mut() else { this.drag = Some(ActiveDrag { id:
  ev.drag(cx).id, last_cursor: pos }); return; }; let delta = pos -
  active.last_cursor; let Some(geom) = this.geoms.get_mut(&active.id) else {
  return; }; geom.origin = clamp_origin(geom.origin + delta, geom.size,
  window.viewport_size()); active.last_cursor = pos; cx.notify(); }))`.
  Keying the initial `ActiveDrag` off `ev.drag(cx).id` (not off which
  header's listener ran) is required: `on_drag_move` fires on EVERY header
  while any `PanelDrag` is active (`div.rs:359-381`), so the first listener
  to run seeds the true payload id and the rest see `this.drag` already set
  and compute a zero delta for the same `pos`.
- Pane root `.on_drop::<PanelDrag>(cx.listener(|this, _: &PanelDrag,
  window, cx| { let Some(active) = this.drag.take() else { return; };
  this.commit_drag(active.id, window, cx); }))`: drop fires on mouse-up
  hovered over the pane (`div.rs:2904-2933`); the pane follows the cursor,
  so hover holds; the drag session always ends on mouse-up regardless
  (`GPUI/src/window.rs:5829-5839`), so `this.drag` cannot leak past the next
  mouse-up.
- `fn commit_drag(&mut self, id: PaletteId, window: &mut Window, cx: &mut
  Context<Self>)`: snap the pane origin (`snap_origin`), `persist_geom(id,
  cx)`, `cx.notify()`; this is the factored release path the test calls directly.
- `persist_geom(id, cx)`: `persistence::save_slot(entry with panel x/y +
  current open/surface/size, cx)`; geometry is the host's to persist,
  open/surface remain the session's.
- Clamp-on-viewport-change: origins are clamped every drag move and inside
  `snap_origin`; a window resize without a drag leaves stale-but-bounded
  origins until the next interaction; accepted (same one-shot geometry
  semantics as plan 02 overlay floats, `02:1046-1053`).

Tests (`panel_host.test.rs`, `#[gpui_kit::test]`): construct session+host;
zero open slots → render in a test window produces no `palette-panel-*`
pane; open two panel slots via `set_panel_open` → two panes; drag commit
math: call the host's `commit_drag(id, window, cx)` with a pane parked 10px
from the left edge and assert the geometry snapped flush. Pointer-level drag
simulation needs kit `test-support`, which this repo does not enable (survey
`seams.md:209-212,273-280`); the interaction itself is manual QA 6, stated
honestly.
Done-check: `cargo test -p gpui-starter --lib floating_ui::panel_host`
green.

## Phase 8: Page controls and gating

### Step 8.1: Control cards

Files: `src/features/pages/floating_ui/mod.rs` (page),
`locales/en.yml`, `locales/zh-CN.yml`.

Build: for each `PaletteId::ALL`, a card row. Card icon per id, all from
the component default 106 the app actually registers (the full-catalog
`Layers`/`StickyNote` compile via the assets enum but render blank on
native — the app's asset source embeds only the defaults): Color →
`Icon::new(IconName::Palette).small()`,
Assets → `Icon::new(IconName::Inbox).small()`, Note →
`Icon::new(IconName::FileText).small()`. All three verified in
`gpui-kit-assets-0.7.1/default-icons.txt`. `Icon::new` at
`GC/src/icon.rs:105`, `impl Sizable for Icon` at `:215`. Then localized
title + one-line description (`floating_ui_palette_*` keys), an open-state
indicator (kit `Badge`, child-carrying RenderOnce, `GC/src/badge.rs:41-99`;
neutral variant; text = surface label) when open, and buttons:
- Window toggle: `Button::new(("floating-ui-window", slug)).outline()
  .label(localize(if window_open { "floating_ui_close_window" } else {
  "floating_ui_open_window" })).disabled(!native_windows).on_click(...)` →
  `session.update(cx, |s, cx| { if let Err(err) = s.toggle_window(id, cx) {
  tracing::warn!(target: "gpui_starter::floating_ui", error = %err,
  "palette window toggle failed"); } })`. `native_windows` comes from
  `crate::ui::floating::probe(cx).native_windows` (`02:371-379`); on wasm
  the button renders disabled and the `floating_ui_window_unavailable`
  muted line renders beneath the card group ("Hide an inapplicable action
  or disable it with a useful reason"). A failed native spawn leaves the
  card showing the palette closed, so state stays honest; the warn is the
  diagnostics trail (no `From<anyhow::Error> for AppError` exists,
  `src/foundation/errors.rs:14-33`, so `events::emit_error` is not an
  option here).
- Panel toggle: same shape; label flips between `floating_ui_show_panel`
  and `floating_ui_hide_panel` based on `session.surface(id) == Some(Panel)
  && is_open(id)`; on_click → `session.update(... set_panel_open(id,
  !open, cx))`.
- Reset: `Button::new(("floating-ui-reset", slug)).ghost()
  .label(localize("floating_ui_reset_position"))`, rendered only while that
  palette's window is open; on_click → `session.update(...
  reset_window_position(id, cx))`.
- All buttons call the same session methods any other path uses: one
  command, one implementation (coding guides "Events, actions, and focus").
- Keys added this phase (call sites are the Phase 8 controls; every earlier
  key — including the `floating_ui_palette_*` trio, which landed with their
  first call site in Step 1.1 — landed with its own step):
  `floating_ui_open_window`, `floating_ui_close_window`,
  `floating_ui_show_panel`, `floating_ui_hide_panel`,
  `floating_ui_reset_position`, `floating_ui_window_unavailable`.

Tests: page construction smoke in `views.test.rs`: `FloatingUiPage::new`
with a seeded session renders in a test window without panic and the sync
card reflects `PaletteSyncState`.
Done-check: `cargo test -p gpui-starter --lib floating_ui` green;
`cargo test -p gpui-starter --test qa_localization` green.

## Phase 9: Gates

### Step 9.1: Full gates

Commands, in order (AGENTS.md):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

Build: fix fallout only. No Cargo.toml/Cargo.lock changes exist in this
plan; verify `git diff --stat` touches only Seam Touches files plus
`src/features/pages/floating_ui/`.
Done-check: all four pass; `git diff --stat` lists only those files.

## Platform & Wasm Gating

cfg gates:

- `session.rs`: `toggle_window` spawn path behind
  `#[cfg(not(target_family = "wasm"))]` with a
  `#[cfg(target_family = "wasm")]` early-return `Err`; close path, panel
  path, persistence, registry observation uncfg'd (all compile and run on
  wasm; the registry simply never gains native entries there).
- No cfg in AppRoot, page, views, host: all exist on every platform; on wasm
  the host is the palette surface and the native buttons disable with a
  visible reason.
- No new dependencies anywhere; the wasm and not-wasm dep tables
  (`Cargo.toml:310-417`) and `Cargo.lock` are untouched.

Capability-registry entry (written by `floating_ui::init(cx)` from
`src/app/init.rs:274`; rendered on the diagnostics page automatically,
survey `seams.md:118-131`):

| Key | macOS | Windows | Linux | wasm |
| --- | --- | --- | --- | --- |
| `floating_ui_palettes` | supported_enabled | supported_enabled | supported_enabled | supported:false, enabled, degraded, reason "native palette windows unavailable on wasm; in-window panels are the fallback" |

Platform behavior matrix (what each platform gets, including hidden):

| Behavior | macOS | Windows | X11 | Wayland | wasm |
| --- | --- | --- | --- | --- | --- |
| Palette as window | NSPopUp-level panel above app, all spaces (plan 02 `02:83`) | WS_EX_TOPMOST tool window (`02:84`) | NOTIFICATION-type window, WM-dependent stacking (`02:85`) | xdg child toplevel (`02:86`) | n/a (panel only) |
| Window drag | native titlebar strip (plan 02 shell, `02:596-599`) | not movable (PopUp style 0; plan 02 risk 2, `02:1028-1033`); "Reset position" is the re-anchor path | `start_window_move` header (`02:596-599`) | `start_window_move` header | n/a |
| Window resize | disabled (`resizable(false)`, forced by AlwaysOnTop per `02:617-618`) | same | same | same | n/a |
| Panels | in-window, draggable, edge-snap | same | same | same | same (the primary surface) |
| Position restore | spawn-time origin, clamped to visible bounds | same | same | same | in-memory only (config save no-op, `config_store.rs:521-528`) |
| Live color sync | event queue → both windows redraw | same | same | same | same window only (panels), queue still exercised |

## i18n

Crate facade `crate::i18n::localize` / `localize_with_args`
(`src/services/i18n.rs:3-13`); keys land in BOTH catalogs in lockstep
(`tests/qa_localization.rs:188-198`); snake_case top-level quoted scalars
(`tests/qa_localization.rs:20-71`). All call sites use literal first
arguments so the scanner sees every key.

| Key | en | zh-CN |
| --- | --- | --- |
| `nav_floating_ui` | "Floating UI" | "浮动工具" |
| `floating_ui_title` | "Floating UI" | "浮动工具" |
| `floating_ui_subtitle` | "Tool palettes as always-on-top windows, with in-window panels as the portable fallback." | "常置顶的工具调板窗口；窗口内面板作为可移植回退。" |
| `floating_ui_persist_hint` | "Position and visibility are restored on launch." | "启动时恢复位置与可见状态。" |
| `floating_ui_sync_color` | "Synced color" | "同步颜色" |
| `floating_ui_sync_asset` | "Last picked" | "最近选择" |
| `floating_ui_palette_color` | "Color palette" | "调色板" |
| `floating_ui_palette_assets` | "Asset palette" | "素材面板" |
| `floating_ui_palette_note` | "Sticky note" | "便签" |
| `floating_ui_open_window` | "Open window" | "打开窗口" |
| `floating_ui_close_window` | "Close window" | "关闭窗口" |
| `floating_ui_show_panel` | "Show panel" | "显示面板" |
| `floating_ui_hide_panel` | "Hide panel" | "隐藏面板" |
| `floating_ui_reset_position` | "Reset position" | "重置位置" |
| `floating_ui_close` | "Close" | "关闭" |
| `floating_ui_color_swatch` | "Color" | "颜色" |
| `floating_ui_asset_pick` | "Asset %{asset}" | "素材 %{asset}" |
| `floating_ui_note_placeholder` | "Write a note" | "写点笔记" |
| `floating_ui_window_unavailable` | "Native palette windows are unavailable in this build; panels are shown instead." | "此构建不支持原生调板窗口，请改用面板。" |

`localize_with_args` placeholder form `%{asset}` matches
`src/services/i18n.rs:7-13` (call-site precedent `src/app/init.rs:379-383`).
zh-CN short labels carry no terminal punctuation; the two sentence-values end
with 。 per the design guides' CJK punctuation rule.

## Design Compliance

- Surfaces and elevation: panes reuse the popover family treatment
  (`cx.theme().popover`, hairline `border`, `cx.theme().radius`, one
  `shadow_md`) per "all surfaces of the same kind should share one treatment";
  native palette windows get their chrome from plan 02's `FloatingShell`
  (same tokens, `02:576-596`) plus the OS shadow; no second invented
  surface. No raw colors except swatch/preset data values, the guides'
  audited data-color exception ("Color and themes").
- Density: palette grids and panel headers are compact contexts: `size_6`
  swatches, `gap_1` grid, 32px header, `text_sm` muted titles ("compact for
  toolbars and repeated professional data", "Density tiers"). Page copy
  stays medium.
- Tokens and spacing: all spacing via scale helpers (`p_2`, `p_6`, `gap_1`,
  `gap_2`, `gap_4`); the only `px()` uses are measured/persisted pane
  origins and sizes, the documented physical-geometry exception plan 02's
  overlay panes also take (`02:688-691`; "Zoom, base font, and rem").
- Interaction states: every swatch/glyph has rest, hover (hairline ring),
  focus-visible ring, and `aria_toggled` for the selected swatch, per "State
  must be visible"; disabled window buttons on wasm lower emphasis and the
  reason is visible text, not a tooltip-only cue ("Hide an inapplicable
  action or disable it with a useful reason" / "Feedback and overlays").
- Buttons and labels: page actions are result verbs in sentence case ("Open
  window", "Show panel", "Reset position"); no ellipsis (they act
  immediately); the close icon button carries an accessible name
  (`floating_ui_close`); "icon-only buttons require a tooltip and an
  accessible name" (name delivered via accessibility_label). Palette titles
  are localized nouns.
- A11y: pane roots expose `Role::Pane` with the palette title; swatches and
  glyphs are `Role::Button` with names; the page heading is a level-1
  heading with a polite live region (notifications-page pattern,
  `notifications.rs:48-59`); color changes are announced through the sync
  card's live re-render; every command has a keyboard path: window/panel
  toggles and reset are page buttons reachable by Tab/Enter
  ("Accessibility checklist"). Pointer-only drag is a documented gap
  (Risks); panels stay fully operable without dragging (open/close/reset
  from the page).
- Motion: none added; panel snap is an immediate ≤24px position commit
  ("Motion explains change"; no default animation).
- Platform fit: same information architecture on every platform; wasm shows
  panels plus a visible reason instead of hiding the feature (coding guides
  "Platform and capability boundaries"; semantic contract preserved:
  palettes open, persist, and sync everywhere; only the surface differs).

## Verification

Repo gates (AGENTS.md; run in Phase 9 exactly as named):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

`just wasm-check` is the wasm proof: the lib-only nightly check
(`justfile:24-25`) compiles the session's wasm refusal branch, the panel
host as sole palette surface, and the no-op config save path.

Feature-specific manual QA (macOS dev flow `bash scripts/macos-dev-app.sh`,
CONTRIBUTING.md:32-38; wasm via `just wasm-serve`):

1. Sidebar → "Floating UI"; page renders heading, sync card, three palette
   cards.
2. "Open window" (Color): a borderless always-on-top palette opens at the
   main window's top-right; the main window keeps focus; the card badge
   shows the window surface.
3. Click swatches in the palette window: the page's synced-color swatch and
   hex update live in the main window without focusing it (event-queue
   sync); the selected ring in the palette window follows.
4. Open the Asset palette as a window too; picking glyphs updates "Last
   picked" live in the main window while the palette window also re-renders.
5. Sticky note window: type text; quit and relaunch: note text, open
   surface, and spawn origin restore from `AppConfig` (inspect the state
   file's `floating_palettes` array if in doubt).
6. "Show panel" (any palette on native, all palettes on wasm): an in-window
   panel appears; drag by its header: it follows the cursor, clamps at
   window edges, and snaps flush when released within 24px of the left/right
   edge; "Hide panel" and the panel close button both close it; relaunch
   restores the snapped origin.
7. Esc in a palette window closes it (plan 02 binding); the page card flips
   to closed and the config persists `open: false` (relaunch confirms).
8. Diagnostics page: `floating_ui_palettes` supported/enabled (native) or
   degraded with the wasm reason.
9. Wasm (`just wasm-check` then `just wasm-serve`): page shows disabled
   window buttons plus the unavailable reason; panels drag and snap inside
   the browser window; the sync card updates on swatch click.
10. Windows/Linux spot check (if hosts available): Windows palette windows
    are not draggable; verify "Reset position" respawns at the default
    anchor; on Linux the WM may not keep the palette above other apps
    (plan 02 risk 1, `02:1022-1027`), which the capability/diagnostics
    entry states.

Acceptance criteria:

- All four gates pass; `git diff --stat` shows only Seam Touches files plus
  `src/features/pages/floating_ui/`.
- Color picked in a separate palette window appears in the main window
  without focusing it (manual QA 3), the event-queue path, backed by the
  `state.test.rs` observer test.
- Relaunch restores open palettes (windows at persisted origins; panels at
  persisted, snapped origins) and the sticky note text (QA 5/6).
- `tests/qa_migration.rs` passes unchanged (no `use gpui::` /
  `gpui_component::` imports anywhere in the new module; kit facade only).
- Locale catalogs in lockstep (`tests/qa_localization.rs` passes); every key
  in the i18n table present in both files.

## Risks & Open Questions

Ranked:

1. **Plan 02 is not implemented yet** (`ls src/ui/` shows only `widgets/`;
   no `floating` module; checked this session). This plan consumes plan
   02's contract as written (`02:279-407`). If the implemented surface
   drifts (e.g. `floating_snapshot` shape), `session.rs` is the single
   consumer seam to adjust. Tried: `ls docs/plans/` (01–11 and 16 exist),
   `ls src/ui/`, grep for `floating` under `src/`: plan 02 exists only as
   a document.
2. **Panel drag release depends on `on_drop` hover.** Drop listeners fire
   only when mouse-up lands hovered over the pane
   (`GPUI/src/elements/div.rs:2904-2933`); the pane follows the cursor each
   move, so the pointer stays inside, but a mouse-up delivered while the
   pointer is outside the window can leave the panel unsnapped (the gpui
   drag session still ends on that mouse-up, `GPUI/src/window.rs:5829-5839`).
   Mitigation: the unsnapped geometry persists on the next toggle/drag;
   snap correctness is manual QA 6; no crash path exists.
3. **Windows palette windows cannot be dragged or resized** (PopUp style 0,
   `start_window_move` no-op; plan 02 risk 2, `02:1028-1033`). "Reset
   position" is the documented re-anchor path; position persistence on
   Windows is spawn-time only. Not runtime-verifiable in this session (no
   Windows host).
4. **Linux always-on-top is WM-dependent** (plan 02 risk 1, `02:1022-1027`):
   palettes may not stay above other apps on some WMs; the capability
   entry and probe facts carry the caveat.
5. **Restored native windows position at spawn-time origins only.** Plan 02
   documents no move/position API (`02:74-78`, `02:1046-1053`); OS-level
   drags after spawn are not reflected in the persisted origin. Documented
   in the Interface Contract; `reset_window_position` is the recovery path.
6. **Drag/geometry UI-level tests are manual.** Kit `test-support`
   (TestWindowExt/ElementSnapshot) is not enabled in this repo (survey
   `seams.md:209-212` and its Open Questions; only the gpui crate's
   `test-support` dev-dep exists, `Cargo.toml:212-216`), so pointer-level
   drag simulation and pane-geometry assertions are out of reach; the
   geometry math is unit-tested pure (`geometry.test.rs`,
   `commit_drag`) and the interaction is manual QA 6. Tried: survey +
   Cargo.toml feature/dev-dep list.
7. **Live persistence writes are not unit-tested.** `AppState` fields are
   private (`config_store.rs:48-59`), so an out-of-module test cannot seed
   the store without touching the real user config directory; the
   debounced-write machinery is covered by `config_store.test.rs` and the
   feature's persistence is manual QA 5/6/7. Tried: looked for a public
   test constructor (`clean_state` lives inside the in-module test file,
   `config_store.test.rs:265-270`).
8. **Two stacked absolute layers** (plan 02 overlay host + palette panel
   host) rely on tree order for stacking (no `z_index`, `02:138-139`).
   Palette panels intentionally sit above everything, including plan 02
   overlay floats; documented in Seam #8. If plan 03's notch floater must
   sit above palettes, that plan's host mounts later in the tree; no
   conflict today.
9. **Emoji glyph rendering on minimal Linux setups** depends on an
   installed emoji font; a missing font shows tofu. The glyph set is data;
   no runtime font probing. Not verifiable in this session.
10. **`AppConfig` schema growth**: `floating_palettes` is additive with
    `#[serde(default)]` (sanctioned, `config_store.rs:63-65`); snapshot
    fixtures updated in Phase 2. Old configs without the field load as
    empty (all palettes closed); intended.

Reviewer notes that required no plan change: the API/seam spot-check
confirmations, the phase-order and removal-recipe approvals, and the plan-02
unimplemented-status disclosure (already risk 1) were applied as
confirmations only. Plan 02's "six pub fields" wording for `CapabilityStatus`
(02:811) is plan 02's error, not this plan's; the actual count is five
(`src/runtime/capabilities.rs:6-13`), stated correctly above.
