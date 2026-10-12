# 26 Layer Shell Widgets Plan

## Goal

On Linux Wayland sessions, widget floats requested through plan 02's
`spawn_floating` open as `zwlr_layer_surface_v1` surfaces (desktop-pinned
`Layer::Background`/`Bottom` or above-windows `Layer::Top`, anchored to the
output, absent from taskbar and Alt-Tab), with click-through toggled through
the public Wayland-only `Window::set_input_region`. Compositors without the
protocol (GNOME/Mutter) are feature-detected at spawn and degrade to plan 02's
existing float without caller changes; X11, macOS, Windows, and wasm are
untouched. This plan is Wayland-only and adds no X11 work; X11 keeps 02's
PopUp path plus 25's shim.

## Dependencies

- `docs/plans/02-floating-window-toolkit.md`, which 26 amends. Consumed
  contract items: `FloatingSpec` (builder pattern, gains the `layer_shell`
  option via Amendment 1), `spawn_floating`/`close_floating`/`respawn_floating`
  (the spawn path 26 extends), `FloatingHandle::Native` (passthrough target),
  `FloatingWindowEntry` (gains the `surface` field via Amendment 1; struct is
  `#[non_exhaustive]`, consumers 03/04/09/10 only read it), the
  `FloatingWindowRegistry` global-set pattern, the capability-constant pattern
  (`CAPABILITY_*`), and the `floating::init(cx)` boot hook (26 adds one cfg'd
  line inside it). Amendment 1 is appended to 02's doc, append-only, no
  reflow — 02's contract block stays line-stable for consumers 03/04/09/10.
- `docs/plans/25-*` (sibling, not on disk at planning time): per the feature
  spec, 25 owns the desktop-widget click-through toggle surface and dispatches
  to this plan's `set_input_passthrough` on Wayland. Signature reconciliation
  is Open Question 1; the amendment records the exact item 25 consumes.
- `docs/plans/27-*` (sibling, not on disk at planning time), the widget UI
  plan. Expected to consume `FloatingSpec::layer_shell`, `surface` on
  snapshot entries, `spawn_floating_after_displays`, and the two capability
  names. Noted in Open Question 1; nothing in 26 assumes 27 exists (all gates
  pass with 27 absent).
- Surveys: `_survey/desktop-widgets.md` (primary reference for the Linux
  verdict and the unverified-items list) and `_survey/seams.md` §3 (platform
  cfg-gate style), §6 (wasm rules), §8 (gates).

Execution order: 02 first, then 26. 25 and 27 may land in any order around 26
(26's contract items they consume are append-only additions).

## Research Digest

Verified this session against the pinned sources on disk
(`ls ~/.cargo/registry/src/*/ | grep -E 'gpui-pre-0.3.9|gpui-kit-0.7.3'` →
both present). Short paths: `GPUI` =
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-0.3.9/src`,
`LINUX` = `.../gpui-pre-linux-0.3.9/src/linux`, `KIT` =
`.../gpui-kit-0.7.3`, `REPO` = `/home/thragg/fo/gpui-starter`. The survey and
evidence files were read in full; everything load-bearing below was
re-verified against the 0.3.9/0.7.3 sources directly (the survey was written
against these same versions).

Core types (`GPUI/platform/layer_shell.rs`, 83 lines, read in full):

- `LayerShellOptions` fields at `:58-77`: `namespace: String` (compositor
  rule matching, immutable after creation), `layer: Layer`, `anchor: Anchor`,
  `exclusive_zone: Option<Pixels>`, `exclusive_edge: Option<Anchor>`,
  `margin: Option<(Pixels, Pixels, Pixels, Pixels)>` in CSS order
  top/right/bottom/left (`:72-74`), `keyboard_interactivity:
  KeyboardInteractivity`.
- `Layer::{Background, Bottom, Top, Overlay}` at `:8-22`; default is
  `Overlay` (`:20-21`). `Anchor` bitflags TOP=1 BOTTOM=2 LEFT=4 RIGHT=8,
  combinable (`:24-39`). `KeyboardInteractivity::{None, Exclusive,
  OnDemand}` at `:42-55`; default `OnDemand` (`:53-54`).
- `LayerShellNotSupportedError` at `:79-83`, Display text
  `"Compositor doesn't support zwlr_layer_shell_v1"`.
- Module gate: `#[cfg(all(target_os = "linux", feature = "wayland"))] pub mod
  layer_shell;` (`GPUI/platform.rs:6-8`). `WindowKind::LayerShell(..)` variant
  carries the same gate (`GPUI/platform.rs:2724-2728`).
- The `wayland` feature is always on in this app's graph: the kit enables
  `font-kit/x11/wayland/runtime_shaders` on `gpui-pre-platform`
  (`KIT/Cargo.toml:460-468`). Re-export chain verified: `GPUI/gpui.rs:195`
  (`pub use platform::*`) → `KIT/src/lib.rs:101` (`pub use ::gpui::*`), so
  `gpui_kit::layer_shell::{Layer, Anchor, KeyboardInteractivity,
  LayerShellOptions, LayerShellNotSupportedError}` and
  `gpui_kit::WindowKind` resolve on Linux builds. App-side code gates with
  `#[cfg(target_os = "linux")]` only: a `feature = "wayland"` cfg inside app
  code would test gpui-starter's own features, not gpui-pre's.

Backend binding (`LINUX/wayland/window.rs`):

- The layer branch at `:161-204`: returns
  `Err(LayerShellNotSupportedError.into())` when the global is absent
  (`:162-164`); otherwise `get_layer_surface(&surface, target_output,
  layer, namespace, ..)` (`:166-173`), `set_size` from `params.bounds`
  (`:175-177`), `set_anchor` (`:179`), `set_keyboard_interactivity`
  (`:180-182`), `set_margin` (`:184-191`), `set_exclusive_zone`
  (`:193-195`), `apply_exclusive_edge` (`:197-199`). Enum mapping functions
  in `LINUX/wayland/layer_shell.rs:1-28`. The global is bound as versions
  `1..=5` (`LINUX/wayland/client.rs:305`).
- Output selection: `WindowOptions::display_id` → `WindowParams::display_id`
  → `target_output` (`LINUX/wayland/client.rs:1311-1319`) → passed to
  `get_layer_surface` (`:166-173`). `None` means the compositor picks.
- Error propagation to app code: `WaylandSurfaceState::new` Err → `?` in
  `WaylandWindow::new` (`LINUX/wayland/window.rs:844-852`) → `?` in the
  client's `open_window` (`client.rs:1333`) → `Window::new` → `App::
  open_window` returns it unchanged (`Err(e)` arm, `GPUI/app.rs:1526-1529`;
  signature `anyhow::Result<WindowHandle<V>>` at `:1500-1503`). No
  `.context()` wrap anywhere in the chain, so
  `err.downcast_ref::<LayerShellNotSupportedError>()` works. `KIT/src/
  lib.rs:176-192` wraps this as
  `Result<(AnyWindowHandle, Entity<V>)>`.

Input region (click-through):

- `Window::set_input_region(Option<&[Bounds<Pixels>]>)` is public and
  documented Wayland-only: `Some(&[])` receives no pointer/touch input,
  `None` resets (`GPUI/window.rs:2512-2520`). The `PlatformWindow` default is
  a no-op (`GPUI/platform.rs:1336`); the X11 backend has no override (grep
  over `LINUX/x11/window.rs` for `set_input_region`: zero hits, run this
  session), so on X11 the call silently does nothing; the X11 arm belongs
  to 25's XShape shim, not here.
- Wayland implementation `LINUX/wayland/window.rs:2134-2160`: builds a
  `wl_region` from the rects and commits immediately so the change applies
  before the next click (`:2158-2160`).
- Runtime layer tuning also exists: `Window::set_exclusive_zone` /
  `set_exclusive_edge` (`GPUI/window.rs:2488-2504`). Layer itself is
  creation-only; runtime topmost does not exist on Wayland (compositor owns
  z-order; survey Part 3 and `gpui-windowing.md` capability table).

Session detection:

- Async spawn shape (load-bearing for the deferred-open helper):
  `App::spawn(&self, f) -> Task<R>` where `f: AsyncFnOnce(&mut AsyncApp) ->
  R + 'static` (`GPUI/app.rs:2297-2301`); the `Task` comes
  back directly, so `.detach()` needs no extra call. `AsyncApp`'s public methods are
  `refresh` (`GPUI/app/async_context.rs:150`), `background_executor`
  (`:159`), `is_test` (`:165`), `foreground_executor` (`:170`), `update`
  (`:176`), `subscribe` (`:184`), `open_window` (`:199`), `spawn` (`:218`),
  and the global helpers (`:229-277`); `upgrade` exists only as the private
  `self.app.upgrade()` on the internal `Weak<AppCell>` (`:92,106,122`), so
  task code reaches app state exclusively through `cx.update`.
  `BackgroundExecutor::timer(Duration) -> Task<()>` at
  `GPUI/executor.rs:182-183`.
- `App::compositor_name() -> &'static str` (`GPUI/app.rs:1877-1878`, trait
  default `""` at `GPUI/platform.rs:551-553`). The Wayland connection
  returns `"Wayland"` (`LINUX/wayland/client.rs:1503-1505`); X11 returns
  `"X11"` (`LINUX/x11/client.rs:1631-1633`); the test platform inherits the
  default `""`. This is live connection state, not an env-var guess, and is
  the session probe this plan uses. (`guess_compositor()`,
  `GPUI/platform.rs:205-212`, is env-only and stays unused.)
- The X11 backend has no `WindowKind::LayerShell` branch (grep over
  `LINUX/x11/window.rs` for `LayerShell`: zero hits, run this session;
  evidence `linux.md:98-99`): a LayerShell request on X11 silently creates a
  normal window. Never send the kind off Wayland.
- Window drag: `start_window_move` only acts on xdg toplevels
  (`LINUX/wayland/window.rs:2085-2090`), so layer surfaces reposition by
  respawn with new anchor/margin, not by drag.

Upstream worked example (reference implementation, fetched this session with
`curl -s https://raw.githubusercontent.com/zed-industries/zed/93d7456/crates/
gpui/examples/layer_shell.rs`): a transparent bottom-anchored clock. Exact
shape verified: `cfg(all(target_os = "linux", feature = "wayland"))`;
`WindowOptions { titlebar: None, window_bounds: Windowed(Bounds { origin:
(0,0), size: 500x200 }), app_id: "gpui-layer-shell-example",
window_background: WindowBackgroundAppearance::Transparent, kind:
WindowKind::LayerShell(LayerShellOptions { namespace: "gpui", anchor:
Anchor::LEFT | Anchor::RIGHT | Anchor::BOTTOM, margin: Some((0, 0, 40, 0)),
keyboard_interactivity: KeyboardInteractivity::None, ..Default::default() })
}`. It sets no layer (gpui default Overlay), no display_id, no exclusive
zone. The options mapping this plan builds is the same field set with the
same value types; the snapshot provenance (gpui-pre 0.3.9 = zed@93d7456,
crates.io API, survey Path key) makes the example the exact reference for
the pinned sources.

Known upstream issue zed#46378 (open; fetched this session): `cx.displays()`
returns an empty list during startup until a window has rendered a frame,
and layer surfaces must pick their output at creation, so targeted output
selection fails at cold start and a `None` output is placed arbitrarily by
the compositor. The workaround recorded in the issue is a deferred open:
sleep ~100 ms, then read `cx.displays()` and spawn. This plan ships that
workaround as `spawn_floating_after_displays`.

Compositor support matrix (survey, wayland.app table): KWin 6.7 (v5),
Hyprland, Sway, niri, COSMIC, river, labwc, Wayfire implement the protocol;
Mutter (GNOME), Weston, Cage do not (mutter#973). Blur is the KWin-only
`org_kde_kwin_blur` protocol; 26 does not request `Blurred` on layer
surfaces.

Binding survey items (these stay risks, never assumptions): Mutter's
honoring of empty input regions is UNVERIFIED (survey Part 3, flagged);
Windows DComp click-through, `kCGDesktopWindowLevel`, and sparse-MSIX widget
registration are 25's and out of scope here.

Repo facts (read this session): app boots through `gpui_kit::application()`
(`REPO/src/web.rs:25`); `crate::notifications::initialize(cx);` sits at
`REPO/src/app/init.rs:274` (02's `floating::init` hooks after it); platform
module cfg style `REPO/src/platform/mod.rs:2-3`, family gate with exclusion
comment `REPO/src/platform/desktop_shell/mod.rs:1-4`; raw-handle reach-in
precedent `REPO/src/platform/liquid_glass.rs:230-240` (not needed here;
no unsafe anywhere in this plan); capability registry
`REPO/src/runtime/capabilities.rs:6-12` (FIVE pub fields; 02's "six" is a
known slip, INDEX §8) with helpers `supported_enabled`/`degraded`/`error`
(`:16-51`); tracing targets follow `gpui_starter::<module>`
(`REPO/src/runtime/events.rs:51`); kit-facade import rule enforced by
`REPO/tests/qa_migration.rs:386-405`; `Edges<T>` exists cross-platform
(`GPUI/geometry.rs:1753`); test platform opens windows
(`GPUI/platform/test/platform.rs:557-561`) with a non-empty display list
(`:210`) and `compositor_name() == ""`, so Wayland-branch tests on the test
platform exercise the degradation arms only.

## Architecture

Entities and state ownership:

- `LayerShellSpec` (cross-platform value object, private fields + builders
  + `pub(crate)` readers): what the caller wants from the surface: layer,
  anchor placement, margin, exclusive zone/edge, keyboard mode, namespace.
  No behavior; the readers exist because `wayland::plan` is a sibling
  module and cannot see the fields.
- `LayerSpawnPlan` (linux-only, `pub(crate)`): what the spawn path does:
  `layer_options: Option<gpui LayerShellOptions>` (None unless the spec asks
  AND the session is Wayland) plus `display_id: Option<DisplayId>`
  (best-effort output selection). Produced by `wayland::plan(&spec, cx)`.
- `LayerShellState` (app `Global`, cross-platform): the feature-detection
  record: `available: Option<bool>` (None = unprobed; Some(true) = a layer
  surface opened once; Some(false) = `LayerShellNotSupportedError` seen),
  `fallbacks: u32` (float degradations after a layer attempt),
  `last_error: Option<SharedString>`. Mutated only through
  `state::record_available` / `record_unsupported`, which also rewrite the
  two capability entries so the diagnostics page is always current.
  `observe_global` consumers (plan 27's status UI) re-render on change.
- `set_input_passthrough(handle, on, cx)`: stateless function; the Wayland
  arm of 25's toggle surface. Guards `cx.compositor_name() == "Wayland"`
  and `FloatingHandle::Native`, then calls
  `window.set_input_region(Some(&[]) | None)` inside
  `AnyWindowHandle::update`.
- `spawn_floating_after_displays(spec, build, cx)`: zed#46378 workaround.
  Immediate spawn when `cx.displays()` is non-empty; otherwise a detached
  `cx.spawn` loop (100 ms timer, max 10 tries) that spawns once displays
  appear or the budget runs out.

Events and data flow:

```text
caller (27 demo / future widgets)
  └─ FloatingSpec::new(..).layer_shell(LayerShellSpec::new().layer(Background))
       └─ spawn_floating(spec, build, cx)                    [02, amended]
            ├─ non-Linux / non-Wayland session: option ignored (one info log),
            │   plain float path, entry.surface = Window
            └─ Wayland session:
                 ├─ wayland::plan → WindowKind::LayerShell(..), Transparent,
                 │   titlebar None, display_id best-effort
                 ├─ open_window Ok  → record_available(cx); surface = LayerShell
                 └─ open_window Err(LayerShellNotSupportedError)
                      → record_unsupported(cx) (capability → degraded, reason
                        "compositor lacks zwlr_layer_shell_v1 (e.g. GNOME/Mutter)")
                      → retry once through the plain float path → surface = Window

25's click-through toggle (on Wayland) → set_input_passthrough(handle, on, cx)
  └─ AnyWindowHandle::update → Window::set_input_region(Some(&[]) | None)
```

Module tree (inside plan 02's module; `layer_shell/` is the 26-owned part):

```text
src/ui/floating/                 [02]
├── mod.rs                       [02, +2 lines from 26: pub mod + init hook]
├── spec.rs                      [02, +builder/reader/field from 26]
├── registry.rs                  [02, +surface field from 26]
├── spawn.rs                     [02, +layer branch from 26]
└── layer_shell/                 [26, all new]
    ├── mod.rs            re-export seam, init(cx) capability registration
    ├── spec.rs           LayerShellSpec, WidgetLayer, WidgetKeyboardInteractivity,
    │                     Edge4, FloatSurface (cross-platform, pure data)
    ├── spec.test.rs
    ├── wayland.rs        cfg(target_os = "linux"): enum/bitflag conversion,
    │                     LayerSpawnPlan, plan(), is_layer_shell_unsupported()
    ├── wayland.test.rs
    ├── state.rs          LayerShellState global + capability transitions
    ├── state.test.rs
    ├── passthrough.rs    set_input_passthrough (session-guarded)
    ├── passthrough.test.rs
    ├── defer.rs          spawn_floating_after_displays (zed#46378 workaround)
    └── defer.test.rs
```

cfg-gate diagram (which module compiles where; no shims in this plan; those
are 25's):

```text
                          macOS  Windows  Linux  wasm
layer_shell/mod.rs          ✓       ✓       ✓      ✓   (init writes unsupported rows)
layer_shell/spec.rs         ✓       ✓       ✓      ✓   (pure data)
layer_shell/state.rs        ✓       ✓       ✓      ✓
layer_shell/passthrough.rs  ✓       ✓       ✓      ✓   (session guard → false off Wayland)
layer_shell/defer.rs        ✓       ✓       ✓      ✓
layer_shell/wayland.rs      —       —       ✓      —   (#[cfg(target_os = "linux")])
  uses gpui layer_shell types (module itself cfg'd out off Linux,
  GPUI/platform.rs:6-8, so the gate must be target_os only)
spawn.rs layer branch       —       —       ✓      —   (#[cfg] arms inside 02's spawn.rs)
```

## Interface Contract

Public surface at `crate::ui::floating::layer_shell` (re-exported from
`crate::ui::floating` for the spec builder and entry field; all gpui types
via `gpui_kit::*`, never `use gpui::`, `tests/qa_migration.rs:386-405`).
Identical items are recorded in 02's Amendment 1; if the two ever disagree,
02's amendment block is the contract of record and this section follows it.

```rust
/// Which z-order band a layer surface occupies. Mirror of the Wayland
/// protocol layers; converted to gpui's `Layer` on Linux only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetLayer {
    Background,
    Bottom,
    /// Above normal windows. Default for desktop widgets (gpui's own
    /// default is Overlay; this diverges deliberately: widgets sit above
    /// app windows, below docks/overlays).
    #[default]
    Top,
    Overlay,
}

/// Keyboard delivery for a layer surface. Mirror of the protocol modes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetKeyboardInteractivity {
    /// No keyboard focus, ever. Default: widgets do not steal keys.
    #[default]
    None,
    /// Exclusive grab while stacked above other layer surfaces (docks).
    Exclusive,
    /// Focusable like a normal window.
    OnDemand,
}

/// A single screen edge, for the exclusive-zone edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge4 { Top, Right, Bottom, Left }

/// Caller's request for a Wayland layer-shell surface. Pure data; compiles
/// on every platform and is inert (ignored, one info log per process) off
/// Wayland sessions.
#[derive(Clone, Debug)]
pub struct LayerShellSpec { /* private fields */ }
impl Default for LayerShellSpec {
    /// layer Top, namespace "gpui-starter.widgets", keyboard None,
    /// anchor/margin derived from the FloatingSpec's FloatAnchor,
    /// no exclusive zone/edge.
    fn default() -> Self;
}
impl LayerShellSpec {
    pub fn new() -> Self;
    pub fn layer(self, layer: WidgetLayer) -> Self;
    /// Overrides the anchor derived from FloatAnchor. Center = unanchored
    /// (protocol centers the surface); corners set both edge bits.
    pub fn anchor(self, placement: Placement9) -> Self;
    /// CSS order top/right/bottom/left, matching gpui's LayerShellOptions.
    pub fn margin(self, margin: Edges<Pixels>) -> Self;
    pub fn exclusive_zone(self, zone: Pixels) -> Self;
    /// Ignored with a warning unless the anchor includes this edge (the
    /// compositor treats any other value as a protocol error).
    pub fn exclusive_edge(self, edge: Edge4) -> Self;
    pub fn keyboard(self, mode: WidgetKeyboardInteractivity) -> Self;
    pub fn namespace(self, ns: impl Into<String>) -> Self;

    // Crate-internal readers, used by `wayland::plan` (a sibling module, so
    // the private fields are not visible there; Rust privacy is
    // module-scoped). The `requested_` prefix avoids colliding with the
    // same-named builders. Not part of the pub surface.
    pub(crate) fn requested_layer(&self) -> WidgetLayer;
    pub(crate) fn requested_anchor(&self) -> Option<Placement9>;   // None = derive from FloatAnchor
    pub(crate) fn requested_margin(&self) -> Option<Edges<Pixels>>; // None = derive from FloatAnchor offset
    pub(crate) fn requested_exclusive_zone(&self) -> Option<Pixels>;
    pub(crate) fn requested_exclusive_edge(&self) -> Option<Edge4>;
    pub(crate) fn requested_keyboard(&self) -> WidgetKeyboardInteractivity;
    pub(crate) fn requested_namespace(&self) -> &str;
}

/// Which surface kind actually backs a float. Appended to
/// FloatingWindowEntry as `pub surface: FloatSurface` (02 Amendment 1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FloatSurface {
    #[default]
    Window,
    LayerShell,
}

// On FloatingSpec (02 Amendment 1):
//   pub fn layer_shell(self, spec: LayerShellSpec) -> Self;
//   pub fn layer_shell_spec(&self) -> Option<&LayerShellSpec>;

pub const CAPABILITY_LAYER_SHELL_WIDGETS: &str = "layer_shell_widgets";
pub const CAPABILITY_LAYER_SHELL_INPUT_PASSTHROUGH: &str = "layer_shell_input_passthrough";

/// Wayland arm of the desktop-widget click-through toggle. 25's toggle
/// surface dispatches here when the session is Wayland. Applies
/// `Window::set_input_region(Some(&[]) | None)` to the native window and
/// returns true; returns false for overlay handles, dead handles, or any
/// non-Wayland session (macOS/Windows/X11 arms live in 25's shims).
pub fn set_input_passthrough(handle: &crate::ui::floating::FloatingHandle,
                             passthrough: bool, cx: &mut App) -> bool;

/// zed#46378 workaround: spawn a float once displays are enumerable.
/// Spawns immediately when `cx.displays()` is non-empty; otherwise retries
/// every 100 ms up to 10 times, then spawns anyway (compositor-chosen
/// output). Logs the outcome; never panics. Mirrors spawn_floating's build
/// signature (Fn, retained for respawn).
pub fn spawn_floating_after_displays(
    spec: crate::ui::floating::FloatingSpec,
    build: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    cx: &mut App,
);

/// True when the error is the compositor's layer-shell rejection. Downcasts
/// to `LayerShellNotSupportedError` first, then falls back to matching the
/// Display text ("Compositor doesn't support zwlr_layer_shell_v1") in case
/// a future wrap breaks the downcast chain. Linux-only; `false` elsewhere.
#[cfg(target_os = "linux")]
pub fn is_layer_shell_unsupported(err: &anyhow::Error) -> bool;

/// Called once from `floating::init` (02 Amendment 1). Registers the two
/// capability rows per the matrix in Platform & Wasm Gating. Idempotent.
pub fn init(cx: &mut App);
```

Guarantees for consumers: `spawn_floating` keeps 02's error contract: the
layer-shell option never adds a new `Err` case; a compositor rejection is
absorbed by the float fallback and recorded in `layer_shell_widgets`
(degraded with reason) plus `LayerShellState`. Off Wayland the option is a
no-op. `set_input_passthrough` never panics and never touches overlay
handles. No unsafe code exists anywhere in this feature.

## Module Layout

| File | Purpose |
| --- | --- |
| `src/ui/floating/layer_shell/mod.rs` | Re-export seam; `init(cx)` capability registration; doc comment naming removal seams |
| `src/ui/floating/layer_shell/spec.rs` | Cross-platform spec types (`LayerShellSpec`, `WidgetLayer`, `WidgetKeyboardInteractivity`, `Edge4`, `FloatSurface`) |
| `src/ui/floating/layer_shell/spec.test.rs` | Defaults, builder overrides, derive-from-anchor pure mapping tests |
| `src/ui/floating/layer_shell/wayland.rs` | cfg(linux): `Placement9`→`Anchor` bits, `Edges`→margin tuple, enum mirrors→gpui enums, `LayerSpawnPlan` + `plan()`, `is_layer_shell_unsupported` |
| `src/ui/floating/layer_shell/wayland.test.rs` | Conversion table, error classification, session-guard tests |
| `src/ui/floating/layer_shell/state.rs` | `LayerShellState` global, `record_*` transitions, capability writers |
| `src/ui/floating/layer_shell/state.test.rs` | Global transitions + capability snapshot tests |
| `src/ui/floating/layer_shell/passthrough.rs` | `set_input_passthrough` with session guard |
| `src/ui/floating/layer_shell/passthrough.test.rs` | Off-Wayland/dead-handle false tests |
| `src/ui/floating/layer_shell/defer.rs` | `spawn_floating_after_displays` (zed#46378 deferred open) |
| `src/ui/floating/layer_shell/defer.test.rs` | Immediate-path spawn test on the test platform |

Test files wire with the repo convention `#[cfg(test)] #[path = "<name>.test.rs"]
mod <name>_test;` (seams §7). The whole `layer_shell` module's tests run on
the Linux CI lane (`cargo test -p gpui-starter --lib`, `ci.yml:245`);
`wayland.test.rs` compiles only there.

## Seam Touches

| # | File | Change | Why minimal |
| --- | --- | --- | --- |
| 1 | `docs/plans/02-floating-window-toolkit.md` | Append `## Amendment 1 (plan 26)` at EOF (done in this planning session) | Contract addendum; append-only, zero reflow of the line-stable block |
| 2 | `src/ui/floating/mod.rs` (02's) | `pub mod layer_shell;` + `pub use layer_shell::{FloatSurface, LayerShellSpec, WidgetLayer, WidgetKeyboardInteractivity, Edge4, set_input_passthrough, spawn_floating_after_displays, CAPABILITY_LAYER_SHELL_WIDGETS, CAPABILITY_LAYER_SHELL_INPUT_PASSTHROUGH};` + `layer_shell::init(cx);` at the end of `floating::init` | Module mount + boot hook; two-line pattern per seams §2/§3 |
| 3 | `src/ui/floating/spec.rs` (02's) | Private field `layer_shell: Option<LayerShellSpec>` (default None) + builder `layer_shell(self, ..)` + reader `layer_shell_spec(&self)` | One field, two methods; builder pattern per 02's existing style |
| 4 | `src/ui/floating/registry.rs` (02's) | `FloatingWindowEntry` gains `pub surface: FloatSurface` (entry stays `#[non_exhaustive]`); `RegistryEntry` carries it; `floating_snapshot` maps it through | One field through the existing clone-out; consumers 03/04/09/10 only read entries |
| 5 | `src/ui/floating/spawn.rs` (02's) | cfg(linux) arms: `plan(&spec, cx)` before options construction; hoist options construction into `make_options(use_layer: bool)`; patch kind/background/titlebar/display_id when the plan has layer options; on `Err` with `is_layer_shell_unsupported` → log warn, `record_unsupported`, retry once via `make_options(false)`, tag `surface = Window`; on Ok with layer → `record_available`, tag `surface = LayerShell`; non-linux sets `surface = Window` | ~25 cfg'd lines inside the existing native branch; shared helpers stay in 26's module |

No `Cargo.toml` change (the wayland feature is already enabled via the kit,
`KIT/Cargo.toml:460-468`; no new crates). No `src/app/init.rs` change (rides
02's `floating::init`). No `locales/` change. No `src/runtime/events.rs`
change (capability rows + `LayerShellState` notify carry the state).

### Removal Recipe

Mirror of `src/gallery/mod.rs:2-7` style. Execute after 25/27 (consumers of
`set_input_passthrough` and the spec builder) are gone:

1. Delete `src/ui/floating/layer_shell/` entirely.
2. In `src/ui/floating/mod.rs`, remove the `pub mod layer_shell;` line, the
   `pub use layer_shell::{..};` block, and the `layer_shell::init(cx);` call
   inside `floating::init`.
3. In `src/ui/floating/spec.rs`, remove the `layer_shell` field and the
   `layer_shell`/`layer_shell_spec` methods.
4. In `src/ui/floating/registry.rs`, remove the `surface` field from
   `FloatingWindowEntry`/`RegistryEntry` and the snapshot mapping.
5. In `src/ui/floating/spawn.rs`, remove the cfg(linux) layer arms and the
   `make_options` hoist (restore single inline construction); remove the
   `surface` tagging (field is gone).
6. Delete the `## Amendment 1 (plan 26)` section from
   `docs/plans/02-floating-window-toolkit.md`.
7. Run `cargo fmt --all -- --check`, `cargo clippy --workspace
   --all-features`, `cargo test --workspace --all-features`,
   `just wasm-check`; grep `layer_shell` across `src/` → zero hits.

## Implementation Phases

Prerequisite: plan 02 has landed (`src/ui/floating/` exists with working
spawn/registry/init). No unsafe code in any step, so there is no safety argument to
write; the CI lanes that compile the linux-only code are the Linux check and
test lanes (`ci.yml:136-139, 245`) plus the local gates; macOS/Windows lanes
prove the cfg discipline by compiling the module without `wayland.rs`.

## Phase 1: Cross-platform spec types

### Step 1.1: Mount the module

Files: `src/ui/floating/layer_shell/mod.rs` (new), `src/ui/floating/mod.rs`
(edit).
Build: `mod.rs` with the contract re-exports (`pub use spec::{..};` plus the
capability constants). Declare every later-phase file now as an empty
compiling module (`spec.rs` filled this phase; empty `state.rs`,
`passthrough.rs`, `defer.rs`; `wayland.rs` arrives in Phase 2), so no
`todo!()` stubs exist at any point. `mod.rs` carries the doc comment naming
the removal seams (Seam Touches list). `floating/mod.rs` gains `pub mod
layer_shell;` and the `pub use` block from Seam Touches row 2 (initially
covering only the spec types).
Tests: none.
Done-check: `grep -n "pub mod layer_shell" src/ui/floating/mod.rs` returns
the line; `cargo check -p gpui-starter` passes.

### Step 1.2: `spec.rs` types

Files: `src/ui/floating/layer_shell/spec.rs`.
Build: `WidgetLayer`, `WidgetKeyboardInteractivity` (both with the defaults
from the contract), `Edge4`, `LayerShellSpec` with private fields, the full
builder set, and the seven `pub(crate) fn requested_*` readers from the
contract (wayland.rs is a sibling module, so it cannot touch the private
fields; privacy is module-scoped), `Default for LayerShellSpec` (layer Top,
namespace `"gpui-starter.widgets"`, keyboard None, anchor/margin None =
derive, exclusive zone/edge None). Derives: `Clone, Debug` on
`LayerShellSpec`; `Clone, Copy, Debug, Default, PartialEq, Eq` on the enums
(match 02's `FloatLevel` style). `anchor`/`margin` stored as `Option<..>` so
"derive from FloatAnchor" is representable. Derivation itself happens in
`wayland.rs`; `spec.rs` stays free of gpui-session context.
Tests: none yet.
Done-check: `cargo check -p gpui-starter` passes; `grep -c "pub fn"
src/ui/floating/layer_shell/spec.rs` ≥ 15 (new + 7 builders + 7 readers).

### Step 1.3: `FloatingSpec` amendment

Files: `src/ui/floating/spec.rs` (02's).
Build: private field `layer_shell: Option<LayerShellSpec>` initialized to
`None` in `FloatingSpec::new`; builder `pub fn layer_shell(mut self, spec:
LayerShellSpec) -> Self` (consume-self, fluent, matches 02's builder set);
reader `pub fn layer_shell_spec(&self) -> Option<&LayerShellSpec>`.
Tests: extend 02's `spec.test.rs`: builder sets and reader returns the spec;
default is None.
Done-check: `cargo test -p gpui-starter --lib floating::spec` green.

### Step 1.4: Registry `surface` field

Files: `src/ui/floating/registry.rs` (02's).
Build: `FloatSurface` import; `FloatingWindowEntry` gains `pub surface:
FloatSurface` after `handle` (struct stays `#[non_exhaustive]`; five-field
literals inside the crate gain the sixth entry: 02's own struct literals
and any 03/04/09/10 construction sites, which is why this lands before those
plans read the field; they don't construct it, they read it, so no consumer
edit); `RegistryEntry` carries `surface`; `floating_snapshot` maps it.
`spawn.rs` sets `FloatSurface::Window` on every existing spawn site for now
(Phase 2 tags the layer arm).
Tests: extend 02's `registry.test.rs`: snapshot entry reports `Window` by
default.
Done-check: `cargo test -p gpui-starter --lib floating::registry` green.

### Step 1.5: Spec tests

Files: `src/ui/floating/layer_shell/spec.test.rs` (new), wired in `spec.rs`.
Build (plain `#[test]`): default `LayerShellSpec` reports layer Top,
keyboard None, namespace "gpui-starter.widgets", anchor/margin None; each
builder overrides exactly its field and consumes self; `Edge4`/`WidgetLayer`
derive `Copy` (compile-level, asserted by use).
Done-check: `cargo test -p gpui-starter --lib floating::layer_shell::spec`
green.

### Step 1.6: `state.rs` core (record helpers — REQUIRED before Step 2.2)

Step 2.2's spawn integration calls `state::record_available` /
`state::record_unsupported`, so the state module's data shape and both
record helpers land HERE, in Phase 1, not in Phase 3 (post-review fix:
with state.rs empty until Phase 3, Step 2.2's done-check cannot compile on
the Linux lane where the cfg(target_os = "linux") branch referencing these
functions is built).

Files: `src/ui/floating/layer_shell/state.rs` (fill the Phase 1 empty module).
Build:
- `#[derive(Default)] pub struct LayerShellState { pub(crate) available:
  Option<bool>, pub(crate) fallbacks: u32, pub(crate) last_error:
  Option<SharedString> }` implementing `Global` (shape mirrors
  `CapabilityRegistry`, `src/runtime/capabilities.rs:54-58`); mutation via
  `cx.default_global`-style set helpers so `observe_global` consumers
  re-render (`GPUI/app.rs:2414-2421`, same pattern 02's registry uses).
- `pub(crate) fn record_available(cx: &mut App)`: available = Some(true),
  write `layer_shell_widgets` = `CapabilityStatus::supported_enabled()`.
- `pub(crate) fn record_unsupported(err: &anyhow::Error, cx: &mut App)`:
  available = Some(false), `fallbacks += 1`, last_error set, write
  `layer_shell_widgets` = `CapabilityStatus::degraded(err)` (helper at
  `src/runtime/capabilities.rs:27-38`; five fields handled by the helper).
Tests: none new (the record helpers are exercised end-to-end by Step 2.2's
spawn tests once the Wayland arm exists).
Done-check: `cargo check -p gpui-starter` passes with state.rs filled;
`grep -n "record_available" src/ui/floating/layer_shell/state.rs` returns
the fn.

## Phase 2: Wayland planning and the spawn branch

### Step 2.1: `wayland.rs` conversion and planning

Files: `src/ui/floating/layer_shell/wayland.rs` (new), declared in `mod.rs`
as `#[cfg(target_os = "linux")] pub(crate) mod wayland;`.
Build:
- `fn anchor_bits(placement: Placement9) -> Anchor`: Top→TOP, Bottom→BOTTOM,
  Left→LEFT, Right→RIGHT, corners→both bits, Center→`Anchor::empty()`
  (protocol centers an unanchored surface).
- `fn margin_tuple(m: Edges<Pixels>) -> (Pixels, Pixels, Pixels, Pixels)`:
  CSS order top/right/bottom/left, matching `LayerShellOptions::margin`
  (`GPUI/platform/layer_shell.rs:72-74`).
- `fn layer_of(l: WidgetLayer) -> Layer`, `fn keyboard_of(k:
  WidgetKeyboardInteractivity) -> KeyboardInteractivity`, `fn edge_bits(e:
  Edge4) -> Anchor` (single bit each).
- `pub(crate) struct LayerSpawnPlan { pub layer_options:
  Option<LayerShellOptions>, pub display_id: Option<DisplayId> }`.
- `pub(crate) fn plan(spec: &FloatingSpec, cx: &App) -> LayerSpawnPlan`:
  returns the empty plan unless `spec.layer_shell_spec().is_some()` &&
  `cx.compositor_name() == "Wayland"` (imports of `Anchor`/`Layer`/
  `LayerShellOptions` resolve only on Linux builds, hence the file gate).
  When active, read the request through the `pub(crate)` readers (private
  fields are invisible from this sibling module): effective placement =
  `ls.requested_anchor()` or the FloatAnchor's placement (Screen variant's
  `placement`; a `FloatAnchor::Window` request logs one warn (layer
  surfaces anchor to output edges, not window rects) and degrades to the
  plain float path by returning the empty plan); `anchor_bits` +
  `margin_tuple` (`ls.requested_margin()`, else derived from the FloatAnchor
  `offset`: `top = offset.y` when the placement touches top, `left =
  offset.x` when it touches left, mirrored for bottom/right, 0 elsewhere);
  `ls.requested_namespace()`; `ls.requested_keyboard()`;
  `ls.requested_exclusive_zone()`; `ls.requested_exclusive_edge()` validated
  against the anchor bits (single bit AND contained, else `tracing::warn!`
  and dropped, mirroring the backend's own check,
  `LINUX/wayland/window.rs:482-496`, so the caller learns early).
  `display_id`: for `FloatAnchor::Screen { display: Some(id), .. }` →
  `cx.find_display(id)` (`GPUI/app.rs:1578`) mapped to its id, else
  `cx.primary_display()` (`GPUI/app.rs:1561`), else `None` with a
  `tracing::warn!` naming zed#46378 when the displays list is empty
  (cold-start case; the deferred-open helper is the fix).
- `pub fn is_layer_shell_unsupported(err: &anyhow::Error) -> bool`:
  `err.downcast_ref::<LayerShellNotSupportedError>().is_some() ||
   err.to_string().contains("zwlr_layer_shell_v1")` (belt-and-braces for a
  future context wrap; the current chain has none, Research Digest).
Tests: `wayland.test.rs` (new, wired in `wayland.rs`): anchor_bits table
(9 placements incl. Center→0); margin_tuple order; edge bits;
`is_layer_shell_unsupported` true for
`LayerShellNotSupportedError.into()` and for a string-wrapped copy, false
for `anyhow::anyhow!("other")`; `plan()` returns the empty plan when the
spec has no layer request (plain `FloatingSpec`), and, inside
`#[gpui_kit::test]`, when `compositor_name()` is `""` (test platform),
proving the session guard.
Done-check: `cargo test -p gpui-starter --lib floating::layer_shell` green
on a Linux host.

### Step 2.2: Spawn-path integration

Files: `src/ui/floating/spawn.rs` (02's).
Build (all inside the existing native branch):
1. Hoist the existing `WindowOptions` construction into a closure `let
   make_options = |use_layer: bool| -> WindowOptions { ..02's existing
   field set.. }` (behavior-identical for `use_layer == false`).
2. `#[cfg(target_os = "linux")] let plan =
   crate::ui::floating::layer_shell::wayland::plan(&spec, cx);` with a
   `#[cfg(not(target_os = "linux"))] let plan = NoPlan;` stand-in (or gate
   via a `let use_layer = ..` boolean; keep the non-linux code path
   byte-identical in behavior).
3. When the plan carries layer options: `make_options(true)` patches
   `kind: WindowKind::LayerShell(opts)`, `window_background:
   WindowBackgroundAppearance::Transparent`, `titlebar: None`,
   `display_id: plan.display_id` (the upstream example's option set; cite
   `crates/gpui/examples/layer_shell.rs` at zed@93d7456 in the doc comment).
4. Spawn via `gpui_kit::open_window` as 02 does. On `Ok`: if the layer
   options were used → `state::record_available(cx)`,
   `surface = FloatSurface::LayerShell`; else `FloatSurface::Window`.
5. On `Err(err)`: `#[cfg(target_os = "linux")] if used_layer &&
   is_layer_shell_unsupported(&err)`: `tracing::warn!(target:
   "gpui_starter::ui::floating::layer_shell", ..)` with the fallback note,
   `state::record_unsupported(&err, cx)`, retry `gpui_kit::open_window(
   make_options(false), ..)` once; on success tag `FloatSurface::Window` and
   proceed to registry set as normal; if the retry also fails, return that
   error (still one of 02's two documented Err causes). Non-layer errors
   propagate unchanged.
6. Registry set uses the resolved `surface`.
Tests: extend 02's `spawn.test.rs`: on the test platform (compositor ""),
a spec WITH `layer_shell(..)` spawns a `Native` float with
`entry.surface == Window` (degradation arm end-to-end); a spec without the
option is unchanged (`surface == Window`).
Done-check: `cargo test -p gpui-starter --lib floating` green; `grep -n
"LayerShellNotSupportedError\|make_options" src/ui/floating/spawn.rs` shows
the arms.

## Phase 3: State, capabilities, passthrough, deferred spawn

### Step 3.1: `state.rs` init and capability matrix

Files: `src/ui/floating/layer_shell/state.rs` (extend — the struct and both
record helpers landed in Step 1.6, before the spawn integration that calls
them; this step adds init only).
Build:
- `pub fn init(cx: &mut App)`: writes the initial capability matrix (table
  in Platform & Wasm Gating) using `crate::capabilities::set`
  (`src/runtime/capabilities.rs:65-79` pattern): non-Linux/wasm → both rows
  unsupported with reason "Wayland layer-shell only"; Linux X11 session
  (`cx.compositor_name() == "X11"`) → `layer_shell_widgets` unsupported
  reason "layer shell requires a Wayland session; X11 uses the floating
  toolkit", passthrough unsupported reason "input regions are Wayland-only;
  X11 click-through is plan 25's XShape shim"; Linux Wayland session →
  `layer_shell_widgets` supported_enabled (optimistic until a spawn proves
  otherwise; Mutter rejection flips it to degraded at first spawn) and
  `layer_shell_input_passthrough` supported_enabled. Reason strings are
  English literals (precedent `src/platform/input/shortcuts.rs:290-305`).
  Idempotent (second call rewrites the same values).
Tests: `state.test.rs` (`#[gpui_kit::test]`): on the test platform `init`
writes both rows unsupported with the Wayland-only reason
(`crate::capabilities::snapshot` check, `capabilities.rs:81-85`);
`record_unsupported` flips `layer_shell_widgets` to degraded with
`last_error` set and increments `fallbacks`; `record_available` restores
supported_enabled; an `observe_global::<LayerShellState>` flag fires on
record.
Done-check: `cargo test -p gpui-starter --lib floating::layer_shell::state`
green.

### Step 3.2: Boot hook inside `floating::init`

Files: `src/ui/floating/mod.rs` (02's), `src/ui/floating/layer_shell/mod.rs`.
Build: at the end of 02's `floating::init`, add
`crate::ui::floating::layer_shell::init(cx);` (no cfg; the function
compiles everywhere and writes the correct unsupported rows off Linux).
`layer_shell::init` logs at target `gpui_starter::ui::floating::layer_shell`.
Tests: none new (state.test covers init; boot smoke via existing lib tests).
Done-check: `grep -n "layer_shell::init" src/ui/floating/mod.rs` returns the
line; `cargo test -p gpui-starter --lib` green.

### Step 3.3: `passthrough.rs`

Files: `src/ui/floating/layer_shell/passthrough.rs` (new).
Build: `pub fn set_input_passthrough(handle: &FloatingHandle, passthrough:
bool, cx: &mut App) -> bool`; matches `FloatingHandle::Native(h)`; guards
`cx.compositor_name() == "Wayland"`; then `h.update(cx, |_, window, _|
window.set_input_region(if passthrough { Some(&[]) } else { None })).is_ok()`
(the `Some(&[])` and `None` semantics are documented at
`GPUI/window.rs:2512-2520`). Overlay arm and the guard-failure path return
false. No unsafe; no raw handles: this is public API, unlike 25's shims.
Tests: `passthrough.test.rs` (`#[gpui_kit::test]`): on the test platform
(compositor "") returns false for both a live native handle (spawn a float
first) and an `Overlay` handle; never panics.
Done-check: `cargo test -p gpui-starter --lib
floating::layer_shell::passthrough` green.

### Step 3.4: `defer.rs`

Files: `src/ui/floating/layer_shell/defer.rs` (new).
Build: `pub fn spawn_floating_after_displays(spec, build, cx)`: if
`!cx.displays().is_empty()` → `let _ = spawn_floating(spec, build, cx);`
(log the result at warn on Err). Else, using the exact async shapes
(`App::spawn(&self, f: AsyncFnOnce(&mut AsyncApp) -> R) -> Task<R>`
returns the `Task` directly, `GPUI/app.rs:2297-2301`; `AsyncApp`'s public
surface is `update`/`background_executor`/`refresh`/`is_test`/
`foreground_executor` plus the global/spawn helpers,
`GPUI/app/async_context.rs:150-227`, with no public
`upgrade`; and
`BackgroundExecutor::timer(Duration) -> Task<()>`,
`GPUI/executor.rs:182-183`):

```rust
cx.spawn(async move |cx| {
    for _ in 0..10 {
        if cx.update(|cx| !cx.displays().is_empty()) {
            break;
        }
        cx.background_executor().timer(Duration::from_millis(100)).await;
    }
    let result = cx.update(|cx| spawn_floating(spec, build, cx));
    tracing::warn!(target: LOG, ?result, "deferred layer-shell spawn settled");
})
.detach();
```

`cx.update` on `AsyncApp` returns `R` directly (`async_context.rs:176-177`),
so the loop check and the final spawn both go through it without result
unwrapping. Uses only `std::time::Duration` (allowed; the wasm clock rule
bans `Instant`/`SystemTime`, seams §6, and the timer path is native in
practice). The build closure moves into the task; `Fn` (not `FnOnce`)
keeps it callable once, matching 02's retained-factory constraint.
Tests: `defer.test.rs` (`#[gpui_kit::test]`): test platform displays are
non-empty (`GPUI/platform/test/platform.rs:210`), so the call spawns
immediately; assert `floating_snapshot(cx).len()` grew by one and the
entry is Native/Window. The retry loop is manual-QA territory (zed#46378
cold start).
Done-check: `cargo test -p gpui-starter --lib floating::layer_shell::defer`
green.

## Phase 4: Gates and cross-platform proof

### Step 4.1: Full gates

Files: none (verification only).
Build/commands, in order (AGENTS.md):
1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-features`
3. `cargo test --workspace --all-features`
4. `just wasm-check`
Plus: `git diff --stat` shows only the Seam Touches files.
Done-check: all four pass; `grep -rn "use gpui::" src/ui/floating/layer_shell/`
returns nothing (kit facade rule).

### Step 4.2: Cross-compile proof for the cfg fence

Files: none.
Build: on the Linux dev host run `cargo check -p gpui-starter --target
wasm32-unknown-unknown` is covered by `just wasm-check` (Step 4.1);
macOS/Windows compile proof comes from the CI check lanes
(`ci.yml:47-56` mac/win jobs compile the crate; `wayland.rs` is absent
there by cfg). If a Linux-only host must prove the fence before CI:
`cargo check -p gpui-starter` after temporarily commenting the `wayland`
module decl is NOT sanctioned; rely on CI lanes instead.
Done-check: CI green on all three OS check lanes; wasm-check green.

## Platform & Wasm Gating

cfg gates:

- `src/ui/floating/layer_shell/wayland.rs`:
  `#[cfg(target_os = "linux")] pub(crate) mod wayland;` in `layer_shell/mod.rs`
  (module-gate style of `src/platform/mod.rs:2-3`). The gate is `target_os`
  only: the gpui `wayland` feature is transitively always on
  (`KIT/Cargo.toml:460-468`) and a `feature =` cfg in app code would test
  the wrong crate's features.
- `spawn.rs` layer arms: `#[cfg(target_os = "linux")]` inside 02's native
  branch; the non-linux path compiles identically to pre-amendment 02.
- Everything else in `layer_shell/` compiles on every target so the spec
  builder and capability rows exist everywhere (coding-guides "Platform and
  capability boundaries": the semantic contract (`layer_shell()` is a hint)
  holds on all platforms; presentation differs).
- wasm: `target_os = "linux"` is false for wasm32, so `wayland.rs` and the
  spawn arms are compiled out; the wasm overlay path in 02 is untouched; a
  wasm `.layer_shell(..)` call is the same inert no-op as on macOS/Windows.

Capability-registry entries (written by `layer_shell::init`, rendered by the
diagnostics page):

| Key | macOS | Windows | Linux X11 | Linux Wayland | wasm |
| --- | --- | --- | --- | --- | --- |
| `layer_shell_widgets` | unsupported, "Wayland layer-shell only" | same | unsupported, "requires a Wayland session; X11 uses the floating toolkit" | supported_enabled → degraded("compositor lacks zwlr_layer_shell_v1…") on first failed spawn | unsupported, "Wayland layer-shell only" |
| `layer_shell_input_passthrough` | unsupported, "Wayland layer-shell only" (mac arm is 25's) | same (25's shim) | unsupported, "input regions are Wayland-only; X11 click-through is plan 25's XShape shim" | supported_enabled | unsupported, "Wayland layer-shell only" |

Native-only deps: none. No `Cargo.toml` table changes at all.

Platform behavior matrix:

| Behavior | Linux Wayland + protocol | Linux Wayland, Mutter | Linux X11 | macOS/Windows | wasm |
| --- | --- | --- | --- | --- | --- |
| `.layer_shell(..)` request | layer surface (spec'd layer/anchor/margin) | float fallback + degraded row | ignored (info log), PopUp/float per 02 | ignored (info log) | ignored, overlay per 02 |
| Taskbar/Alt-Tab entry | none (layer surfaces are not toplevels) | normal window (list shows it) | per 02's PopUp | per 02 | n/a |
| Click-through toggle | `set_input_region(Some(&[]))` | same call; Mutter honoring unverified (risk 1) | false (25's XShape shim owns it) | false (25's shims own it) | false |
| Reposition | respawn with new anchor/margin (drag is a toplevel no-op, `LINUX/wayland/window.rs:2085-2090`) | 02's respawn | 02's respawn | 02's respawn | 02's overlay update |
| Keyboard | `KeyboardInteractivity::None` default | n/a (fallback float) | 02's focus flag (ignored) | 02's focus flag | n/a |

## i18n

No catalog keys. The feature adds no user-facing copy: state surfaces
through the capability registry (English reason literals, precedent
`src/platform/input/shortcuts.rs:290-305`) and tracing logs; widget UI copy
belongs to plan 27. Both catalogs stay untouched;
`tests/qa_localization.rs:188-198` lockstep is trivially preserved.

## Design Compliance

Brief (platform plan; 27 owns the widget UI and reads the design guides in
full):

- State visibility: every degradation (no protocol, X11 session, non-Linux)
  is a capability row with a reason on the diagnostics page, per "State must be
  visible" (design guides, Feedback and overlays family); nothing degrades
  silently beyond a single info log per process.
- Focus and keyboard: `WidgetKeyboardInteractivity::None` is the default, so
  widgets never steal focus; the a11y implication (layer-surface content is
  still an accesskit tree via the backend's unix adapter) is inherited from
  gpui, and 27's widget content owns its a11y checklist pass.
- No new visual surface: no tokens, spacing, or components are introduced;
  the `FloatSurface` tag on snapshot entries is data, not chrome. 02's
  `FloatingShell` treatment stays the one surface treatment for floats
  ("Surfaces and elevation": one treatment per kind).
- Interface language: the only new copy is capability reason strings:
  factual, sentence case, no verbs-with-ellipsis.

## Verification

Repo gates (AGENTS.md; run in Step 4.1 exactly as named):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

`just wasm-check` proves the wasm compile (module compiled out, overlay
path untouched). The Linux CI lane runs the layer_shell unit tests; macOS
and Windows check lanes prove the cfg fence.

Feature-specific manual QA: this feature cannot be fully verified on one
OS; per-lane checks:

1. Linux Wayland, wlroots family (sway/Hyprland/niri) or KWin: launch the
   app (or a plan-27 demo once it lands; until then a temporary spawn via
   the gallery floating section with `.layer_shell(..)`), verify: the widget
   appears at the requested anchor+margin; no taskbar/Alt-Tab entry;
   `Layer::Background`/`Bottom` renders beneath normal windows; `Top`
   renders above; typing elsewhere is unaffected (keyboard None).
2. Same session, click-through: call `set_input_passthrough(handle, true)`
   (via 25's toggle once it lands, else a temporary button); clicks fall
   through to the window below; toggle off restores input. Esc closes.
3. Respawn with a different anchor (02's `respawn_floating` after a spec
   change): surface re-anchors. Direct drag does nothing (expected).
4. Diagnostics page: `layer_shell_widgets` supported/enabled on this
   session; `layer_shell_input_passthrough` supported/enabled.
5. Linux Wayland, GNOME (Mutter): spawn → float fallback opens (normal
   window); diagnostics shows `layer_shell_widgets` degraded with the
   zwlr reason; log carries the warn. Click-through on the fallback float:
   record whether Mutter honors the empty region; this closes the survey's
   unverified item either way; if it does not honor it, note it in 25's
   toggle UI copy (survey-binding rule: no silent assumption).
6. Linux X11 (`WAYLAND_DISPLAY` unset, `DISPLAY` set): `.layer_shell(..)`
   request logs the info line and opens 02's float; capability rows show
   the X11 reasons.
7. Multi-output Wayland: spawn with
   `FloatAnchor::Screen { display: Some(id) }` via
   `spawn_floating_after_displays` from a cold start; the widget lands on
   the requested output (zed#46378 workaround); a warm spawn targets
   directly.
8. macOS/Windows: no behavior change; feature compiles (CI check lanes);
   capability rows read unsupported.
9. wasm (`just wasm-build` + web dev flow): floats are 02's overlay panes;
   capability rows unsupported; no console errors.

Acceptance criteria:

- All four gates pass with only the Seam Touches files changed.
- Every public item in the Interface Contract exists with the exact
  signatures (grep/`cargo doc` checkable).
- A Wayland spec degrades to a working float on the test platform and on
  Mutter, with the capability row carrying the reason (tested in
  `spawn.test.rs` extension + manual QA 5).
- No `use gpui::` imports in the new module; no new dependencies; no
  unsafe code (`grep -rn "unsafe" src/ui/floating/layer_shell/` empty).
- 02's existing contract guarantees still hold (its consumers' plans need
  no edits: `FloatingWindowEntry` gained a field on a `#[non_exhaustive]`
  struct; `spawn_floating`'s Err set is unchanged).

## Risks & Open Questions

Ranked:

1. **Mutter's honoring of empty input regions is unverified** (survey
   binding). Tried: the survey and `linux.md` flag it; the protocol text
   makes it a compositor obligation but no Mutter runtime test exists in
   this repo's research. Consequence: click-through may silently no-op on
   GNOME Wayland. Mitigation: manual QA step 5 records the outcome; the
   capability row for passthrough stays protocol-level, and 25's toggle UI
   copy is the place to surface a Mutter caveat if QA finds one.
2. **zed#46378: startup output selection.** Displays are empty until a
   first frame, and layer output choice is creation-only; a cold-start
   targeted spawn lands `display_id: None` (compositor-chosen output).
   Tried: fetched the issue this session (open; linked fix unmerged).
   Mitigation: `spawn_floating_after_displays` implements the recorded
   100 ms deferred-open workaround; manual QA 7 exercises it. Residual:
   compositor may still place an untargeted surface arbitrarily; accepted
   and logged.
3. **Sibling plans 25/27 are not on disk.** Tried: `ls docs/plans/` this
   session: 01-24 only. This plan defines the items the feature spec says
   they consume (`set_input_passthrough` for 25's toggle; spec builder,
   `surface`, `spawn_floating_after_displays`, capability names for 27) and
   records them in 02's Amendment 1. If their actual contracts differ
   (e.g., 25 expects a per-window toggle object rather than a free
   function), the reconciliation edit is theirs; nothing in 26 blocks it
   because both call sites are additive.
4. **Error-chain downcast could break on a future gpui bump** if a
   `.context()` appears between `WaylandSurfaceState::new` and
   `App::open_window`. Tried: read every hop this session
   (`window.rs:844-852`, `client.rs:1333`, `app.rs:1526-1529`); no wrap
   today. Mitigation: `is_layer_shell_unsupported` also matches the Display
   text, so detection survives a context wrap.
5. **X11 silently downgrades a LayerShell request to a normal window**
   (no branch in the X11 backend; grep run this session). Mitigation: the
   session guard means the kind is never sent off Wayland; an X11 session
   logs the ignored option and uses 02's path. Residual: nothing, provided
   the guard stays.
6. **Layer is creation-only; no runtime z-order changes on Wayland**
   (compositor-owned; survey Part 3). Mitigation: respawn is the
   documented reposition/re-layer path; 25's "runtime topmost" concept
   degrades to respawn on Wayland and must not promise a toggle there.
7. **Exclusive-edge misuse is a protocol error.** The backend already
   logs-and-ignores invalid edges (`LINUX/wayland/window.rs:482-496`), and
   `wayland::plan` re-validates (single bit, contained in anchor) so the
   caller learns at spawn time. No fatal path reachable from this API.
8. **Test platform cannot exercise a real layer surface** (compositor ""
   → session guard declines). Tried: verified the test platform's
   `compositor_name` default and non-empty displays this session. Coverage
   answer: unit tests cover conversion/classification/degradation; real
   surface behavior is manual QA 1-7 on Wayland sessions. Known limit,
   same class as the repo's other GPU/platform tests (INDEX §8).

Open questions:

- 25/27 contract reconciliation (risk 3): what exact shape does 25's toggle
  surface take, and does 27 want the `FloatSurface` tag exposed through
  anything richer than `floating_snapshot`? Tried: plans absent from disk;
  nothing to read. Both are additive on 26's side; whoever lands second
  adjusts their call sites.
- Does any real-world compositor return a `compositor_name()` other than
  "Wayland"/"X11"/""? Tried: grepped both Linux backends; the strings are
  compile-time literals, so the equality checks are exhaustive for 0.3.9.
  Not an open risk; noted for the record.
- INDEX.md registration (§2 catalog, §3 batches, §7 tracking) deliberately
  not done by this plan: sibling plans 25/27 are being written in parallel
  and each would edit the same table rows, so the track owner adds all
  three at once. 26 never claims INDEX registration anywhere.
