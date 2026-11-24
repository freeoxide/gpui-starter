# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- `gpui-form` is re-pinned to the fork @`9239b1e`, the hmziqagent `chore/jiff-1.0` branch: the kit-0.11.0 tracking branch plus the fork's workspace `jiff` line moving 0.2 → 1.0. gpui-kit 0.11.0 takes jiff 1.0 civil types in its pickers, so the kit, the app and the fork's date picker now build one `jiff` (1.0.2), where the fork's date picker built a second, 0.2 copy with its own `Date` type. The fork needs no source changes; its workspace check, clippy with `-D warnings` and test suite pass on 1.0.2. `Cargo.lock` keeps a `jiff` 0.2 block: `serde_with` declares it as an optional dependency (feature `jiff_0_2`, still there in the latest 3.25.0) and nothing enables it, so it is listed but never built. `qa_migration` pins the new rev and fails if the form component lists a jiff 0.x dependency again.
- Upgraded the GPUI foundation to **gpui-kit 0.11.0** (from 0.10.0), which pins the `gpui-pre` snapshot family at **0.3.14** (0.3.12 before). The exact `wgpu`/`naga` =30.0.1 and wasm pins (`wasm-bindgen` =0.2.129, `web-sys`/`js-sys` =0.3.106) survive unchanged, `gpui-devtools` stays at 0.6.0, and `koruma` 0.9, the es-fluent set and rust-i18n 4 resolve as before; `qa_migration` invariants and the manifest/lock pins move to 0.11.0/0.3.14. `gpui-form` is re-pinned to the fork @`602d5d5`, the hmziqagent `chore/track-kit-0.11.0` tracking branch: workspace lines only plus the forms-demo example's `WindowBounds::centered` call, which 0.3.14 gives a display argument; the fork's workspace check and test suite pass on kit 0.11.0 (freeoxide `master` still requires kit `^0.8.1`). The root dev-dependency on `gpui` now inherits the workspace pin, as `gpui-auth`'s does. Source adaptations for the 0.3.14 APIs: (1) `Path` is no longer the tessellated, field-public `Path<Pixels>` but an opaque, non-generic `Path` that cannot be translated once built, so the canvas path cache stores each path at its paint origin and rebuilds it when the origin moves instead of shifting cached vertices. A pan therefore rebuilds the visible paths, and `pan_frames_reuse_every_paint_cache` becomes `pan_frames_rebuild_paths_but_nothing_else`: pans still rehash no content, reshape no labels and reproject no minimap anchors, and a repaint at an unchanged origin builds nothing. The 2048-point stroke chunking goes with it, its cause (gpui's u16 index buffers) being gone, so long strokes use the cache like every other shape; the theme brush pad keeps committed strokes as points and strokes them at the pad origin per paint, dropping its translated-path memo. (2) Every `Element` now has a blanket `IntoElement`, so the five identity impls on element types are deleted (`CanvasElement`, `WaveformElement`, `TourTarget`, and `BoundsProbe` in two test files). (3) The kit's calendar, date picker and time field take `jiff` civil types instead of `chrono`: the gallery pickers move to jiff (`ZonedDateTime::now()`, `Span` arithmetic, matchers over `jiff::civil::Date`) and the workspace `jiff` line, declared but unused until now, moves 0.2 → 1.0 to match the kit. The fork's own date picker follows onto jiff 1.0 with the `9239b1e` re-pin above, so one `jiff` is built. (4) `gpui::FillOptions` is no longer lyon's and has no `with_fill_rule`, so the tutorial spotlight mask states its even-odd rule as a struct literal; `rounded_rect_path` exposes its eight corner points (`rounded_rect_corners`) so the mask geometry test can still check them, a built `Path` no longer exposing vertices. (5) `TableState::set_selected_*` no longer scrolls, so the gallery data table's Go-to actions scroll explicitly with `scroll_to_row`/`scroll_to_col`, as upstream's own demo now does (`go_to_cell_scrolls_the_table_to_the_selected_cell` covers the action); the analytics page's reload re-selection keeps the viewport, which is the case that change was made for. Tests that read built-path geometry are replaced where an equivalent exists: the element test that checked zoom scaling through tessellated bounds becomes the unit test `stroke_points_scale_by_zoom_around_the_doc_bounds_origin` on the extracted `scaled_stroke_points`, and `idle_repaints_rebuild_nothing_and_pans_rebuild_paths_without_rehashing` pins the new cache behavior. Wall-clock, one Linux machine, debug build, per `window.draw`: a pan over 900 on-screen shapes falls from ~18–20 ms to ~7–8 ms, the shipped 5k-body fit view from ~56 ms to ~27–32 ms and the 20k preset from ~62–66 ms to ~34–42 ms. Version-stamped notes are re-verified on 0.3.14: the Wayland window-icon gap still holds (only the X11 backend reads the window icon), the kit's default icon set is identical through 0.11.0, and the gpui-agent note moves to the 0.3.14 pins with its patch trial re-run (a second hunk now fails in `src/window.rs`, where 0.3.14 inserted the `native_view` module inside the hunk's span).
- `gpui-auth`'s dev-dependency on `gpui` inherits the workspace `gpui-pre` pin (`{ workspace = true, features = ["test-support"] }`) instead of declaring its own `0.3.9` floor, so the crate cannot drift from the kit's snapshot at the next bump; `Cargo.lock` is unchanged. Notes stamped at older snapshots are re-verified on gpui-pre 0.3.12: the Wayland window-icon gap still holds (`src/app/assets.rs`, `docs/releasing.md`); the gpui-agent research note moves to the 0.3.12 pins with its patch trial re-run, where the same hunks fail but the wgpu failure is one changed line (`get_mapped_range()?`), not the ~11 it recorded; `docs/plans/INDEX.md` points plan executors at the 0.10.0/0.3.12 sources and its default-icon-set diff runs through 0.10.0 (still identical).
- Upgraded the GPUI foundation to **gpui-kit 0.10.0** (from 0.8.1, via 0.9.0/0.9.1), which pins the `gpui-pre` snapshot family at **0.3.12** (0.3.10 before); `gpui-devtools` moves 0.5.1 → 0.6.0 (targets `gpui-pre ^0.3.11`); `gpui-form` is re-pinned to the fork @`d4f703e`, the hmziqagent `chore/track-kit-0.10.0` branch ahead of the freeoxide merge (freeoxide `master` still requires kit `^0.8.1`, which cannot unify with the 0.10.0 line and would split the graph into two gpui copies; the branch bumps the fork's `gpui-kit`/`gpui-kit-assets` to 0.10.0 and `gpui`/`gpui_platform` to 0.3.12 with no source changes — the fork touches none of 0.9/0.10's breaking surface). The exact `wgpu`/`naga` =30.0.1 and wasm pins (`wasm-bindgen` =0.2.129, `web-sys`/`js-sys` =0.3.106) survive unchanged: the new snapshot family's carets accept them, so `wasm/build.sh`'s CLI check stays at 0.2.129. `qa_migration` invariants and the manifest/lock pins move to 0.10.0/0.3.12 with the new form rev (git source hmziqagent until the freeoxide merge). Source adaptations for the 0.9/0.10 APIs: the theme mapper learns kit 0.10's new `list.highlight.foreground` search-match color (flat `list_highlight_foreground` field → dotted canonical, or the gallery mapper test reports the key rejected), and `.cargo/config.toml` sets `RUST_MIN_STACK = 8388608` because kit 0.10's layout/paint recursion crosses the 2 MiB default test-thread stack on a debug-build first frame (the gallery `every_section_renders_a_frame` tests aborted with SIGABRT without it); that file leaves `.gitignore`, where it was reserved for machine-local MSVC env, so CI and fresh clones run the suite with the floor — machine-local toolchain env belongs in `$CARGO_HOME/config.toml`. The rest of the 0.9/0.10 breaking surface — `#[non_exhaustive]` theme structs, `Window::text_system()` returning `Rc`, `DisplayMap` losing `UnwindSafe`, `EditorState::text_gutter_margin` → `text_origin_offset`, `TextareaMode::Extras = TextareaExtras` — has zero call sites in the app

- Upgraded the GPUI foundation to **gpui-kit 0.8.1** (from 0.8.0), which pins the `gpui-pre` snapshot family at **0.3.10** (0.3.9 before); `gpui-form` is re-pinned to the freeoxide fork @`62a8b7d`, the kit-0.8.1 tracking merge (PR #14), where the form crates move 0.10.0 → 0.11.0 and the fork's `fluent_kv` attribute removal has no call site here; the exact `wgpu`/`naga` pins move =29.0.4 → =30.0.1 (one wgpu/naga copy with `gpui-pre-wgpu`), the wasm lane moves `wasm-bindgen` =0.2.129 with `web-sys`/`js-sys` =0.3.106 and `wasm/build.sh`'s CLI check follows; `qa_migration` invariants and the manifest/lock pins move to 0.8.1/0.3.10 with the new form rev. Source adaptations for the 0.3.10 gpui APIs: `Keystroke::parse`/`KeyBinding::new` now take static strings with explicit modifiers only, so the `secondary-` bindings and runtime-built keystrokes route through a `keys::binding`/`keys::label(os)` pair over `KeyBinding::load`/`parse_with_os` (app init, sidebar page bindings, title-bar a11y label, command-palette gallery, questionnaire shortcut renderer); `Modifiers::secondary` takes `&App` (canvas explore tool); `PlatformInput::ScrollWheel` → `Scroll` with `ScrollWheelEvent`/`on_scroll_wheel` renamed to `ScrollEvent`/`on_scroll` (scroll-bounce demo, canvas element, music page, query activity log); wgpu 30 engine deltas are the `RequestAdapterOptions.apply_limit_buckets` field, fallible `get_mapped_range`, and per-slot `Option<VertexBufferLayout>` pipeline buffers

- Upgraded the GPUI foundation to **gpui-kit 0.8.0** (from 0.7.3), which still pins the `gpui-pre` snapshot family at **0.3.9**; `gpui-form` is re-pinned to the freeoxide fork @`23efdef`, the fork's kit-0.8.0 bump merge, where the package is renamed `freeoxide-gpui-form` (lib name stays `gpui_form`), the eight form crates move 0.5.2 → 0.10.0, and `component-shape` leaves the graph; the lock still holds one kit family and one gpui snapshot. `qa_migration` invariants and the manifest/lock pins move to 0.8.0/0.3.9 with the new form package name
- The two exhaustive `SelectionFormat` matches (markdown editor, editor html_render) grow the new `SourceAcrossStyles` arm: label "Selection: Source Across Styles", toggle cycle Plain → Source → SourceAcrossStyles → Plain
- The motion scroll-bounce demo moves onto the 0.8.0 wrapper-owned wheel engine: the list pane demonstrates the engine and the `.scroll_wheel(false)` opt-out through a runtime wheel toggle, and the short-content pane pins child-owned wheel because zero-extent content cannot bounce under the wrapper; every production scroll surface keeps the platform default
- The remaining 0.8.0 breaking changes land without source changes: `ResizeHandle`, `TabVariant`, and `InvocationKind` have zero call sites; nothing calls `Easing::sample` or assumes a clamped sample (the motion demos clamp per segment, and the spinner's bounce overshoot is the showcased look); the kit's test helpers are consumed through the dev-dependency and never implemented here; the WebView rename, the gpui-fast backend, and the shell data-dir change have no surface in the app (the gallery's webview section renders a wry code sample as text, and no wry dependency exists); the `set_line_number`, `set_searchable`, and `set_selected_row` call sites keep their 0.7.3 behavior (the two toggles always pass a changed value so the kit's notify-on-change still fires, the one selection call runs from a reload timer outside input dispatch, and the kit's own row-click paths stop propagation); Home/End selecting the first/last row is the desired table behavior, and the editor-end no-chain change matches the bounded editors (the page scrolls once the pointer leaves them)

### Added

- Analytics table: double-click column fit (`TableDelegate::fit_column`), with fitted and dragged widths written back through `TableEvent::ColumnWidthsChanged` so they survive segment and range reloads (two tests pin the behavior)
- `ResizablePanel::fixed` keeps the dragged sidebar width across window resizes; all 16 scoped sliders carry `accessibility_label`, adding one new key pair (`music_seek`) to both catalogs; the messaging scroller demo reports the top row (`MessageScrollerState::logical_scroll_top`) and splices streamed rows away (`MessageScrollerState::splice`) instead of resetting, preserving the reading anchor
- WGPU embed page (plan 11): a `WgpuEngine` bridge (device, queue, blocking texture readback) plus particle, mesh, and post-process demos, recovered from the stashed plan-11 work; pause no longer lands a stale frame (test-pinned), and the error panel keeps the stale-frame drain running; wasm shows the capability-reason panel
- Canvas shaders page (plan 07): four WGSL fragment demos (gradient, raymarched SDF, image warp, mouse ripple) rendered through the plan-11 bridge onto the plan-01 canvas, each with a slider-driven uniform, play/pause, and an FPS overlay; wasm shows the capability-reason panel

### Changed

- Upgraded the GPUI foundation to **gpui-kit 0.7.3** (from 0.7.1), which pins the `gpui-pre` snapshot family at **0.3.9** (zed@93d7456); the app compiles and passes all gates with no source changes — 0.7.3's API surface is additive here (diff viewer, `InputState::add_selection`, dialog entrances, `TooltipDefaults`) and its behavior deltas are kit-internal defaults the suite absorbs (TextView paragraph gap 0.75rem, Markdown table column widths, `visible_rows_changed` firing `0..1`/`0..0` for one-row/empty tables). `qa_migration` invariants and the manifest/lock pins move to 0.7.3/0.3.9; the freeoxide `gpui-form` rev's caret requirements (`gpui-kit 0.7.0`, `gpui-pre 0.3.7`) unify on the same resolution, keeping the graph at one kit family and one gpui snapshot

### Added

- `docs/plans/`: showcase master plan — reviewed implementation plans for 20 removable example features (canvas engine, floating windows, notch floater, menu-bar charts, shaders, wgpu embed, multi-user canvas, tutorial mode, music player, image editor, system controls, microphone/audio waveform, GH Actions bundle fix) plus the INDEX with dependency batches, seam-collision map, icon policy, and full-showcase removal recipe; two research appendices under `_survey/` back every API citation

- `tests/qa_migration.rs` locks the migration's dependency-graph invariants: kit 0.7.0 + `gpui-pre` 0.3.7 manifest pins, exactly one gpui-kit family and one unified `gpui-pre` snapshot in the lockfile, the freeoxide `gpui-form` rev pin, single-copy `koruma` 0.9 / `es-fluent` 0.16 registry satellites, and the retired gpui-form `[patch]` table's absence — the merged rev resolves from git unpatched

### Changed

- Migrated the GPUI foundation to **gpui-kit 0.6.0** from crates.io, built on `gpui-pre` (the published snapshot of Zed's gpui) — the whole graph resolves to exactly one gpui copy and the zed git `[patch]` entries are gone
- Bumped the gpui-kit ecosystem (`gpui-kit`, `gpui-component`, `gpui-kit-assets`) to **0.6.1**; the pinned `gpui-form` rev's caret requirement unifies on the same checkout, and 0.6.1's slimmed optional deps drop syntect/jni/rustls-platform-verifier from the lockfile
- `gpui-form` is re-pinned to the freeoxide fork @`f7e2fb0b`, the `sync/gpui-kit-0.7.0` migration branch HEAD (no stayhydated rev targets kit 0.7); the run-1 vendoring is retired, and the temporary `[patch]` that resolved the then-unpushed rev from the local sibling checkout is deleted now that the rev is merged upstream — the git pin resolves as written. `koruma` 0.9 and `es-fluent` 0.16 follow the migrated graph's single-copy registry lines
- `gpui-query` resolves through a `[patch.crates-io]` override to our `gpui-pre-0.6` fork branch (a one-line manifest swap onto `gpui-pre`); no sources are vendored for this override
- Ported the form page to the gpui-form 0.5.2 API: `#[gpui_form(component(input))]` declarations, koruma 0.9 builder validators, `{field}_input` generated members, `InputEvent::Change` subscriptions; two adaptations are intentional — empty fields now fail the codegen-added required validator, and `website` is required-enforced at the holder level
- The i18n service is ported to es-fluent 0.16 (`localize` keeps its `&'static str` id and resolves through `localize_in_domain`): an invalid message id now falls back to the id string instead of failing the retired 0.18 static registry's eager key check
- Migrated the gpui-kit ecosystem (`gpui-kit`, `gpui-component`, `gpui-kit-assets`) 0.6.4 → **0.7.0**, with `gpui` + `gpui_platform` (`gpui-pre` family) 0.3.5 → **0.3.7**; kit 0.7 pins the whole `gpui-pre` snapshot family at `=0.3.7`
- The main window opens through `gpui_kit::open_window` after `gpui_kit::init`; the launcher keeps the manual `cx.open_window` + `Root::new().bg(transparent_black())` path, because the kit's root plugin paints an opaque background that only an instance style refinement overrides
- Deleted the manual sheet/dialog/notification layer rendering (`Root::render_*_layer` is gone in 0.7); the root plugin auto-hosts every overlay layer above app content
- Theme mutations go through `Theme::update` / `Theme::set_scrollbar_mode`, which reconcile tokens, rebuild the Base projection, and refresh every window — no `Theme::global_mut` mutation sites remain

## [0.3.0] - 2026-06-05

### Added

- Auto-updater with signed manifests and Ed25519 verification
- Crash report generation and storage for application panics
- Frame-time debugger for performance profiling
- Background WebSocket support (optional feature)
- gpui-query async data fetching library (TanStack Query-inspired)
- gpui-query-v2 next-gen async state management
- Vendored gpui-component library from longbridge
- Vendored gpui-form for declarative form handling
- Error boundary with action-based triggers
- Release workflow with codesigning and notarization
- Deploy workflow for Cloudflare Pages

### Changed

- 60 new blog posts covering GPUI development topics
- Improved documentation coverage across all pages
- Updated FAQ entries with better SEO targeting
- Enhanced structured data and meta optimization
- Internal linking improvements across all pages

## [0.2.0] - 2025-06-15

### Added

- Desktop notification system with native OS backends, in-app toast fallback, and persistent inbox
- Diagnostics page showing live app state, subsystem status, and debug actions
- Telemetry module with disabled/local/remote modes and consent gate
- Secure storage module with OS keyring integration (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- Single-instance guard with IPC forwarding for deep links
- First-run detection and setup experience
- Undo/redo stack with command pattern and keyboard shortcuts (Cmd+Z/Cmd+Y)
- Custom title bar replacing native window chrome with drag regions and traffic light support
- Global keyboard shortcuts (Alt+Space hotkey, Cmd+K launcher)
- Background task manager for async operations
- Connectivity state monitoring with network probes
- File logging with tracing-appender
- Capabilities registry for runtime feature detection
- Lifecycle state machine for app startup/shutdown/crash handling
- Configuration migration system for schema changes

### Changed

- Expanded documentation site with 5 new reference pages (command launcher, notifications, secure storage, routing, testing)
- Added 12 new blog posts covering GPUI development topics
- Added 8 new FAQ entries across Features and Advanced categories
- Updated llms.txt with comprehensive feature coverage

## [0.1.0] - 2025-05-15

### Added

- **Multi-page architecture** with sidebar navigation and page routing via GPUI
- **21 built-in themes** with live hot-reloading and custom theme support
- **Internationalization (i18n)** supporting English and Chinese (zh-CN) via `es-fluent`
- **Form validation** with `gpui-form` derive macros and `koruma` validation rules
- **Command launcher** (Cmd+K) with fuzzy search across all app actions
- **macOS system tray** integration with app icon and quick-access menu
- **Secure credential storage** via OS keychain (`keyring` crate)
- **SQLite database** integration with `rusqlite` for local data persistence
- **Animated app preview** component with Three.js wireframe scenes
- **Documentation site** built with Astro Starlight
- **Custom marketing landing page** with 3D animations and glassmorphism design
- **Privacy policy** and **terms of use** pages

### Changed

- Moved web sources from `src/web` to `web` directory for cleaner project structure
- Enabled `apple-native` feature for keyring on macOS
- Switched to blocking `reqwest` for connectivity checks

### Fixed

- Animation now pauses on hover for better UX
- Added logging to secure storage operations for debugging
