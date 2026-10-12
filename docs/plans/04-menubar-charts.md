# 04 Menu Bar Charts Popover Plan

## Goal

On macOS, clicking the existing menu-bar (status item) icon opens a gpui
popover window anchored under that icon showing live CPU and memory charts
(sparkline, line, bar), refreshing at a bounded 1 Hz and pausable. On Windows
the tray menu gains a Charts item and on Linux a launcher command exists; both
open an in-window sheet hosting the same charts. Wasm gets no entry point and a
capability row naming why.

## Dependencies

Plan 02 (floating window toolkit), Interface Contract items consumed exactly:

- `FloatingWindowId`, `FloatingSpec::new(id, size)` with `.title` / `.level` /
  `.anchor` / `.focus`, `FloatLevel::AlwaysOnTop`,
  `FloatAnchor::Screen { display, placement, offset }`, `Placement9::TopLeft`
  (plan doc `docs/plans/02-floating-window-toolkit.md`, Interface Contract
  section).
- `spawn_floating(spec, build, cx)`, `close_floating(&handle, cx)`,
  `floating_snapshot(cx)` returning entries with pub `id`/`handle`
  (`FloatingHandle::Native`).
- Plan 02's `init(cx)` must have run (its own plan wires it into `app::init`);
  this plan's Phase 4 is the only phase that touches plan 02 surface. Phases
  1-3, 5, 6 build and pass without plan 02. The dead-key cleanup (Step 4.3)
  also rides in Phase 4 because its reader swap (Step 4.2) is there; executing
  it earlier would strand `macos.rs:81` on a missing catalog key.

No other plan is consumed. Plan 01 (canvas) is unrelated.

## Research Digest

Crate paths (verified on disk this session):
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` —
`tray-icon-0.24.1`, `sysinfo-0.31.4`, `gpui-component-0.7.1`,
`gpui-pre-0.3.8` (short names below). Repo files cited relative to
`/home/thragg/fo/gpui-starter`.

Async spawn arities (load-bearing for the 1 Hz loop):

- `App::spawn` takes a one-parameter closure:
  `AsyncFn: AsyncFnOnce(&mut AsyncApp) -> R + 'static`
  (`gpui-pre-0.3.8/src/app.rs:2176-2180`). The two-parameter entity-capturing
  form belongs to `Context<T>::spawn`:
  `AsyncFnOnce(WeakEntity<T>, &mut AsyncApp)` (`gpui-pre-0.3.8/src/app/context.rs:229-238`),
  whose body is literally `let this = self.weak_entity(); self.app.spawn(
  async move |cx| f(this, cx).await)` (`:236-238`). From `&mut App`, the same
  loop is written by capturing a `WeakEntity` manually; repo precedent for
  one-parameter `App::spawn` loops: `src/platform/desktop_shell/tray/macos.rs:144-150`,
  `src/platform/desktop_shell/tray/windows.rs:145-152`.
- `WeakEntity<T>::update<C: AppContext, R>(&self, cx, impl FnOnce(&mut T,
  &mut Context<T>) -> R) -> Result<R>` works with `C = AsyncApp`
  (`gpui-pre-0.3.8/src/app/entity_map.rs:803-817`); `AsyncApp` implements
  `AppContext` and exposes `background_executor()` the same way
  `system_monitor.rs:219` uses it inside its `Context::spawn` loop
  (`system_monitor.rs:217-230`).

Tray events carry icon geometry, the anchor input:

- `TrayIconEvent::Click { id, position: PhysicalPosition<f64>, rect: Rect,
  button, button_state }` (`tray-icon-0.24.1/src/lib.rs:556`, Click variant
  `:558-570`); `Rect { size: PhysicalSize<u32>, position:
  PhysicalPosition<f64> }` (`tray-icon-0.24.1/src/lib.rs:635-638`). Values are
  physical pixels; gpui `Pixels` are logical, so conversion needs a scale
  factor.
- macOS delivery converts the NSStatusItem frame to a top-left-origin rect
  before `to_physical` (`tray-icon-0.24.1/src/platform_impl/macos/mod.rs:515-528`),
  so `rect.position` measures from the top of the main screen. gpui screen
  coordinates are also top-left-origin logical (`MAC/src/display.rs:121-129`
  per plan 02), making the two comparable after the scale division.
- Scale factor source: `Window::scale_factor(&self) -> f32`
  (`gpui-pre-0.3.8/src/window.rs:2973`), reachable off the input thread via
  `root_window(cx)` (`src/app/window.rs:20-22`) + `AnyWindowHandle::update`
  (repo precedent `src/platform/desktop_shell/tray/windows.rs:115-123`).
  `PlatformDisplay` has no scale factor (trait members are id/uuid/bounds/
  visible_bounds/default_bounds, `gpui-pre-0.3.8/src/platform.rs:568-586`);
  hence the window, not the display.
- Anchor rect for placement: primary display `visible_bounds()` excludes the
  menu bar; `cx.primary_display()` (`gpui-pre-0.3.8/src/app.rs:1492`).
- Pixels arithmetic: `Pixels`' field is `pub(crate)`
  (`gpui-pre-0.3.8/src/geometry.rs:2689`), so anchor math subtracts through
  operators instead: `px(x) - bounds.origin.x` (repo precedent
  `src/gallery/sections/theme/brush.rs:104`).

Existing tray code to extend (seam pattern from survey §3):

- macOS: `src/platform/desktop_shell/tray/macos.rs:73` `pub fn setup(cx)`;
  left-click Up currently sends `()` over flume and the `cx.spawn` loop
  (`macos.rs:144-153`) calls `crate::launcher::open_launcher`. OS thread never
  touches `cx` (`macos.rs:92-96` comment). Tooltip built at `macos.rs:79-82`
  from `tray_open_launcher`, the key's only user (`grep tray_open_launcher
  src/ tests/` → one hit, `macos.rs:81`), so this plan replaces and deletes it.
- Windows: menu-as-data `MenuItemSpec` + `menu_items()`
  (`src/platform/desktop_shell/tray/windows.rs:35-56`), id map
  `action_for_menu_id` (`windows.rs:78-85`), `TrayAction` enum
  (`windows.rs:71-76`), `apply_action` (`windows.rs:125-139`), capability
  reporting (`windows.rs:166, 175, 192, 203-213`). Tests pin the id list and
  mapping (`windows.test.rs:27-52, 55-61, 64-80`); left-click launcher mapping
  pinned below those.
- Linux has no tray at all (`src/platform/desktop_shell/mod.rs:1-4`: tray-icon
  0.24 needs a GTK loop gpui does not run). The Linux fallback therefore
  cannot be a tray menu item; it is the launcher command (below).
- Tray setup call site: `src/web.rs:42-44`
  (`#[cfg(any(target_os = "macos", target_os = "windows"))]
  crate::tray::setup(cx);`).

Charts (gpui-kit 0.7.1 = gpui-component 0.7.1 facade):

- `LineChart<T, X, Y>` accessors `.x(fn)`/`.y(fn)`
  (`gpui-component-0.7.1/src/chart/line_chart.rs:176-186`), `.x_axis(bool)`
  (`:219`), `.y_axis(bool)` (`:257`), `.y_domain(min, max)` (`:236`),
  `.grid(bool)` (`:224`), `.interactive(bool)` (`:100`), `.y_tick_count`
  (`:279`), `.y_tick_format` (`:285`), `.stroke` (repo usage
  `src/gallery/sections/charts/chart.rs:1106`). `AreaChart` mirrors these
  (`gpui-component-0.7.1/src/chart/area_chart.rs:88, 100, 128, 181-196, 224,
  229, 241, 262, 284, 290`). A sparkline is an axis-less `AreaChart`; no
  dedicated sparkline type exists in the kit
  (`gpui-component-0.7.1/src/chart/mod.rs:9-15` exports
  Line/Area/Bar/Candlestick/Pie/Radar/Sankey only).
- `BarChart<T, B, V>` uses `.band(fn)`/`.value(fn)`
  (`gpui-component-0.7.1/src/chart/bar_chart.rs:223-229`), `.name` (`:172`),
  `.fill` (`:253`, closure bound `Fn(&T, Bounds<f32>, Bounds<f32>,
  BarAlignment) -> Bg + 'static`, so render-time `cx` cannot be captured; bind
  the color first, precedent `src/gallery/sections/charts/chart.rs:808`),
  `.value_axis(bool)` (`:352`), `.value_tick_count` (`:365`),
  `.value_tick_format` (`:382`), `.band_tick_count` (`:407`), `.id` (`:134`).
  Repo usage: `src/gallery/sections/charts/chart.rs:804-812`.
- Chart color tokens: `pub chart_2: Hsla`
  (`gpui-component-0.7.1/src/theme/theme_color.rs:135`; `chart_4` same struct),
  read as `cx.theme().chart_2` (`system_monitor.rs:289-292` precedent).

Sheet fallback:

- `WindowExt for Window`: `open_sheet(cx, build)` /
  `open_sheet_at(placement, cx, build)` / `has_active_sheet(cx) -> bool` /
  `close_sheet(cx)` (`gpui-component-0.7.1/src/window_ext.rs:15-28`); the
  default `open_sheet` delegates to `open_sheet_at(Placement::Right, ...)`
  (`:94-98`). Build closure type `Fn(Sheet, &mut Window, &mut App) -> Sheet`
  (`gpui-component-0.7.1/src/root.rs:332-334`). `open_sheet_at` replaces any
  active sheet (`root.rs:325-349` takes the old sheet's focus handle then
  overwrites `active_sheet`); hence the no-op guard below.
- `Sheet::new(&mut Window, &mut App)` (`gpui-component-0.7.1/src/sheet.rs:59`)
  + `.title` (`:77`) / `.size` (`:89`) / `.overlay` (`:101`) /
  `.overlay_closable` (`:107`) builders; `impl ParentElement` (`:123`) gives
  `.child`. `Placement` is re-exported by `gpui_kit::component` (defined
  `gpui-base-0.7.1/src/geometry.rs:12`). Repo usage:
  `src/gallery/sections/overlays/sheet.rs:246-266` (`.size(px(400.))`
  precedent at `:252`).
- Pause control: `Switch` is stateless-controlled: `.checked(bool)`
  (`gpui-component-0.7.1/src/switch.rs:56`), `.accessibility_label` (`:73`),
  `.on_change(Fn(&bool, &mut Window, &mut App))` (`:92-97`, doc: owner writes
  the requested value and notifies).

Commands seam (Linux + Windows entry point):

- `CommandId` variants (`src/services/commands.rs:12-29`), `availability` arm
  (`commands.rs:45-99`), registry entry via the
  `command(id, "command_x", "command_x_subtitle", icon)` helper
  (`commands.rs:102-201`, helper `:306-313`), `execute` arm
  (`commands.rs:203-285`), `disabled_reason` helper (`:315-317`). The palette
  consumes `registry()` automatically (survey §1 optional entries). Restart's
  `cfg!(any(unix, windows))` availability gate (`commands.rs:92-98`) is the
  platform-gating precedent.

Capability registry:

- `CapabilityStatus` five pub fields: `supported`, `enabled`, `degraded`,
  `reason`, `last_error` (`src/runtime/capabilities.rs:6-13`; no `Default`
  impl), helpers `supported_enabled/degraded/error` (`:16-51`), `set`
  (`:64-79`), `initialize` at boot (`src/app/init.rs:92`) plus the
  `DEFAULT_ENABLED_CAPABILITIES` list (`init.rs:95-108`) and its loop
  (`:109-115`). Platform-specific entries are `set` directly by their owners
  (`windows.rs:203-213` precedent), not added to the default list.

Metrics crate pick: `sysinfo = { version = "0.31", default-features = false,
features = ["system"] }` in the native-only table. Justification: already in
`Cargo.lock` at 0.31.4 (`Cargo.lock:8538-8541`), so no new packages enter the
lock; API verified on disk: `System::new()`
(`sysinfo-0.31.4/src/common/system.rs:45`), `refresh_memory` (`:143`),
`refresh_cpu_usage` (`:180`), `global_cpu_usage() -> f32` (`:425`),
`total_memory() -> u64` (`:479`), `used_memory() -> u64` (`:527`);
`MINIMUM_CPU_UPDATE_INTERVAL` is 200 ms on macOS/Linux/Windows
(`sysinfo-0.31.4/src/unix/apple/system.rs:71`,
`unix/linux/system.rs:87`, `windows/system.rs:43`), which the 1 Hz loop
clears. Its optional `windows` dep constrains `>=0.54, <=0.57`
(`sysinfo-0.31.4/Cargo.toml:193-195`); the lock already carries windows 0.57.0
(`Cargo.lock:10263-10264`). The `system` feature alone covers CPU+memory
(`sysinfo-0.31.4/Cargo.toml:150-158`); `default` would pull disk/network/user/
component/multithread (`Cargo.toml:125-132`) for nothing. The survey's newer
0.39.6 is NOT on disk (`ls ~/.cargo/registry/src/*/ | grep sysinfo` → 0.31.4
only) and unverifiable here; 0.31.4 is chosen for verifiability and zero lock
growth.

Window/view plumbing:

- `gpui_kit::open_window<V: Render>(options, cx, build) ->
  Result<(AnyWindowHandle, Entity<V>)>` (`gpui-kit-0.7.1/src/lib.rs:149-164`);
  the test platform implements `open_window` too
  (`gpui-pre-0.3.8/src/platform/test/platform.rs:493-506`).
- `impl<V: Render> From<Entity<V>> for AnyView`
  (`gpui-pre-0.3.8/src/view.rs:29`), so `ChartsPanel::view(window, cx).into()`
  satisfies plan 02's `-> AnyView` factory.
- `RootWindow(pub AnyWindowHandle)` global (`src/app/window.rs:11`), set in a
  test via `cx.set_global(crate::app::window::RootWindow(handle))` (tuple field
  is pub).

i18n facade: `crate::i18n::localize("key")` / `localize_with_args`
(`src/services/i18n.rs:3-13`); a missing key renders as the key itself, and
both catalogs must stay in lockstep (`tests/qa_localization.rs:188-198`). Tray
keys live at `locales/en.yml:125-128`.

Wasm constraints: `std::time::Duration` is safe (only `Instant`/`SystemTime`
panic, survey §6); the sampler's timer rides
`cx.background_executor().timer` exactly like `system_monitor.rs:219`, which
compiles on wasm. `sysinfo` goes under
`[target.'cfg(not(target_family = "wasm"))'.dependencies]`
(`Cargo.toml:310-353`); the fixture sampler branch covers wasm and tests.

## Architecture

Entities and state ownership:

- `Sample`: plain value. `seq: u64`, `cpu_percent: f32`,
  `mem_used_bytes: u64`, `mem_total_bytes: u64`. No behavior.
- `Sampler`: owned by `MetricsState`; produces one `Sample` per call.
  Native variant wraps `sysinfo::System` (refresh_cpu_usage +
  refresh_memory); wasm/test variant is a deterministic fixture generator
  (sine-driven cpu/mem). Chosen at construction by
  `cfg(target_family = "wasm")`; tests construct either explicitly.
- `MetricsState`: `Entity` owning the ring buffer (`VecDeque<Sample>`,
  cap 120), the `Sampler`, and `paused: bool`. Its `start` fn spawns the
  1 Hz loop from `&mut App` with a captured `WeakEntity` (the `Context::spawn`
  expansion, `gpui-pre-0.3.8/src/app/context.rs:236-238`); the loop exits when
  the entity drops, so closing the popover or sheet stops all sampling. Pause
  is for keeping the surface open without redraw churn. `set_paused(bool)` is
  the only other mutation.
- `ChartsPanel`: `Render` view; owns `Entity<MetricsState>`; renders
  sparkline (axis-less `AreaChart`, cpu), line (`LineChart`, cpu, 0-100
  domain), bar (`BarChart`, memory GiB, last 30 samples), and a footer with
  live values plus the pause `Switch`. One implementation, two hosts
  (popover window, sheet).
- `popover` module: pure rect→anchor math plus `toggle_popover`, the only
  plan-02 consumer.
- `sheet` module: `open_sheet(cx)`: root-window sheet host for Windows/
  Linux; callable anywhere.
- No `AppEventQueue` events: entry points call the module directly
  (tray action pump, command `execute`); nothing else needs to observe
  chart state.

Events and data flow:

- macOS click: the tray-icon thread (parks on `TrayIconEvent::receiver`)
  matches Left/Up and forwards `TrayAction::ToggleCharts { rect }` over
  flume; the `cx.spawn` pump `cx.update`s `toggle_popover(rect, cx)`, which
  reads `floating_snapshot`: open → `close_floating`; closed → compute
  `FloatAnchor::Screen` from the rect and `spawn_floating` with a
  `ChartsPanel` factory. The hotkey thread still sends
  `TrayAction::OpenLauncher` → `open_launcher`.
- Windows menu pick: `MenuEvent` id `gpui-starter-charts` →
  `TrayAction::OpenCharts` → `apply_action` →
  `crate::features::menubar_charts::open_sheet(cx)` → `root_window(cx)`
  `.update` → `window.open_sheet(...)` hosting the same `ChartsPanel`.
- Linux / any native: launcher command `OpenMenubarCharts` → same
  `open_sheet`.
- Sampling: timer tick → `this.update` → if `!paused`, `sampler.sample()`
  push + `cx.notify()` → both hosts re-render from the same entity.

Module tree:

```text
src/features/menubar_charts/
├── mod.rs            seam: init(cx) capability rows, IconRect, re-exports
├── metrics.rs        Sample, Sampler (sysinfo | fixture), MetricsState + 1 Hz loop
├── metrics.test.rs
├── panel.rs          ChartsPanel view: sparkline / line / bar + pause footer
├── panel.test.rs
├── popover.rs        IconRect → FloatAnchor math, toggle_popover (plan 02)
├── popover.test.rs
└── sheet.rs          open_sheet on the root window
```

## Interface Contract

Public surface at `crate::features::menubar_charts` (all gpui types via
`gpui_kit::*`; no `use gpui::` anywhere, per `tests/qa_migration.rs:386-405`):

```rust
/// Tray-icon rect in physical pixels, top-left origin, as delivered by
/// TrayIconEvent::Click (tray-icon-0.24.1/src/lib.rs:558-570; macOS flip at
/// platform_impl/macos/mod.rs:515-528). Decouples this module from the
/// tray_icon crate so non-tray callers (tests, future hosts) can anchor too.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconRect {
    pub x: f64,
    pub y: f64,
    pub width: u32,
    pub height: u32,
}

/// Register capability rows. Call once from `app::init` after
/// `capabilities::initialize`. Idempotent; no key bindings.
pub fn init(cx: &mut App);

/// Open the charts sheet on the recorded root window.
/// Returns false when no root window exists (logged warn, no panic).
/// No-op returning true when a sheet is already active: `open_sheet_at`
/// replaces the active sheet (root.rs:325-349), so this plan never swaps out
/// a sheet it did not open.
pub fn open_sheet(cx: &mut App) -> bool;

/// macOS path: toggle the anchored popover. Closes it if our id is in
/// `floating_snapshot`; otherwise spawns via plan 02 at
/// `FloatLevel::AlwaysOnTop`, `focus(false)`, anchored under `rect`.
/// Returns false when the anchor or spawn fails (no root window for the
/// scale factor, or plan-02 `spawn_floating` errored).
#[cfg(not(target_family = "wasm"))]
pub fn toggle_popover(rect: IconRect, cx: &mut App) -> bool;

pub const CAPABILITY_MENUBAR_CHARTS: &str = "menubar_charts";
pub const CAPABILITY_MENUBAR_CHARTS_ANCHORED: &str = "menubar_charts_anchored";

// Crate-internal (pub(crate)), not part of the cross-feature seam:
// ChartsPanel::view(window, cx) -> Entity<ChartsPanel>  — the shared view.
// MetricsState::{new, start, set_paused, samples, is_paused}
```

Guarantees: neither entry point panics on any platform; both tolerate a
missing root window. `open_sheet` works identically on macOS if a consumer
prefers the sheet. The popover id is the stable `FloatingWindowId`
`"menubar-charts"`.

## Module Layout

Every module wires its sibling test with the repo convention
`#[cfg(test)] #[path = "<name>.test.rs"] mod <name>_test;`
(`src/shell/route.rs:171-173`).

| File | Purpose |
| --- | --- |
| `src/features/menubar_charts/mod.rs` | Re-export seam, `IconRect`, `init` capability rows, consts |
| `src/features/menubar_charts/metrics.rs` | `Sample`, `Sampler` (sysinfo/fixture), `MetricsState` ring + loop |
| `src/features/menubar_charts/metrics.test.rs` | Fixture formula, ring cap, pause flag tests |
| `src/features/menubar_charts/panel.rs` | `ChartsPanel`: three charts + value footer + pause Switch |
| `src/features/menubar_charts/panel.test.rs` | View construction, controlled pause toggle, sheet open, capability rows |
| `src/features/menubar_charts/popover.rs` | `anchor(rect, scale, visible)` math, `toggle_popover` (native-only module) |
| `src/features/menubar_charts/popover.test.rs` | Pure anchor-math table; toggle round-trip |
| `src/features/menubar_charts/sheet.rs` | `open_sheet` on the root window |

## Seam Touches

| # | File | Change | Why minimal |
| --- | --- | --- | --- |
| 1 | `Cargo.toml:310-353` | `sysinfo = { version = "0.31", default-features = false, features = ["system"] }` in the not-wasm table | Only new dep; version already locked (Cargo.lock:8538) |
| 2 | `src/features/mod.rs` | `pub mod menubar_charts;` | One-line mount beside `pub mod pages;` |
| 3 | `src/app/init.rs` (after the `DEFAULT_ENABLED_CAPABILITIES` loop, `:115`) | `crate::features::menubar_charts::init(cx);` | Single boot hook for capability rows |
| 4 | `src/services/commands.rs` | `CommandId::OpenMenubarCharts` variant, availability arm (enabled `cfg!(not(target_family = "wasm"))` + localized disabled reason, Restart precedent `commands.rs:92-98`), `command(...)` registry entry with `IconName::ChartPie`, `execute` arm calling `open_sheet` | The four mechanical arms every command adds (survey §1) |
| 5 | `src/platform/desktop_shell/tray/macos.rs` | Channel payload `()` → private `enum TrayAction { OpenLauncher, ToggleCharts { rect: IconRect } }`; click handler forwards `ToggleCharts` with the event's rect; pump loop matches instead of always opening the launcher; tooltip key `tray_open_launcher` → `menubar_charts_tooltip` | Payload enum mirrors windows.rs `TrayAction` shape; no new threads or channels |
| 6 | `src/platform/desktop_shell/tray/windows.rs` | `CHARTS_MENU_ID` const + menu item after Show Window, `TrayAction::OpenCharts` variant + id-map arm + `apply_action` arm calling `crate::features::menubar_charts::open_sheet` | Menu-as-data pattern keeps it three small edits (`windows.rs:35-85, 125-139`) |
| 7 | `src/platform/desktop_shell/tray/windows.test.rs` | Extend pinned id vector (`:27-52`) and mapping test (`:64-80`) with the new id | Tests already enumerate ids |
| 8 | `locales/en.yml`, `locales/zh-CN.yml` | 11 new keys (i18n section) in both; delete now-dead `tray_open_launcher` from both (Phase 4, after its reader swap in Step 4.2) | Lockstep rule `tests/qa_localization.rs:188-198`; dead keys are deleted per AGENTS.md |
| 9 | `Cargo.lock` | Regenerate after the manifest edit; stage with it | Removal-direction enforcement `tests/qa_migration.rs:167-186`; a stale lock breaks `cargo build --locked` on the commit |

Untouched on purpose: `src/web.rs` (tray setup call already exists,
`web.rs:42-44`), `src/lib.rs` (module lives under the existing `pub mod
features`), `tests/e2e_navigation.rs` (no new page; count stays 10 at
`tests/e2e_navigation.rs:14-19`), tray `mod.rs`, `macos.test.rs` (pins
`icon_pixels` only, `macos.test.rs:3-29`).

### Removal Recipe

Mirror of `src/gallery/mod.rs:2-7` style:

1. Revert `src/platform/desktop_shell/tray/macos.rs`: channel payload back to
   `()`, left-click forwards `()` again, pump loop back to
   `crate::launcher::open_launcher`, tooltip back to a `tray_open_launcher`
   lookup (re-add that key to both catalogs).
2. Revert `src/platform/desktop_shell/tray/windows.rs` and
   `windows.test.rs`: drop `CHARTS_MENU_ID`, the menu item, the
   `TrayAction::OpenCharts` arms, and the test expectations.
3. Revert `src/services/commands.rs`: remove the variant, availability arm,
   registry entry, execute arm.
4. Remove `crate::features::menubar_charts::init(cx);` from
   `src/app/init.rs`.
5. Revert `src/features/mod.rs` to drop `pub mod menubar_charts;`.
6. Delete the 11 locale keys from `locales/en.yml` and `locales/zh-CN.yml`.
7. Delete `src/features/menubar_charts/` entirely.
8. Remove the `sysinfo` line from `Cargo.toml`'s not-wasm table and
   regenerate `Cargo.lock`.
9. Run `cargo fmt --all -- --check`, `cargo clippy --workspace
   --all-features`, `cargo test --workspace --all-features`; no runtime
   state outlives the module (popover/sheet die with their windows).

## Implementation Phases

## Phase 1: Dependency, locale keys, metrics model

### Step 1.1: Manifest and module skeleton

Files: `Cargo.toml`, `src/features/mod.rs`,
`src/features/menubar_charts/mod.rs` (new).
Build: add the `sysinfo` line to the
`[target.'cfg(not(target_family = "wasm"))'.dependencies]` table exactly as in
Seam Touches #1; `pub mod menubar_charts;` in `src/features/mod.rs`; `mod.rs`
containing the `IconRect` struct from the Interface Contract (with its doc
comment), module declarations added as later steps create the files
(`pub(crate) mod metrics;` now, rest later), and a doc comment naming the
removal seams.
Tests: none.
Done-check: `cargo check -p gpui-starter` passes;
`grep -n 'sysinfo' Cargo.toml` shows exactly one line under the not-wasm
table.

### Step 1.2: Locale keys

Files: `locales/en.yml`, `locales/zh-CN.yml`.
Build: append the 11 keys from the i18n section to both files (quoted
scalars, top-level snake_case; parser rules `tests/qa_localization.rs:20-71`).
Do NOT delete `tray_open_launcher` here: `macos.rs:81` still reads it, and a
missing key renders as the raw key string (`src/services/i18n.rs:4-6`). Its
cleanup is Step 4.3, after Step 4.2 swaps the reader.
Tests: `cargo test -p gpui-starter --test qa_localization`.
Done-check: that command exits 0.

### Step 1.3: `Sample` + `Sampler`

Files: `src/features/menubar_charts/metrics.rs` (new),
`src/features/menubar_charts/mod.rs`.
Build:
- `pub(crate) struct Sample { pub seq: u64, pub cpu_percent: f32,
  pub mem_used_bytes: u64, pub mem_total_bytes: u64 }`
  (`Clone, Copy, Debug, PartialEq`).
- `pub(crate) struct Sampler` with private enum inner:
  - `#[cfg(not(target_family = "wasm"))] System(sysinfo::System)`,
    constructed via `Sampler::system()` that calls `System::new()` then one
    immediate `refresh_cpu_usage()` to prime usage accounting (first read
    otherwise returns 0 until `MINIMUM_CPU_UPDATE_INTERVAL` elapses,
    `sysinfo-0.31.4/src/common/system.rs:163-179`).
  - `Fixture { seq: u64, phase: f64 }`, deterministic:
    `cpu = 25.0 + 20.0 * phase.sin()` clamped to 0-100, `phase += 0.35` per
    sample, `mem_total = 16 GiB` const, `mem_used = total * (0.55 + 0.08 *
    (phase * 0.5).sin())`.
- `pub(crate) fn sample(&mut self) -> Sample`: System arm runs
  `refresh_cpu_usage()` + `refresh_memory()` then reads
  `global_cpu_usage()`, `used_memory()`, `total_memory()`; seq from an
  internal counter.
- `Sampler::fixture()` constructor (no cfg, so tests on native use it).
Tests (`metrics.test.rs`, plain `#[test]`): fixture sample 0 and 1 match the
formula (cpu in 0-100, mem_used < mem_total); two successive samples have
increasing `seq`.
Done-check: `cargo test -p gpui-starter --lib menubar_charts::metrics` green.

### Step 1.4: `MetricsState` ring + loop

Files: `src/features/menubar_charts/metrics.rs`.
Build:
- `const INTERVAL: Duration = Duration::from_millis(1000);` (clears the 200 ms
  minimum on every native platform; bounded refresh per spec).
- `const MAX_SAMPLES: usize = 120;`
- `pub(crate) struct MetricsState { samples: VecDeque<Sample>, sampler:
  Sampler, paused: bool }` (plain entity data). `MetricsState::new()` picks
  `Sampler::system()` on native, `Sampler::fixture()` on wasm (`cfg` at the
  call, not on the struct).
- `pub(crate) fn start(this: &Entity<MetricsState>, cx: &mut App)`. The loop
  is spawned from `App`, whose `spawn` takes a one-parameter closure
  (`AsyncFnOnce(&mut AsyncApp)`, `gpui-pre-0.3.8/src/app.rs:2176-2180`), so
  the weak handle is captured manually — exactly the expansion
  `Context::spawn` performs internally (`gpui-pre-0.3.8/src/app/context.rs:236-238`):
  ```rust
  pub(crate) fn start(this: &Entity<MetricsState>, cx: &mut App) {
      let this = this.downgrade();
      cx.spawn(async move |cx| loop {
          cx.background_executor().timer(INTERVAL).await;
          if this.update(cx, |this, cx| this.tick(cx)).is_err() {
              break; // entity released: popover/sheet closed
          }
      })
      .detach();
  }
  ```
  `WeakEntity::update` accepts `&mut AsyncApp` as its `AppContext`
  (`gpui-pre-0.3.8/src/app/entity_map.rs:803-817`). Same loop shape as
  `system_monitor.rs:217-230`, whose two-parameter closure is legal only
  because it runs inside `Context<Self>` (`system_monitor.rs:197, 217`).
- `fn tick(&mut self, cx: &mut Context<Self>)`: if `!self.paused`, sample,
  push back, pop front over cap, `cx.notify()`. Paused ticks are no-ops that
  keep the timer alive (resume is instant).
- `pub(crate) fn set_paused(&mut self, paused: bool, cx: &mut
  Context<Self>)`: writes the flag, `cx.notify()`.
- `pub(crate) fn samples(&self) -> &VecDeque<Sample>` reader plus
  `is_paused()`.
Tests (`#[gpui_kit::test]`, `TestAppContext` —
`src/state/config_store.test.rs:280-281` precedent): construct entity via
`cx.new(|_| MetricsState::new())`; call `tick` four times directly through
`entity.update` (no timer dependency); assert len 4 and `seq` strictly
increasing; `set_paused(true)` then tick → len still 4; assert ring cap by
ticking `MAX_SAMPLES + 5` times total and checking len == `MAX_SAMPLES`.
Done-check: `cargo test -p gpui-starter --lib menubar_charts::metrics` green.

## Phase 2: `ChartsPanel` view

### Step 2.1: Panel render

Files: `src/features/menubar_charts/panel.rs` (new),
`src/features/menubar_charts/mod.rs`.
Build:
- `pub(crate) struct ChartsPanel { metrics: Entity<MetricsState> }`;
  `pub(crate) fn view(window: &mut Window, cx: &mut App) ->
  Entity<ChartsPanel>` that creates the `MetricsState` entity, calls
  `MetricsState::start(&entity, cx)`, and wraps it (view-constructor idiom
  `system_monitor.rs:193-195`).
- `impl Render`: `v_flex().size_full().p_3().gap_3()` with:
  1. Sparkline row: `h_flex().justify_between()` with a `text_sm` muted
     `localize("menubar_charts_cpu")` label and current
     `format!("{:.0}%", last.cpu_percent)` in `text_sm` foreground, above
     `AreaChart::new(samples.iter().cloned().collect::<Vec<_>>()).x(|s| s.seq.to_string()).y(|s|
     s.cpu_percent as f64).stroke(cx.theme().chart_2).fill(
     linear_gradient(0., linear_color_stop(cx.theme().chart_2.opacity(0.4),
     1.), linear_color_stop(cx.theme().background.opacity(0.1), 0.)))
     .x_axis(false).y_axis(false).grid(false).interactive(false)
     .y_domain(0., 100.).id("menubar-charts-spark")` in an `h_16()` box
     (AreaChart builders at area_chart.rs:88-290; gradient precedent
     `system_monitor.rs:289-293`). `.stroke`/`.fill` here take values, not
     closures, so reading `cx.theme()` inline is fine.
  2. Line chart: same accessor pair on `LineChart` over an owned
     `samples.iter().cloned().collect::<Vec<_>>()` (constructors are
     `new<I: IntoIterator<Item = T>>` with `where T: 'static`,
     line_chart.rs:26-30, 57-60 — borrowed `&Sample` items do not satisfy
     `'static`; gallery precedent passes an owned Vec, `chart.rs:805`),
     `.y_domain(0., 100.)
     .y_axis(true).y_tick_count(3).y_tick_format(|v| format!("{v:.0}%"))
     .x_axis(false).grid(true).stroke(cx.theme().chart_4)
     .id("menubar-charts-line")`, `min_h(rems(10.))` flex_1 wrapper
     (`system_monitor.rs:259-296` layout precedent).
  3. Bar chart: bind the bar color before any closure (the `.fill` closure
     bound is `+ 'static`, `bar_chart.rs:253`, so render borrows of `cx`
     cannot be captured; precedent `chart.rs:808`):
     ```rust
     let bar_color = cx.theme().chart_2;
     ```
     then the last `samples.iter().rev().take(30).rev().cloned()` slice;
     `BarChart::new(slice).band(|s| s.seq.to_string()).value(|s|
     s.mem_used_bytes as f64 / GIB).name(localize("menubar_charts_memory"))
     .fill(move |_, _, _, _| bar_color).value_axis(true)
     .value_tick_count(3).value_tick_format(|v| format!("{v:.0} GiB"))
     .id("menubar-charts-bar")` (usage precedent `chart.rs:804-812`).
  4. Footer `h_flex().gap_2().items_center().justify_between().text_sm()`:
     memory text `format!("{:.1} / {:.0} GiB", last.mem_used_bytes as f64 /
     GIB, last.mem_total_bytes as f64 / GIB)` muted, preceded by muted
     `localize("menubar_charts_memory_used")`; `Switch::new("menubar-charts-pause")
     .checked(state.is_paused()).accessibility_label(localize(
     "menubar_charts_pause")).on_change(cx.listener(|this, checked, _, cx|
     { this.metrics.update(cx, |m, cx| m.set_paused(*checked, cx)); }))`
     (controlled-value contract, switch.rs:88-97); when paused, a muted
     `localize("menubar_charts_paused")` label beside it (state visible,
     not color alone).
- No empty-state branch: first tick lands within 1 s; render charts with
  whatever exists (fixture tests cover n=1; kit charts take owned-item
  iterators — `T: 'static` — never borrows, so every branch clones).
- `const GIB: f64 = 1024.0 * 1024.0 * 1024.0;`
- Reads state via `self.metrics.read(cx)` inside render; no mutation in
  render (coding guide "state in render" failure mode).
Tests (`panel.test.rs`, `#[gpui_kit::test]`): open a test window via
`cx.update(|cx| gpui_kit::open_window(WindowOptions::default(), cx,
|window, cx| ChartsPanel::view(window, cx)))` (test platform opens windows,
`gpui-pre-0.3.8/src/platform/test/platform.rs:493-506`); assert the result is
`Ok`. Then one direct `tick` through the panel's metrics entity and assert
`samples().len() >= 1` (do not wait on the real timer).
Done-check: `cargo test -p gpui-starter --lib menubar_charts::panel` green;
`cargo check -p gpui-starter` green.

## Phase 3: Sheet fallback + command entry point

### Step 3.1: `open_sheet`

Files: `src/features/menubar_charts/sheet.rs` (new),
`src/features/menubar_charts/mod.rs`.
Build:
- `pub fn open_sheet(cx: &mut App) -> bool`:
  1. `let Some(handle) = crate::app::window::root_window(cx) else {
     tracing::warn!(target: "gpui_starter::menubar_charts", "no root window
     recorded; charts sheet ignored"); return false; }`
  2. `handle.update(cx, |_, window, cx| { if
     window.has_active_sheet(cx) { return true; }
     window.open_sheet(cx, |sheet, window, cx| sheet
     .size(px(420.)).overlay(true).overlay_closable(true)
     .title(localize("menubar_charts_title"))
     .child(ChartsPanel::view(window, cx))); true })`. `WindowExt`
     (`window_ext.rs:15-25`, default Right placement `:94-98`), Sheet
     builders (`sheet.rs:59-107`), size precedent (`overlays/sheet.rs:252`).
     Map the update error to `false`. Import `WindowExt as _` and
     `Placement` from `gpui_kit::component` (`overlays/sheet.rs:5` style;
     `Placement` re-exported from `gpui-base-0.7.1/src/geometry.rs:12`).
- Tracing target `gpui_starter::menubar_charts` (target-string rule, survey
  §9).
Tests (appended to `panel.test.rs`): inside `#[gpui_kit::test]`, open a
window, set the global
(`cx.update(|cx| cx.set_global(crate::app::window::RootWindow(handle)))` —
tuple field pub, `src/app/window.rs:11`), call `open_sheet` → true;
`handle.update(... window.has_active_sheet(cx))` → true; second `open_sheet`
→ true and still exactly one active sheet (no-op path).
Done-check: `cargo test -p gpui-starter --lib menubar_charts` green.

### Step 3.2: Command registration

Files: `src/services/commands.rs`.
Build (Seam Touches #4): variant `OpenMenubarCharts`; availability
```rust
CommandId::OpenMenubarCharts => CommandAvailability {
    enabled: cfg!(not(target_family = "wasm")),
    disabled_reason: disabled_reason(
        "command_reason_menubar_charts_unavailable",
        cfg!(target_family = "wasm"),
    ),
},
```
(Restart gate precedent `commands.rs:92-98`; helper `:315-317`); registry
entry `command(CommandId::OpenMenubarCharts, "command_menubar_charts",
"command_menubar_charts_subtitle", IconName::ChartPie)` (variant
verified in the component enum the app registers: default-icons.txt:20 is
the only chart icon in the default 106; `Activity` exists only in the full
`gpui_kit::assets::IconName` catalog the app does not register); execute arm
`CommandId::OpenMenubarCharts => {
crate::features::menubar_charts::open_sheet(cx); }`.
Tests: none new (command coverage is the palette's existing smoke + manual
QA 4).
Done-check: `grep -c "OpenMenubarCharts" src/services/commands.rs` prints 4;
`cargo check -p gpui-starter` green.

## Phase 4: macOS anchored popover, tray routing, dead-key cleanup (requires plan 02 landed)

### Step 4.1: Anchor math + `toggle_popover`

Files: `src/features/menubar_charts/popover.rs` (new),
`src/features/menubar_charts/mod.rs`.
Build:
- `pub(crate) fn anchor(rect: IconRect, scale: f32, visible:
  Bounds<Pixels>) -> FloatAnchor`:
  ```rust
  let logical_x = px((rect.x / scale as f64) as f32);
  let offset = point(logical_x - visible.origin.x, px(0.));
  FloatAnchor::Screen {
      display: None, // None = primary display (plan 02 contract)
      placement: Placement9::TopLeft,
      offset,
  }
  ```
  The popover's top edge lands on the visible-frame top edge (directly
  under the menu bar; visible bounds already exclude it,
  `MAC/src/display.rs:121-129` via plan 02), left edge under the icon;
  plan 02's `resolve_bounds` clamps into the visible rect when the icon sits
  near a screen edge. Subtraction stays in operator space: `Pixels`' field
  is `pub(crate)` (`gpui-pre-0.3.8/src/geometry.rs:2689`; operator precedent
  `src/gallery/sections/theme/brush.rs:104`).
- `pub fn toggle_popover(rect: IconRect, cx: &mut App) -> bool`:
  1. If `floating_snapshot(cx).iter().any(|e| e.id.0 == "menubar-charts")`:
     `close_floating(&entry.handle, cx)` for that entry, return true.
     (`SharedString` implements `PartialEq<&str>`; plan 02 Research Digest.)
  2. Scale: `root_window(cx)` → `handle.update(cx, |_, window, _|
     window.scale_factor())` (`window.rs:2973`); no root window or update
     error → warn + return false.
  3. `visible = cx.primary_display().map(|d| d.visible_bounds())`
     (`gpui-pre-0.3.8/src/app.rs:1492`; None → return false).
  4. `spawn_floating(FloatingSpec::new(FloatingWindowId("menubar-charts"
     .into()), size(px(360.), px(540.))).title(
     crate::i18n::localize("menubar_charts_title"))
     .level(FloatLevel::AlwaysOnTop).anchor(anchor(rect, scale, visible))
     .focus(false), |window, cx| ChartsPanel::view(window, cx).into(), cx)`
     with `.into()` to `AnyView` via `impl<V: Render> From<Entity<V>> for
     AnyView` (`gpui-pre-0.3.8/src/view.rs:29`). Map `Err` → warn + false.
- `mod.rs`: `#[cfg(not(target_family = "wasm"))] pub(crate) mod popover;`
  plus `#[cfg(not(target_family = "wasm"))] pub use popover::toggle_popover;`
  (module-gate style `src/platform/mod.rs:2-3`). Plan 02 keeps a wasm overlay
  path, but this module is native-only surface and stays out of the wasm
  check entirely.
Tests (`popover.test.rs`):
- Plain `#[test]` anchor table: scale 2.0, rect at physical (400, 0),
  visible origin (0, 25) size 1440x875 → offset.x == px(200.), offset.y ==
  px(0.), placement TopLeft, display None; scale 1.0 case; left-of-visible
  case (negative result: must not panic, clamping is plan 02's job).
- `#[gpui_kit::test]` toggle round-trip (needs plan 02): set `RootWindow`
  global, `toggle_popover(rect, cx)` → true and `floating_snapshot` contains
  id "menubar-charts"; `toggle_popover` again → true and snapshot empty.
Done-check: `cargo test -p gpui-starter --lib menubar_charts::popover`
green.

### Step 4.2: macOS tray click routing

Files: `src/platform/desktop_shell/tray/macos.rs`.
Build (Seam Touches #5):
- Replace `flume::unbounded::<()>` with
  `flume::unbounded::<TrayAction>()`, the enum declared locally in
  `macos.rs` (private, mirroring `windows.rs:71-76`):
  `enum TrayAction { OpenLauncher, ToggleCharts { rect:
  crate::features::menubar_charts::IconRect } }`.
- Click thread: on Left/Up build
  `TrayAction::ToggleCharts { rect: IconRect { x: rect.position.x, y:
  rect.position.y, width: rect.size.width, height: rect.size.height } }`
  from the matched event (`tray-icon-0.24.1/src/lib.rs:558-570`) and send;
  log field `source = "tray_click"` message `"Charts toggle trigger"`.
- Hotkey thread: sends `TrayAction::OpenLauncher`.
- Pump loop (`macos.rs:144-153`): match on OpenLauncher →
  `crate::launcher::open_launcher`; ToggleCharts →
  `cx.update(|cx| crate::features::menubar_charts::toggle_popover(rect,
  cx))`.
- Tooltip: `crate::i18n::localize("menubar_charts_tooltip")` replacing the
  `tray_open_launcher` lookup (`macos.rs:79-82`); keep the `(⌥Space)`
  suffix hint so the launcher path stays discoverable:
  `format!("{}  (⌥Space)", ...)`.
Tests: none runnable off macOS; `macos.test.rs` pins only `icon_pixels`
(verified, `macos.test.rs:3-29`) and stays untouched.
Done-check: `cargo check -p gpui-starter` green on the dev host (macOS
compile of `macos.rs` is exercised by check on macOS; on Linux CI this file
is cfg'd out, same as today, `tray/mod.rs:1-4`).

### Step 4.3: Dead-key cleanup

Files: `locales/en.yml`, `locales/zh-CN.yml`.
Build: delete `tray_open_launcher` from both catalogs. Order matters: this
step follows Step 4.2, which removed the key's only reader (`macos.rs:81`).
Deleting earlier would make the macOS tooltip render the raw key string
(missing-key behavior, `src/services/i18n.rs:4-6`) on any tree where Phase 4
had not landed.
Tests: `cargo test -p gpui-starter --test qa_localization`.
Done-check: `grep -rn tray_open_launcher src/ tests/ locales/` returns
nothing and the qa_localization command exits 0.

## Phase 5: Windows tray menu item

### Step 5.1: Menu item + action

Files: `src/platform/desktop_shell/tray/windows.rs`,
`src/platform/desktop_shell/tray/windows.test.rs`.
Build (Seam Touches #6-7):
- `const CHARTS_MENU_ID: &str = "gpui-starter-charts";`
- `menu_items()`: insert `MenuItemSpec::Action { id: CHARTS_MENU_ID, label:
  crate::i18n::localize("menubar_charts_tray_menu") }` between Show Window
  and Open Settings.
- `TrayAction::OpenCharts` variant; `action_for_menu_id` arm.
- `apply_action`: `TrayAction::OpenCharts => {
  crate::features::menubar_charts::open_sheet(cx); }`
- Tests: extend `menu_lists_show_settings_and_quit_with_a_separator_before_quit`
  (`windows.test.rs:27`) expected vector to `[SHOW_MENU_ID, CHARTS_MENU_ID,
  SETTINGS_MENU_ID, QUIT_MENU_ID]`; `every_menu_action_id_maps_to_an_action`
  (`:55`) needs no edit (it iterates); `menu_id_mapping_is_exact` (`:64`)
  gains a `CHARTS_MENU_ID => Some(TrayAction::OpenCharts)` assertion. The
  non-empty-label loop covers the new key (catalogs carry it since Step 1.2).
Tests: the updated `windows.test.rs` (runs on the Windows host; the module
is cfg'd per-OS in `tray/mod.rs:3-4`, so Linux never compiles it; only the
Windows test lane runs it).
Done-check: `grep -c "CHARTS_MENU_ID" src/platform/desktop_shell/tray/
windows.rs` prints 3 (const + menu spec + map arm); `cargo check -p
gpui-starter` green.

## Phase 6: Capability rows + boot hook

### Step 6.1: `init` and registration

Files: `src/features/menubar_charts/mod.rs`, `src/app/init.rs`.
Build:
- `pub fn init(cx: &mut App)`:
  - `menubar_charts`: native → `CapabilityStatus::supported_enabled()`;
    wasm → struct literal `{ supported: false, enabled: false, degraded:
    false, reason: Some("no menu bar or tray on wasm".into()),
    last_error: None }` (all five fields, `capabilities.rs:6-13`; no
    `Default` exists, hence the full literal; literal precedent plan 02
    Step 5.2).
  - `menubar_charts_anchored`: macOS → `supported_enabled()`; Windows/
    Linux/wasm → `{ supported: false, enabled: false, degraded: false,
    reason: Some("menu-bar popover is macOS only; sheet fallback".into()),
    last_error: None }`.
- `src/app/init.rs`: one line after the `DEFAULT_ENABLED_CAPABILITIES`
  loop (which ends at `:115`): `crate::features::menubar_charts::init(cx);`.
Tests (appended to `panel.test.rs`): `#[gpui_kit::test]` calling `init`
inside `cx.update`, then `crate::capabilities::snapshot(cx)` contains both
keys and `supported` matches `cfg!(not(target_family = "wasm"))` /
`cfg!(target_os = "macos")`.
Done-check: `cargo test -p gpui-starter --lib menubar_charts` green;
`grep -n "menubar_charts::init" src/app/init.rs` shows the call.

## Phase 7: Lock, gates

### Step 7.1: Lock regeneration and full gates

Files: `Cargo.lock` (regenerated).
Build/commands, in order:
1. `cargo check -p gpui-starter`, letting the Step 1.1 manifest edit settle
   `Cargo.lock` (expected: no new packages; sysinfo 0.31.4 and windows 0.57.0
   are already locked, `Cargo.lock:8538, 10263`); stage the lock with the
   manifest (`tests/qa_migration.rs:167-186` enforces lock/manifest sync for
   the removal direction; a stale lock fails `cargo build --locked` either
   way).
2. `cargo fmt --all -- --check`
3. `cargo clippy --workspace --all-features`
4. `cargo test --workspace --all-features`
5. `just wasm-check`
Done-check: all five pass; `git diff --stat` shows only the Seam Touches
files.

## Platform & Wasm Gating

cfg gates:

- `sysinfo` sits in `[target.'cfg(not(target_family = "wasm"))'.
  dependencies]` (`Cargo.toml:310-353`); `metrics.rs` fences only the
  `Sampler::System` variant and `Sampler::system()` with
  `#[cfg(not(target_family = "wasm"))]`, fixture arm uncfg'd. The module,
  `MetricsState`, and `ChartsPanel` compile on wasm (exercised by
  `just wasm-check`).
- `popover` is a native-only module:
  `#[cfg(not(target_family = "wasm"))]` in `mod.rs`
  (module-gate style `src/platform/mod.rs:2-3`), so `toggle_popover` needs
  no separate fn gate and the wasm build never sees the plan-02 imports.
  `open_sheet`, `init`, `IconRect` are uncfg'd.
- Tray files are OS-gated exactly as today (`tray/mod.rs:1-9`); Linux never
  links tray-icon (`desktop_shell/mod.rs:1-4`).

Capability-registry entries (written by `menubar_charts::init`, rendered on
the diagnostics page):

| Key | macOS | Windows | Linux | wasm |
| --- | --- | --- | --- | --- |
| `menubar_charts` | supported_enabled | supported_enabled | supported_enabled | unsupported, reason "no menu bar or tray on wasm" |
| `menubar_charts_anchored` | supported_enabled | unsupported, "menu-bar popover is macOS only; sheet fallback" | same as Windows | same as Windows |

Platform behavior matrix (what each platform gets, including hidden):

| Behavior | macOS | Windows | Linux | wasm |
| --- | --- | --- | --- | --- |
| Menu-bar icon | existing tray-icon NSStatusItem (unchanged glyph) | tray icon in notification area | none (no GTK loop, `desktop_shell/mod.rs:3-4`) | none |
| Icon left click | toggles anchored popover (plan 02 `AlwaysOnTop`/PopUp, `focus(false)`) | opens launcher (unchanged, `windows.rs:89-100`) | n/a | n/a |
| Charts entry point | icon click | tray menu "Charts" | launcher command `OpenMenubarCharts` | hidden: command disabled with localized reason |
| Charts surface | popover window 360x540 under icon | right sheet 420 wide, same `ChartsPanel` | same sheet | none |
| Esc dismissal | plan 02 `DismissFloating` in the shell | sheet's overlay_closable / focus handling | same | n/a |
| Data source | sysinfo (system feature) | sysinfo | sysinfo | fixture (compiles; unreachable) |

Native-only deps added:

```toml
[target.'cfg(not(target_family = "wasm"))'.dependencies]
sysinfo = { version = "0.31", default-features = false, features = ["system"] }
```

## i18n

Crate facade `crate::i18n::localize` (`src/services/i18n.rs:3-5`); every key
lands in BOTH catalogs (`tests/qa_localization.rs:188-198`). Number and unit
formatting (`{:.0}%`, `GiB`) stays locale-neutral `format!`; no assembled
sentences.

| Key | en | zh-CN |
| --- | --- | --- |
| `menubar_charts_title` | "System activity" | "系统活动" |
| `menubar_charts_tooltip` | "System charts" | "系统图表" |
| `menubar_charts_tray_menu` | "Charts" | "图表" |
| `menubar_charts_cpu` | "CPU" | "处理器" |
| `menubar_charts_memory` | "Memory" | "内存" |
| `menubar_charts_memory_used` | "Memory in use" | "已用内存" |
| `menubar_charts_pause` | "Pause refresh" | "暂停刷新" |
| `menubar_charts_paused` | "Paused" | "已暂停" |
| `command_menubar_charts` | "Open system charts" | "打开系统图表" |
| `command_menubar_charts_subtitle` | "Live CPU and memory charts" | "实时处理器与内存图表" |
| `command_reason_menubar_charts_unavailable` | "System charts are unavailable on this platform" | "此平台不支持系统图表" |

`tray_open_launcher` is deleted in Step 4.3 (dead after the tooltip swap in
Step 4.2; `windows.rs` uses literal `"GPUI Starter"` at `:185` and its menu
keys are `tray_show_window`/`tray_open_settings`/`tray_quit`).

## Design Compliance

- Surfaces/elevation: the popover window's chrome is plan 02 `FloatingShell`'s
  single popover-family treatment; the sheet is the kit's themed Sheet. This
  plan adds no second anchored surface and no custom shadows ("Surfaces and
  elevation": one treatment per kind).
- Density: popover/sheet content is compact data view: `text_sm` labels,
  `gap_3` groups, charts as the flexible region with `flex_1()` +
  `min_h` ("Density tiers": compact for repeated professional data).
- Tokens: all color from `cx.theme()` (`chart_2`/`chart_4`/`foreground`/
  `muted_foreground`/`background`, `system_monitor.rs:289-292` precedent);
  no raw colors, no literal radii. The two window geometries (360x540
  popover, `px(420.)` sheet width) are surface sizes, matching the
  `px(400.)` sheet precedent (`overlays/sheet.rs:252`); everything inside
  uses rem helpers ("Zoom, base font, and rem").
- Spacing: `gap_2`/`gap_3`/`p_3` scale helpers only; related label/value
  pairs tighter than chart groups ("Spatial grammar": parts of one control
  vs one content group).
- Interaction states: pause is a kit `Switch` (controlled value + semantic
  on/off states), with a visible "Paused" text so the state is never carried
  by the switch position or color alone ("State must be visible"; switch
  contract `switch.rs:88-97`). No custom clickable divs.
- Charts: standard kit chart components with semantic tokens, y-domain fixed
  0-100 for CPU so spikes do not rescale ("Compose from the standard semantic
  component").
- A11y: values are duplicated as visible text next to every chart (charts
  are the visual, text is the accessible reading); the Switch carries
  `accessibility_label`; the popover opens with `focus(false)` so the tray
  click never steals key focus; capability rows are visible on the
  diagnostics page with reasons ("Accessibility checklist": keyboard reach,
  names on controls, state not by color alone).
- Interface language: labels are nouns ("CPU", "Memory in use") and the
  command is a verb phrase ("Open system charts"), sentence case, no
  ellipsis (no dialog opened), no terminal punctuation on short labels;
  zh-CN uses full-width punctuation only in sentence-style reason strings,
  none on control labels ("Capitalization, punctuation, and symbols").
- Motion: none added; kit chart `appear` left at defaults, honoring "Motion
  explains change" (no new ambient animation).
- Platform fit: macOS keeps the native status-item interaction (menu-bar
  icon, popover under it), Windows keeps its tray-menu idiom, Linux routes
  through the command palette. Same information model, platform-native
  entry points ("Internationalization and platform fit").

## Verification

Repo gates (AGENTS.md; run in Step 7.1 exactly as named):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

`just wasm-check` proves the fixture sampler, `MetricsState`, `ChartsPanel`,
`open_sheet`, and `init` compile for wasm32 (`justfile:24-25`, lib-only
nightly check).

Feature-specific manual QA (macOS dev flow `bash scripts/macos-dev-app.sh`,
CONTRIBUTING.md:32-38):

1. macOS: app launches, menu-bar glyph present as today. Click it: popover
   opens with its top edge directly under the menu bar and its left edge
   under the icon; main window does not lose key focus.
2. Charts populate within ~1 s and advance once per second; sparkline, line
   (0-100 % axis), and bar (GiB axis) all redraw.
3. Toggle the pause Switch: values freeze, "Paused" appears; toggle back:
   next tick resumes. Close the popover (Esc, shell close button, or clicking
   the icon again): sampling stops (no leftover window in
   `floating_snapshot`, asserted via a log line added temporarily and
   removed before commit).
4. Launcher still opens via ⌥Space; its tray-tooltip hint is correct.
5. Click the icon with the popover open: it closes (toggle, not stack).
6. Diagnostics page: `menubar_charts` supported/enabled;
   `menubar_charts_anchored` supported on macOS.
7. Windows: tray right-click menu lists Charts between Show Window and Open
   Settings; picking it opens the right sheet with live charts; left click
   still opens the launcher. Diagnostics: `menubar_charts_anchored`
   unsupported with the macOS-only reason.
8. Linux: launcher command "Open system charts" opens the sheet; the command
   is enabled on native. Diagnostics rows as on Windows.
9. Wasm build (`just wasm-check` + existing web dev flow): command palette
   shows the command disabled with the localized reason; diagnostics shows
   `menubar_charts` unsupported.

Acceptance criteria:

- All four gates pass with only the Seam Touches files changed.
- Every public item in the Interface Contract exists with those signatures
  (`grep`/`cargo doc` checkable).
- No `use gpui::` / `gpui_component::` imports added
  (`tests/qa_migration.rs` and `tests/qa_localization.rs` suites pass).
- Locale catalogs in lockstep with `tray_open_launcher` gone; page count
  untouched (`tests/e2e_navigation.rs` passes unchanged).

## Risks & Open Questions

Ranked:

1. **Popover does not dismiss on outside click.** macOS status-item popovers
   conventionally close when the user clicks elsewhere; plan 02 offers only
   Esc / close button / `close_floating`, and its PopUp window is
   non-activating, so no focus-loss signal reaches us. Tried: read plan 02's
   Interface Contract (DismissFloating + close_floating only) and the survey
   (§b: no public set-level/position or focus-loss hooks in gpui-pre
   0.3.8). Mitigation: tray-click toggle + Esc + shell close; if a consumer
   needs outside-click, that is a plan 02 extension (an app-level click
   observer), not a patch here.
2. **Anchoring uses the root window's scale factor, not the menu bar
   display's.** `PlatformDisplay` exposes no scale factor (verified, trait
   members at `gpui-pre-0.3.8/src/platform.rs:568-586`); on mixed-DPI
   multi-display setups where the root window is on a different display
   than the menu bar icon, the offset can be wrong. Tried: searched for a
   display-level scale API (grep over platform.rs, none). Mitigation:
   anchor math is one pure fn (`anchor`); if a display scale API appears,
   it is a one-line change. Manual QA 1 on a single-display Mac is the
   acceptance bar.
3. **Plan 02 is a hard dependency of Phase 4 only, and is not implemented
   yet.** Phases 1-3, 5, 6 land without it; if plan 02's contract shifts
   (for example the `spawn_floating` signature), Steps 4.1-4.3 must be
   re-checked against its landed Interface Contract before execution. Tried:
   `ls src/ui/` shows no `floating/` directory; this plan pins the exact
   contract items it consumes (Dependencies section). Step 4.3 (dead-key
   cleanup) is sequenced inside Phase 4 because its reader swap (4.2) is
   there.
4. **sysinfo pinned to 0.31, not the survey's 0.39.6.** 0.39 is not on disk
   and its API could not be verified this session (the survey checked only
   crates.io metadata); 0.31.4 is verified line-by-line and already in the
   lock. Cost: older API surface we do not extend. Tried: `ls
   ~/.cargo/registry/src/*/ | grep sysinfo` → only 0.31.4.
5. **First CPU sample can read 0 %.** sysinfo needs two refreshes separated
   by at least `MINIMUM_CPU_UPDATE_INTERVAL`; construction primes one
   refresh, but with the 1 s interval the very first displayed point may
   still be 0 on slow machines. Documented behavior; the line chart's fixed
   0-100 domain makes it a visible flat point, not a layout jump.
6. **macOS left-click behavior changes from launcher to charts popover.**
   Intentional per the feature spec ("click opens a gpui popover"); the
   launcher keeps the ⌥Space global hotkey and the tooltip still hints it.
   If product wants both, the fallback is a right-click/status menu, future
   work, not this plan.
7. **`open_sheet` is open-only when any sheet is active.** It cannot tell
   *which* sheet is open (kit `has_active_sheet` is boolean,
   `window_ext.rs:25`), and `open_sheet_at` would replace the active sheet
   (`root.rs:325-349`), so it no-ops rather than swapping a gallery sheet it
   does not own. Tried: no sheet identity API in `sheet.rs`/`root.rs`.
   Consequence: repeated menu picks while a sheet is open do nothing,
   acceptable; Esc closes.
8. **Windows tray tests only execute on a Windows host.** The module is
   cfg'd per-OS in `tray/mod.rs:3-4`, so Linux never compiles
   `windows.test.rs`; only the Windows test lane runs it (CI test jobs at
   `ci.yml:245, 285` per survey §8). Not runnable in this environment;
   the done-check for Step 5.1 is compile + grep on the dev host.
9. **Em-dash usage kept deliberately low** per AGENTS.md's "em-dash spam"
   ban; the handful that remain mark genuine parentheticals. No other
   prose-rule exceptions were skipped.
