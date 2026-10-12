# 27 Desktop Widgets Plan

## Goal

A Desktop Widgets sidebar page lists four built-in widget kinds (clock, battery,
brightness/volume quick sliders, CPU/memory sparkline) with a per-kind show
toggle, a screen-position picker over plan 02's `Placement9`, always-on-top and
click-through toggles over plan 25, and a Wayland layer choice over plan 26.
Enabled widgets open as persistent on-desktop windows through plan 02's float
spawn path, their layout persists in `AppConfig` and respawns on launch, and
the page renders the widget-window capability rows (25/26 probes) so every
degradation is visible. Widgets that have no data plan behind them degrade
honestly: no battery data renders a no-data state, and kinds whose data plan
is not landed are hidden from the catalog (both stated exactly, below).

## Dependencies

Plans consumed, with the exact Interface Contract items:

- `docs/plans/02-floating-window-toolkit.md` (base contract block 02.md:286-407,
  including its plan-25 and plan-26 amendments at 02.md:429-484 and
  02.md:1135-1207): `FloatingSpec` builder set (`new/title/level/anchor/focus`
  plus the amended `click_through` and `layer_shell`), `FloatLevel`,
  `Placement9`, `FloatAnchor::Screen`, `FloatingWindowId`,
  `FloatingHandle::{Native, Overlay}`, `spawn_floating`, `close_floating`,
  `floating_snapshot`, `FloatingWindowEntry`, `DismissFloating` (Esc in every
  float shell), and the `FloatingWindowRegistry` global observed through
  `cx.observe_global`. 27 does NOT amend 02: this plan adds no window machinery
  and changes no contract signature, so no amendment note is required (the
  append-only discipline applies only when 02's contract changes).
- `docs/plans/25-widget-window-shims.md` (contract 25.md:362-417, restated in
  02.md:429-484): `FloatingSpec::click_through`/`is_click_through`,
  `set_floating_click_through(handle, enabled, cx) -> bool`,
  `set_floating_always_on_top(handle, enabled, cx) -> bool`, and the
  `floating_click_through` capability row.
- `docs/plans/26-layer-shell-widgets.md` (contract 26.md:304-419, restated in
  02.md:1146-1192): `LayerShellSpec`, `WidgetLayer`,
  `FloatingSpec::layer_shell`, `spawn_floating_after_displays`, the
  `layer_shell_widgets`/`layer_shell_input_passthrough` capability rows, and
  `FloatingWindowEntry.surface` (read to show which surface backs a widget).
- `docs/plans/17-system-controls.md` (contract 17.md:112-184): the
  `SystemControlsSnapshot` global, `snapshot(cx)`, `refresh(cx)`,
  `set_volume(level, cx)`, `set_brightness(level, cx)`, `ControlState<T>`,
  `BatteryStatus`, `BatteryChargeState`, `VolumeStatus`, `BrightnessStatus`.
  Consumed at `crate::system_controls::*` (17's lib re-export, 17 seam 2).
- `docs/plans/04-menubar-charts.md` (04.md:285-331 contract plus the crate
  items it marks internal at 04.md:323-325): `MetricsState::{new, start,
  set_paused, samples, is_paused}` and `Sample { seq, cpu_percent,
  mem_used_bytes, mem_total_bytes }` (04.md:224-237, 460-489), reached at
  `crate::features::menubar_charts::metrics` (04 declares `pub(crate) mod
  metrics;`, 04.md:407, so the path is crate-visible with no 04-side edit).
- `docs/plans/15-tutorial-mode.md` (contract 15.md:119-191): `tutorial::
  target(id, child)` wraps on the manager page, conditional on plan 15 being
  landed (grep guard; the same conditional-wrap pattern plan 24 uses for its
  optional plan-15 wiring, INDEX §8). No tour steps are added: 15's demo tour
  is a fixed five-step definition (15.md:154-158) and amending it is out of
  scope; 27 only marks targets so a future tour can use them.
- `docs/plans/09-floating-ui.md` (wasm posture precedent, 09.md:382-433):
  palettes degrade to in-window panels on wasm; widgets degrade the same way
  through 02's overlay float path, with the manager page itself routable.

Execution order: 02 → 25 → 26 → 27 (batch 7, INDEX §3; 25 before 26, 26 before
27). Forward reference, stated plainly: INDEX §3's batch-7 row currently
names only 25, with 26 recorded as pending its own row (`INDEX.md:124`);
this plan's Step 6.1 is what makes the row name all three. 17 (batch 5) and
04 (batch 2) land before 27 in batch order; the reduced-build fallback for
their absence is specified in Platform & Wasm Gating, never silently
assumed.

## Research Digest

Verified this session against the pinned sources on disk
(`ls ~/.cargo/registry/src/*/ | grep -E 'gpui-pre-0.3.9|gpui-kit-0.7.3'` →
both present; gpui-component-0.7.3, gpui-base-0.7.3, gpui-kit-assets-0.7.3,
chrono-0.4.45 also present and read directly). Short paths: `GPUI` =
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-0.3.9/src`,
`COMP` = `.../gpui-component-0.7.3/src`, `BASE` = `.../gpui-base-0.7.3/src`,
`ASSETS` = `.../gpui-kit-assets-0.7.3`, `REPO` = `/home/thragg/fo/gpui-starter`.
The desktop-widgets survey and its four evidence files were read in full;
everything load-bearing below was re-verified against the 0.3.9/0.7.3 sources
this session.

Window machinery (all owned by 02/25/26; 27 composes, never re-derives):

- `WindowKind::PopUp` is the widget window kind on macOS (non-activating
  NSPanel, popup level 101, all spaces), Windows
  (`WS_EX_TOOLWINDOW | WS_EX_TOPMOST`, no taskbar/Alt-Tab), and X11
  (override-redirect, `_NET_WM_WINDOW_TYPE_NOTIFICATION`); `WindowKind::
  LayerShell` is the Wayland surface (survey Answers; evidence
  `gpui-windowing.md:71-115, 159-171`). `FloatLevel::AlwaysOnTop` maps to
  `PopUp` uniformly (`02:667` mapping pinned by 25's amendment, 02.md:467-476).
- Click-through and runtime topmost are 25's shims; on Wayland the public
  `Window::set_input_region(Option<&[Bounds<Pixels>]>)` is the mechanism
  (`GPUI/window.rs:2513-2520`, re-read this session: `Some(&[])` = no pointer
  or touch input, `None` = reset). Runtime z-order on Wayland is
  compositor-owned; layer/level are creation-time only (26.md:118-121), so a
  Wayland layer or level change respawns the widget.
- `spawn_floating_after_displays` (26.md:403-407) is the zed#46378 workaround
  27 uses for respawn-on-launch: immediate when `cx.displays()` is non-empty,
  else 100 ms retries up to 10, then spawns anyway. The test platform's
  display list is non-empty (`GPUI/platform/test/platform.rs:513`), so the
  immediate arm is the one tests exercise.
- Session detection for the Wayland-only layer control: `App::compositor_name(
  ) -> &'static str` (`GPUI/app.rs:1877-1878`, verified this session; "Wayland"
  from the Wayland client, "X11" from the X11 client, `""` default elsewhere,
  26.md:137-143).

Observation and lifecycle (this session):

- `Context<T>::observe_global<G>(...)` at `GPUI/app/context.rs:168`; the
  manager page and widget views re-render off the `FloatingWindowRegistry`
  and `SystemControlsSnapshot` globals (page precedent
  `REPO/src/features/pages/notifications.rs:13-17`).
- Timer loop shape: `cx.spawn(async move |this, cx| loop { cx.
  background_executor().timer(DURATION).await; if this.update(cx, |this, cx|
  ...).is_err() { break; } })` inside `Context<Self>` (repo precedent
  `REPO/src/gallery/sections/charts/system_monitor.rs:217-230`; `Context::
  spawn` two-parameter closure expansion at `GPUI/app/context.rs:230,
  236-238`; `BackgroundExecutor::timer(Duration) -> Task<()>` at
  `GPUI/executor.rs:183`). The loop dies with the entity, so closing a widget
  window stops its sampling (04's own loop relies on the same property,
  04.md:232-237).
- `AnyWindowHandle::update` at `GPUI/window.rs:7545`; `Window::remove_window`
  at `:2384` (both reached only through 02's `close_floating`; 27 never calls
  them directly).
- Clock source: `crate::platform::clock` re-exports `std::time` natively and
  `web_time` on wasm (`REPO/src/platform/clock.rs:1-8`). `chrono` is a common
  dependency (`REPO/Cargo.toml:27`) with the wasm `wasmbind` feature in the
  wasm table (`REPO/Cargo.toml:371`); `Local` on wasm reads the timezone from
  `js_sys::Date` (chrono-0.4.45 `src/offset/local/mod.rs:68`), so
  `chrono::Local::now()` is the clock widget's time source on every target.
  No `std::time` clock is used outside the facade.

Component surface (verified against gpui-component-0.7.3 / gpui-base-0.7.3):

- `Switch::new(id).checked(bool).label(..).on_change(Fn(&bool, &mut Window,
  &mut App))` (`COMP/switch.rs:15,35,56,79-95`; `on_click` is the alias,
  switch.rs:78-83).
- `Toggle::new(id).label(..).tooltip(..).checked(bool).on_click(Fn(&bool,
  ..))` (`COMP/button/toggle.rs:34-113`): the placement picker's 3x3 keypad.
- `Slider::new(&Entity<SliderState>)` with `.disabled(bool)` and Styled
  (`COMP/slider.rs:86-133`); `SliderState` builders `min/max/step/
  default_value`, `set_value(value, window, cx)`, `value() -> SliderValue`
  (`BASE/src/slider.rs:170-285`); events `SliderEvent::{Change, Release(
  SliderValue)}` via `impl EventEmitter<SliderEvent> for SliderState`
  (`BASE/src/slider.rs:15-19, 867`). The slider element announces role, value,
  min, max, and step (`BASE/src/slider.rs:504-512`); it has no label builder,
  so each slider is paired with a visible adjacent label row.
- `AreaChart::new(owned_items).x(Fn(&T)->X).y(Fn(&T)->Y).stroke(Hsla).
  fill(Background).x_axis(bool).y_axis(bool).grid(bool).interactive(bool).
  y_domain(min,max).id(ElementId)` (`COMP/chart/area_chart.rs:26-290`,
  verified line by line this session). Plan 04's panel is the exact usage
  precedent including the `linear_gradient`/`linear_color_stop` fill
  (04.md:513-522; gradient fns at `GPUI/color.rs:865, 893`; chart color token
  `theme().chart_2` exists, `COMP/theme/schema.rs:367-379`). Charts take owned
  items (`T: 'static`), so every render clones the sample slice (04.md:540-543).
- `Button::label/.icon/.accessibility_label` at `COMP/button/button.rs:373,
  393, 399`.
- Element identity: `ElementId`'s complete `From` set
  (`GPUI/window.rs:7652-7754`, re-read this session) is scalars (`From<
  String>` at `:7676`, `From<SharedString>`, `From<&'static str>`, numeric
  and Uuid forms) plus 2-tuples whose second element is an integer or
  `EntityId` only (`(&'static str, usize)` at `:7712`, likewise u64/u32/
  EntityId, `(SharedString, usize)`). There is no `(&'static str, &str)`
  impl and no 3-tuple impl, so every repeated control id in this feature is
  a `format!`-built `String` (the 02 close-button precedent for exactly this
  gap, 02.md:149-153).
- Icon policy: `IconName` is generated from the default 106 only
  (`ASSETS/default-icons.txt`, 106 lines, re-read this session). Verified
  present: `battery-charging`/`battery-full`/`battery-low`/`battery-medium`/
  `battery-warning`/`battery` (:8-13), `chart-pie` (:20), `cpu` (:33),
  `memory-stick` (:61), `panel-bottom` (:70), `settings-2` (:87), `sun` (:96).
  NOT present: any clock, gauge, monitor, volume, or activity icon; the volume
  row uses the INDEX §4 stand-in `Settings2`, the memory footer uses
  `MemoryStick`, and the page/command icon is `PanelBottom`. No full-catalog
  variants are referenced anywhere in this plan.

Repo integration facts (read this session):

- `AppConfig` pub-field schema pinned by `tests/snapshot_tests.rs`
  (`REPO/src/state/config_store.rs:63-90`; default serialization fixture at
  `REPO/tests/snapshot_tests.rs:26-56`, round-trip literal from :59). New
  fields need `#[serde(default)]`; the 09/18 precedent for a persisted
  per-feature layout vector is `floating_palettes` with a `sanitize` step
  (09.md:612-640). `update_config` runs the closure synchronously then
  re-normalizes and arms the 300 ms debounced save; `with_config` is the
  borrowed read (`config_store.rs:274-287`, `:228-233`; warns and no-ops when
  the global is absent, so unit tests need no fixture, `:275-278`).
- Capability registry: `CapabilityStatus` five pub fields
  (`REPO/src/runtime/capabilities.rs:6-12`), `set` via `default_global`
  notify-on-write (`:64-79`), `snapshot(cx) -> BTreeMap` (`:81-85`). Features
  outside `DEFAULT_ENABLED_CAPABILITIES` register via `capabilities::set`
  beside startup (the 14/21 pattern, INDEX §4; the default list at
  `REPO/src/app/init.rs:95-109` is not touched).
- Boot seam: 02's `floating::init` hooks after
  `crate::notifications::initialize(cx);` (`REPO/src/app/init.rs:274`; 02 seam
  2, INDEX §5 init row); 27's hook goes after 02's line.
- Command surface: `CommandId` variant + `availability` arm + `command(id,
  "command_x", "command_x_subtitle", icon)` registry entry + `execute` arm
  navigating via `events::emit(Navigate)` (`REPO/src/services/commands.rs:
  12-29, 45-99, 102-200, 203-285, 287-289, 306-313`).
- Page seam: `Page` variant + `title/title_key/host/icon` arms + `all()`
  entry (`REPO/src/shell/sidebar.rs:4-105`), `VALID_HOSTS` line
  (`REPO/src/shell/route.rs:10-21`), AppRoot field/construct/view arm
  (seams survey §1 rows 10-12), `e2e_navigation.rs` count literal 10 → 11
  (`REPO/tests/e2e_navigation.rs:14-19`). Appended pages sit at index >= 10;
  digit bindings bind only `i < 9` (INDEX §3), so appending after `Gallery`
  rebinds nothing.
- i18n facade `crate::i18n::localize` with both catalogs in lockstep
  (`tests/qa_localization.rs:188-198`); scanner blind spot for indirect keys
  means the `nav_desktop_widgets` key is hand-added to both files (seams §5).
- Test wiring convention `#[cfg(test)] #[path = "<name>.test.rs"] mod
  <name>_test;` (`REPO/src/platform/desktop_shell/tray/windows.rs:12-14`).
  `#[gpui_kit::test]` with `TestAppContext` is the in-crate context-test
  precedent (`REPO/src/state/config_store.test.rs:280-281`, seams §7).
- CI lanes that compile and run this feature: macos-latest check
  (`REPO/.github/workflows/ci.yml:31`), ubuntu-24.04 check + lib tests
  (`:116, 206`), windows-latest check + lib tests (`:174, 255`). There is no
  macOS test lane; macOS-only behavior is check-lane compile plus manual QA.
- Kit-facade import rule: no `use gpui::` anywhere
  (`REPO/tests/qa_migration.rs:386-405`).

Survey-unverified items that bind this plan (restated so they are never
assumed): Windows click-through on the DirectComposition path; Mutter
honoring empty input regions; the `kCGDesktopWindowLevel` citation
(desktop-below on macOS stays out of scope, so the pinned layer choice is
offered on Wayland only); sparse-MSIX widget registration (no OS-surface
integration exists in this plan at all).

## Architecture

Entities and state ownership:

- `WidgetKind` (`Clock`, `Battery`, `System`, `Metrics`): value enum; the
  catalog's stable identity and the persistence key. `System` is the
  brightness/volume quick-sliders widget.
- `catalog.rs`: `available_kinds()` (data-plan-aware), per-kind defaults
  (window size, default placement, default layer), `PersistedWidget` +
  `sanitize` + `slot`/`save_slot` (the 09 persistence shape). Owns nothing
  live; pure data plus config reads/writes.
- `spawn.rs`: the composition layer and the only module that talks to 02/25/26.
  `widget_id(kind) -> FloatingWindowId`, `widget_spec(kind, &PersistedWidget,
  cx) -> FloatingSpec`, `widget_view_factory(kind) -> Rc<dyn Fn(&mut Window,
  &mut App) -> AnyView>`, and the public setters (`set_widget_enabled`,
  `set_widget_placement`, `set_widget_always_on_top`,
  `set_widget_click_through`, `set_widget_layer`) plus `spawn_persisted(cx)`.
  Stateless: open-state truth lives in 02's registry; desired-state truth
  lives in `AppConfig.widgets_layout`.
- Widget views (`clock.rs`, `battery.rs`, `system_sliders.rs`, `metrics.rs`):
  one `Render` entity each, built inside the float window by the factory.
  `ClockWidget` owns a 1 s tick loop; `BatteryWidget` observes
  `SystemControlsSnapshot` and owns a 30 s `system_controls::refresh` loop;
  `SystemSlidersWidget` owns two `SliderState` entities and the
  release-commits/suppress-window sync policy; `MetricsWidget` owns its own
  `MetricsState` entity (04's ring + 1 Hz loop). Loops exit when the entity
  drops, which happens when 02 closes the window.
- `DesktopWidgetsPage`: manager view. Observes `FloatingWindowRegistry`
  (open/closed rows) and re-reads `crate::capabilities::snapshot` for the six
  capability rows. Holds no widget state of its own; every control calls a
  `spawn.rs` setter and persists through `catalog::save_slot`.

Events and data flow:

```text
manager row toggle ─► set_widget_enabled(kind, on, cx)
    ├─ on:  catalog::save_slot(enabled) → spawn_floating(widget_spec, factory)
    │        (boot/respawn path uses spawn_floating_after_displays, 26)
    └─ off: save_slot(disabled) → floating_snapshot find id → close_floating
runtime toggles ─► set_floating_{always_on_top,click_through}(handle, ..) [25]
    └─ Wayland topmost returns false → respawn with new level (creation-time)
placement/layer change ─► save_slot → close_floating + spawn_floating (02 has
    no move API; respawn is the documented reposition path, 02.md:355-360)
launch ─► desktop_widgets::init: capability row + spawn_persisted (native)
wasm  ─► init skips spawn; the page's constructor DEFERS spawn_persisted via
    Window::defer (GPUI/window.rs:2607) so it runs after open_window returns
    and note_root_window lands (src/app/window.rs:104-107) — 02's overlay
    path needs the root window, 02.md:420-427, and page construction happens
    inside the build closure where the root is not yet noted
battery widget ─► observe SystemControlsSnapshot + 30 s refresh ─► 17's global
sliders widget  ─► SliderEvent::Release → system_controls::set_{volume,brightness}
metrics widget  ─► own 04 MetricsState ─► 1 Hz samples ─► AreaChart sparkline
clock widget    ─► 1 s tick ─► chrono::Local::now()
```

Module tree (new; nothing here is compiled into 02's module):

```text
src/features/pages/desktop_widgets/
├── mod.rs              DesktopWidgetsPage view, init(cx), CAPABILITY_DESKTOP_WIDGETS
├── mod.test.rs
├── catalog.rs          WidgetKind, defaults, PersistedWidget, sanitize, slot/save_slot
├── catalog.test.rs
├── spawn.rs            widget_id/spec/factory, the five setters, spawn_persisted
├── spawn.test.rs
├── clock.rs            ClockWidget (tick loop, chrono Local)
├── clock.test.rs
├── battery.rs          BatteryWidget (17 snapshot observe + refresh loop)
├── battery.test.rs
├── system_sliders.rs   SystemSlidersWidget (SliderState pair, release-commit)
├── system_sliders.test.rs
├── metrics.rs          MetricsWidget (04 MetricsState + AreaChart sparkline)
└── metrics.test.rs
```

cfg-gate diagram (which module compiles where; 27 itself adds no platform
shims, all platform reach lives in 02/25/26):

```text
                            macOS  Windows  Linux  wasm
catalog/spawn/mod/views       ✓       ✓       ✓      ✓   (views compile everywhere;
                                                     data feeds degrade at runtime)
spawn.rs layer attach         ✓       ✓       ✓      ✓   (guarded by compositor_name()
                                                     == "Wayland" at runtime; 26's
                                                     spec is cross-platform data)
init(cx) spawn_persisted      ✓       ✓       ✓     no   (#[cfg(not(target_family="wasm"))];
                                                     wasm respawns from the page)
```

Data-plan gates (compile-time, honest degradation):

- With 17 and 04 landed (the batch-order state): all four kinds available.
  `ControlState::Unsupported/Unavailable` renders a no-data state inside the
  battery and sliders widgets (17's own vocabulary; a desktop without a
  battery shows "No data" with the reason, never a fake value).
- Reduced build (17 absent): `Battery` and `System` kinds are hidden from
  `available_kinds()` and their files/arms deleted per the fallback recipe in
  Platform & Wasm Gating. Reduced build (04 absent): `Metrics` hidden the
  same way. `Clock` needs no data plan and is always available.

## Interface Contract

Public surface at `crate::features::pages::desktop_widgets` (all gpui types
via `gpui_kit::*`; no `use gpui::`, `tests/qa_migration.rs:386-405`). 02/25/26
types are referenced by their paths and are not re-owned here.

```rust
/// Built-in widget kinds. `System` is the brightness/volume quick-sliders
/// widget. Stable string forms are the persistence keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WidgetKind { Clock, Battery, System, Metrics }

impl WidgetKind {
    pub fn as_str(&self) -> &'static str;          // "clock" | "battery" | "system" | "metrics"
    pub fn from_str(s: &str) -> Option<Self>;
    /// i18n key for the display title (also the float title).
    pub fn title_key(&self) -> &'static str;       // widgets_kind_*
}

/// Kinds this build offers. Data-plan-dependent kinds drop out in reduced
/// builds (see the plan's fallback recipe); the clock never does.
pub fn available_kinds() -> Vec<WidgetKind>;

/// Desired layout for one widget, persisted in AppConfig.widgets_layout.
/// Fields are pub because this is a serialized record (schema-pinned).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PersistedWidget {
    pub kind: String,          // WidgetKind::as_str; unknown dropped by sanitize
    pub enabled: bool,         // default false
    pub placement: String,     // "top_left".."bottom_right"; unknown → "center"
    pub always_on_top: bool,   // default true (widgets are ambient)
    pub click_through: bool,   // default false
    pub layer: String,         // "above" | "pinned"; unknown → "above"; Wayland only
}

/// Sanitized desired layout from config (sanitize: known kinds only, one
/// entry per kind with first-wins, placement/layer normalized, cap 8).
pub fn widget_layout(cx: &App) -> Vec<PersistedWidget>;

/// Show/hide on the desktop. Persists the desired state; `on` spawns the
/// float (spawn errors log at warn and return false), `off` closes it through
/// plan 02. Returns false when the spawn/close did not complete.
pub fn set_widget_enabled(kind: WidgetKind, on: bool, cx: &mut App) -> bool;

/// Change the screen position: persists, then (if open) close + respawn at
/// the new anchor. 02 has no move API (02.md:355-360); respawn is the path.
pub fn set_widget_placement(kind: WidgetKind, placement: Placement9, cx: &mut App) -> bool;

/// Runtime always-on-top: persists, applies 25's toggle; when the runtime
/// apply returns false (Wayland: z-order is compositor-owned) the widget is
/// respawned at the new FloatLevel, which is creation-time on every platform.
pub fn set_widget_always_on_top(kind: WidgetKind, on: bool, cx: &mut App) -> bool;

/// Runtime click-through: persists, applies 25's toggle. Returns the shim's
/// bool (false on wasm overlay floats, test platform, gone handles).
pub fn set_widget_click_through(kind: WidgetKind, on: bool, cx: &mut App) -> bool;

/// Wayland layer choice ("above" = WidgetLayer::Top, "pinned" =
/// WidgetLayer::Background). Persists; if open, close + respawn so the new
/// LayerShellSpec applies (layer is creation-only, 26.md:118-121). Off
/// Wayland the persisted value is inert (one info log per process at spawn).
pub fn set_widget_layer(kind: WidgetKind, layer: &str, cx: &mut App) -> bool;

/// Respawn every enabled widget that is not already open. Uses 26's
/// spawn_floating_after_displays (zed#46378-safe). Native boot path; the
/// wasm page calls it on first open instead.
pub fn spawn_persisted(cx: &mut App);

/// The stable float id for a kind ("widget-clock" etc.), shared by spawn,
/// the manager rows, and tests.
pub fn widget_id(kind: WidgetKind) -> FloatingWindowId;

/// Capability row owned by this feature (aggregate widget-window posture).
pub const CAPABILITY_DESKTOP_WIDGETS: &str = "desktop_widgets";

/// Registers CAPABILITY_DESKTOP_WIDGETS (native supported_enabled; wasm
/// supported:false, enabled, degraded, overlay reason) and, native only,
/// runs spawn_persisted. Called once from app::init after floating::init.
pub fn init(cx: &mut App);
```

Guarantees for consumers: no setter panics on any platform; every setter
tolerates a missing widget (closed, never spawned, unsupported kind) and a
missing config global (`update_config` warns and no-ops,
`config_store.rs:275-278`, so the persisted desired state simply does not
survive a unit-test context). Widget content state does not survive a
respawn (placement/layer/level changes rebuild the view through the retained
factory); sliders re-read the 17 snapshot and metrics restart sampling, so
the loss is bounded to the clock's seconds phase, which is invisible. Esc
closes a focused widget through 02's shell-bound `DismissFloating`
(02.md:394-396); the next `refresh`/`observe_global` marks the row closed
and the desired state stays `enabled: true` in config only when the close
came from outside the manager (a respawn then happens on next launch; the
manager row shows closed, which is the visible truth).

## Module Layout

| File | Purpose |
| --- | --- |
| `src/features/pages/desktop_widgets/mod.rs` | Page view (rows, placement keypad, toggles, capability rows), `init(cx)`, capability const, wasm deferred respawn (Window::defer in `new`) |
| `src/features/pages/desktop_widgets/mod.test.rs` | Capability registration, page construct, row-state-from-registry tests |
| `src/features/pages/desktop_widgets/catalog.rs` | `WidgetKind`, per-kind defaults, `PersistedWidget`, `sanitize`, `slot`/`save_slot`, placement string mapping |
| `src/features/pages/desktop_widgets/catalog.test.rs` | Sanitize table, placement round-trip, defaults tests |
| `src/features/pages/desktop_widgets/spawn.rs` | `widget_id`/`widget_spec`/`widget_view_factory`, the five setters, `spawn_persisted` |
| `src/features/pages/desktop_widgets/spawn.test.rs` | Spawn/despawn/toggle lifecycle on the test platform |
| `src/features/pages/desktop_widgets/clock.rs` | `ClockWidget`: pure `clock_parts(chrono::DateTime<Local>)` + 1 s tick view |
| `src/features/pages/desktop_widgets/clock.test.rs` | `clock_parts` formatting table |
| `src/features/pages/desktop_widgets/battery.rs` | `BatteryWidget`: pure `battery_model(&ControlState<BatteryStatus>)` + observe/refresh view |
| `src/features/pages/desktop_widgets/battery.test.rs` | `battery_model` table incl. Unsupported/Unavailable |
| `src/features/pages/desktop_widgets/system_sliders.rs` | `SystemSlidersWidget`: `SliderState` pair, release-commit, suppress-window sync |
| `src/features/pages/desktop_widgets/system_sliders.test.rs` | Percent math + suppress-window tests |
| `src/features/pages/desktop_widgets/metrics.rs` | `MetricsWidget`: 04 `MetricsState` + AreaChart sparkline + memory footer |
| `src/features/pages/desktop_widgets/metrics.test.rs` | Footer formatting + view construct |

Every module wires its sibling test with `#[cfg(test)] #[path = "<name>.test.rs"]
mod <name>_test;` (`src/platform/desktop_shell/tray/windows.rs:12-14` precedent).

## Seam Touches

| # | File | Change | Why minimal |
| --- | --- | --- | --- |
| 1 | `src/features/pages/mod.rs` | `pub mod desktop_widgets;` + `pub use desktop_widgets::{DesktopWidgetsPage, WidgetKind};` | Standard two-line page mount; `pub mod` because `app/init.rs` calls `desktop_widgets::init` by path (09's directory-page precedent) |
| 2 | `src/shell/sidebar.rs` | `Page::DesktopWidgets` variant + the five match arms + `all()` entry appended after `Gallery` (index 10) | Exhaustive matches force exactly these; appended index >= 10 rebinds no digit key (INDEX §3) |
| 3 | `src/shell/route.rs` | `Page::DesktopWidgets.host(),` in `VALID_HOSTS` | One line |
| 4 | `src/shell/root/app_root/state.rs` | Field `desktop_widgets_page: Entity<DesktopWidgetsPage>` + construct in `new` + `unchecked_active_page_view` arm | Three touches, seams survey §1 rows 10-12 |
| 5 | `src/app/init.rs` | `crate::features::pages::desktop_widgets::init(cx);` directly after 02's `floating::init` line (after `notifications::initialize` at `:274`) | Single boot hook; must run after 02's registry/Esc init (INDEX §5 init ordering) |
| 6 | `src/state/config_store.rs` | `#[serde(default)] pub widgets_layout: Vec<crate::features::pages::desktop_widgets::catalog::PersistedWidget>,` on `AppConfig` + `widgets_layout: Vec::new(),` in `Default` + a `#[cfg(test)] pub(crate) fn install_for_tests(config: AppConfig, cx: &mut App)` that sets the `AppState` global directly from the provided config with no disk read and no debounced save armed (promote the module's own test scaffolding — `clean_state` + the debounce lock live private in `config_store.test.rs:281-285` — onto the cfg(test) surface so out-of-module feature tests can seed config safely; `AppState`'s fields are private at `:48-59`, so the constructor must live in this file) | One field + one test-only fn; import precedent `NotificationInboxItem` at `:18`; additive schema, no renames (survey §4); without the constructor, Steps 4.5/5.2's config-dependent tests cannot pass (update_config no-ops global-less at `:275-278`, and initialize risks the real state file) |
| 7 | `tests/snapshot_tests.rs` | Default-serialization inline yaml gains `widgets_layout: []`; every exhaustive `AppConfig {` literal gains `widgets_layout: Vec::new()` (grep the file; the round-trip literal at `:59+`) | Schema pin (`config_store.rs:63-65`); same class as the 09/18 fixture updates |
| 8 | `src/services/commands.rs` | `CommandId::OpenDesktopWidgets` + `availability` arm (always enabled) + `command(...)` entry with `IconName::PanelBottom` + `execute` arm `navigate(Page::DesktopWidgets, cx)` | The four mechanical arms every command adds |
| 9 | `tests/e2e_navigation.rs` | Count `10` → `11`, comment fix, `Page::DesktopWidgets` in `expected`, `("gpui-starter://desktop-widgets", AppRoute::Page(Page::DesktopWidgets))` route case | The page-count rule (INDEX §3) |
| 10 | `locales/en.yml` + `locales/zh-CN.yml` | The i18n table's keys in both, `nav_desktop_widgets` hand-added inside the nav block | Lockstep rule `tests/qa_localization.rs:188-198`; nav key is scanner-invisible |
| 11 | `docs/plans/INDEX.md` | The widgets track's deferred registration, executed once here: 26's rows (26 deferred them to the track owner, 26.md:992-995) plus 27's own: §2 catalog rows for 26 and 27; §3 batch-7 row and DAG updated to name 25 → 26 → 27 (25 is already registered at INDEX.md:72, :124, :290, :365; 26 is recorded as pending at INDEX.md:124); §4 capability-list entries for `layer_shell_widgets`/`layer_shell_input_passthrough` (26) and `desktop_widgets` (27) plus 27's i18n prefix row (26 adds no keys); §5 seam rows gain `26`/`27` where they touch shared files and a new 27-only config_store row; §6 removal order inserts `26` (currently absent from the leaf list) and `27` before `25`; §7 tracking rows for 26 and 27 | Docs-only; 27 is the last widgets-track plan to land, so it owns the track's single INDEX pass rather than editing rows a parallel session also owns |

No `Cargo.toml` change: no new crate, feature, or table entry. The plan uses
chrono (common table, `Cargo.toml:27`), kit components, and the existing
02/25/26 surface only. `Cargo.lock` is untouched.

### Removal Recipe

Mirror of `src/gallery/mod.rs:2-7` header style. Execute after nothing (27 is
a leaf; batch 7's 25/26 do not depend on it):

1. Delete `src/features/pages/desktop_widgets/` entirely.
2. `src/features/pages/mod.rs`: drop `pub mod desktop_widgets;` and the
   `pub use desktop_widgets::{..}` line.
3. `src/shell/sidebar.rs`: drop the `Page::DesktopWidgets` variant, its five
   match arms, and the `all()` entry.
4. `src/shell/route.rs`: drop the `VALID_HOSTS` entry.
5. `src/shell/root/app_root/state.rs`: drop the field, constructor line,
   import, and `unchecked_active_page_view` arm.
6. `src/app/init.rs`: drop the `desktop_widgets::init(cx);` line.
7. `src/state/config_store.rs`: drop the `widgets_layout` field, its `Default`
   line, and the type import. Old config files carrying a `widgets_layout`
   key keep parsing (serde ignores unknown fields; nothing denies them).
8. `tests/snapshot_tests.rs`: revert the inline yaml and literals.
9. `src/services/commands.rs`: drop `CommandId::OpenDesktopWidgets` and its
   three arms.
10. `tests/e2e_navigation.rs`: restore the count to the remaining page total
    and drop the added `expected` entry and route case.
11. Both locale files: drop every `widgets_*`, `nav_desktop_widgets`,
    `command_desktop_widgets*` key.
12. `docs/plans/INDEX.md`: drop the rows step 11 added.
13. Run `cargo fmt --all -- --check`, `cargo clippy --workspace
    --all-features`, `cargo test --workspace --all-features`,
    `just wasm-check`; grep `desktop_widgets`, `WidgetKind`,
    `widgets_layout`, `widget-clock` across `src/ tests/ locales/`: zero hits.

## Implementation Phases

Prerequisite: plans 02, 25, and 26 landed (`src/ui/floating/` exists with the
amended contract surface); 17 and 04 landed per batch order (reduced-build
fallback in Platform & Wasm Gating covers their absence). No unsafe code
exists anywhere in this feature (all platform reach is behind 02/25/26
public functions), so no step carries a safety argument; the CI lanes that
compile the feature are the macos-latest check (`ci.yml:31`), ubuntu-24.04
check + tests (`:116, 206`), and windows-latest check + tests (`:174, 255`).

## Phase 1: Module mount, catalog, persistence, locale keys

### Step 1.1: Mount the module

Files: `src/features/pages/desktop_widgets/mod.rs` (new),
`src/features/pages/mod.rs`.

Build: `mod.rs` with only the module doc comment naming the removal seams
(no module declarations yet; nothing references types that do not exist, so
the tree compiles). `pages/mod.rs` gains `pub mod desktop_widgets;`
(alphabetical, after `about`'s cfg block group, before `diagnostics`). The
capability const joins in Phase 5's `init` step, the page struct in Phase 5.

Tests: none.
Done-check: `grep -n "pub mod desktop_widgets" src/features/pages/mod.rs`
returns the line; `cargo check -p gpui-starter` green (an empty doc-only
module compiles).

### Step 1.2: `catalog.rs` kinds and defaults

Files: `src/features/pages/desktop_widgets/catalog.rs` (new),
`src/features/pages/desktop_widgets/mod.rs`,
`src/features/pages/mod.rs`.

Build: `mod.rs` gains `pub mod catalog;` now that the file exists, and
`pages/mod.rs` gains `pub use desktop_widgets::WidgetKind;` (the page
re-export joins in Phase 5). Then, in `catalog.rs`:

Build:
- `pub enum WidgetKind { Clock, Battery, System, Metrics }` with derives
  `Clone, Copy, Debug, PartialEq, Eq, Hash`; `as_str`/`from_str`/
  `title_key` per the contract (`from_str` is an associated fn, not From, to
  stay Option-returning).
- `pub(crate) struct KindDefaults { pub size: Size<Pixels>, pub placement:
  Placement9 }` and `pub(crate) fn defaults(kind) -> KindDefaults`:
  Clock `220x120` TopRight; Battery `240x140` TopLeft; System `260x180`
  BottomRight; Metrics `280x180` BottomLeft. Window geometry is the documented
  `px()` exception (external-surface boundary, design guides "Zoom, base font,
  and rem").
- `pub fn available_kinds() -> Vec<WidgetKind>`: `[Clock, Battery, System,
  Metrics]` in the full build; the data-plan gates are the two const fns
  `pub(crate) const fn system_controls_available() -> bool { true }` and
  `pub(crate) const fn metrics_available() -> bool { true }` (Battery/System
  filter on the first, Metrics on the second); the fallback recipe flips
  these to `false` and deletes the corresponding files.
- `pub(crate) fn placement_to_str(Placement9) -> &'static str` and
  `pub(crate) fn placement_from_str(&str) -> Option<Placement9>`: the nine
  snake_case forms ("top_left" .. "bottom_right", "center").
- `pub(crate) fn placement_offset(placement: Placement9) -> Point<Pixels>`:
  16 px inset from the touched edges (top row `y = 16`, bottom row `y = -16`,
  left column `x = 16`, right column `x = -16`, Center `(0, 0)`); 02 applies
  the offset after placement and clamps into visible bounds (02.md:564-571),
  so the sign convention is "push away from the anchor".

Tests: none yet (Step 1.5).
Done-check: `cargo check -p gpui-starter` green.

### Step 1.3: `PersistedWidget`, sanitize, slot/save

Files: `src/features/pages/desktop_widgets/catalog.rs`.

Build (09's persistence shape, 09.md:612-640):
- `PersistedWidget` exactly per the contract, derives `Clone, Debug,
  PartialEq, Serialize, Deserialize`.
- `pub(crate) fn sanitize(raw: &[PersistedWidget]) -> Vec<PersistedWidget>`:
  keep known `kind` strings only, one entry per kind (first wins), placement
  via `placement_from_str` else "center", layer normalized to "above" |
  "pinned" (unknown → "above"), cap the input at 8 entries.
- `pub fn widget_layout(cx: &App) -> Vec<PersistedWidget>`:
  `crate::app_state::with_config(cx, |c| sanitize(&c.widgets_layout))`.
- `pub(crate) fn save_slot(entry: PersistedWidget, cx: &mut App)`:
  `crate::app_state::update_config(cx, |config| { let v = &mut
  config.widgets_layout; v.retain(|p| p.kind != entry.kind);
  v.push(entry.clone()); })`; the one writer shape for every later phase.
- `pub(crate) fn slot(kind, cx) -> Option<PersistedWidget>`: read +
  `into_iter().find`; `None` means "not persisted yet" and callers compose
  defaults + `enabled: false`.

Tests: none yet.
Done-check: `cargo check -p gpui-starter` green.

### Step 1.4: Config schema and fixtures

Files: `src/state/config_store.rs`, `tests/snapshot_tests.rs`.

Build:
- `config_store.rs`: import `crate::features::pages::desktop_widgets::
  catalog::PersistedWidget` (the `NotificationInboxItem` precedent at `:18`);
  add `#[serde(default)] pub widgets_layout: Vec<PersistedWidget>,` to
  `AppConfig` after `show_frame_time` and `widgets_layout: Vec::new(),` to
  `Default`. No `normalized()` change: sanitize runs at every read (09's
  precedent for `floating_palettes`).
- `tests/snapshot_tests.rs`: add `widgets_layout: []` to the default
  serialization inline yaml; run `grep -n "AppConfig {" tests/snapshot_tests.rs`
  and add `widgets_layout: Vec::new(),` to every exhaustive literal (at
  minimum the round-trip literal).

Tests: the existing snapshot tests are the tests.
Done-check: `cargo test -p gpui-starter --test snapshot_tests` green.

### Step 1.5: Catalog tests

Files: `src/features/pages/desktop_widgets/catalog.test.rs` (new), wired in
`catalog.rs`.

Build (plain `#[test]`): `from_str(as_str(k)) == Some(k)` for all four;
`from_str("nope").is_none()`; placement round-trip for all nine +
`placement_from_str("garbage").is_none()`; `placement_offset` table (nine
cases, Center zero); sanitize: unknown kind dropped, duplicate kind
first-wins, unknown placement → "center", unknown layer → "above", cap at 8;
`available_kinds().len() == 4` in the full build.

Done-check: `cargo test -p gpui-starter --lib
features::pages::desktop_widgets::catalog` green.

### Step 1.6: Locale keys

Files: `locales/en.yml`, `locales/zh-CN.yml`.

Build: append the full i18n table below to both files (identical key sets);
`nav_desktop_widgets` goes inside the existing nav block. Quoted scalars,
one nesting level, `[a-z0-9_]` names (parser rules, seams §5).

Tests: `cargo test -p gpui-starter --test qa_localization` green.
Done-check: that command green; `diff <(grep -c . locales/en.yml)
<(grep -c . locales/zh-CN.yml)` not required (values differ), but the
lockstep test is the gate.

## Phase 2: Clock and metrics widgets

### Step 2.1: `clock.rs` pure parts

Files: `src/features/pages/desktop_widgets/clock.rs` (new),
`clock.test.rs` (new).

Build:
- `pub(crate) fn clock_parts(now: chrono::DateTime<chrono::Local>) ->
  (SharedString, SharedString)`: `("%H:%M".format, "%Y-%m-%d".format)`.
  Fixed formats are deliberate: numeric date and 24-hour time read naturally
  in both shipped locales, so no locale-conditional format table exists to
  drift (stated in Design Compliance).
- `pub(crate) struct ClockWidget { time: SharedString, date: SharedString }`.

Tests (`clock.test.rs`, plain `#[test]`): a fixed `DateTime::parse_from_rfc3339
(..).with_timezone(&Local)` input maps to the expected pair shape (assert
`.contains(':')` length 5 for time, `YYYY-MM-DD` shape for date; the local
offset makes exact literals environment-dependent, so shape-assert only).

Done-check: `cargo test -p gpui-starter --lib
features::pages::desktop_widgets::clock` green.

### Step 2.2: `ClockWidget` view

Files: `src/features/pages/desktop_widgets/clock.rs`.

Build:
- `impl ClockWidget { pub(crate) fn new(_window: &mut Window, cx: &mut
  Context<Self>) -> Self }`: `let now = chrono::Local::now(); let (time,
  date) = clock_parts(now);` then the tick loop, exactly the repo's shape
  (`system_monitor.rs:217-230`):

  ```rust
  cx.spawn(async move |this, cx| loop {
      cx.background_executor().timer(std::time::Duration::from_secs(1)).await;
      if this.update(cx, |this, cx| {
          let (time, date) = clock_parts(chrono::Local::now());
          this.time = time; this.date = date; cx.notify();
      }).is_err() { break; }
  }).detach();
  ```

  `std::time::Duration` is the timer's argument type and is fine (the wasm
  rule bans `Instant`/`SystemTime`, not `Duration`; the timer path is web-
  backed on wasm, 26.md:739-745).
- `impl Render`: `v_flex().size_full().items_center().justify_center().
  gap_1()`, time `text_xl().font_weight(FontWeight::BOLD)` foreground, date
  `text_sm()` muted. No icons (no clock icon exists in the default 106).
- `pub(crate) fn view(window: &mut Window, cx: &mut App) ->
  Entity<ClockWidget>` (`cx.new(|cx| Self::new(window, cx))`,
  `system_monitor.rs:193-195` idiom).

Tests: none new (loop is timer-based; manual QA covers the tick).
Done-check: `cargo check -p gpui-starter` green.

### Step 2.3: `metrics.rs` view

Files: `src/features/pages/desktop_widgets/metrics.rs` (new).

Build:
- `use crate::features::menubar_charts::metrics::{MetricsState, Sample};`
  (04's `pub(crate) mod metrics` path, 04.md:407).
- `pub(crate) struct MetricsWidget { metrics: Entity<MetricsState> }`;
  `new` creates the entity and calls `MetricsState::start(&entity, cx)`
  (04.md:470-489: the loop exits when the entity drops, so window close stops
  sampling); subscribes `cx.observe(&self.metrics, |_, _, cx| cx.notify())`.
- `pub(crate) fn memory_footer(last: &Sample) -> SharedString`: `format!(
  "{:.1} / {:.0} GiB", last.mem_used_bytes as f64 / GIB,
  last.mem_total_bytes as f64 / GIB)` with `const GIB: f64 =
  1024.0 * 1024.0 * 1024.0;` (04.md:544-545's constant).
- `impl Render`: `v_flex().size_full().p_3().gap_2()`:
  1. header row: `IconName::Cpu` + muted `text_sm`
     `localize("widgets_cpu")` label, current
     `format!("{:.0}%", last.cpu_percent)` foreground.
  2. sparkline: the exact 04 usage (04.md:513-522) with this feature's ids:
     `AreaChart::new(samples.iter().cloned().collect::<Vec<_>>())
     .x(|s| s.seq.to_string()).y(|s| s.cpu_percent as f64)
     .stroke(cx.theme().chart_2).fill(linear_gradient(0.,
     linear_color_stop(cx.theme().chart_2.opacity(0.4), 1.),
     linear_color_stop(cx.theme().background.opacity(0.1), 0.)))
     .x_axis(false).y_axis(false).grid(false).interactive(false)
     .y_domain(0., 100.).id("widgets-metrics-spark")` inside `h_16()`.
     Bind `let samples = state.samples().iter().cloned().collect::<Vec<_>>();`
     before the chart (owned items, `T: 'static`, 04.md:540-543).
  3. footer: `IconName::MemoryStick` + muted `localize("widgets_memory")` +
    `memory_footer(last)` foreground; empty list renders the footer with
    `localize("widgets_no_data")` instead (first sample lands within 1 s,
    but the branch exists and is honest).
- wasm: 04's fixture sampler is chosen by `MetricsState::new` under
  `cfg(target_family = "wasm")` (04.md:461-463), so the widget shows the same
  deterministic waveform as 04's wasm surface; nothing extra to gate.

Tests: none here (Step 2.4).
Done-check: `cargo check -p gpui-starter` green on the dev host;
`just wasm-check` stays green at Phase 7 (interim runs allowed).

### Step 2.4: Metrics tests

Files: `src/features/pages/desktop_widgets/metrics.test.rs` (new), wired in
`metrics.rs`.

Build (plain `#[test]` for the pure fn): `memory_footer(&Sample { seq: 1,
cpu_percent: 0.0, mem_used_bytes: GIB as u64, mem_total_bytes: 4 * GIB as
64 })` contains "1.0 / 4 GiB". Plus one `#[gpui_kit::test]` constructing the
widget entity (`cx.new(|cx| MetricsWidget::new_for_test(cx))`, where
`new_for_test` is `pub(crate)` and only builds the struct without `start`) to
prove the view type is constructible headless.

Done-check: `cargo test -p gpui-starter --lib
features::pages::desktop_widgets::metrics` green.

## Phase 3: Battery and system-sliders widgets

### Step 3.1: `battery.rs` pure parts

Files: `src/features/pages/desktop_widgets/battery.rs` (new),
`battery.test.rs` (new).

Build:
- `pub(crate) struct BatteryModel { pub icon: IconName, pub pct: String,
  pub state_key: &'static str, pub no_data: Option<SharedString> }`.
- `pub(crate) fn battery_model(state: &ControlState<BatteryStatus>) ->
  BatteryModel`: `Ready(b)` → pct `format!("{:.0}%", b.charge * 100.)`;
  icon/state mapping is exhaustive over 17's six `BatteryChargeState`
  variants (17.md:134), by design rather than fall-through: `Charging` →
  `BatteryCharging` + `widgets_battery_charging`; `Full` → `BatteryFull` +
  `widgets_battery_full`; `Discharging` → `widgets_battery_discharging`
  with the charge tier picking the icon (charge < 0.1 → `BatteryWarning`,
  < 0.25 → `BatteryLow`, else `BatteryMedium`); `Empty` → `BatteryLow`, no
  state line (charge is already near zero; the pct number carries it);
  `Paused`/`Unknown` → `Battery`, no state line (icons verified at
  `ASSETS/default-icons.txt:8-13`). `Unsupported { reason_key }` /
  `Unavailable { error }` → `Battery` icon, pct `"--"`, `no_data:
  Some(localize("widgets_no_data"))` plus the control's own reason text
  (`ControlState::reason`, 17.md:127-131).
- `pub(crate) struct BatteryWidget { _subscriptions: Vec<Subscription> }`.

Tests (`battery.test.rs`, plain `#[test]`): Ready table covering all six
`BatteryChargeState` variants (incl. `Empty` → BatteryLow with no state
line) plus the Discharging charge tiers picking Warning/Low/Medium;
Unsupported sets `no_data` with the reason key; Unavailable carries the
error text. Construct `ControlState` values directly (17's types are plain
data, 17.md:120-131).

Done-check: `cargo test -p gpui-starter --lib
features::pages::desktop_widgets::battery` green.

### Step 3.2: `BatteryWidget` view

Files: `src/features/pages/desktop_widgets/battery.rs`.

Build:
- `new`: `cx.observe_global::<crate::system_controls::SystemControlsSnapshot>(
  |_, cx| cx.notify())` (kept in `_subscriptions`,
  `notifications.rs:13-17` pattern; the global notify fires on every refresh
  swap, 17.md:64-66) plus the 30 s refresh loop (`system_monitor.rs:217-230`
  shape) calling `crate::system_controls::refresh(cx)`; `refresh` no-ops on
  wasm (17.md:175), and the widget renders the pending/no-data state there,
  which is the honest wasm posture for a hardware widget.
- `impl Render`: read `crate::system_controls::snapshot(cx).battery`, map
  through `battery_model`: `v_flex().size_full().p_3().gap_2().items_start()`
  with an `h_flex().gap_2().items_center()` header (icon, `text_sm` muted
  `localize(WidgetKind::Battery.title_key())`), then `text_2xl().font_weight(
  FontWeight::BOLD)` pct, then `text_sm` muted state line, then, when
  `no_data`, a muted two-line block: `widgets_no_data` + the reason. Color is
  never the sole cue: the warning-tier icon change always accompanies the pct
  number.
- `pub(crate) fn view(window, cx) -> Entity<BatteryWidget>` idiom.

Tests: none new (model covered in 3.1; loop is timer-based).
Done-check: `cargo check -p gpui-starter` green.

### Step 3.3: `system_sliders.rs` pure parts

Files: `src/features/pages/desktop_widgets/system_sliders.rs` (new),
`system_sliders.test.rs` (new).

Build:
- `pub(crate) fn percent_of(level: f32) -> f32`: `(level.clamp(0., 1.) *
  100.).round()` (slider side of 17's 0..=1 levels).
- `pub(crate) fn level_of(percent: f32) -> f32`: `(percent / 100.).clamp(0.,
  1.)` (commit side; 17's setters clamp again, 17.md:176-178).
- `pub(crate) fn should_sync(now: crate::platform::clock::Instant,
  suppress_until: Option<crate::platform::clock::Instant>) -> bool`:
  `suppress_until.is_none_or(|t| now >= t)` (the snapshot-sync gate; the
  facade keeps this wasm-safe, `src/platform/clock.rs:1-8`).

Tests (`system_sliders.test.rs`, plain `#[test]`): `percent_of` clamps both
ends and rounds; `level_of` clamps; `should_sync` true when None, false
before the deadline, true at/after it.

Done-check: `cargo test -p gpui-starter --lib
features::pages::desktop_widgets::system_sliders` green.

### Step 3.4: `SystemSlidersWidget` view

Files: `src/features/pages/desktop_widgets/system_sliders.rs`.

Build:
- `pub(crate) struct SystemSlidersWidget { brightness:
  Entity<SliderState>, volume: Entity<SliderState>, suppress_until:
  Option<crate::platform::clock::Instant>, _subscriptions:
  Vec<Subscription> }`.
- `new`: `cx.new(|_| SliderState::new().min(0.).max(100.).step(1.))` twice
  (`BASE/src/slider.rs:198-233` builders); default values seeded from
  `crate::system_controls::snapshot(cx)` where `Ready`, else 0;
  `cx.observe_global::<SystemControlsSnapshot>(..notify..)`; per-slider
  `cx.subscribe(&state, |this, _, event: &SliderEvent, _, cx|)` (the
  `EventEmitter` impl is verified at `BASE/src/slider.rs:867`) matching:

  ```rust
  match event {
      SliderEvent::Change(_) => {
          this.suppress_until = Some(crate::platform::clock::Instant::now()
              + std::time::Duration::from_secs(2));
      }
      SliderEvent::Release(v) => {
          let SliderValue::Single(percent) = v else { return; };
          this.commit(which, percent, cx);
      }
  }
  ```

  (`SliderValue` has exactly the `Single`/`Range` arms, `BASE/src/
  slider.rs:34-40`; the widget only builds Single sliders, so the `else`
  arm is unreachable but required).
- `fn commit(&mut self, which, percent: f32, cx)`: brightness →
  `crate::system_controls::set_brightness(level_of(percent), cx)`, volume →
  `set_volume(level_of(percent), cx)` (fire-and-forget, optimistic, 17.md:176-178,
  184). Then keep `suppress_until` set for one more refresh cycle (the
  optimistic write needs the round-trip to settle).
- Global observer handler `sync_from_snapshot`: when
  `should_sync(clock::Instant::now(), self.suppress_until)`, `set_value` both
  sliders from the snapshot where `Ready` (`set_value(value, window, cx)`,
  `BASE/src/slider.rs:273-283`; the observer has `&mut Window` through
  `observe_global_in`, `notifications.rs:13-17`).
- `impl Render`: `v_flex().size_full().p_3().gap_3()`, two rows, each
  `v_flex().gap_1()`: an `h_flex().gap_2().items_center()` label row
  (`IconName::Sun` + muted `localize("widgets_brightness")` + foreground
  `format!("{:.0}%", percent)`; `IconName::Settings2` +
  `localize("widgets_volume")` for volume, the INDEX §4 stand-in for the
  absent Volume2) above `Slider::new(&self.brightness).horizontal()` and
  `Slider::new(&self.volume)`. When the control is `Unsupported`/
  `Unavailable`, the row renders the muted `widgets_no_data` + reason line
  and the slider `.disabled(true)` (visible disabled state, not a silent
  no-op). The visible label row doubles as the slider's accessible name
  context (the kit Slider element exposes role/value/min/max itself,
  `BASE/src/slider.rs:504-512`; it has no label builder).
- `pub(crate) fn view(window, cx)` idiom.

Tests: none new (pure math in 3.3; interaction is manual QA 2/3/5/6).
Done-check: `cargo check -p gpui-starter` green.

## Phase 4: Spawn, despawn, toggles

### Step 4.1: `spawn.rs` spec and factory

Files: `src/features/pages/desktop_widgets/spawn.rs` (new).

Build:
- `const LOG: &str = "gpui_starter::features::desktop_widgets";` (tracing-
  target rule, seams §9).
- `pub fn widget_id(kind) -> FloatingWindowId`: `FloatingWindowId(
  format!("widget-{}", kind.as_str()).into())`.
- `pub(crate) fn widget_spec(kind, persisted: &PersistedWidget, cx: &App) ->
  FloatingSpec`:

  ```rust
  let defaults = catalog::defaults(kind);
  let placement = catalog::placement_from_str(&persisted.placement)
      .unwrap_or(defaults.placement);
  let mut spec = FloatingSpec::new(widget_id(kind), defaults.size)
      .title(crate::i18n::localize(kind.title_key()))
      .level(if persisted.always_on_top { FloatLevel::AlwaysOnTop } else { FloatLevel::Floating })
      .anchor(FloatAnchor::Screen {
          display: None,
          placement,
          offset: catalog::placement_offset(placement),
      })
      .focus(false)
      .click_through(persisted.click_through);
  if cx.compositor_name() == "Wayland" {
      let layer = if persisted.layer == "pinned" { WidgetLayer::Background } else { WidgetLayer::Top };
      spec = spec.layer_shell(LayerShellSpec::new().layer(layer));
      // namespace stays LayerShellSpec's default "gpui-starter.widgets" (26.md:341-344)
  }
  spec
  ```

  Both layer values ride layer-shell on Wayland (taskbar-free either way);
  26's spawn absorbs a Mutter rejection into the float fallback
  (02.md:1194-1207), so `pinned` on GNOME degrades to a plain float with the
  degraded capability row, never an error.
- `pub(crate) fn widget_view_factory(kind) -> Rc<dyn Fn(&mut Window, &mut
  App) -> AnyView>`: match kind → `Rc::new(|window, cx| cx.new(|cx|
  ClockWidget::new(window, cx)).into())` (and the other three views); the
  `Fn` bound matches 02's retained-factory constraint (02.md:413-418).

Tests: none here.
Done-check: `cargo check -p gpui-starter` green.

### Step 4.2: Enable/disable and `spawn_persisted`

Files: `src/features/pages/desktop_widgets/spawn.rs`.

Build:
- `pub fn set_widget_enabled(kind, on, cx) -> bool`:
  - persist: compose `slot(kind, cx).unwrap_or(default_entry(kind))` with
    `enabled: on` and `save_slot` (`default_entry`: defaults + `enabled:
    false, always_on_top: true, click_through: false, layer: "above"`).
  - `on`: `let entry = ..; let spec = widget_spec(kind, &entry, cx); match
    spawn_floating(spec, widget_view_factory(kind), cx) { Ok(_) => true,
    Err(err) => { tracing::warn!(target: LOG, error = %err, kind =
    kind.as_str(), "widget spawn failed"); false } }`. A same-id respawn of
    an already-open widget is the caller's (manager) responsibility; the
    manager toggles only when the row state says closed.
  - `off`: find the entry: `floating_snapshot(cx).into_iter().find(|e| e.id
    == widget_id(kind))`; `close_floating(&e.handle, cx)` when found; return
    the found/closed bool.
- `pub fn spawn_persisted(cx: &mut App)`: for each sanitized
  `widget_layout(cx)` entry with `enabled` and `available_kinds().contains(
  kind)`: skip when `floating_snapshot(cx)` already holds `widget_id(kind)`;
  else `spawn_floating_after_displays(widget_spec(kind, &entry, cx),
  widget_view_factory(kind), cx)` (26.md:403-407; logs its own outcome). On
  the test platform `displays()` is non-empty so the immediate arm runs.

Tests: Step 4.5.
Done-check: `cargo check -p gpui-starter` green; `grep -n
"spawn_floating_after_displays" src/features/pages/desktop_widgets/spawn.rs`
shows the boot call.

### Step 4.3: Placement and layer setters

Files: `src/features/pages/desktop_widgets/spawn.rs`.

Build:
- `pub fn set_widget_placement(kind, placement, cx) -> bool`: save_slot with
  `placement: placement_to_str(placement)`; then `respawn(kind, cx)`.
- `pub fn set_widget_layer(kind, layer: &str, cx) -> bool`: validate `layer
  == "above" || layer == "pinned"` else return false; save_slot; `respawn`.
- `fn respawn(kind, cx) -> bool`: if open (snapshot entry found):
  `close_floating(&handle, cx)` then `spawn_open(kind, cx)`; if closed:
  true (desired state persisted; the next spawn uses it). `spawn_open` is
  the extracted spawn half of `set_widget_enabled` (spec + factory +
  `spawn_floating` + warn-log; `set_widget_enabled(kind, true, cx)` calls it
  too). Rationale: 02 has no move API and 26's layer is creation-only
  (02.md:355-360, 26.md:118-121), so close+respawn is the documented
  reposition path; the retained factory makes content rebuild automatic.

Tests: Step 4.5.
Done-check: `cargo check -p gpui-starter` green.

### Step 4.4: Runtime toggles

Files: `src/features/pages/desktop_widgets/spawn.rs`.

Build:
- `pub fn set_widget_click_through(kind, on, cx) -> bool`: persist the field;
  find the open handle; `set_floating_click_through(&handle, on, cx)` (25;
  returns false on wasm/test/overlay, never errors); closed widget → true
  (persisted, applied at next spawn through `widget_spec`).
- `pub fn set_widget_always_on_top(kind, on, cx) -> bool`: persist; find the
  handle; `let applied = set_floating_always_on_top(&handle, on, cx);` if
  `!applied` and the handle is open: `respawn(kind, cx)` (Wayland: z-order
  is compositor-owned and the level is creation-time; the respawn applies
  the persisted level through `widget_spec`). Return `applied ||
  respawned`.

Tests: Step 4.5.
Done-check: `cargo check -p gpui-starter` green.

### Step 4.5: Lifecycle tests

Files: `src/features/pages/desktop_widgets/spawn.test.rs` (new), wired in
`spawn.rs`.

Build (`#[gpui_kit::test]`, test platform; 02's spawn works headless,
`GPUI/platform/test/platform.rs:493-506` via 02.md:707-709):
- `set_widget_enabled(Clock, true, cx)` → `floating_snapshot(cx)` has
  `widget_id(Clock)` with a `Native` handle;
- `set_widget_enabled(Clock, false, cx)` → entry gone (02's close tracker
  removes it at the effects boundary; also assert via a second snapshot
  after `cx.run_until_parked()` if the platform defers);
- `set_widget_enabled(Battery, true, cx)` on the test platform → true; the
  widget renders the pending/no-data state without panicking (17's global
  absent in the test context: `snapshot(cx)` falls back to the pending
  default, `cx.try_global...unwrap_or_else(pending_snapshot)` at
  17.md:245);
- `set_widget_placement(Clock, Placement9::BottomLeft, cx)` while open →
  still exactly one Clock entry (respawn swapped it), and
  `widget_layout(cx)` carries `bottom_left`;
- `set_widget_click_through(Clock, true, cx)` → returns `false` on the test
  platform (no shim, 25.md:382-384), no panic, persisted value is `true`;
- `set_widget_always_on_top(Clock, false, cx)` → test platform returns
  `false` from the shim then the respawn path re-spawns (snapshot still one
  entry) and returns true overall; persisted `always_on_top == false`;
- `spawn_persisted(cx)` with a config carrying one enabled clock entry,
  seeded via `crate::app_state::install_for_tests(config, cx)` (the
  cfg(test) constructor this plan adds to config_store — see Seam Touches;
  NOT `update_config`, which no-ops without an `AppState` global and, with
  `app_state::initialize`, would load and debounce-save the developer's
  REAL state.json) → one Clock entry; calling it again is
  idempotent (still one; the already-open guard);
- every setter on a closed/unknown kind returns without panicking
  (`set_widget_layer(Metrics, "pinned", cx)` closed → true + persisted).

Done-check: `cargo test -p gpui-starter --lib
features::pages::desktop_widgets::spawn` green.

## Phase 5: Manager page and app seam

### Step 5.1: `DesktopWidgetsPage` view

Files: `src/features/pages/desktop_widgets/mod.rs`.

Build:
- `pub struct DesktopWidgetsPage { _subscriptions: Vec<Subscription>,
  wasm_spawned: bool }`; `new(window, cx)`: subscribe
  `cx.observe_global_in::<crate::ui::floating::FloatingWindowRegistry>(
  window, |_, _, cx| cx.notify())` (row open state follows the registry).
- wasm first-open respawn: in `new`, under
  `#[cfg(target_family = "wasm")]`, `window.defer(cx, move |_, cx| {
  spawn_persisted(cx); })` (`Window::defer` at `GPUI/window.rs:2607`).
  DEFERRED, not called inline — post-review fix: `AppRoot::new` constructs
  every page entity INSIDE `gpui_kit::open_window`'s build closure
  (`src/shell/root/app_root/state.rs:43-53`), and `note_root_window` runs
  only after `open_window` returns (`src/app/window.rs:104-107`; wasm
  shares the path via `create_new_window`, `src/web.rs:48`), so at
  construction time `root_window(cx)` is `None` and an inline
  `spawn_persisted` would fire 02's single wasm Err for every enabled
  widget. The deferred closure runs after the effect cycle completes —
  root window noted, 02's overlay path valid. There is no distinct
  "first page open" event — construction IS boot. The `wasm_spawned` flag
  is dropped: the page is constructed exactly once, so one deferred call
  suffices; struct keeps only `_subscriptions`.
- `impl Render` (structure mirrors `notifications.rs`'s page layout):
  1. Heading row: `div().id("widgets-title").a11y(Role::Heading,
     localize("widgets_title")).aria_level(1).text_xl().font_weight(
     FontWeight::BOLD)` + muted `text_sm` subtitle
     `localize("widgets_subtitle")`. Wrapped in the plan-15 conditional
     target (Step 5.1b).
  2. Status line: `localize_with_args("widgets_open_count", &[("count",
     &open_count.to_string())])` muted `text_sm` (the facade takes
     `&[(&str, &str)]`, `src/services/i18n.rs:12`).
  3. One row card per `available_kinds()` (kept in a fixed catalog order:
     Clock, Battery, System, Metrics): `v_flex().gap_2().p_4().
     border_1().border_color(cx.theme().border).rounded(cx.theme().radius)`
     (one card treatment, tokens only). Row header `h_flex().gap_2().
     items_center().justify_between()`: leading `h_flex().gap_2()` with the
     kind icon rendered as a child element (Clock → none, title only;
     Battery → `IconName::Battery`; System → `IconName::Sun`; Metrics →
     `IconName::Cpu`; an `IconName` is `IntoElement` directly,
     `COMP/icon.rs:53-57`) + `text_sm` title (foreground,
     `font_weight(FontWeight::MEDIUM)`, the repo's own usage precedent
     `src/gallery/page.rs`); trailing `Switch::new(format!(
     "widgets-switch-{}", kind.as_str())).checked(open).
     label(localize("widgets_enabled")).
     on_change(cx.listener(|_, on, _, cx| { set_widget_enabled(kind, *on,
     cx); }))` (controlled value: the switch renders registry state, the
     listener requests the change; `switch.rs:88-97` contract). Every
     repeated control id in this step is a `format!`-built `String` because
     `ElementId` has no `(&'static str, &str)` or 3-tuple `From`
     (`GPUI/window.rs:7652-7754`, Research Digest).
  4. When the row is open, a controls block `v_flex().gap_3().pl_6()`:
     - Placement: muted `text_sm` `localize("widgets_placement")` label
       above a 3x3 `h_flex` grid of `Toggle::new(format!("widgets-pos-{}-{}",
       kind.as_str(), code)).small().checked(current == cell).label(code)`.
       `.tooltip(localize(full placement name))` (tooltip builder at
       `toggle.rs:72-77`; `.small()` via the `Sizable` impl at
       `toggle.rs:144`), `.on_click(cx.listener(move |_, _, _, cx| {
       set_widget_placement(kind, cell, cx); }))`. `code` is the
       two-letter keypad code ("TL", "T", "TR", "L", "C", "R", "BL", "B",
       "BR"); the nine codes are stable domain ids, not translations.
     - Two `Switch` rows: `widgets_always_on_top` and `widgets_click_through`
       (checked from the persisted slot, listeners calling the Step 4.4
       setters). On Wayland the always-on-top switch is the respawn-backed
       one; the row shows the muted note `widgets_toggle_respawn` (state
       visible: the change applies on respawn).
     - Layer row (Wayland only: rendered when `cx.compositor_name() ==
       "Wayland"`): `RadioGroup::new(format!("widgets-layer-{}",
       kind.as_str())).selected_index(pinned?1:0).on_change(cx.listener(
       move |_, ix, _, cx| { set_widget_layer(kind, if *ix == 1 { "pinned" }
       else { "above" }, cx); }))` with two children `Radio::new(format!(
       "widgets-layer-{}-above", kind.as_str())).label(localize(
       "widgets_layer_above"))` and the `pinned` twin
       (`Radio::label` at `radio.rs:78`; `RadioGroup::on_change Fn(&usize)`
       at `radio.rs:336-342`; Radio at `:41`). Off Wayland the row renders
       disabled-style muted text `localize("widgets_layer_off_wayland")`
       (visible reason, control absent rather than silently missing).
  5. Capability section: muted `text_sm` heading
     `localize("widgets_capabilities")`, then for each of the six keys
     (`desktop_widgets`, `floating_windows`, `floating_always_on_top`,
     `floating_click_through`, `layer_shell_widgets`,
     `layer_shell_input_passthrough`) present in
     `crate::capabilities::snapshot(cx)`: an `h_flex().justify_between()`
     row, `text_sm` name (monospace-styled identifier is not required; plain
     name) + status text: supported+enabled → `localize(
     "widgets_cap_supported")` muted; degraded/error → warning-colored
     `localize("widgets_cap_degraded")`/`localize("widgets_cap_error")`
     foreground plus the reason string when present. Text carries the state;
     color only accents it.
  6. Tutorial targets (conditional, Step 5.1b): heading wrap
     `widgets-heading`, the rows container wrap `widgets-list`, the
     capability section wrap `widgets-controls`.
- Row open-state derivation: `let open = |kind| floating_snapshot(cx).
  iter().any(|e| e.id == widget_id(kind));` computed once per render into a
  small `BTreeMap<WidgetKind, bool>`.

Tests: Step 5.2.
Done-check: `cargo check -p gpui-starter` green.

### Step 5.1b: Plan-15 conditional target wraps

Files: `src/features/pages/desktop_widgets/mod.rs`.

Build: grep guard first (`ls src/features/tutorial 2>/dev/null`): when plan
15 is landed, the three region wraps become
`crate::features::tutorial::target("widgets-heading", heading_el).into_any_element()`
(etc.; `target()` returns `TourTarget: IntoElement`, 15.md:124) and
`use crate::features::tutorial` joins the imports. When absent, no wrap and
no import. The ids `widgets-heading`, `widgets-list`, `widgets-controls` are
reserved here regardless (the registry pattern 24 uses for its five ids).

Tests: none (targets are layout-transparent, 15.md:120-123).
Done-check: `grep -c 'tutorial::target("widgets-' src/features/pages/desktop_widgets/mod.rs`
returns 3 when 15 is landed, 0 otherwise; a green check is required only for
the state that exists at implementation time.

### Step 5.2: Page tests

Files: `src/features/pages/desktop_widgets/mod.test.rs` (new), wired in
`mod.rs`.

Build (`#[gpui_kit::test]`):
- The render's data layer is two extracted `pub(crate)` fns so it is testable
  without a window: `pub(crate) fn row_kinds(cx: &App) -> Vec<WidgetKind>`
  (renders `available_kinds()`; the render maps over it) and
  `pub(crate) fn open_map(cx: &App) -> std::collections::BTreeMap<
  WidgetKind, bool>` (the kind→open derivation from `floating_snapshot`).
  Tests: `row_kinds(cx).len() == available_kinds().len()` (4 in the full
  build); `open_map(cx)` is all-false on a fresh context; after
  `set_widget_enabled(Clock, true, cx)` the Clock entry flips true.
- `init(cx)` in a context where `crate::capabilities::initialize(cx)` has
  run first: `crate::capabilities::snapshot(cx)` contains
  `desktop_widgets` with `supported: true, enabled: true` (native test
  host; the `02.md:870-873` probe-test pattern).
- `init` calls `spawn_persisted` on non-wasm builds (the test host is
  non-wasm): with an empty config (seeded via
  `crate::app_state::install_for_tests(AppConfig::default(), cx)` — the
  cfg(test) constructor this plan adds to config_store; `update_config`
  alone no-ops without the global and `app_state::initialize` would touch
  the developer's real state file), `floating_snapshot(cx)` stays empty;
  after `install_for_tests(with_clock_entry(), cx)` and a second `init(cx)`
  call, the snapshot holds
  exactly the Clock entry (also proves idempotence of the already-open
  guard).

Done-check: `cargo test -p gpui-starter --lib
features::pages::desktop_widgets` green.

### Step 5.3: Sidebar, route, AppRoot, pages mount

Files: `src/shell/sidebar.rs`, `src/shell/route.rs`,
`src/shell/root/app_root/state.rs`, `src/features/pages/mod.rs`.

Build:
- `Page::DesktopWidgets` variant; arms: title "Desktop Widgets",
  `title_key "nav_desktop_widgets"`, `host "desktop-widgets"`, `icon
  IconName::PanelBottom`; `all()` entry appended after `Page::Gallery`.
- `VALID_HOSTS` gains `Page::DesktopWidgets.host(),`.
- AppRoot: field, construct `cx.new(|cx| DesktopWidgetsPage::new(window,
  cx))`, view arm.
- `pages/mod.rs`: add `pub use desktop_widgets::DesktopWidgetsPage;` to the
  existing `pub mod desktop_widgets;` from Step 1.1.

Tests: existing suites.
Done-check: `cargo test -p gpui-starter --lib` green; `grep -n
"DesktopWidgets" src/shell/sidebar.rs` shows the arms.

### Step 5.4: Navigation test

Files: `tests/e2e_navigation.rs`.

Build: count 10 → 11 (comment "eleven feature pages + Gallery" per the
literal's own wording convention: update the parenthetical), expected array
gains `Page::DesktopWidgets` (after Gallery to mirror `all()`), route case
`("gpui-starter://desktop-widgets", AppRoute::Page(Page::DesktopWidgets))`.

Done-check: `cargo test -p gpui-starter --test e2e_navigation` green.

### Step 5.5: Command and boot hook

Files: `src/services/commands.rs`, `src/app/init.rs`.

Build:
- `CommandId::OpenDesktopWidgets`; `availability` arm: always enabled; a
  registry entry `command(CommandId::OpenDesktopWidgets,
  "command_desktop_widgets", "command_desktop_widgets_subtitle",
  IconName::PanelBottom)`; `execute` arm `navigate(Page::DesktopWidgets,
  cx)`. The command opens a page immediately: no ellipsis in its label
  (design guides "Capitalization, punctuation, and symbols").
- `src/app/init.rs`: directly after 02's `crate::ui::floating::init(cx);`
  line (which follows `notifications::initialize` at `:274`):
  `crate::features::pages::desktop_widgets::init(cx);`.
- `init(cx)` body (in `mod.rs`, if not already built in Step 5.2's
  context): `crate::capabilities::set(CAPABILITY_DESKTOP_WIDGETS, status,
  cx)` with status native = `CapabilityStatus::supported_enabled()`, wasm =
  the five-field literal `{ supported: false, enabled: true, degraded: true,
  reason: Some("widgets render as in-window overlay panes on the web
  build".into()), last_error: None }` (mirrors 02's `floating_windows` wasm
  row, 02.md:963); then `#[cfg(not(target_family = "wasm"))]
  spawn_persisted(cx);`.

Tests: Step 5.2 covers the capability row; boot smoke via existing
`tests/e2e_lifecycle.rs`.
Done-check: `grep -n "desktop_widgets::init" src/app/init.rs` returns the
line; `cargo test -p gpui-starter --lib` green.

## Phase 6: INDEX registration

### Step 6.1: INDEX rows for the widgets track (26's deferred rows plus 27's)

Files: `docs/plans/INDEX.md`.

Context: 25 is already registered (INDEX.md:72 §2, :124 §3, :290 §5, :365
§7). 26 is on disk but deliberately unregistered ("26 on disk, pending its
own INDEX row", INDEX.md:124; 26.md:992-995 records the deferral: parallel
sessions would edit the same table rows, so the track owner adds all three
at once, and 26 never claims INDEX registration). 27 is the last plan of
the track, so this step is that deferred pass and registers both 26 and 27
in one edit. Build (docs-only, additive):

- §2 catalog: a row for 26, `| 26 | Layer shell widgets | Wayland
  layer-shell surface option on 02 floats (Layer::Background/Top), set_input
  passthrough, zed#46378 deferred spawn, capability rows; Mutter fallback |
  02 | none (27 consumes its contract) | native Wayland only; X11/mac/win/
  wasm untouched | 4 |` (scope text from 26.md:5-13; phase count 4 from
  26's `## Phase` headings, the INDEX §2 counting rule), and the 27 row:
  `| 27 | Desktop widgets manager | sidebar page: widget kinds with
  spawn/despawn, Placement9 picker, always-on-top + click-through (25),
  Wayland layer (26), persisted layout, respawn-on-launch, capability rows;
  built-ins: clock, battery, quick sliders (17), CPU/mem sparkline (04) |
  02, 25, 26 (17/04 data feeds) | sidebar page | native windows + Wayland
  layer-shell; wasm overlay panes | 7 |`. The header's widgets-track note
  (INDEX.md:10-16) gains plan 27 and replaces its "which owns its own INDEX
  pass" clause for 26 with the fact that this step registered it.
- §3: the batch-7 row (INDEX.md:124) is rewritten to `| 7. widgets track |
  25, 26, 27 | after 02 (batch 1); serially in that order: 25 amends 02 and
  adds the shims; 26 layers the Wayland surface option (second lander greps
  first); 27 consumes both contracts (also after 17/04 for the data-fed
  widgets) |`; in the DAG the `├─ 25 widget shims (amends 02; Wayland
  dispatch shared with 26)` line becomes `├─ 25 widget shims ─ 26 layer
  shell ─ 27 desktop widgets (25/26 amend 02 append-only; 27 consumes both)`.
- §4 i18n prefix table: `| 26 | none (no keys; capability reason literals
  only, 26.md:836-842) | - |` and `| 27 | widgets_* (+ command_desktop_
  widgets*) | nav_desktop_widgets |`; capability list gains
  `layer_shell_widgets`, `layer_shell_input_passthrough` (26) and
  `desktop_widgets` (27; set via `capabilities::set` inside `init`, not in
  `DEFAULT_ENABLED_CAPABILITIES`).
- §5 seam map: append `27` to the rows for `sidebar.rs`, `route.rs`,
  `app_root/state.rs`, `features/pages/mod.rs`, `app/init.rs`,
  `locales/*`, `tests/snapshot_tests.rs`, `commands.rs`,
  `e2e_navigation.rs` (26 touches none of these; its seam rows are the
  floating-module row at INDEX.md:290 and the 02-amendment row at
  INDEX.md:304, which gain `27` only where 27 also touches them: neither,
  since 27 amends no 02 file); new row `src/state/config_store.rs +
  tests/snapshot_tests.rs | 09, 18, 27 | additive #[serde(default)] fields;
  27 adds widgets_layout |`; new row `docs/plans/INDEX.md | 27 | this
  step's registration rows`.
- §6 removal order: insert `26` and `27` before `25` in the leaf list (26 is
  currently absent from that list; it belongs there because 27 consumes its
  contract).
- §7 tracking: `| 26 | layer-shell-widgets | 7 | |` and `| 27 |
  desktop-widgets | 7 | |`.
- §2 peer-coordination paragraph: append the 25↔26↔27 note (25 and 26 both
  amend 02 append-only; 27 consumes both siblings' amendments and adds no
  02 amendment).

Tests: none.
Done-check: `grep -c "| 26 |" docs/plans/INDEX.md` >= 3 and `grep -c
"| 27 |" docs/plans/INDEX.md` >= 3 (catalog, §5/§6 rows, §7 for each);
`grep -c "pending its own INDEX row" docs/plans/INDEX.md` returns 0; the
file still renders as valid Markdown tables.

## Phase 7: Gates

### Step 7.1: Full gates

Files: none (verification only).

Commands, in order (AGENTS.md):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

Plus the CI lanes this feature rides: macos-latest check (`ci.yml:31`),
ubuntu-24.04 check + lib tests (`:116, 206`), windows-latest check + lib
tests (`:174, 255`) green on the PR.

Done-check: all four local gates pass; `git diff --stat` shows only the Seam
Touches files; `grep -rn "use gpui::" src/features/pages/desktop_widgets/`
returns nothing.

## Platform & Wasm Gating

cfg gates:

- The whole feature compiles on every target. The only cfg is `init`'s
  spawn arm (`#[cfg(not(target_family = "wasm"))]`, the 02 spawn-shape
  precedent at 02.md:787-798) and the page's wasm first-open respawn block
  (`#[cfg(target_family = "wasm")]`).
- Wayland-only behavior is runtime-guarded, not cfg'd: layer spec attach and
  the layer row check `cx.compositor_name() == "Wayland"`
  (`GPUI/app.rs:1877-1878`); on X11/macOS/Windows the persisted layer is
  inert (26's spawn logs one info line per process, 02.md:1203-1205).
- wasm posture (09's panel-fallback logic applied to widgets): the page is
  routable; enabled widgets spawn as 02 overlay panes inside the main
  window (anchored by the same `Placement9` math in window space);
  click-through/topmost toggles return false (25); `MetricsWidget` runs 04's
  fixture sampler; `BatteryWidget`/`SystemSlidersWidget` render the no-data
  state (17's `refresh` no-ops on wasm, 17.md:175); respawn-on-launch
  happens on first page open instead of boot (the overlay path needs the
  root window, 02.md:420-427).

Capability-registry entries:

| Key | macOS | Windows | Linux X11 | Linux Wayland | wasm |
| --- | --- | --- | --- | --- | --- |
| `desktop_widgets` (27, set in `init`) | supported_enabled | supported_enabled | supported_enabled | supported_enabled | supported:false, enabled, degraded, reason "widgets render as in-window overlay panes on the web build" |
| `floating_windows`, `floating_always_on_top` (02), `floating_click_through` (25) | rendered by the page as-is from `capabilities::snapshot` | same | same | same | same (unsupported/degraded rows with their reasons) |
| `layer_shell_widgets`, `layer_shell_input_passthrough` (26) | unsupported rows rendered | same | X11 reasons | live (flips degraded on Mutter) | unsupported rows |

Reduced-build fallback (data plans absent; the honest-degradation recipe,
executed only when 27 lands before 04/17 outside batch order):

- 17 absent (`src/services/system_controls` missing; check with `ls
  src/services/system_controls 2>/dev/null`): in `catalog.rs` set
  `system_controls_available() -> false`; delete `battery.rs`,
  `battery.test.rs`, `system_sliders.rs`, `system_sliders.test.rs` and their
  module lines; drop the `Battery`/`System` arms from `widget_view_factory`
  and `defaults`. `available_kinds()` then returns Clock + Metrics; the
  persisted `battery`/`system` entries sanitize away on next read.
- 04 absent (`src/features/menubar_charts` missing): set
  `metrics_available() -> false`; delete `metrics.rs`, `metrics.test.rs`,
  module line, factory arm.
- Both fallbacks are removal-shaped edits local to this feature; nothing
  outside `src/features/pages/desktop_widgets/` changes, and the plan's
  other seam touches are unaffected. The decision this plan states: with the
  data plan landed but the control unavailable (no battery, unsupported
  platform API), the widget renders a visible no-data state; with the data
  plan absent from the build, the kind is hidden from the catalog. Hidden,
  not fake data, in both cases.

Native-only deps: none. No `Cargo.toml` table changes; nothing joins the
`cfg(not(target_family = "wasm"))` table.

## i18n

Crate facade `crate::i18n::localize` / `localize_with_args`
(`src/services/i18n.rs:3-13`); both catalogs in lockstep
(`tests/qa_localization.rs:188-198`). `nav_desktop_widgets` is hand-added
inside the nav block (scanner blind spot, seams §5). Placement keypad codes
("TL".."BR") and capability key names are stable identifiers, not
translations.

| Key | en | zh-CN |
| --- | --- | --- |
| `nav_desktop_widgets` | "Desktop Widgets" | "桌面小部件" |
| `widgets_title` | "Desktop Widgets" | "桌面小部件" |
| `widgets_subtitle` | "Persistent widget windows on the desktop" | "常驻桌面的小部件窗口" |
| `widgets_open_count` | "%{count} widgets on the desktop" | "桌面上的小部件：%{count} 个" |
| `widgets_kind_clock` | "Clock" | "时钟" |
| `widgets_kind_battery` | "Battery" | "电池" |
| `widgets_kind_system` | "Brightness & volume" | "亮度和音量" |
| `widgets_kind_metrics` | "CPU & memory" | "CPU 和内存" |
| `widgets_enabled` | "On desktop" | "在桌面显示" |
| `widgets_placement` | "Screen position" | "屏幕位置" |
| `widgets_placement_top_left` | "Top left" | "左上" |
| `widgets_placement_top` | "Top" | "上" |
| `widgets_placement_top_right` | "Top right" | "右上" |
| `widgets_placement_left` | "Left" | "左" |
| `widgets_placement_center` | "Center" | "居中" |
| `widgets_placement_right` | "Right" | "右" |
| `widgets_placement_bottom_left` | "Bottom left" | "左下" |
| `widgets_placement_bottom` | "Bottom" | "下" |
| `widgets_placement_bottom_right` | "Bottom right" | "右下" |
| `widgets_always_on_top` | "Always on top" | "总是置顶" |
| `widgets_click_through` | "Click-through" | "点击穿透" |
| `widgets_toggle_respawn` | "Reapplies when the widget respawns" | "小部件重新生成后生效" |
| `widgets_layer` | "Layer" | "层级" |
| `widgets_layer_above` | "Above windows" | "窗口上方" |
| `widgets_layer_pinned` | "Desktop-pinned" | "固定到桌面" |
| `widgets_layer_off_wayland` | "Layer choice is available on Wayland only" | "层级选项仅在 Wayland 下可用" |
| `widgets_no_data` | "No data" | "暂无数据" |
| `widgets_battery_charging` | "Charging" | "充电中" |
| `widgets_battery_full` | "Full" | "已充满" |
| `widgets_battery_discharging` | "On battery" | "使用电池" |
| `widgets_brightness` | "Brightness" | "亮度" |
| `widgets_volume` | "Volume" | "音量" |
| `widgets_cpu` | "CPU" | "CPU" |
| `widgets_memory` | "Memory" | "内存" |
| `widgets_capabilities` | "Widget window capabilities" | "小部件窗口能力" |
| `widgets_cap_supported` | "Supported" | "可用" |
| `widgets_cap_degraded` | "Degraded" | "降级" |
| `widgets_cap_error` | "Unavailable" | "不可用" |
| `command_desktop_widgets` | "Open desktop widgets" | "打开桌面小部件" |
| `command_desktop_widgets_subtitle` | "Manage on-desktop widget windows" | "管理桌面小部件窗口" |

## Design Compliance

Full weight: this plan owns a product page and four widget surfaces.

- Primary task and hierarchy ("Start from the task", "Hierarchy"): the page's
  job is "which widgets are on my desktop and how do they behave"; the row
  cards are the object list, the per-row switch is the primary action, and
  the controls block appears only for an open widget so the reading order is
  show → place → behavior. One focal point per card (the switch); no
  competing badges or alerts anywhere on the page.
- Components ("Components and composition"): stock kit parts only (Switch,
  Toggle, RadioGroup, Slider, Button-less keypad via Toggle, AreaChart). No
  custom clickable divs, no bespoke segmented control; the keypad composes
  nine Toggles because no single kit control expresses a 3x3 screen-position
  picker, and each Toggle keeps the component's own geometry, states, and
  a11y contract.
- Tokens before values ("Color and themes", "Radius, spacing, and
  density"): colors from `cx.theme()` by role (chart_2 for the sparkline,
  warning color only as accent on degraded rows); radius via
  `cx.theme().radius`; layout via rem helpers (`p_3`, `gap_2`, `text_sm`,
  `h_16`). The `px()` uses are the audited exceptions: window sizes and the
  16 px placement inset are external-surface geometry (design guides "Zoom,
  base font, and rem" exception list), and the GIB constant is data math.
- Density ("Density tiers"): compact/`small` for the keypad toggles inside a
  control block; medium (default) for rows and switches; widget surfaces use
  the compact end (`p_3`, `text_sm`) matching 02's float-shell density
  ("compact for toolbars and repeated professional data").
- Spatial grammar: card padding `p_4` (lg) with `gap_2` (sm) internals
  matches the notification-card relationships; the controls block indents
  `pl_6` under its row (real hierarchy: controls belong to the row);
  label-above-slider pairs are `gap_1` (parts of one control), slider rows
  `gap_3` (separate groups).
- Surfaces ("Surfaces and elevation"): widget windows ride 02's
  `FloatingShell` chrome (the one float treatment); widget content draws on
  `cx.theme().background` with no extra shadow or border beyond the shell's
  hairline. The page's cards use one card treatment (hairline + theme
  radius) and never nest cards.
- Interaction states ("Make the result understandable before the click"):
  every toggle is a controlled Switch/Toggle/Radio rendering its true state
  from the registry/config; the no-data slider rows render visibly disabled
  with a reason (not silently inert); the capability rows state
  supported/degraded/unavailable in text with color as accent only; hover
  and focus states come from the kit components. The respawn-backed
  always-on-top note (`widgets_toggle_respawn`) makes the Wayland
  deferred-effect visible.
- Pointer conventions: default cursors; no hover-revealed actions; every
  control is a labeled, keyboard-operable kit component (Switch and Toggle
  activate from the keyboard; Slider supports increment/decrement a11y
  actions, `BASE/src/slider.rs:514-530`).
- Keyboard and focus ("Accessibility checklist"): Esc closes a focused
  widget (02's shell-bound action); Tab order follows the reading order;
  icon-adjacent labels name every control; the keypad toggles carry tooltips
  plus text labels, so no icon-only control exists on the page; the page's
  heading announces via `Role::Heading`.
- Interface language ("Interface language", "Buttons and confirmation
  dialogs"): nouns for objects (widget titles, "Layer"), verbs for commands
  ("Open desktop widgets"), sentence case, no terminal periods on short
  labels, no ellipsis on immediate actions. zh-CN values use full-width
  punctuation in sentence-shaped strings and omit it on short labels; the
  clock's fixed numeric formats avoid a per-locale format table that could
  drift (stated trade-off, not an oversight).
- Motion: none added; window appearance animation stays the OS
  utility-window behavior (02's posture).
- Data-heavy ("Designing data-heavy interfaces"): the sparkline is axis-less
  with a fixed 0-100 domain and a current-value header (the 04 sparkline
  pattern), readable at widget size without scrolling; no widget scrolls.

## Verification

No cargo gate was run during planning (doc-only session, the convention INDEX
§8 records for every plan; all commands below are the implementer's gates,
quoted from AGENTS.md and the justfile).

Repo gates (Step 7.1, run exactly as named):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

`just wasm-check` (`justfile:24-25`) proves the feature compiles on wasm32
with the no-data widget states and the overlay respawn path as the only
reachable ones there.

Feature-specific manual QA: this feature cannot be fully verified on one OS;
per-lane checks (the manager page is reached via the sidebar or the
`Open desktop widgets` command on every lane):

macOS (needs a Mac; dev flow `bash scripts/macos-dev-app.sh`):

1. Enable Clock: a 220x120 non-activating panel opens top-right above other
   apps, on all spaces (Mission Control), no Dock activation.
2. Enable Battery: on a MacBook the charge percent, state line, and tiered
   icon update after `system_controls` refreshes; on a desktop Mac (no
   battery) the widget shows the visible no-data state with the reason.
3. Enable Brightness & volume: dragging brightness changes the backlight,
   volume changes the output; releasing commits (keyboard media keys between
   releases confirm the snapshot syncs back within one refresh).
4. Click-through on: clicks over the clock land on the window behind; off:
   input restored. Always-on-top off then on: level visibly changes.
5. Placement keypad: pick bottom-left; the widget respawns at the bottom-left
   inset. Quit and relaunch: enabled widgets respawn at their placements
   (persisted layout).
6. Diagnostics/`widgets_capabilities` rows: `desktop_widgets`,
   `floating_windows`, `floating_click_through` supported; layer rows show
   the Wayland-only reason.

Windows (needs a Windows host):

7. Same enable flow: widget windows are borderless topmost tool windows,
   absent from the taskbar and Alt-Tab.
8. Click-through is the survey-flagged unverified item on the
   DirectComposition path: record the outcome in the PR (25's QA 6 inherits);
   if pass-through fails, the toggle still returns true and the behavior gap
   is documented, never silently patched with `WS_EX_LAYERED` (25 risk 1).
9. Sliders: volume (MMDevice) and brightness move real output on the Windows
   lane's 17 backend; battery percent reads via starship.

Linux X11 (dev host or any X session, real server):

10. Enable flow: PopUp-kind widgets are override-redirect, drawn above, no
    taskbar entry. Click-through via XShape: clicks fall through on both
    spawn kinds. Always-on-top off (Floating kind, WM-managed): the EWMH
    toggle flips `_NET_WM_STATE_ABOVE/SKIP_TASKBAR/STICKY` (inspect with
    `xprop _NET_WM_STATE`), honoring per WM policy (25 QA 8-10).
11. Sliders/battery per the Linux 17 backends (zbus/libpulse/brightness);
    brightness may need permissions, which surfaces as the visible no-data
    reason row.

Linux Wayland (wlroots family or KWin, plus GNOME/Mutter for the open
question):

12. KWin/Hyprland/Sway: layer row visible; "Desktop-pinned" respawns the
    widget as a Background layer surface beneath normal windows with no
    taskbar/Alt-Tab entry; "Above windows" back on top. Click-through passes
    pointer events through the layer surface (empty input region honored).
13. GNOME (Mutter): spawn degrades to a normal float (26's fallback) with
    `layer_shell_widgets` degraded on the capability rows; record whether
    click-through passes through (closes the survey's Mutter item either
    way; 25 QA 12 inherits).
14. Cold-start respawn uses `spawn_floating_after_displays`: on a multi-output
    session, relaunch and confirm widgets land (zed#46378 posture; a
    compositor-chosen output is the documented fallback, one warn log).
15. Layer row absent with the Wayland-only note on X11/macOS/Windows builds
    (rendered reason, not a missing control).

wasm (`just wasm-build` + the web dev flow):

16. The page routes (`#/desktop-widgets`); enabling Clock and CPU & memory
    spawns overlay panes inside the main window; on a clean profile with a
    persisted layout the panes respawn on load (the deferred spawn runs
    after the root window is noted — construction itself is boot on wasm);
    the clock ticks (web timer)
    and the metrics sparkline runs the fixture sampler; battery/sliders show
    the no-data state; toggles for click-through report not applied by
    design; capability rows show the overlay reason; reload respawns on
    first page open (config persistence is a no-op on wasm,
    `config_store.rs:521-528`, so a fresh default layout is the expected
    state there).

Acceptance criteria:

- All four repo gates pass with only the Seam Touches files changed, and the
  macos/windows/ubuntu CI lanes pass on the PR.
- Every public item in the Interface Contract exists with the exact
  signatures (grep/`cargo doc` checkable), including the capability const.
- `widget_layout` round-trips through `AppConfig` (snapshot tests updated,
  `qa_migration` unaffected: no lock change, no `use gpui::`).
- The full-build catalog offers four kinds; every degradation named above is
  a visible state (no-data rows, capability reasons, layer note), never a
  silent absence.
- No new dependency, no `Cargo.toml` edit, no unsafe code
  (`grep -rn "unsafe" src/features/pages/desktop_widgets/` empty).

## Risks & Open Questions

Ranked:

1. **Windows click-through on the DirectComposition path is unverified at
   runtime** (survey-binding). The widget click-through switch calls 25's
   shim unchanged; the risk and its manual closure (QA 8) are 25's, restated
   here because this page is the toggle's most prominent caller. Tried:
   source reading only (no Windows host in any session); QA 8 records the
   outcome in the PR.
2. **Mutter honoring of empty input regions is unverified** (survey-binding).
   Click-through on GNOME Wayland may no-op despite the protocol-level call
   succeeding. Tried: survey + linux.md flag it; no GNOME session was run.
   Closure: QA 13; if Mutter ignores it, the capability row keeps the
   protocol-level status and the PR documents the behavior gap.
3. **EWMH honoring is WM policy on X11.** Widgets spawned AlwaysOnTop are
   override-redirect PopUp windows there, already above and taskbar-free;
   only the Floating-kind (always-on-top off) path depends on EWMH requests.
   Tried: backend atom list read via 25 (`LINUX/x11/window.rs:65-82`).
   Mitigation: best-effort contract inherited from 25; QA 10.
4. **Respawn loses transient content state.** Placement/layer/level changes
   rebuild the widget view (02 has no move API). Sliders re-read the 17
   snapshot, metrics restart sampling, the clock loses sub-second phase.
   Tried: 02's platform window method list has no move/set-level
   (02.md:74-78); 26 confirms layer is creation-only. Accepted; documented
   in the contract guarantees.
5. **Wayland always-on-top toggle is respawn-backed.** The switch persists
   and respawns; a slow compositor makes the swap perceptible. Mitigation:
   the `widgets_toggle_respawn` note makes the deferred effect visible; QA
   12 covers the swap.
6. **zed#46378 startup output selection.** Cold-start layer-shell spawns may
   place on a compositor-chosen output when displays are not yet
   enumerable. Mitigation: `spawn_floating_after_displays` (26's recorded
   workaround); QA 14. Residual: accepted, logged.
7. **Data-plan absence outside batch order.** If 27 executes before 17/04
   land, the reduced-build fallback (Platform & Wasm Gating) deletes the
   dependent widget files instead of failing to compile. Tried: batch table
   orders 17 (batch 5) and 04 (batch 2) before 27 (batch 7); the fallback is
   the guard, not the plan.
8. **Overlay pane coordinate staleness on wasm** (02 risk 5 inherited):
   moving/resizing the main window leaves widget panes at stale offsets
   until the next toggle/respawn. Mitigation: same as 02's
   (`respawn_floating` path); QA 16 does not assert pane tracking.
9. **`SliderValue` event plumbing.** `SliderEvent::Change/Release` carry
   `SliderValue` (Single or Range); the widget only ever builds Single
   sliders, but the match must handle both arms (`BASE/src/slider.rs:34-40`).
   Verified against source this session; the risk is an implementer
   unwrapping instead of matching. The plan's step text names the match.
10. **Icon availability.** Every icon used (`PanelBottom`, `Battery*`,
    `Sun`, `Settings2`, `Cpu`, `MemoryStick`) is verified in the default 106
    by line number this session (`ASSETS/default-icons.txt:8-13, 33, 61, 70,
    87, 96`). The volume stand-in (`Settings2`) is the INDEX §4 recorded
    mapping; if a volume glyph is ever wanted, the escape hatch is a
    section-local SVG, not `AllAssets`.

Open questions (each with what was tried):

- Windows DComp click-through (risk 1): source-verified only; needs QA 8 on
  a real Windows host. No Windows host existed in any planning session.
- Mutter empty-input-region honoring (risk 2): docs-level only; needs QA 13.
- `kCGDesktopWindowLevel` / desktop-below placement off Wayland: the survey
  could not verify the citation, so no pinned-below option is offered on
  macOS/Windows; X11 `_NET_WM_STATE_BELOW` exists in 25's pure builder but
  no public API sends it. Tried: survey Part 1 + macos evidence; no Apple
  doc verified the constant. Desktop-pinned stays Wayland-only, and the
  page says so.
- Sparse-MSIX widget registration and all OS-surface integrations
  (WidgetKit, Windows widget board, plasmoids): out of scope by the survey's
  recommendation; nothing in this plan depends on them. Tried: survey Parts
  1-3 effort numbers; no integration specced.
- Whether a future gpui-pre bump exposes runtime set-level/topmost or a
  move API: not checked beyond the 0.3.9 snapshot; a pin bump re-opens this
  plan's Research Digest (INDEX §4 pin-update rule). The respawn-based
  setters are the version-pinned answer.
