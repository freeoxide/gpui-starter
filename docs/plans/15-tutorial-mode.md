# 15 Tutorial Mode Plan

## Goal

A guided-tour overlay walks the user through existing UI: a full-window dim mask with a cutout spotlight and glow ring on the current target, plus a tooltip card (title, body, Back/Next/Skip, progress dots) driven by keyboard (arrows, Enter, Escape) and pointer. Targets are resolved through a registry that pages feed by wrapping elements with a capture helper, so any page can publish spotlight bounds by stable id. A demo tour covers Home and Gallery, respects `App::reduce_motion`, and announces step changes through the existing live-region pattern.

## Dependencies

None. This plan consumes no other plan's Interface Contract; it builds on repo seams only (events queue, capabilities, command registry, i18n facade, A11yExt live regions, all cited in Research Digest). Plans that want spotlight tours consume this plan's contract (see Interface Contract).

## Research Digest

Every gpui/gpui-kit claim below was verified against the crate sources on disk this session (registry key `index.crates.io-1949cf8c6b5b557f`, located with `ls ~/.cargo/registry/src/*/ | grep -E 'gpui-pre-0.3.8|gpui-kit-0.7.1'`). Short paths: `GPUI` = `gpui-pre-0.3.8/src`, `GC` = `gpui-component-0.7.1/src`, `GB` = `gpui-base-0.7.1/src`.

### Target bounds: AccessKit is not a path; paint/prepaint-time capture is

- `window/a11y.rs:150` — `pub(crate) node_bounds: FxHashMap<NodeId, Bounds<Pixels>>`. The AccessKit node-bounds map is crate-private; there is no public query. The only public `element_bounds` is the IME `InputHandler` path (`GPUI/platform.rs:2110-2113`). So "resolve bounds via AccessKit" is not available to app code. Tried: `grep -rn "node_bounds|element_bounds"` across `GPUI` — all hits are `pub(crate)` or input-handler plumbing.
- Prepaint/paint capture is fully public. `Element::prepaint` receives the element's resolved `bounds: Bounds<Pixels>` (`GPUI/element.rs:85-93`); the one-off `canvas(prepaint, paint)` element hands those bounds to a closure (`GPUI/elements/canvas.rs:9-22`), and `Canvas` implements `Styled` (`canvas.rs:91-95`), so `.absolute().size_full().top_0().left_0()` works on it, exactly the idiom `GB/popup.rs:145-180` uses to capture trigger bounds.
- Layout-transparent wrapper precedent: `AnimationElement` implements `Element` by delegating to a child `AnyElement`: `request_layout` returns `(child.request_layout(window, cx), child)`, `prepaint`/`paint` call `element.prepaint(window, cx)` / `element.paint(window, cx)` (`GPUI/elements/animation.rs:382-493`). Because the wrapper returns the child's `LayoutId`, the `bounds` argument the wrapper's `prepaint` receives is the child's resolved bounds. `AnyElement::request_layout/prepaint/paint` are public (`GPUI/element.rs:611-631`). `TourTarget` copies this template and publishes `bounds` in `prepaint`.
- Frame ordering (verified, load-bearing): a view's `render()` runs inside the layout pass — `ViewElement::request_layout` calls `view.render(window, cx)` (`GPUI/view.rs:324-341`, render at `:339`) — and `draw_roots` lays out the whole root tree (`root_element.request_layout`, `GPUI/window.rs:3581`) before the prepaint pass begins (`root_element.prepaint_as_root`, `GPUI/window.rs:3586`). `TourTarget` publishes during prepaint, so any view render — including the tour controller's — reads registry values written by the previous frame's prepaint. The overlay is one frame behind target geometry, never same-frame current. The kit behaves the same way: `GB/popup.rs:145-180` re-requests a frame (`window.request_animation_frame()` after the first capture) rather than assuming same-frame visibility.
- Freshness: hitboxes and listeners are per-frame buffers, cleared when the frame renders (`GPUI/window.rs:5312-5316` doc); every drawn frame re-runs prepaint for the whole tree, so a `TourTarget` re-publishes its bounds every drawn frame even when its owning view was not notified. Combined with the one-frame-behind read above: static targets already carry correct bounds when a step lands on them, moving targets (resize, scroll) converge one frame later via the change-notify, and the waiting placeholder (Step 4.2) is a required path after route changes, not defensive code.
- `From<&'static str> for ElementId` exists (`GPUI/window.rs:7556`); `From<(&'static str, usize)>` too (`:7574`).

### Overlay mechanics

- `Style::default()` is `Display::Block`, `Position::Relative` (`GPUI/style.rs:774,783`). `Position::Absolute` places the element "relative to this item's closest positioned ancestor, if any. Otherwise… relative to the origin" and takes it out of flow (`style.rs:1236-1246`). Since every div defaults to Relative, an `.absolute().size_full().top_0().left_0()` child of `AppRoot`'s root `v_flex` covers the window. `.left(px(..))/.top(px(..))` position setters are in kit use (`GB/resizable/resize_handle.rs:560-561`, `GB/text_selection.rs:3051`).
- Mouse blocking: `InteractiveElement::occlude()` sets `HitboxBehavior::BlockMouse` (`GPUI/elements/div.rs:1250` → `:743-745`); a hitbox is inserted whenever `hitbox_behavior != Normal` (`div.rs:2407-2408`), no `.id()` required (we add one anyway). BlockMouse makes hitboxes behind it report not-hovered/not-scrolled (`GPUI/window.rs:888-902`), which is what a modal scrim wants. Children painted later keep their own hitboxes above the occluding one; this is how `GB/alert_dialog.rs:49` keeps "presses on the popup… off the backdrop behind it".
- Keyboard: `key_context` on `InteractiveElement` (`GPUI/elements/div.rs:830`); `KeyBinding::new(keystrokes, action, context: Option<&str>)` (`GPUI/keymap/binding.rs:33`). The kit's own menus bind exactly our keys with a context scope: `KeyBinding::new("enter", …, Some(CONTEXT))`, `"escape"`, `"left"`, `"right"` (`GC/menu/popup_menu.rs:22-29`). Repo keybinding precedent: `src/app/init.rs:289-298`, `src/shell/root/app_root/state.rs:144-166`.
- Focus: `Window::focused(cx) -> Option<FocusHandle>` (`GPUI/window.rs:2329`); `FocusHandle::focus(window, cx)` (`window.rs:610`; used at `src/features/command_palette.rs:335-337`); `.track_focus(&handle)` (`src/shell/root/app_root/render.rs:227`). Focus trap: `.focus_trap(id, &handle)` from `gpui_kit::component::FocusTrapElement` (`command_palette.rs:5,189-192`); trap a roleless wrapper, not the role-bearing node, or gpui drops the wrapped node's a11y (`command_palette.rs:186-188` comment).
- Window access from App level: `root_window(cx)` global (`src/app/window.rs:11-22`); `AnyWindowHandle::update(cx, |AnyView, &mut Window, &mut App| R)` (`GPUI/window.rs:7405-7417`). `gpui_kit::open_window` wraps the view in `base::Root` (`gpui-kit-0.7.1/src/lib.rs:149-164`), so the kit Root's deferred layers (dialogs, notifications) paint above `AppRoot`'s subtree, i.e. above the tour overlay. Not run-verified; see Risks.
- Entity plumbing: `WeakEntity::update(&self, cx, f)` works with `&mut App` (`GPUI/app/entity_map.rs:803-813`); `Entity::downgrade()` (`entity_map.rs:470`); `cx.entity()` (`command_palette.rs:129`).

### Painting the mask and glow

- `window.paint_path(path, color)` (`GPUI/window.rs:4621`); `PathBuilder` (`GPUI/path_builder.rs:25`): `move_to :125`, `line_to :130`, `arc_to :155`, `close :199`, `build() -> Result<Path<Pixels>> :244`; style via `PathStyle::Fill(FillOptions)` (`:19-21`). `FillRule` is re-exported (`path_builder.rs:12`), `FillOptions::with_fill_rule` exists (`lyon_tessellation-1.0.20/src/lib.rs:458,515`), and EvenOdd is already the `fill_rule` default (`lyon_tessellation-1.0.20/src/lib.rs:457-458` field doc; repo `Cargo.lock` carries lyon 1.0.x). An even-odd fill of one outer rectangle subpath plus one rounded-rect subpath is the dim mask with a true rounded cutout.
- `window.paint_quad(quad(bounds, corner_radii, background, border_widths, border_color, border_style))` (`GPUI/window.rs:4550`, free fn `quad` at `:7675-7687`); `BorderStyle::{Solid, Dashed}` (`GPUI/scene.rs:597-603`); `Edges::all(v)` (`GPUI/geometry.rs:1820`), `Corners::all(v)` (`:2309`), `From<f32> for Corners<Pixels>` (`:2568`); `transparent_black()` used as quad background (`window.rs:7694-7703`, `command_palette.rs:176`).
- `Bounds` helpers: `top/bottom/left/right` (`GPUI/geometry.rs:1296-1323`), `intersect` (`:1143`), `contains` (`:1468`).
- Dim color: `cx.theme().overlay` is the themed scrim the kit's own Dialog uses (`GC/dialog/dialog.rs:319-326`; field at `GC/theme/theme_color.rs:311`). Glow ring: `cx.theme().primary`, radius `cx.theme().radius_lg` (used at `render.rs:62`).

### Motion and reduced motion

- `App::reduce_motion()` / `App::set_reduce_motion(bool)` (`GPUI/app.rs:1222-1232`). `AnimationExt::with_animation` "automatically respect[s] `App::reduce_motion`… the element is rendered in a static state" (`GPUI/elements/animation.rs:74-82`); `Animation::new(Duration)` (`animation.rs:33`). Nothing at boot wires the OS preference; the repo's only control is the Gallery → Motion toggle (`src/gallery/sections/motion/scroll_bounce.rs:121-128`). The tour therefore animates only through `with_animation` and reads `cx.reduce_motion()` where it must skip motion itself.
- Theme motion tokens: `cx.theme().motion_tokens()` (`GC/theme/mod.rs:516`); `duration_fast` 120 ms, `duration_normal` 180 ms (`GC/theme/motion.rs:15-23`).

### Theming / components for the card

- `cx.theme().popover` + `popover_foreground` (`GC/theme/theme_color.rs:199-201`). The popover family's ring/shadow helpers `popover_ring`/`popover_shadow` are `pub(crate)` (`GC/styled.rs:36,59`); app code cannot reuse them; the card uses `.shadow_lg()` from the Styled scale instead.
- `Styled::opacity(f32)` (`GPUI/styled.rs:746`). Size scale: `size_1` = `rems(0.25)` = 4 px (`gpui-pre-macros-0.3.8/src/styles.rs:939-941`). Trap: `_1_5` suffixes are fractions (`relative(0.2)`), not 1.5 steps (`styles.rs:1123-1126`); never use `size_1_5` for a 6 px dot.
- `Button` builders verified: `.primary()` (`GC/button/button.rs:52`), `.ghost()` (`:82`), `.outline()` (`:332`), `.disabled(bool)` (`:525`; Disableable, used at `src/gallery/sections/buttons/button.rs:85`).
- `Rems::to_pixels(rem_size) -> Pixels` (`GPUI/geometry.rs:3256`); `window.rem_size()` (`GPUI/window.rs:2986`).
- Icons: `IconName::Map` is compile-verified by repo usage (`src/gallery/sections/buttons/icon.rs:138`); `compass.svg` exists in `gpui-kit-assets-0.7.1/assets/icons/` but the `IconName` variant is generated at build time, so this plan uses `Map`.

### Repo seams consumed (all read this session)

- Events: `crate::events::emit(AppEventKind::Navigate(route), cx)` (`src/runtime/events.rs:48-57`); AppRoot drains in an `observe_global` observer (`src/shell/root/app_root/state.rs:63-66`); `set_route` no-ops on equal route (`state.rs:235-238`).
- Capabilities: `capabilities::set(name, CapabilityStatus::supported_enabled(), cx)` (`src/runtime/capabilities.rs:64-79`); always-on list `DEFAULT_ENABLED_CAPABILITIES` (`src/app/init.rs:95-108`).
- Commands: `CommandId` enum (`src/services/commands.rs:12-29`), `availability` arm (`:45-99`), `command(id, title_key, subtitle_key, icon)` helper (`:306-313`), `execute` arm (`:203-285`).
- i18n facade: `crate::i18n::localize("key")` and `localize_with_args("key", &[("name", value)])` with `%{name}` placeholders (`src/services/i18n.rs:3-13`; call site `src/app/init.rs:379-383`). Catalog key grammar and the both-files lockstep rule: `tests/qa_localization.rs:20-71,188-198`. Scanner blind spot: only literal first args are checked (`qa_localization.rs:77-128`); the tour resolves keys through `step.title_key`, so catalog completeness for those keys is covered by a unit test instead (Phase 2).
- Live-region pattern: `.a11y(Role::X, label)` + `.a11y_live(accesskit::Live::Polite)` (`src/services/accessibility.rs:23-40`), precedents at `src/shell/root/app_root/render.rs:149-158` (page title), `src/features/command_palette.rs:286-291` (palette status), `src/features/pages/notifications.rs:48-53`. The checklist rule is documented at `docs/accessibility-checklist.md:48-50`.
- Render/root wiring: `AppRoot::render` returns `v_flex().size_full().child(title_bar).child(content_area).child(status_bar)` (`render.rs:285-290`); `NavItem::render` builds the sidebar row (`render.rs:317-347`); `AppRoot::new` constructs page entities and sets globals (`state.rs:41-186`).
- Demo targets: Home heading/button (`src/features/pages/home.rs:39-75`); Gallery section nav `Sidebar::new("gallery-sidebar")` and search input header (`src/gallery/page.rs:71-94`).
- Removal-seam header style to mirror: `src/gallery/mod.rs:1-7`.
- Wasm: no new dependencies (nothing added to any `Cargo.toml` table; `Cargo.lock` untouched). A11y announcements are inert on wasm because the gpui web stack ships no AccessKit bridge (`src/services/accessibility.rs:3-6,67-69`); the overlay itself is pure gpui and works. Gates: AGENTS.md quality gates; `just wasm-check` = `cargo +nightly check --target wasm32-unknown-unknown -p gpui-starter --lib` (`justfile:24-25`, survey §6). Note CI runs `cargo test -p gpui-starter --lib` only (`.github/workflows/ci.yml:245,285`), so sibling in-module tests run in CI automatically.
- No cargo command was run for this plan (doc-only scope); all gate commands are quoted from `AGENTS.md` and `justfile`.

## Architecture

Entities and state ownership:

- `TargetRegistry` (GPUI `Global`, app-singleton like `AppEventQueue`): `HashMap<&'static str, Bounds<Pixels>>` plus an optional `WeakEntity<TourController>` subscriber. Written only by `TourTarget::prepaint` (dedup: a publish that changes the stored value notifies the subscriber; an identical value does not, so no notify loop).
- `TourController` (entity, owned by `AppRoot`, one per app): `active: Option<ActiveTour>`, `focus_handle: FocusHandle`, `restore_focus: Option<FocusHandle>`. `ActiveTour` holds the `TourDefinition` and a pure `TourProgress`. The controller renders the overlay; inactive renders an empty div.
- `TourProgress` (pure value in `tour.rs`): step index + total, saturating `advance`/`go_back`.
- Pages own nothing; they wrap existing elements with `tutorial::target(id, child)` at render time.

Events and data flow:

```
page render ──target(id, child)──► TourTarget::prepaint ──publish(id, bounds)──► TargetRegistry (Global)
                                                                        │ value changed?
                                                                        ▼ notify (WeakEntity)
AppRoot::render = target("tour-viewport", v_flex(title_bar, content, status_bar, tour))
     │                                 last child ──► TourController::render ◄──resolve(id)──┘
     │                                                    │
     │                                                    ├─ inactive: empty div (no layout footprint)
     │                                                    └─ active:
     │                                                        div.focus_trap("tour", &handle)  key_context("Tour")  on_action(TourNext/Back/Dismiss)
     │                                                          │   absolute size_full top_0 left_0 — the wrapper itself must be the
     │                                                          │   positioned full-size box (taffy has no Static; see Step 4.2)
     │                                                          └─ div#tour-overlay  a11y(Dialog)  occlude  absolute size_full
     │                                                               ├─ canvas(mask)  absolute size_full: even-odd dim path + glow quad
     │                                                               └─ tooltip card  absolute left/top (place_card)
     │                                                                    ├─ title  (Heading, level 2)
     │                                                                    ├─ body   (muted text)
     │                                                                    ├─ dots + "n / m" status (Role::Status + a11y_live Polite)
     │                                                                    └─ Skip (ghost) | Back (outline, disabled at step 0) | Next/Finish (primary)
     └─ keys: right/enter→TourNext, left→TourBack, escape→TourDismiss  (bindings scoped to context "Tour")
          advance/back into a step with route → events::emit(Navigate) → AppRoot observer → set_route
          target painted on the new page → registry publish → controller notified → overlay re-renders with spotlight
```

Module tree:

```
src/features/tutorial/
├── mod.rs            public surface + removal-recipe header (mirrors src/gallery/mod.rs)
├── target.rs         TourTarget element (layout-transparent capture) + TargetRegistry global
├── tour.rs           TourStep, TourDefinition, TourProgress, demo_tour()
├── controller.rs     TourController entity, TourNext/TourBack/TourDismiss actions, TourHandle global, initialize()
├── mask.rs           pure geometry: cutout_bounds, place_card, rounded_rect_path, spotlight paint
└── target.test.rs / tour.test.rs / controller.test.rs / mask.test.rs
```

Frame-ordering facts (the registry is read one frame behind): all view renders run in the layout pass (`ViewElement::request_layout` calls `view.render`, `GPUI/view.rs:324-341`; whole-tree layout at `GPUI/window.rs:3581`) and all `TourTarget` publishes run in the prepaint pass that follows (`GPUI/window.rs:3586`). The controller's render therefore reads bounds written by the previous frame's prepaint. This is why `publish` notifies only on change: a target that moved (window resize, scroll) triggers exactly one more frame and the overlay converges one frame later; a static target already carries correct bounds when a step lands on it. A step whose target has never been painted (first step after a route change) resolves to None and renders the transparent input-blocking placeholder — a required branch, not defensive code — until that first publish's notify re-renders the overlay with the spotlight.

## Interface Contract

Everything below is the exact pub surface at `crate::features::tutorial` (module doc in `mod.rs` states the contract; removal recipe in the same header). No new dependencies; no config schema changes; no new event kinds.

```rust
/// Wrap `child` and publish its resolved window-space bounds every drawn
/// frame under `id`. Layout- and a11y-transparent (no layout node, no
/// element id, child keeps its own element-id path). `id` must be unique
/// among the siblings of one parent.
pub fn target(id: &'static str, child: impl IntoElement) -> TourTarget;

/// Last published bounds for `id`, or None if it has never painted.
pub fn resolve(id: &str, cx: &App) -> Option<Bounds<Pixels>>;

/// One tour stop. `route` is emitted as a Navigate event when the step
/// becomes current (None = stay on the current page).
pub struct TourStep {
    pub target: &'static str,
    pub title_key: &'static str,   // i18n key; must exist in both catalogs
    pub body_key: &'static str,    // i18n key; must exist in both catalogs
    pub route: Option<AppRoute>,
}

pub struct TourDefinition {
    pub id: SharedString,          // tracing label
    pub steps: Vec<TourStep>,      // non-empty by contract
}

/// Pure step machine: saturating at both ends.
pub struct TourProgress { /* step_ix: usize, total: usize */ }
impl TourProgress {
    pub fn new(total: usize) -> Self;
    pub fn ix(&self) -> usize;
    pub fn total(&self) -> usize;
    pub fn is_last(&self) -> bool;
    pub fn advance(&mut self) -> bool;   // false when already on the last step
    pub fn go_back(&mut self) -> bool;   // false when already on the first step
}

/// Home → Gallery demo tour (5 steps, ids: home-heading, home-cta,
/// sidebar-gallery, gallery-nav, gallery-search). `"tour-viewport"` is a
/// reserved internal id published by the AppRoot wrap; do not use it as a
/// step target.
pub fn demo_tour() -> TourDefinition;

/// Start the demo tour in the root window. No-op (warn log) when the root
/// window or controller is not up yet.
pub fn start(cx: &mut App);

/// Start an arbitrary tour in the root window.
pub fn start_tour(definition: TourDefinition, cx: &mut App);

/// Close the active tour from App level (same path as Escape/Skip).
pub fn dismiss(cx: &mut App);

// Keyboard actions (bindings registered by initialize(), scoped to key
// context "Tour": right + enter → TourNext, left → TourBack,
// escape → TourDismiss). Handlers live on the overlay root.
#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = tutorial, no_json)]
pub struct TourNext;
pub struct TourBack;    // same derives
pub struct TourDismiss; // same derives

/// Overlay owner. Constructed once by AppRoot; Render renders the empty div
/// (inactive) or the spotlight overlay (active). Not constructed by consumers.
pub struct TourController;

/// Global handle set by AppRoot::new; start/dismiss go through it plus the
/// RootWindow global (src/app/window.rs:11-22).
pub(crate) struct TourHandle(pub Entity<TourController>); // Global impl

/// Bind the tour keys. Called once from app::init.
pub fn initialize(cx: &mut App);
```

Consumers (other plans) use: `target()` to mark spots, `TourDefinition`/`TourStep` to author tours, `start_tour`/`dismiss` to run them, `resolve` to read geometry in tests or adjacent features. The registry, `TourTarget` internals, `TourController` methods, and `mask.rs` are private to the module.

## Module Layout

| File | Purpose |
|---|---|
| `src/features/tutorial/mod.rs` | Public surface re-exports + module doc with removal recipe |
| `src/features/tutorial/target.rs` | `TourTarget` capture element; `TargetRegistry` global; `publish`/`resolve` |
| `src/features/tutorial/tour.rs` | `TourStep`, `TourDefinition`, `TourProgress`, `demo_tour()` |
| `src/features/tutorial/controller.rs` | `TourController` entity + overlay render; actions; `TourHandle`; `initialize()`; `start`/`start_tour`/`dismiss` |
| `src/features/tutorial/mask.rs` | Pure spotlight geometry and paint helpers (`cutout_bounds`, `place_card`, `rounded_rect_path`, `paint_spotlight`) |
| `src/features/tutorial/target.test.rs` | Registry dedupe/notify tests, capture element compile surface |
| `src/features/tutorial/tour.test.rs` | Progress machine, demo-tour sanity, i18n key presence |
| `src/features/tutorial/controller.test.rs` | Action keystroke parsing, TourHandle absence tolerance |
| `src/features/tutorial/mask.test.rs` | Placement/clamping math, path builds, cutout padding |

## Seam Touches

Every edit outside `src/features/tutorial/`:

1. `src/features/mod.rs` — add `pub mod tutorial;` (file currently 3 lines, `command_palette`, `pages`, `palette`).
2. `src/shell/root/app_root/state.rs` — `AppRoot` gains field `tutorial: Entity<TourController>` (struct at `:17-38`); construct in `AppRoot::new` next to the page entities (`:46-56`) and set the global: `cx.set_global(crate::features::tutorial::TourHandle(tutorial.clone()));`; return it from `Self { .. }` (`:168-185`).
3. `src/shell/root/app_root/render.rs` — the returned root becomes `tutorial::target("tour-viewport", v_flex()…)` (layout-transparent wrap of the existing `v_flex().size_full()` at `:285-290`, so the viewport bounds are published every frame); that `v_flex` gains one last child `.child(self.tutorial.clone())` so the overlay is the window's last-painted subtree; in `NavItem::render` wrap the returned row for the Gallery page only: `if matches!(self.page, Page::Gallery) { tutorial::target("sidebar-gallery", row).into_any_element() } else { row.into_any_element() }` (`:336-346`). Why minimal: the overlay must be the last-painted subtree of the window, `tour-viewport` must publish whether or not a tour is active (so it wraps the root rather than living inside the overlay), and the sidebar row is the only per-page element AppRoot itself renders.
4. `src/features/pages/home.rs` — wrap the heading div (`:40-47`) with `tutorial::target("home-heading", …)`; wrap the Get Started button (`:56-62`) with `tutorial::target("home-cta", …)`; add one outline Button `("home-start-tour")` labeled `tutorial_button_start` next to `start-demo-task` that calls `crate::features::tutorial::start(cx)`. No state, no layout change beyond the extra button.
5. `src/gallery/page.rs` — wrap `nav` (`Sidebar::new("gallery-sidebar")` built at `:71-94`) with `tutorial::target("gallery-nav", …)`; wrap the search-input header div (`:78-84`) with `tutorial::target("gallery-search", …)`. Both survive every gallery state (empty filter included), which is why the section header is not a target.
6. `src/services/commands.rs` — `CommandId::StartTour` variant (`:12-29`); `availability` arm returning always-enabled (`:45-99`); registry entry `command(CommandId::StartTour, "tutorial_command", "tutorial_command_subtitle", IconName::Map)` (`:102-200`); `execute` arm calling `crate::features::tutorial::start(cx)` (`:203-285`).
7. `src/app/init.rs` — call `crate::features::tutorial::initialize(cx);` inside the `runtime_services_init` step (`:202-263`); add `"tutorial"` to `DEFAULT_ENABLED_CAPABILITIES` (`:95-108`).
8. `locales/en.yml` + `locales/zh-CN.yml` — the `tutorial_*` keys from the i18n table below, in both files (lockstep enforced by `tests/qa_localization.rs:188-198`).
9. `docs/accessibility-checklist.md` — one bullet under the live-region item (`:48-50`): tour step changes announce "Step n of m: title" through a Polite `Role::Status`. (Addition only; the `qa_docs` sentinels "Keyboard And Focus" and "Done Criteria" are untouched.)

### Removal Recipe

Ordered steps mirroring the `src/gallery/mod.rs:1-7` header style (the recipe is also the `src/features/tutorial/mod.rs` module doc):

1. Delete `src/features/tutorial/`.
2. Revert `src/features/mod.rs` (drop `pub mod tutorial;`).
3. Revert `src/shell/root/app_root/state.rs`: remove the `tutorial` field, its construction, and the `TourHandle` global set.
4. Revert `src/shell/root/app_root/render.rs`: unwrap the root from `target("tour-viewport", …)`, drop the overlay child from the root `v_flex`, and drop the Gallery-only `target(...)` wrap in `NavItem::render`.
5. Revert `src/features/pages/home.rs`: drop the two `target(...)` wraps and the `home-start-tour` button.
6. Revert `src/gallery/page.rs`: drop the two `target(...)` wraps.
7. Revert `src/services/commands.rs`: remove `CommandId::StartTour`, its availability arm, registry entry, and execute arm.
8. Revert `src/app/init.rs`: remove the `tutorial::initialize` call and the `"tutorial"` capability entry.
9. Remove every `tutorial_*` key from `locales/en.yml` and `locales/zh-CN.yml`.
10. Revert the tour bullet in `docs/accessibility-checklist.md`.

No `Cargo.toml`, `Cargo.lock`, config-store, route, sidebar, or page-count edits exist to revert (no new page; `tests/e2e_navigation.rs:14-19` count stays 10).

## Implementation Phases

## Phase 1: Target capture foundation

### Step 1.1: Module scaffold

Files: `src/features/tutorial/mod.rs`, `src/features/mod.rs`, `src/features/tutorial/target.rs`.
Build: `mod.rs` with the module doc (contract + removal recipe) and `mod target;` only — no re-exports yet, so an empty `target.rs` compiles. Each re-export lands with the item it names: `pub use target::resolve;` and `pub(crate) use target::TargetRegistry;` in Step 1.2 (Step 3.2's `TourController::new` writes `crate::features::tutorial::TargetRegistry`), `pub use target::{TourTarget, target};` in Step 1.3, and `mod tour;` plus its re-exports in Phase 2. Phase 3 adds `mod controller;` with `pub(crate) use controller::TourHandle;` (Seam Touch 2 and the menu item write `crate::features::tutorial::TourHandle`). Test-module wiring (`#[cfg(test)] #[path = "target.test.rs"] mod target_test;`) lives at the bottom of `target.rs`. `features/mod.rs` gains `pub mod tutorial;`.
Tests: none yet.
Done-check: `cargo check -p gpui-starter` passes with an empty `target.rs` (just `use gpui_kit::{prelude::*, *};`).

### Step 1.2: TargetRegistry global

Files: `src/features/tutorial/target.rs`, `src/features/tutorial/mod.rs` (`pub use target::resolve;`).
Build:
```rust
#[derive(Default)]
pub(crate) struct TargetRegistry {
    bounds: HashMap<&'static str, Bounds<Pixels>>,
    subscriber: Option<WeakEntity<TourController>>,
}
impl Global for TargetRegistry {}

/// Store `bounds` for `id`; notifies the subscribed controller only when the
/// value changed, so steady-state frames do not schedule renders. Returns
/// whether the stored value changed.
pub(crate) fn publish(id: &'static str, bounds: Bounds<Pixels>, cx: &mut App) -> bool

/// Last published bounds for `id`, if any.
pub fn resolve(id: &str, cx: &App) -> Option<Bounds<Pixels>>
```
`publish` uses `cx.default_global::<TargetRegistry>()` (pattern: `src/runtime/events.rs:56`); on change it calls `registry.subscriber` via `WeakEntity::update(cx, |_, cx| cx.notify())` (`GPUI/app/entity_map.rs:803-813`), ignoring the `Err` (controller released). `resolve` uses `cx.try_global::<TargetRegistry>()` (pattern: `capabilities.rs:81-85`). Note the `events.rs:71-81` re-entrancy concern does not apply: nothing observes this global.
Tests: `target.test.rs`: `publish_returns_changed_only_for_new_value` and `resolve_reads_back_published_bounds` against a `TestAppContext` (`#[gpui_kit::test]`, pattern `src/state/config_store.test.rs:280-281`): `cx.update(|cx| publish("a", b1, cx))` returns true, the identical publish returns false, a moved-bounds publish returns true; `resolve("a", cx)` reads back the last value.
Done-check: `cargo test -p gpui-starter --lib tutorial` passes; grep confirms no `observe_global::<TargetRegistry>` anywhere.

### Step 1.3: TourTarget element

Files: `src/features/tutorial/target.rs`, `src/features/tutorial/mod.rs` (`pub use target::{TourTarget, target};`).
Build: the capture element, delegating exactly like `AnimationElement` (`GPUI/elements/animation.rs:382-493`):
```rust
pub struct TourTarget {
    key: &'static str,
    child: Option<AnyElement>,
}

/// Publish the wrapped child's resolved bounds under `key` every frame.
/// Adds no layout node and no element id: the child's element-id path and
/// a11y node are unchanged.
pub fn target(key: &'static str, child: impl IntoElement) -> TourTarget {
    TourTarget { key, child: Some(child.into_any_element()) }
}

impl IntoElement for TourTarget { type Element = Self; fn into_element(self) -> Self { self } }

impl Element for TourTarget {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> { None }             // keeps child id paths stable
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> { None }
    fn request_layout(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, window: &mut Window, cx: &mut App) -> (LayoutId, AnyElement) {
        let mut child = self.child.take().expect("TourTarget renders once per frame");
        ((child.request_layout(window, cx), child))
    }
    fn prepaint(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, bounds: Bounds<Pixels>, child: &mut AnyElement, window: &mut Window, cx: &mut App) {
        publish(self.key, bounds, cx);
        child.prepaint(window, cx);
    }
    fn paint(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, _: Bounds<Pixels>, child: &mut AnyElement, _: &mut (), window: &mut Window, cx: &mut App) {
        child.paint(window, cx);
    }
}
```
Import `Element, GlobalElementId, InspectorElementId, LayoutId` from the `gpui_kit` glob (kit re-exports `::gpui::*`, `gpui-kit-0.7.1/src/lib.rs:97`). Why `id() -> None`: returning `Some` would prepend an id to the child's element-id path and reset any keyed state inside it once, at wrap time; `None` leaves the child exactly as before the wrap.
Tests: extend `target.test.rs`: render a `target("probe", div().id("probe-child").size_full())` inside a `TestAppContext` window (`cx.add_window_view`, pattern `GB/popup.rs` tests), draw a frame, assert `resolve("probe", cx)` returns the window-content bounds.
Done-check: `cargo test -p gpui-starter --lib tutorial` green; `cargo clippy --workspace --all-features` clean.

## Phase 2: Tour model

### Step 2.1: Types and progress machine

Files: `src/features/tutorial/tour.rs`, `mod.rs` (`mod tour;` + re-export).
Build: `TourStep`, `TourDefinition`, `TourProgress` exactly as in Interface Contract. `TourProgress::advance/go_back` saturate and return whether they moved. Derives: `Clone, Debug` on all three; `TourProgress` also `PartialEq, Eq`.
Tests: `tour.test.rs`: `progress_saturates_at_both_ends` (new(3): go_back false at 0, advance true twice, false on last), `progress_reports_ix_total_is_last`.
Done-check: `cargo test -p gpui-starter --lib tutorial` green.

### Step 2.2: Demo tour definition

Files: `src/features/tutorial/tour.rs`.
Build: `demo_tour()` returning the 5 steps (ids and routes fixed):

| step | target | route | title_key / body_key |
|---|---|---|---|
| 0 | `home-heading` | `Some(AppRoute::page(Page::Home))` | `tutorial_step_home_heading_title` / `_body` |
| 1 | `home-cta` | None | `tutorial_step_home_cta_title` / `_body` |
| 2 | `sidebar-gallery` | None | `tutorial_step_sidebar_title` / `_body` |
| 3 | `gallery-nav` | `Some(AppRoute::page(Page::Gallery))` | `tutorial_step_gallery_nav_title` / `_body` |
| 4 | `gallery-search` | None | `tutorial_step_gallery_search_title` / `_body` |

Definition `id: "demo".into()`.

This step also edits `locales/en.yml` and `locales/zh-CN.yml`, adding the full `tutorial_*` key block from the i18n section below. The catalog edit belongs here (not Phase 5) so the i18n-key test is green from its first run and no phase depends on a later catalog change.

Tests: `tour.test.rs`: `demo_tour_targets_are_unique_and_nonempty`; `demo_tour_i18n_keys_resolve`: for every step, `crate::i18n::localize(step.title_key) != step.title_key` and same for body (localize returns the key itself when missing, `src/services/i18n.rs:2`). This covers the `qa_localization.rs:77-128` scanner blind spot for indirectly resolved keys.
Done-check: `cargo test -p gpui-starter --lib tutorial` green including the i18n-key test; `cargo test --test qa_localization` green (key parity).

## Phase 3: Controller state machine and actions

### Step 3.1: Actions and key bindings

Files: `src/features/tutorial/controller.rs`, `mod.rs`.
Build:
```rust
#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = tutorial, no_json)]
pub struct TourNext;
// TourBack, TourDismiss identical (pattern: src/shell/root/actions.rs:4-14)

const TOUR_CONTEXT: &str = "Tour"; // private: only controller.rs and its bindings use it

/// Bind tour keys (context-scoped; inert unless the overlay is focused).
pub fn initialize(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("right", TourNext, Some(TOUR_CONTEXT)),
        KeyBinding::new("enter", TourNext, Some(TOUR_CONTEXT)),
        KeyBinding::new("left", TourBack, Some(TOUR_CONTEXT)),
        KeyBinding::new("escape", TourDismiss, Some(TOUR_CONTEXT)),
    ]);
}
```
Keystroke names and context scoping verified against `GC/menu/popup_menu.rs:22-29`; `KeyBinding::new` signature at `GPUI/keymap/binding.rs:33`.
Tests: `controller.test.rs`: `tour_keystrokes_parse`: `Keystroke::parse("right"/"left"/"enter"/"escape")` all Ok (pattern `src/app/keys.rs:18`).
Done-check: `cargo test -p gpui-starter --lib tutorial` green.

### Step 3.2: TourController entity and lifecycle methods

Files: `src/features/tutorial/controller.rs`; `mod.rs` gains `mod controller;`
plus `pub(crate) use controller::TourHandle;` (Seam Touch 2 references the
handle through the module path — the re-export must exist from this step).
Build:
```rust
pub struct TourController {
    active: Option<ActiveTour>,
    focus_handle: FocusHandle,
    restore_focus: Option<FocusHandle>,
}
struct ActiveTour { definition: TourDefinition, progress: TourProgress }

impl TourController {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Register as the registry subscriber so captures re-render the overlay.
        cx.default_global::<crate::features::tutorial::TargetRegistry>().subscriber =
            Some(cx.entity().downgrade());
        Self { active: None, focus_handle: cx.focus_handle(), restore_focus: None }
    }

    fn begin(&mut self, definition: TourDefinition, window: &mut Window, cx: &mut Context<Self>) {
        if self.active.is_some() || definition.steps.is_empty() { return; }
        let total = definition.steps.len();
        self.restore_focus = window.focused(cx);              // GPUI/window.rs:2329
        self.active = Some(ActiveTour { definition, progress: TourProgress::new(total) });
        self.enter_step(window, cx);
        tracing::info!(target: "gpui_starter::tutorial", "tour started");
    }

    /// Route emission + focus + notify shared by begin/advance/back.
    fn enter_step(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(route) = self.current_step().and_then(|s| s.route) {
            crate::events::emit(crate::events::AppEventKind::Navigate(route), cx);
        }
        self.focus_handle.focus(window, cx);
        cx.notify();
    }

    fn advance(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tour) = &mut self.active else { return };
        if tour.progress.is_last() { self.close(window, cx, "finished"); return; }
        tour.progress.advance();
        self.enter_step(window, cx);
    }

    fn go_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.active.as_mut().is_some_and(|t| t.progress.go_back()) {
            self.enter_step(window, cx);
        }
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>, reason: &'static str) {
        if self.active.take().is_none() { return; }
        if let Some(handle) = self.restore_focus.take() { handle.focus(window, cx); }
        cx.notify();
        tracing::info!(target: "gpui_starter::tutorial", reason, "tour closed");
    }

    fn current_step(&self) -> Option<&TourStep> { ... }
}
impl Focusable for TourController { fn focus_handle(&self, _: &App) -> FocusHandle { self.focus_handle.clone() } }
```
`total` is computed before `definition` moves into `ActiveTour` (field-init order would otherwise borrow a moved value). Tracing target follows the `gpui_starter::<module>` rule (survey §9).
Tests: none new here (window-dependent); the pure parts are covered in Phase 2.
Done-check: `cargo check -p gpui-starter` passes; `cargo clippy --workspace --all-features` clean.

### Step 3.3: App-level entry points

Files: `src/features/tutorial/controller.rs`, `mod.rs` re-exports.
Build:
```rust
pub(crate) struct TourHandle(pub Entity<TourController>);
impl Global for TourHandle {}

pub fn start(cx: &mut App) { start_tour(tour::demo_tour(), cx) }

pub fn start_tour(definition: TourDefinition, cx: &mut App) {
    let Some(controller) = cx.try_global::<TourHandle>().map(|h| h.0.clone()) else {
        tracing::warn!(target: "gpui_starter::tutorial", "tour requested before the root window exists");
        return;
    };
    let Some(root) = crate::app::window::root_window(cx) else { return; };
    let _ = root.update(cx, |_, window, cx| {
        controller.update(cx, |tour, cx| tour.begin(definition, window, cx))
    });
}

pub fn dismiss(cx: &mut App) { /* same shape, calls close(window, cx, "dismissed") */ }
```
`AnyWindowHandle::update` view-agnostic signature verified at `GPUI/window.rs:7405-7417`; `root_window` at `src/app/window.rs:20-22`.
Tests: `controller.test.rs`: `start_without_handle_is_a_noop`: on a fresh `TestAppContext`, run `cx.update(crate::features::tutorial::start)`; assert it returns without panicking and without setting any global (only a warn log is emitted).
Done-check: `cargo test -p gpui-starter --lib tutorial` green.

## Phase 4: Overlay rendering

### Step 4.1: Pure mask geometry

Files: `src/features/tutorial/mask.rs`.
Build (pure except `paint_spotlight`, the only fn taking `Window`):
```rust
/// Spotlight rect: target expanded by `pad` on every side.
pub(crate) fn cutout_bounds(target: Bounds<Pixels>, pad: Pixels) -> Bounds<Pixels>;

/// Tooltip anchor: below the target when it fits, otherwise above, then
/// clamped into the viewport. `card_w`/`card_h_est` are the card's fixed
/// width and a conservative height estimate used only for placement.
pub(crate) fn place_card(
    target: Bounds<Pixels>, viewport: Bounds<Pixels>,
    card_w: Pixels, card_h_est: Pixels, gap: Pixels,
) -> Bounds<Pixels>;

/// Rounded-rect subpath (four `arc_to` corners) appended to `builder`.
pub(crate) fn rounded_rect_path(builder: &mut PathBuilder, rect: Bounds<Pixels>, radius: Pixels);

/// Paint the even-odd dim mask (viewport rect minus cutout) in
/// `cx.theme().overlay`, then the glow ring quad in `cx.theme().primary`.
pub(crate) fn paint_spotlight(
    window: &mut Window, cx: &App, viewport: Bounds<Pixels>, cutout: Bounds<Pixels>, rem: Pixels,
);
```
Exact behavior:
- `cutout_bounds`: `Bounds { origin: point(target.origin.x - pad, target.origin.y - pad), size: size(target.size.width + pad * 2., target.size.height + pad * 2.) }` (no clamping; targets are inside the window).
- `place_card` algorithm, in order: candidate below = `(target.left, target.bottom + gap)`; if `target.bottom + gap + card_h_est > viewport.bottom` use above = `(target.left, target.top - gap - card_h_est)`; clamp `origin.x` into `[viewport.left, viewport.right - card_w]` and `origin.y` into `[viewport.top, viewport.bottom - card_h_est]`; return `Bounds { origin, size: size(card_w, card_h_est) }`.
- `rounded_rect_path`: `move_to(top-left + radius on x)`, four `line_to`/`arc_to` corner arcs (`GPUI/path_builder.rs:155` `arc_to(radii, x_rotation, large_arc, sweep, to)`; sweep true, radii `point(radius, radius)`), `close()` (`:199`).
- `paint_spotlight`: build `PathBuilder::fill().with_style(PathStyle::Fill(FillOptions::default().with_fill_rule(FillRule::EvenOdd)))` (`path_builder.rs:12,19-21`; `with_fill_rule` at `lyon_tessellation-1.0.20/src/lib.rs:515`; EvenOdd is already the default, `lyon_tessellation-1.0.20/src/lib.rs:457-458`, the call is kept explicit at the hole-punching site); outer subpath = viewport rect (`move_to`/`line_to`/`close`); inner subpath = `rounded_rect_path(cutout, cx.theme().radius_lg)` (the token is already `Pixels`, default 8 px; `GC/theme/mod.rs:187,775`); `window.paint_path(builder.build()?, cx.theme().overlay)` (`GPUI/window.rs:4621`; `build()` returns `Result`: log and skip paint on Err). Glow: `window.paint_quad(quad(glow_bounds, corner_radii, transparent_black(), Edges::all(border_w), cx.theme().primary, BorderStyle::Solid))` where `glow_bounds` = cutout, `corner_radii` = `Corners::all(cx.theme().radius_lg)`, `border_w = rems(0.125).to_pixels(rem)` (2 px at default zoom), all per `GPUI/window.rs:7675-7687`, `geometry.rs:1820,2309,3256`, `scene.rs:597-603`.
Tests: `mask.test.rs`: `cutout_expands_symmetrically`; `place_card_below_when_space` (target mid-viewport → card top == target.bottom + gap); `place_card_above_when_no_space_below` (target at bottom edge → card bottom == target.top - gap); `place_card_clamps_to_viewport` (target near right edge / tiny viewport); `mask_path_builds` (build() is Ok for sample bounds, even-odd fill rule set).
Done-check: `cargo test -p gpui-starter --lib tutorial` green.

### Step 4.2: Overlay render

Files: `src/features/tutorial/controller.rs`.
Build: `impl Render for TourController`. Both inactive and target-unresolved paths return `div().into_any_element()`-shaped `AnyElement` (single return type):

```rust
fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let Some(tour) = &self.active else { return div().into_any_element() };
    let step = &tour.definition.steps[tour.progress.ix()];

    // The viewport comes from the "tour-viewport" capture that wraps AppRoot's
    // root v_flex (Seam Touch 3). It publishes during every frame's prepaint
    // and renders read it one frame later (GPUI/window.rs:3581-3586), so once
    // the window has drawn once it is always resolvable when a tour is active.
    let Some(viewport) = crate::features::tutorial::resolve("tour-viewport", cx) else {
        return self.overlay(waiting_placeholder(), cx);
    };
    let Some(target) = crate::features::tutorial::resolve(step.target, cx) else {
        // Fresh navigation: the target has not painted yet. Block input this
        // frame; the first capture notifies this entity and we re-render.
        return self.overlay(waiting_placeholder(), cx);
    };

    let rem = window.rem_size();
    let pad = rems(0.5).to_pixels(rem);
    let cutout = mask::cutout_bounds(target, pad);
    let card_bounds = mask::place_card(
        target, viewport, rems(20.).to_pixels(rem), rems(12.).to_pixels(rem), pad,
    );

    let card = render_card(step, tour, card_bounds, window, cx);   // Step 4.3

    let mask_canvas = canvas(|_, _, _| (), move |bounds, (), window, cx| {
        mask::paint_spotlight(window, cx, bounds, cutout, window.rem_size());
    })
    .absolute().size_full().top_0().left_0();          // GPUI/elements/canvas.rs:91-95, GB/popup.rs:168-171

    let overlay_root = div()
        .id("tour-overlay")
        .a11y(Role::Dialog, crate::i18n::localize("tutorial_dialog"))
        .occlude()
        .absolute().size_full().top_0().left_0()
        .child(mask_canvas)
        .child(card);

    self.overlay(overlay_root, cx)
}

/// The one positioned box everything hangs from, and the ONLY node that
/// carries the key context and actions (see prose below).
fn overlay(&self, child: impl IntoElement, cx: &mut Context<Self>) -> AnyElement {
    div()
        .focus_trap("tour", &self.focus_handle)      // FocusTrapElement trait import required
        .key_context(TOUR_CONTEXT)
        .on_action(cx.listener(|this, _: &TourNext, window, cx| this.advance(window, cx)))
        .on_action(cx.listener(|this, _: &TourBack, window, cx| this.go_back(window, cx)))
        .on_action(cx.listener(|this, _: &TourDismiss, window, cx| this.close(window, cx, "dismissed")))
        .absolute().size_full().top_0().left_0()
        .child(child)
        .into_any_element()
}
```

Two structural rules this shape exists to satisfy, both verified against the layout/dispatch sources:

- **Geometry: the trap wrapper must itself be the absolute full-size box.** Taffy has no Static position — every element defaults Relative (`GPUI/style.rs:774,783`; `taffy-0.13.0/src/style/mod.rs:291-302`) — and an absolutely-positioned child lays out against its immediate parent container (`taffy-0.13.0/src/compute/flexbox.rs:2231-2249`, `block.rs:1462-1479`). An unstyled in-flow wrapper as AppRoot's last flex child has a zero-height box at the window bottom; an absolute overlay inside it resolves `size_full` to 0 and the mask/cutout/card are invisible. With the wrapper itself `.absolute().size_full().top_0().left_0()`, it positions against the wrapped root `v_flex` — exactly the `tour-viewport` box the card clamps against — and the overlay inside it fills the window. The view cache's inserted Relative node (`GPUI/view.rs:442-448`) is why the waiting path routes through the same `overlay()` helper instead of returning a bare absolute div.
- **Dispatch: `key_context` and the three `on_action` handlers live on the node that owns the focused handle.** Action dispatch builds the context stack and bubbles along the focused node's ANCESTOR chain only (`GPUI/key_dispatch.rs:563-571`; context stack from `dispatch_path` at `:435-441`). Focus goes to `self.focus_handle`, tracked by the trap wrapper (`FocusTrapContainer::new` → `child.track_focus(&handle)`, `GB/focus_trap.rs:113-117`). A `key_context("Tour")` or `on_action` on a DESCENDANT (the old overlay_root placement) never enters the dispatch path, so arrows/Enter/Escape would be dead until the user clicked into the card. Same element as the tracked handle — the trait doc's own shape (`key_dispatch.rs:1-27`).

`waiting_placeholder()` is a private helper: `div().id("tour-waiting").size_full().occlude()` (positioning comes from the `overlay()` wrapper). `"tour-viewport"` publishes from boot onward (one frame behind, see Architecture), so once the window has drawn it is always resolvable; in practice only a step target is ever unresolved (the first frame after a route-changing step, while the new page's captures are still ahead of this render). The placeholder is the required state for that frame, and the capture's change-notify re-renders the full overlay on the next one. One capture mechanism serves targets and viewport; no new API.

`focus_trap` on the roleless wrapper (comment rule from `command_palette.rs:186-188`); `Role::Dialog` precedent `command_palette.rs:182`. `on_action` listener signature per `render.rs:228-266`. Until Step 4.3 lands, `render_card` is a minimal stand-in (`fn render_card(..) -> impl IntoElement { div().id("tour-card") }`) so this step's done-check compiles; Step 4.3 replaces its body.
Tests: none automated (window-heavy); manual QA covers.
Done-check: `cargo check -p gpui-starter` passes with the render impl in place. Visual verification is deferred to Phase 5 Step 5.2's done-check; do not add temporary boot-time tour starts to test earlier.

### Step 4.3: Tooltip card

Files: `src/features/tutorial/controller.rs` (replaces the Step 4.2 stand-in body of `render_card`).
Build — module imports (explicit, per repo precedent `src/features/pages/notifications.rs:1-2`; these trait methods are NOT in the glob): `use gpui_kit::component::{ActiveTheme as _, ButtonVariants as _, Disableable as _, FocusTrapElement as _};` (`ActiveTheme` `GC/theme/mod.rs:44-50`, `ButtonVariants` `GC/button/button.rs:48,82`, `Disableable` for `.disabled`, `FocusTrapElement` for `.focus_trap`, `GB/focus_trap.rs`).
(all tokens themed, rem-scale helpers, sizes from kit; `ix`/`total`/`is_last` are locals from the Step 4.2 render body: `let ix = tour.progress.ix(); let total = tour.progress.total(); let is_last = tour.progress.is_last();`):
```rust
let title = crate::i18n::localize(step.title_key);
let body = crate::i18n::localize(step.body_key);
let status_label = crate::i18n::localize_with_args(
    "tutorial_step_status",
    &[("step", &(ix + 1).to_string()), ("total", &total.to_string()), ("title", &title)],
);

let card = v_flex()
    .id("tour-card")
    .absolute()
    .left(px(card_bounds.origin.x)).top(px(card_bounds.origin.y))   // GB/resize_handle.rs:560-561
    .w_80()                                   // 20rem fixed width (styles.rs rem ladder)
    .p_4().gap_3()
    .bg(cx.theme().popover)
    .border_1().border_color(cx.theme().border)
    .rounded(cx.theme().radius_lg)
    .shadow_lg()                              // popover_shadow is pub(crate) in GC/styled.rs:36,59
    .child(div().id("tour-card-title")
        .a11y(Role::Heading, title.clone()).aria_level(2)
        .text_sm().font_weight(FontWeight::SEMIBOLD).child(title))
    .child(div().id("tour-card-body")
        .text_sm().text_color(cx.theme().muted_foreground).child(body))
    .child(h_flex().id("tour-dots")
        .gap_1().items_center()
        .children((0..total).map(|i| div()
            .size_1()                          // 4px; NOT size_1_5 (that is 20%, styles.rs:1123-1126)
            .rounded_full()
            .bg(if i == ix { cx.theme().primary } else { cx.theme().muted })))
        .child(div().flex_1())
        .child(div().id("tour-status")
            .a11y(Role::Status, status_label)
            .a11y_live(accesskit::Live::Polite)   // pattern: command_palette.rs:286-291
            .text_xs().text_color(cx.theme().muted_foreground)
            .child(format!("{}/{}", ix + 1, total))))
    .child(h_flex().gap_2().justify_end()
        .child(Button::new("tour-skip").ghost()
            .label(crate::i18n::localize("tutorial_skip"))
            .on_click(cx.listener(|this, _, window, cx| this.close(window, cx, "skipped"))))
        .child(div().flex_1())
        .child(Button::new("tour-back").outline()
            .disabled(ix == 0)
            .label(crate::i18n::localize("tutorial_back"))
            .on_click(cx.listener(|this, _, window, cx| this.go_back(window, cx))))
        .child(Button::new("tour-next").primary()
            .label(crate::i18n::localize(if is_last { "tutorial_finish" } else { "tutorial_next" }))
            .on_click(cx.listener(|this, _, window, cx| this.advance(window, cx)))));

let card = card.with_animation(
    ("tour-card", ix),                        // From<(&'static str, usize)>, GPUI/window.rs:7574
    Animation::new(cx.theme().motion_tokens().duration_fast),   // auto-respects reduce_motion
    |card, delta| card.opacity(delta),        // GPUI/elements/animation.rs:74-82, styled.rs:746
);
```
Status label carries "Step n of m: title" so dots can stay decorative (no per-dot a11y nodes; state is never color-alone: the text carries it).
Tests: none automated; manual QA.
Done-check: app (after Phase 5) shows the card, dots, and localized strings; typing Gallery → Motion → reduced-motion toggle (`src/gallery/sections/motion/scroll_bounce.rs:121-128`) makes the card appear without fade.

## Phase 5: Wiring and demo targets

### Step 5.1: AppRoot wiring

Files: `src/shell/root/app_root/state.rs`, `src/shell/root/app_root/render.rs`.
Build: exactly Seam Touches 2 and 3: field + construct (`cx.new(|cx| TourController::new(window, cx))` next to `gallery_page`, `state.rs:56`), `cx.set_global(tutorial::TourHandle(tutorial.clone()))` after construction, struct-literal field, the `target("tour-viewport", v_flex()…)` root wrap with `.child(self.tutorial.clone())` as the `v_flex`'s last child (`render.rs:285-290`), and the Gallery-only `target("sidebar-gallery", …)` wrap in `NavItem::render` (`render.rs:336-346`; both arms end `.into_any_element()`).
Tests: none new (page count unchanged; `tests/e2e_navigation.rs:14-19` untouched).
Done-check: `cargo check -p gpui-starter` passes; app boots with no visual change.

### Step 5.2: Home targets and tour button

Files: `src/features/pages/home.rs`.
Build: Seam Touches 4. Button id `"home-start-tour"`, `.outline()`, label `tutorial_button_start`, `on_click(cx.listener(|_, _, _, cx| crate::features::tutorial::start(cx)))` (Context<HomePage> derefs to App, same as `home.rs:73` calls into `crate::tasks`).
Tests: none new.
Done-check: app shows "Take a tour" on Home; clicking starts the overlay on step 1 (heading spotlight).

### Step 5.3: Gallery targets

Files: `src/gallery/page.rs`.
Build: Seam Touches 5: wrap the `nav` value and the search header div. Both wraps are layout-transparent (`TourTarget` adds no node), so no style adjustments.
Tests: none new.
Done-check: walking the tour reaches Gallery; spotlight lands on the section nav, then the search field.

### Step 5.4: Command and capability wiring

Files: `src/services/commands.rs`, `src/app/init.rs`.
Build: Seam Touches 6 and 7: `CommandId::StartTour` everywhere the exhaustive matches require (`commands.rs:45-99` availability, `:102-200` registry, `:203-285` execute), `tutorial::initialize(cx)` inside `runtime_services_init`, `"tutorial"` in `DEFAULT_ENABLED_CAPABILITIES`. The palette picks up the registry entry automatically (`src/features/command_palette.rs:82` via `commands::registry()`).
Tests: none new (palette integration is automatic).
Done-check: command palette lists "Start guided tour"; executing it starts the tour; Diagnostics page shows the `tutorial` capability as supported+enabled.

## Phase 6: Docs, gates, and QA

### Step 6.1: Docs

Files: `docs/accessibility-checklist.md`, `src/features/tutorial/mod.rs` (final module doc).
Build: the live-region bullet (Seam Touches 9); confirm the `mod.rs` header states the contract and the removal recipe.
Done-check: `cargo test --workspace --all-features` passes `qa_docs` (sentinels intact).

### Step 6.2: Gates

Run, in order, and fix until green:
```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```
`just wasm-check` = `cargo +nightly check --target wasm32-unknown-unknown -p gpui-starter --lib` (`justfile:24-25`). No new deps, so `Cargo.lock` is untouched and `tests/qa_migration.rs` (dep/lock/patch guards) needs no changes.
Done-check: all four commands exit 0.

### Step 6.3: Manual QA (acceptance criteria)

On a dev build (`bash scripts/macos-dev-app.sh` on macOS, or the platform's normal run path):

1. Home → "Take a tour": dim mask covers the window including title bar and status bar; spotlight cutout + primary-colored glow ring sit on the Home heading with even padding; card shows below/above the target without leaving the window.
2. Enter/→ advances; ← goes back (Back button disabled on step 1); Escape and Skip close the tour; focus returns to the element focused before the tour.
3. Step 4 (sidebar Gallery row) spotlight tracks the real row; step 5 navigates to Gallery: one brief blocked-transparent frame, then the gallery-nav spotlight (no stale spotlight, no hang).
4. During any step, clicks on dimmed UI do nothing (occlude); the card's three buttons are clickable; Tab cycles only within the card (focus trap).
5. Resize the window mid-step: mask, cutout, and card reposition on the next frame.
6. Gallery → Motion → reduced-motion toggle on, restart the tour: card appears without the fade (with_animation static end state).
7. Switch locale to 简体中文 (Settings or first-run banner): card strings, command entry, and status announcements are Chinese; both catalogs carry every `tutorial_*` key (`cargo test` qa_localization enforces).
8. With a screen reader connected (native builds): each step change announces "Step n of m: <title>" via the Polite status region; the card is reachable as a Dialog.
9. Wasm build (`wasm/build.sh` per `justfile` wasm lane, or CI-equivalent check): tour runs visually; announcements are inert (no AccessKit bridge, `src/services/accessibility.rs:3-6`).
10. Open the command palette mid-tour (secondary-k): palette opens above the tour (kit Root deferred layers) and the tour stays intact behind it; if instead the palette is unreachable or paints under the mask, that is the Risks item 1 and must be reported, not papered over.

## Platform & Wasm Gating

- No `cfg` gates anywhere in the feature: the overlay, registry, capture, keyboard bindings, and commands are pure gpui + gpui-kit and compile on every target.
- No new crates. No `Cargo.toml` table changes (so the native-only `[target.'cfg(not(target_family = "wasm"))'.dependencies]` table and the wasm table at `Cargo.toml:356-417` are untouched), and `Cargo.lock` is not regenerated.
- Capability registry: `"tutorial"` registered `supported_enabled` in `DEFAULT_ENABLED_CAPABILITIES` (`src/app/init.rs:95-108`); it works on all platforms, so no degraded/unsupported entry. Diagnostics renders it automatically.
- Wasm degradation, per surface: dim mask/glow/card/keyboard all work; live-region announcements compile but are inert because the gpui web stack ships no AccessKit bridge (`src/services/accessibility.rs:3-6,67-69` — pre-existing, shared with every other live region in the app); reduced-motion animation gating still applies (`App::reduce_motion` is platform-independent).
- Native-only notes: none. No OS threads, no `std::time` in render paths (only `Animation`/`with_animation`, which use gpui's own clock), no single-instance/IPC involvement.

## i18n

Catalog facade: `crate::i18n::localize` / `localize_with_args` (`src/services/i18n.rs:3-13`). Keys are flat top-level scalars (grammar `tests/qa_localization.rs:20-71`), added to BOTH `locales/en.yml` and `locales/zh-CN.yml` (lockstep `qa_localization.rs:188-198`). `tutorial_step_status` uses `%{name}` placeholders resolved by `localize_with_args`. Step keys are resolved indirectly (`step.title_key`), invisible to the scanner (`qa_localization.rs:77-128`); the Phase 2 unit test `demo_tour_i18n_keys_resolve` closes that gap.

| key | en | zh-CN |
|---|---|---|
| `tutorial_command` | Start guided tour | 开始引导导览 |
| `tutorial_command_subtitle` | Walk through the app with a spotlight guide | 用聚光灯导览应用 |
| `tutorial_button_start` | Take a tour | 开始导览 |
| `tutorial_dialog` | Guided tour | 引导导览 |
| `tutorial_back` | Back | 上一步 |
| `tutorial_next` | Next | 下一步 |
| `tutorial_finish` | Finish | 完成 |
| `tutorial_skip` | Skip tour | 跳过导览 |
| `tutorial_step_status` | Step %{step} of %{total}: %{title} | 第 %{step} 步，共 %{total} 步：%{title} |
| `tutorial_step_home_heading_title` | Welcome | 欢迎 |
| `tutorial_step_home_heading_body` | This is the home page. Every page has a title and a short description. | 这是主页。每个页面都有标题和简短说明。 |
| `tutorial_step_home_cta_title` | Primary actions | 主要操作 |
| `tutorial_step_home_cta_body` | Get Started opens the docs. The outline button runs a demo background task. | “Get Started”打开文档，描边按钮会启动演示后台任务。 |
| `tutorial_step_sidebar_title` | Navigation | 导航 |
| `tutorial_step_sidebar_body` | Every page of the app is one click away in the sidebar. Next stop: the Gallery. | 应用的每个页面都能从侧边栏一键直达。下一站：画廊。 |
| `tutorial_step_gallery_nav_title` | Gallery sections | 画廊分区 |
| `tutorial_step_gallery_nav_body` | Pick a section from the list. Titles and descriptions are both searchable. | 从列表中选择分区。标题和描述都可用于搜索。 |
| `tutorial_step_gallery_search_title` | Search | 搜索 |
| `tutorial_step_gallery_search_body` | Type to filter sections by name or description while you browse. | 浏览时输入即可按名称或描述筛选分区。 |

Short labels carry no terminal punctuation; sentence bodies do (full-width in zh-CN), per the design-guide punctuation rules.

## Design Compliance

- Overlay choice (Feedback and overlays): the tour is a window-scoped spotlight overlay, the smallest surface that fits a multi-step guide; Escape dismisses the topmost layer; focus restores on dismissal. It is not a Dialog component because the spotlight geometry and per-step targeting are behavior this feature owns; the card still declares `Role::Dialog` and reuses the popover surface tokens.
- Surfaces and elevation: card uses `popover` background, `border`, `radius_lg`, and one shadow (`.shadow_lg()`); the kit's popover shadow helpers are `pub(crate)` (`GC/styled.rs:36,59`) so the scale helper is the compliant substitute. The dim mask uses the themed `overlay` scrim token (`GC/dialog/dialog.rs:319-326`), never a raw color; the only raw color value is `transparent_black()` as a quad background, which is colorless by construction.
- Color and emphasis: `primary` appears exactly three times (glow ring, Next/Finish button, active dot), matching "primary is reserved for the explicit default commit"; Skip is ghost, Back is outline; dots' inactive state uses `muted`. Step state is never color-alone: the "n / m" status text carries it.
- Density and spacing (Spatial grammar): card `p_4` (16 px panel padding, `lg`), children `gap_3` (12 px, one content group), dots row internal `gap_1` (4 px, parts of one control); card width `w_80` (20 rem) with wrapping body; all rem-based so zoom scales the card, and `place_card` clamps it inside the viewport.
- Motion (Motion section): one 120 ms card fade per step change via `with_animation` + `motion_tokens().duration_fast`; automatically disabled under `App::reduce_motion` (`GPUI/elements/animation.rs:74-82`); no ambient animation, no glow pulsing, mask never animates.
- Interaction states: Back disabled on step 1 (distinct disabled treatment, kit `Disableable`); all hover/pressed/focus-visible states come from kit Buttons; the overlay traps Tab (`focus_trap` on a roleless wrapper per `command_palette.rs:186-188`) and keeps a visible focus path; blocked UI is inert, not merely dimmed.
- Interface language: sentence-case labels, one-word command buttons, `Finish` on the last step (result verb), `Skip tour` names the whole action; no ellipsis (nothing deferred opens); zh-CN copy written natively with full-width punctuation, not translated word-by-word.
- Accessibility checklist: keyboard-complete path (arrows/Enter/Escape + Tab trap), focus restoration, live-region announcements of step changes through the app's existing Polite pattern (`render.rs:149-158`, `command_palette.rs:286-291`), icon-free labels, decorative dots excluded from the a11y tree, `Role::Dialog` naming the card.

## Verification

Repo gates (run verbatim, all must pass; AGENTS.md quality gates):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

Feature-specific automated coverage (all run as part of `cargo test` above; the in-module `#[cfg(test)]` sibling files also run under CI's `cargo test -p gpui-starter --lib`, `.github/workflows/ci.yml:245,285`):

- `target.test.rs`: publish dedupe/change flag; resolve readback; real-window capture through `TourTarget`.
- `tour.test.rs`: progress saturation; demo-tour target uniqueness; every step i18n key resolves in the embedded catalogs.
- `controller.test.rs`: keystroke constants parse; `start` without a `TourHandle` is a logged no-op.
- `mask.test.rs`: cutout padding symmetry; below/above/clamped placement; even-odd mask path builds.

Manual QA and acceptance criteria: Phase 6, Step 6.3 (ten checks covering spotlight geometry, keyboard paths, focus trap/restore, cross-page step, resize tracking, reduced motion, zh-CN, screen-reader announcements, wasm behavior, palette-over-tour stacking).

## Risks & Open Questions

Ranked:

1. **Kit Root deferred layers vs. the tour overlay (unverified ordering).** The tour overlay is the last child of `AppRoot`'s tree, but the kit `Root` hosts dialogs/notifications/menus as deferred layers that should paint above it. Read `gpui-kit-0.7.1/src/lib.rs:149-164` and `GB/root.rs`; the deferred-priority paint pass was not traced end-to-end and no build was run (doc-only scope). Mitigation: QA check 10; if the palette paints under the mask, the fix is raising the overlay into a deferred layer with a priority below dialog priority: a bounded change inside `controller.rs`.
2. **Context-free global bindings still fire during the tour.** `secondary-k` (palette) and `secondary-1..9` (page jumps) are bound with context `None` (`src/app/init.rs:289-298`, `state.rs:144-166`), so they act while the tour is focused; a page jump unmounts the current target and the overlay sits in the waiting state until Escape/arrows move it on. Pre-existing binding scope; not widened by this plan. Tried: no per-focus binding-scope mechanism exists in the binding API beyond contexts (`GPUI/keymap/binding.rs:33`).
3. **Waiting frame after route-changing steps.** One frame renders the transparent occluding placeholder between a Gallery-navigation step and its first capture: the step advance notifies and navigates, the next frame's layout pass renders the controller before the new page's prepaint has published the target, and the publish's change-notify produces the following frame's full overlay. Derived from the frame-ordering facts (`GPUI/view.rs:324-341`, `GPUI/window.rs:3581-3586`), not observed running. Accepted as specified behavior; if it flickers in QA, the fallback is painting the mask with the previous step's cutout during the wait.
4. **`TourTarget` element is hand-rolled.** Delegation template copied from `AnimationElement` (`GPUI/elements/animation.rs:382-493`); the one behavioral choice (`id() -> None`, so child element-id paths are unchanged) means the wrapper contributes no keyed state. If a future gpui version requires wrapper ids, the child id paths shift once; documented in the element's doc comment.
5. **Card height is estimated for placement.** `place_card` uses a 12 rem estimate; a pathological locale could exceed it in a very short viewport (window minimum height is 320 px, `src/app/window.rs:62-66`, versus a 192 px estimate plus gaps). Not blocking; would surface as slight overflow at minimum window size in QA check 1.
6. **Icon choice.** `IconName::Map` (compile-verified at `src/gallery/sections/buttons/icon.rs:138`) instead of the semantically nicer Compass: `compass.svg` exists in the asset dir, but `IconName` variants are generated at build time and the variant was not compilable-verifiable in a doc-only session (survey seams Open Questions). Swap to `IconName::Compass` is a one-line change if it compiles.
7. **Open question: first-run integration.** The spec asks for a demo tour with manual entry points (Home button + command); auto-starting on first run would touch `src/services/first_run.rs` and the config schema. Not planned; if wanted, it is a consumer of this plan's `start_tour` contract plus one config field.
8. **Residual citation drift, no action.** The reviewer confirmed the remaining cited ranges drift ±1-2 lines with correct substance (checked against the same crate sources); re-pinning every offset adds churn without changing any instruction, so only the materially wrong ones (`GB/popup.rs` canvas idiom, `GC/menu/popup_menu.rs` binding block, `src/app/window.rs` min-size block) were corrected.
