# Ecosystem survey: verified API facts for the feature plans

Inputs for the 20 planned features (canvas, shaders, floating windows, notch, wgpu,
socket.io, charts, media, audio io, system controls, release bundling). Plans 01–11
exist on disk (`ls docs/plans/*.md`); 12–20 do not yet (see Open Questions). Every
gpui/gpui-kit claim below was verified against the crate sources on disk this session;
registry crates were located with `ls ~/.cargo/registry/src/*/ | grep -E 'gpui-pre|gpui-kit'`
and read directly. External crate versions come from the crates.io API
(`curl -A "gpui-starter-survey" https://crates.io/api/v1/crates/<name>`) and docs.rs,
fetched 2026-10-12. Anything not verifiable is listed under Open Questions, not guessed.

## Path key

| Short | Path |
| --- | --- |
| `GPUI` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-0.3.8/src` |
| `MAC` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-macos-0.3.8/src` |
| `APPLE` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-apple-0.3.8/src` |
| `WGPU` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-wgpu-0.3.8/src` |
| `GC` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-component-0.7.1/src` |
| `GB` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-base-0.7.1/src` |
| `KIT` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-kit-0.7.1/src` |
| `REPO` | `/home/thragg/fo/gpui-starter` |

The lib name of `gpui-pre` is `gpui`; the application imports it through `gpui_kit::*`
(`KIT/lib.rs:97` re-exports `::gpui::*`). `component` is `gpui-component` 0.7.1
(`KIT/lib.rs:138`), `base` is `gpui-base` 0.7.1 (`KIT/lib.rs:108`).

## a) Custom Element

Element trait (`GPUI/element.rs:53-113`):

| Method | Signature (abbreviated) |
| --- | --- |
| `request_layout` | `fn request_layout(&mut self, id: Option<&GlobalElementId>, inspector_id: Option<&InspectorElementId>, window: &mut Window, cx: &mut App) -> (LayoutId, Self::RequestLayoutState)` (`GPUI/element.rs:75-81`) |
| `prepaint` | `fn prepaint(&mut self, id, inspector_id, bounds: Bounds<Pixels>, request_layout: &mut Self::RequestLayoutState, window: &mut Window, cx: &mut App) -> Self::PrepaintState` (`GPUI/element.rs:85-93`) |
| `paint` | `fn paint(&mut self, id, inspector_id, bounds: Bounds<Pixels>, request_layout: &mut Self::RequestLayoutState, prepaint: &mut Self::PrepaintState, window: &mut Window, cx: &mut App)` (`GPUI/element.rs:97-106`) |

Associated types `RequestLayoutState`/`PrepaintState: 'static` are declared by the
implementor (`GPUI/element.rs:58-66`). `id() -> Option<ElementId>` and
`source_location()` sit above them (`:68-73`). ManagedElement offers a state-carrying
shortcut: `request_layout(&mut Window, &mut App) -> LayoutId`, `prepaint(...) ->
Option<FocusHandle>`, `paint(...)` (`GPUI/element.rs:610-642`).

Shortcut for one-off drawing: `canvas(prepaint, paint)` builds a `Canvas<T>` element;
the paint closure gets `(Bounds<Pixels>, T, &mut Window, &mut App)`
(`GPUI/elements/canvas.rs:10-22`).

Paint primitives on `Window` (paint phase only, enforced by `debug_assert_paint`,
`GPUI/window.rs:282-287`):

| Primitive | Where | Notes |
| --- | --- | --- |
| `window.paint_quad(quad: PaintQuad)` | `GPUI/window.rs:4550` | Fills, borders, corner radii. Free fn `quad(...)` at `:7675`, `fill(bounds, background) -> PaintQuad` at `:7694` |
| `window.paint_path(path: Path<Pixels>, color: impl Into<Background>)` | `GPUI/window.rs:4621` | Stroked/filled paths; build with `PathBuilder` |
| `PathBuilder` | `GPUI/path_builder.rs:25` | `move_to:125`, `line_to:131`, `curve_to:137` (quadratic), `cubic_bezier_to:144`, `arc_to:155`, `relative_arc_to:172`, `add_polygon:189`, `close:199`, `rotate:233`, `build() -> Result<Path<Pixels>>:244`; style via `stroke(width):88`, `fill():96`, `dash_array:108` |
| `window.paint_image(bounds, image_bounds, corner_radii, data: Arc<RenderImage>, frame_index, grayscale) -> Result<()>` | `GPUI/window.rs:4919` | Raster images; atlas-cached, see (e) |
| `window.paint_glyph(origin, FontId, GlyphId, font_size, Hsla)` / `paint_emoji(...)` | `GPUI/window.rs:4687` / `:4792` | Low-level glyph runs |
| `window.paint_svg(...)` | `GPUI/window.rs:4849` | SVG sprite |
| Text | `GPUI/text_system.rs:638-644` | `window.text_system().shape_line(text: SharedString, font_size, runs: &[TextRun], force_width: Option<Pixels>) -> ShapedLine`; paint with `ShapedLine::paint(origin, line_height, align: TextAlign, align_width: Option<Pixels>, window, cx) -> Result<()>` (`GPUI/text_system/line.rs:108-116`). No `paint_text` method on `Window` in this snapshot |
| `window.paint_layer(bounds, f)` | `GPUI/window.rs:4399` | Nested layer with own z-order |
| `window.paint_drop_shadows` / `paint_inset_shadows` | `GPUI/window.rs:4424` / `:4460` | Shadow primitives |
| `window.paint_surface(bounds, CVPixelBuffer)` | `GPUI/window.rs:5021` | macOS/iOS only (`#[cfg]` directly above at `:5020`) |

Hitbox and mouse events inside elements:

- Hitboxes are inserted in **prepaint**: `window.insert_hitbox(bounds, HitboxBehavior) -> Hitbox` (`GPUI/window.rs:5157`, pushes to `next_frame.hitboxes` at `:5167`). `HitboxBehavior::{Normal, BlockMouse, BlockMouseExceptScroll}` (`GPUI/window.rs:888-902`).
- Queries on the returned `Hitbox`: `is_hovered(&self, window)` (`GPUI/window.rs:860`), `should_handle_scroll(window)` (`:881`).
- Mouse listeners are registered in **paint**: `window.on_mouse_event::<E: MouseEvent>(|&E, DispatchPhase, &mut Window, &mut App|)` (`GPUI/window.rs:5294-5305`); event type inferred from the listener's first parameter. `on_key_event` follows the same pattern (`:5317`).

Draw-phase / stale-frame semantics:

- `DrawPhase::{None, Prepaint, Paint, Focus}` is `pub(crate)` (`GPUI/window.rs:1309-1314`). Paint APIs assert `Paint`; layout/hitbox APIs assert `Prepaint` (`GPUI/window.rs:282-295`).
- Everything painted, every hitbox, and every mouse/key listener goes into the frame's `next_frame` buffers and is cleared when that frame renders — the doc on `on_key_event` states "When the next frame is rendered the listener will be cleared" (`GPUI/window.rs:5312-5316`). Elements must re-register listeners and hitboxes every frame; that is the intended pattern, not a leak.

## b) Windows

`open_window` (`GPUI/app.rs:1431`):
`App::open_window<V: Render>(options: WindowOptions, build_root_view: impl FnOnce(&mut Window, &mut App) -> Entity<V>) -> anyhow::Result<WindowHandle<V>>`. It draws the window once before returning (`window.draw(cx)` at `:1455`).

`WindowOptions` — full field list (`GPUI/platform.rs:2438-2503`; defaults at `:2609-2636`):
`window_bounds: Option<WindowBounds>`, `titlebar: Option<TitlebarOptions>`, `focus: bool` (default true), `show: bool` (true), `kind: WindowKind` (`Normal`), `is_movable: bool` (true), `app_owns_titlebar_drag: bool` (false, macOS-only effect, `:2469-2477`), `inactive_frame_interval: Option<Duration>` (default `Some(33_333 µs)`, `:2479-2481`), `is_resizable: bool` (true), `is_minimizable: bool` (true), `display_id: Option<DisplayId>`, `window_background: WindowBackgroundAppearance`, `app_id: Option<String>`, `window_min_size: Option<Size<Pixels>>`, `window_decorations: Option<WindowDecorations>` (X11/Wayland), `icon: Option<Arc<image::RgbaImage>>` (X11), `tabbing_identifier: Option<String>` (macOS native tabs).

`TitlebarOptions`: `title: Option<SharedString>`, `appears_transparent: bool`, `traffic_light_position: Option<Point<Pixels>>` (`GPUI/platform.rs:2641-2649`).

`WindowKind` variants (`GPUI/platform.rs:2653-2683`): `Normal`, `PopUp` (above all windows), `AnchoredPopup(popup::PopupOptions)` (parent-anchored native popup; size from `window_bounds`, origin ignored; rejected with `popup::PopupNotSupportedError` where unsupported), `Floating` (above parent), `LayerShell(LayerShellOptions)` (Linux + `wayland` feature only), `Dialog` (blocks parent).

Window level control: there is no public `set_level` API. Level is fixed per `WindowKind` at creation — macOS mapping: `Floating -> NSFloatingWindowLevel` (+ `setAcceptsMouseMovedEvents(YES)`), `Normal -> NSNormalWindowLevel`, `PopUp/AnchoredPopup -> NSPopUpWindowLevel` + `NSPanel` with `NSWindowStyleMaskNonactivatingPanel` + `CanJoinAllSpaces | FullScreenAuxiliary` + utility-window animation (`MAC/window.rs:1208-1240`, panel classes at `:1014-1023`). `Dialog` opens via `beginSheet:` on the main window (`MAC/window.rs:1241-1258`).

`PopupOptions` (`GPUI/platform/popup.rs:16-36`): `parent: AnyWindowHandle`, `anchor_rect: Bounds<Pixels>` (parent-window space), `anchor: PopupAnchor`, `gravity: PopupGravity`, `constraint_adjustment: PopupConstraintAdjustment`, plus offset fields.

Lifecycle and control:

| API | Where |
| --- | --- |
| `window.activate_window()` / `request_attention()` / `minimize_window()` | `GPUI/window.rs:6487`, `:6492`, `:6497` |
| `window.toggle_fullscreen()` / `toggle_simple_fullscreen()` | `GPUI/window.rs:6502` / `:6510` — simple fullscreen covers menu bar and notch area (doc at `:6505-6510`), macOS only |
| `window.on_window_should_close(cx, f) -> bool` — return false vetoes close | `GPUI/window.rs:6732-6741` |
| `cx.on_window_closed(handler)` — app-level close observation | `GPUI/app.rs:2631` |
| `cx.windows() -> Vec<AnyWindowHandle>`, `cx.active_window()`, `cx.window_stack()` | `GPUI/app.rs:1407`, `:1424`, `:1419` |
| `cx.refresh_windows()` — redraw all | `GPUI/app.rs:1237` |
| `window.set_window_title(&str)` (`:2924`), `resize(Size)` (`:2774`), `window_bounds()` (`:2464`), `inner_window_bounds()` (`:2469`) | `GPUI/window.rs` |
| Per-window rendering: each `Window` owns its own frame buffers (`next_frame.scene`, `next_frame.hitboxes` — `GPUI/window.rs:4566`, `:5167`); `request_animation_frame` drives that window only | see (d) |
| `window.observe_window_visibility/appearance` | `GPUI/window.rs:2238`, `:2254` |

Multiple live windows: supported; `open_window` returns a distinct `WindowHandle<V>` per
call and `cx.windows()` enumerates them (`GPUI/app.rs:1407`). Focus on open is
controlled by `WindowOptions::focus`.

## c) Shaders

**No public runtime shader API in this snapshot.** Evidence:

- Crate root (`GPUI/gpui.rs`) exports no shader-related item (grep `shader` → no hits); grep across `GPUI/src` hits only comments (`GPUI/scene.rs:28`, `GPUI/window.rs:4479`).
- Shaders are internal, compile-time: Metal source `APPLE/shaders.metal`; WGSL sources `WGPU/shaders_storage.wgsl` (plus siblings), compiled by the built-in renderers (`WGPU/wgpu_renderer.rs` via `device.create_shader_module`).
- The `runtime_shaders` cargo feature (enabled on the platform stack by gpui-kit — `KIT/../Cargo.toml:400-410` lists it under the `gpui-pre-platform` dependency features; chain: `gpui-pre-platform-0.3.8/Cargo.toml:43` → `gpui-pre-macos-0.3.8/Cargo.toml:41` → `gpui-pre-apple-0.3.8/Cargo.toml:37`, an empty feature `[]`) only switches the **built-in** Metal library from a precompiled `.metallib` to `new_library_with_source` of the same built-in source at startup (`APPLE/metal_renderer.rs:30-34`, `:250-260`). It is not a user-shader hook: no registration, no uniforms, no paint call.

Consequence for any shader plan: custom GPU effects must go through an external wgpu
device rendering offscreen and blitted into gpui (see (g) for the handle problem), or
CPU-side paint primitives. There is no in-scene shader primitive.

## d) Animation / frame loop

| API | Where | Notes |
| --- | --- | --- |
| `window.request_animation_frame()` | `GPUI/window.rs:2670-2674` | Schedules a redraw next frame; notifies current view if called from one |
| `window.on_next_frame(callback)` | `GPUI/window.rs:2650` | Runs directly after current frame renders |
| `AnimationExt` trait | `GPUI/elements/animation.rs:82` | `with_animation` (`:84`), `with_animations` (`:102`) |
| `cx.background_executor().timer(Duration) -> Task<()>` | `GPUI/executor.rs:183` | Async timer for playback ticks / polling intervals |
| `cx.background_executor().now() -> Instant` | `GPUI/executor.rs:175` | Clock source |
| `window.spawn(cx, f) -> Task<R>` | `GPUI/window.rs:2693` | Window-context async spawn |
| Tests: `window.simulate_next_frame(cx)` | `GPUI/window.rs:2680-2690` (test-support) | Frame loop simulation |

Inactive-window throttling: `WindowOptions::inactive_frame_interval` (default 33 333 µs,
`GPUI/platform.rs:2479-2481`).

## e) Images

CPU-uploaded RGBA buffer → screen, exact path:

1. `RenderImage::new(data: impl Into<SmallVec<[Frame; 1]>>)` (`GPUI/assets.rs:61-70`), where `Frame` is `image::Frame` (`GPUI/assets.rs:4`). One frame per animation step; `RenderImage` doc states data is **BGRA format** (`GPUI/assets.rs:41-42`).
2. Wrap: `Arc<RenderImage>` converts into `ImageSource::Render` (`GPUI/elements/img.rs:107-110`); `ImageSource` variants are `Resource | Render(Arc<RenderImage>) | Image(Arc<Image>) | Custom(fn)` (`GPUI/elements/img.rs:42-51`).
3. Element: `img(source: impl Into<ImageSource>) -> Img` (`GPUI/elements/img.rs:201`), or paint directly with `window.paint_image(bounds, image_bounds, corner_radii, data, frame_index, grayscale)` (`GPUI/window.rs:4919-4927`). `paint_image` tiles through the sprite atlas (`get_or_insert_with`, `:4948`), so repeated painting of the same `RenderImage` hits a cache keyed by `(ImageId, frame_index)` (`RenderImageParams`, `GPUI/assets.rs:37-41`).
4. Per-frame API on `RenderImage`: `as_bytes(frame_index)` (`GPUI/assets.rs:74`), `size(frame_index)` (`:81`), `delay(frame_index)` (`:98`), `frame_count()` (`:106`).

RGBA→BGRA is manual: `image` 0.25.10 has no `to_bgra8` on `DynamicImage` (docs.rs
checked 2026-10-12), so swap channels in a copy before building the `Frame`.

`img()` styling: `StyledImage` trait (`GPUI/elements/img.rs:148`) gives `grayscale`,
`object_fit`, `with_fallback(fn -> AnyElement)`, `with_loading(fn -> AnyElement)`.

Repo usage (`REPO/src/gallery/sections/media/image.rs`): only URI sources —
`img(REMOTE_SVG)` with `.with_loading`/`.with_fallback` (`image.rs:58-74`) and an
embedded SVG (`:48`). The repo does not construct a `RenderImage` anywhere
(`grep -rn RenderImage src/` → no hits); there is no in-repo precedent for CPU-uploaded
buffers.

## f) gpui-kit 0.7.1

Charts (`GC/chart/`, re-exported via `gpui_kit::component::chart`):

- Chart types: `LineChart`, `AreaChart`, `BarChart`, `CandlestickChart`, `PieChart`, `RadarChart`+`RadarLabel`, `SankeyChart`+`SankeyLabel` (`GC/chart/mod.rs:9-15`).
- `LineChart<T, X, Y>` with `X: PartialEq + Into<SharedString>`, `Y: PlotValue` (`GC/chart/line_chart.rs:25-46`). Builder methods (`GC/chart/line_chart.rs:57-335`): `new(impl IntoIterator<Item = T>)`, `x(fn &T -> X)`, `y(fn &T -> Y)`, `stroke`, curve (`natural`/`linear`/`step_after`), `dot`, `grid(bool)`, `y_domain(min, max)`, `x_axis(bool)`, `y_axis(bool)`, `y_tick_count`, `y_tick_format` (`:285`), `x_tick_count`, `tick_margin`, `point_count`, `grid_columns`, `grid_dashed`, `reference_line(value)` (`:324`), `y_padding(top, bottom)` (`:335`), `interactive(bool)`, `id`, `appear(bool)`, `appear_key`, `name`, `tooltip_title`, `tooltip_value`, `tooltip_value_color`, `tooltip_content`. Generic data accessors mean no fixed data point type — any struct works. Appear-in animation and hover/tooltip are built in (`ChartAppear`, `GC/chart/mod.rs:45-49`).
- Underlying plot layer in `GB/plot/`: `Plot` trait (`GB/plot/mod.rs:107`), `Curve` enum (`:230`), `PlotValue`, `ScaleLinear`/`ScaleBand`/`ScalePoint`, hover/tooltip machinery.
- Repo precedent: `REPO/src/gallery/sections/charts/system_monitor.rs` drives `AreaChart` + `DataTable` off a 500 ms fixture loop (`system_monitor.rs:1-3`, `:26-30`).

Popover/overlay (`GC/popover.rs`): `Popover::new(id) :136`, `anchor(Anchor) :162`,
`offset(Pixels) :169`, `arrow(bool) :177`, `mouse_button :183`, `trigger(element) :189`,
`default_open(bool) :205`, `open(bool) :215`, `on_open_change(cb) :225`,
`trigger_style :238`, `overlay_closable(bool) :244`, `content(closure) :253`,
`appearance(bool) :270`, `track_focus(&FocusHandle) :279`. `Anchor` is the gpui enum
(`GPUI/geometry.rs:2168-2185`, eight corner/edge points). Related overlays:
`GC/hover_card.rs`, `GC/sheet.rs`, `GC/dialog/`, the notification layer on the app root
(`GC/root.rs:29`, `push_notification` at `:371`), and `gpui_base::Popover`/`Positioner`
(`GC/popover.rs:18-19`, `:31-36`).

Drag and drop:

- **No dnd/reorder component.** No `dnd` module in `gpui-component` or `gpui-base` (find for `*dnd*` → no files; grep `draggable|drag.and.drop` hits only dock/table internals: `GC/table/data_table.rs`, `GC/dock/`, `GC/searchable_list/delegate.rs`). Table columns have sorting (`Column::sortable`, `GC/table/column.rs:107`), not row reordering.
- Primitives exist in gpui-pre on divs: `on_drag<T, W>(value, constructor)` (`GPUI/elements/div.rs:615`), `on_drag_move<T>` (`:360`), `on_drop<T>` (`:568`), `external_drag_payload` for OS-level drags (`:641`). Drag preview is an arbitrary `Entity<W: Render>`.
- The dock has its own drag machinery (`GB/dock/drag.rs`) — dock-internal, not a general reorder API.

Verdict: list/tab reorder must be built on `on_drag`/`on_drag_move`/`on_drop` directly.

## g) Platform / macOS interop

- **No public NSWindow accessor.** `Window::platform_window` is `pub(crate)`; grep for `ns_window` across `GPUI/`, `MAC/` finds no public getter. Any AppKit reach-in must go through `objc2-app-kit` directly (e.g. `NSApplication::sharedApplication` and walking `windows`), not through gpui.
- `PlatformWindow: HasWindowHandle + HasDisplayHandle` (raw-window-handle) (`GPUI/platform.rs:1144`) and `impl HasWindowHandle for Window` exists (`GPUI/window.rs:7434-7438`) — but `Window::platform_window` being `pub(crate)` means user code cannot reach the trait object. A wgpu plan renders on its own offscreen device and blits via `RenderImage`/`paint_image`, or renders in a separate native window created outside gpui.
- Safe area: `WindowInsets { safe_area: Edges<Pixels>, ime: Edges<Pixels> }` with `effective()` (`GPUI/platform.rs:1104-1126`), consumed by `window.fully_visible_bounds()` (`GPUI/window.rs:2827-2834`). The doc attributes safe-area to iOS/Android; the macOS crate has no override (`grep "fn insets" MAC/` → no hits). **macOS notch/menu-bar geometry is not provided**; notch plans must query `NSScreen` via objc2-app-kit themselves.
- macOS native bits that do exist: `App::set_menus(impl IntoIterator<Item = Menu>)` (`GPUI/app.rs:2668`), `window.show_window_menu(position)` (`GPUI/window.rs:2880`), native tabs via `tabbing_identifier` (`MAC/window.rs:1225-1230`), `toggle_simple_fullscreen` covering menu bar + notch (`GPUI/window.rs:6505-6512`).
- objc2 versions in-tree: lockfile carries `objc2 0.5.2` and `0.6.4`, `objc2-app-kit 0.3.2`, `objc2-core-audio 0.3.2` (`grep REPO/Cargo.lock`). gpui-pre-macos builds against the 0.6 line. An app-side interop crate should match 0.6/0.3.x to avoid two runtime binding sets.

## h) Audio + system control

cpal 0.18.2 (latest; crates.io API, updated 2026-08-16). All from docs.rs
`docs.rs/cpal/0.18.2/cpal/` fetched 2026-10-12:

- Enumeration: `HostTrait::devices(&self) -> Result<Self::Devices, Error>` where `Devices: Iterator<Item = Self::Device>`; `default_input_device()/default_output_device() -> Option<Device>`; provided `input_devices()/output_devices()` filter iterators. Trait is not dyn-compatible.
- Device identity: `DeviceTrait::description() -> Result<DeviceDescription, Error>`; `DeviceDescription::name() -> &str` (always present), plus `manufacturer()`, `driver()`, `device_type()`, `interface_type()`, `direction()`, `address()`, `extended()` (`docs.rs/cpal/0.18.2/cpal/device_description/struct.DeviceDescription.html`). 0.15's `Device::name()` is gone — name now lives on the description.
- Config: `DeviceTrait::supported_input_configs()/supported_output_configs() -> Result<impl Iterator<Item = SupportedStreamConfigRange>, Error>`; `default_input_config()/default_output_config() -> Result<SupportedStreamConfig, Error>`.
- Streams (typed): `build_input_stream<T, D, E>(&self, config: StreamConfig, data_callback: D, error_callback: E, timeout: Option<Duration>) -> Result<Self::Stream, Error>` where `T: SizedSample`, `D: FnMut(&[T], &InputCallbackInfo) + Send + 'static`, `E: FnMut(Error) + Send + 'static`. Output variant takes `D: FnMut(&mut [T], &OutputCallbackInfo)`. `_raw` variants take `&Data`/`&mut Data` plus a runtime `SampleFormat`. `StreamConfig { channels: ChannelCount, sample_rate: SampleRate, buffer_size: BufferSize }` (docs.rs struct page).
- **No per-device volume/gain API** in cpal (DeviceTrait docs list only `Stream::play()/pause()` as control). Gain must be applied in the data callback; system volume control is outside cpal on every host.
- wasm: cpal does target wasm — `wasm-bindgen` feature provides a Web Audio API backend, optional `audioworklet` backend needs nightly + atomics (README at `raw.githubusercontent.com/RustAudio/cpal/master/README.md`, fetched 2026-10-12).

FFT for spectrum: `rustfft` 6.4.1 (updated 2025-09-18) — `FftPlanner::new()`,
`plan_fft_forward(n)`, in-place `fft.process(&mut buffer)` on `Complex<f32>`
(re-exported `num_complex`), unnormalized output; AVX/FMA, SSE4.1, NEON auto-selected,
`wasm_simd` feature off by default (docs.rs, 2026-10-12). `realfft` 3.5.0 wraps it for
real-valued input. Note: symphonia already pulls `rustfft` for its SIMD filters —
version dedupe should be checked at add time.

Battery: `starship-battery` 0.12.0 (updated 2026-09-27, license ISC) — `Manager`,
`Batteries` iterator, `Battery` instant readings, SI units via `uom`; Linux 2.6.39+,
macOS 10.10+, Windows 7+, BSDs (docs.rs index, 2026-10-12). The repo's charts story
currently uses fixture data where battery would go
(`REPO/src/gallery/sections/charts/system_monitor.rs:1-3`).

Brightness: `brightness` 0.8.0 (updated 2025-09-02, license 0BSD) — async `Brightness`
trait (`get`/`set`/`device_name`), `brightness_devices()` stream, `blocking` module.
Linux (sysfs + logind D-Bus via zbus, systemd 243+; external displays need the
`ddcci-backlight` kernel driver) and Windows via the `windows` crate; **the docs.rs
platform overview names Linux and Windows only — no macOS backend documented**
(docs.rs/brightness/0.8.0, 2026-10-12). On macOS the platform path is CoreGraphics
display APIs through objc2-core-graphics; nothing verified beyond existence of that
crate in the graph.

System volume — no cross-platform crate. Per-platform options, none compile-tested here:

| Platform | Option | State |
| --- | --- | --- |
| macOS | `objc2-core-audio` 0.3.2 (already in `REPO/Cargo.lock`) — CoreAudio HAL default-device volume | Raw FFI; exact property selectors not verified this session |
| Linux | `libpulse-binding` 2.30.1 (PulseAudio/PipeWire native protocol), or `zbus` 5.16.0 (already in lock) speaking D-Bus to PulseAudio/WirePlumber | zbus already a transitive dep; adding pulse introduces a C library link |
| Linux (ALSA-only) | `alsa` 0.12.1 | C library link, no PulseAudio systems |
| Windows | `windows` crate (0.57/0.58/0.61 already in lock) — `IAudioEndpointVolume` MMDevice API | Feature-gated COM interfaces; not verified here |

Bluetooth — no cross-platform crate:

| Platform | Option | State |
| --- | --- | --- |
| macOS | `objc2-io-bluetooth` 0.3.2 (updated 2025-10-04, crates.io) | Raw framework bindings |
| Linux | `zbus` 5.16.0 → `org.bluez` D-Bus | zbus already in lock; BlueZ object model is undocumented here |
| Windows | No verified Rust crate for classic Bluetooth device control | NOT FOUND |

Honest summary: battery is the only system control with a maintained cross-platform
crate. Brightness is Linux+Windows. Volume and Bluetooth have no single crate and need
per-platform FFI/D-Bus code.

## Step 3: external crates (checked 2026-10-12 via crates.io API + docs.rs)

| Crate | Latest | Fit | Risk | License |
| --- | --- | --- | --- | --- |
| `rust_socketio` | 0.6.0 (2024-04-16) | Socket.IO client; async behind `async` feature (docs label it beta) | Stale ~2.5 yrs; `rust_engineio` → tokio-tungstenite + native-tls, no rustls option; no wasm mention in docs | MIT |
| `socketioxide` | 0.18.7 (2026-08-24) | Socket.IO server as Tower service; pairs with axum/hyper | Server-side only; pre-1.0 API churn | MIT |
| `wgpu` | 30.0.1 (2026-08-22) | Offscreen headless render + readback (`map_async` + `Device::poll`; `PollStatus`/`PollError` types confirmed on docs.rs) | gpui-pre-wgpu 0.3.8 pins **wgpu 29.0.4** (`REPO/Cargo.lock`); a user wgpu 30 instance coexists but shares nothing with gpui's renderer | MIT OR Apache-2.0 |
| `image` | 0.25.10 (2026-03-10) | Decode/encode; `image::Frame` feeds `RenderImage` directly; already in lock | No BGRA conversion helper — channel swap is manual | MIT OR Apache-2.0 |
| `rodio` | 0.23.0 (2026-10-11) | Audio playback on cpal; symphonia default decoder (flac, mp3, mp4, vorbis, wav); microphone input module behind a feature | 0.23 is a recent API rework — `OutputStream::try_default` is gone, replaced by `DeviceSinkBuilder::open_default_sink` / `Player`; no workspace precedent | MIT OR Apache-2.0 |
| `cpal` | 0.18.2 (2026-08-16) | Device enumeration + input/output streams, see (h) | 0.15→0.18 renamed device APIs (`description()`); no volume control; wasm behind `wasm-bindgen` feature | Apache-2.0 |
| `hound` | 3.5.1 (2023-09-25) | WAV read/write (`WavReader::open`, `.samples::<i16>()`, `WavWriter::create` + `WavSpec { channels, sample_rate, bits_per_sample, sample_format }`) | Stable but slow-moving; integer/float sample format handling is manual | Apache-2.0 |
| `symphonia` | 0.6.1 (2026-08-13) | Decode/demux: default = WAV/OGG/MKV + FLAC/PCM/Vorbis/ADPCM; opt-in mp3, aac, alac, mp4 | **MPL-2.0** — file-level copyleft, flag for the license audit; Opus not supported | MPL-2.0 |
| `rustfft` | 6.4.1 (2025-09-18) | Spectrum FFT, SIMD auto-select, planner API | None notable; unnormalized output | MIT OR Apache-2.0 |
| `sysinfo` | 0.39.6 (2026-07-09) | CPU/memory/process metrics: `System::new_all`, `refresh_cpu_usage`, `used_memory/total_memory`, `cpus()` | MSRV 1.95 (repo toolchain rustc 1.98.0, `rustc --version` this session — OK); breaking changes between minors; lock already carries 0.31.4 transitively via zed-scap, so two versions would coexist | MIT |
| `objc2` / `objc2-app-kit` | 0.6.5 / 0.3.2 | AppKit interop (NSWindow level, NSScreen notch geometry, NSMenu) | Match gpui's 0.6/0.3.2 to avoid duplicate bindings (lock has both objc2 0.5.2 and 0.6.4 already); unsafe FFI discipline | objc2: MIT; objc2-app-kit: Zlib OR Apache-2.0 OR MIT |
| `starship-battery` | 0.12.0 (2026-09-27) | Battery read cross-platform, see (h) | ISC — nonstandard but permissive | ISC |
| `brightness` | 0.8.0 (2025-09-02) | Brightness get/set Linux+Windows | **No macOS backend**; DDC/CI needs kernel driver; 0BSD | 0BSD |
| `alsa` / `libpulse-binding` / `objc2-io-bluetooth` | 0.12.1 / 2.30.1 / 0.3.2 | Linux/macOS volume + bluetooth building blocks | Each covers one platform; no cross-platform story | Alsa: Apache-2.0/MIT; libpulse: Apache-2.0/MIT; objc2-io-bluetooth: Zlib OR Apache-2.0 OR MIT |

## Release bundling: what already exists on disk

No new crate needed for bundling; the repo ships its own pipeline (verified by `ls`):

- `REPO/packaging/`: `appimage/`, `flatpak/`, `windows/` asset dirs.
- `REPO/scripts/`: `build-appimage.sh`, `build-deb.sh`, `build-flatpak.sh`, `build-rpm.sh`, `build-windows-installer.sh`, `build-windows-portable.sh`, `create-dmg.sh`, `gen-pkgbuild.sh`, `generate-manifest.sh`, `normalize-version.sh`.
- `REPO/Cargo.toml` embeds deb (`target/deb-stage/` file map) and rpm (`target/rpm-stage/`) package metadata, with rpath'd binaries (`$ORIGIN/../lib/gpui-starter`).
- `REPO/.github/workflows/release.yml` publishes on `v*` tags; `justfile` exposes `release-repo`/`release-remote`/`release-workflow` vars and documents upsert-style re-releases (softprops/action-gh-release@v3).
- `cargo-dist` is NOT used (no entry in `Cargo.lock`); wasm lane is `wasm/build.sh --release` (`justfile:35-36`).

Any release-bundling plan should extend this pipeline, not introduce cargo-dist or
cargo-packager mid-stream, unless it argues the swap explicitly.

## Decision inputs

Plans 01–11 are on disk; headers were read this session to confirm topics. Plans
13, 16, 17, 18, 19, 20 are referenced by the workflow but no files exist — inputs are
topic-keyed for those (see Open Questions).

- **03 notch-floator** (`docs/plans/03-notch-floater.md` — macOS non-activating notch panel) — (b): `WindowKind::PopUp` NSPanel + `NonactivatingPanel` + `NSPopUpWindowLevel` + `CanJoinAllSpaces` mapping (`MAC/window.rs:1019-1040`, `:1228-1240`); (g): no safe-area/notch geometry from gpui — query `NSScreen` via objc2-app-kit; `toggle_simple_fullscreen` doc for notch-covering behavior.
- **04 menubar-charts** (`04-menubar-charts.md` — popover with live CPU/memory charts) — (f): `LineChart`/`AreaChart` builders, appear animation, tooltips; Step 3 `sysinfo` row (API + MSRV + dual-version note); repo precedent in `src/gallery/sections/charts/system_monitor.rs`; (d) `background_executor().timer` for the 1 Hz tick; (b) `AnchoredPopup`/`PopupOptions` for anchoring under the status item.
- **07 canvas-shaders** (`07-canvas-shaders.md` — WGSL fragment shaders in canvas) — (c) first: no runtime shader API in gpui. Then Step 3 `wgpu` row + (g) handle problem: private offscreen device, readback, blit via (e) `RenderImage`; note gpui pins wgpu 29 while latest is 30.
- **08 multiuser-canvas** (`08-multiuser-canvas.md` — socket.io sync) — Step 3 `rust_socketio` + `socketioxide` rows: client native-only, stale, native-tls; wasm build cannot use it — demo-peers mode is mandatory for web. (d) for driving UI updates from the socket task.
- **11 wgpu-embed** (`11-wgpu-embed.md` — offscreen GPU demos as images) — (g): no surface from a gpui window; Step 3 `wgpu` row (headless adapter, `map_async` + `Device::poll`); (e): BGRA `RenderImage` + `paint_image` atlas cache for the blit; (d): pacing via timers, `request_animation_frame` for present.
- **13 / 16 / 17 / 18 / 19 / 20 — not on disk.** Topic-keyed from the ask's list (media, audio io, system controls, release bundling are the remaining named topics):
  - media → (e) `RenderImage`/BGRA/manual swap + Step 3 `image`, `symphonia` (MPL-2.0 flag), `hound`, `rodio` rows; (a) `paint_surface` is macOS-only.
  - audio io → (h) cpal enumeration/stream signatures, callback gain, wasm `wasm-bindgen` feature; `rustfft` for spectrum.
  - system controls → (h): battery (starship-battery) is the only clean cross-platform one; brightness Linux+Windows only; volume/bluetooth need per-platform code.
  - release bundling → "Release bundling" section: extend `scripts/` + `release.yml`, no cargo-dist.

## Open Questions

- **Plans 12–20 do not exist on disk.** `ls docs/plans/*.md` shows 01–11 only. The number→topic assignment for 13, 16, 17, 18, 19, 20 is inferred from the ask's topic list, not verified against files. Tried: `ls docs/plans/`, header reads of all 11 files.
- **wgpu 30 vs gpui's 29.** Whether an app can pin wgpu 30 alongside gpui-pre-wgpu 29.0.4 in one binary is a build-graph question this survey did not test (no `cargo check` run — implementation work, out of scope).
- **Websocket-in-wasm client.** No candidate crate verified for a browser Socket.IO path; `rust_socketio` rules out (docs mention no wasm). Needs its own spike if the web build must speak Socket.IO.
- **macOS volume/bluetooth specifics.** Exact CoreAudio property selectors and IOBluetooth usage were not verified — only that `objc2-core-audio` 0.3.2 and `objc2-io-bluetooth` 0.3.2 exist in/around the lock graph. No code was written against them.
- **brightness macOS absence.** Claim rests on the docs.rs platform overview naming Linux and Windows only; the crate README was not fetched to cross-check. Treat "no macOS backend" as strong-but-single-source.
- **rodio 0.23 API.** The rework (`DeviceSinkBuilder`, `Player`, microphone module) is documented on docs.rs but has no in-repo usage; treat the fit line as docs-only until a spike compiles it.
- **cpal BufferSize variants.** `StreamConfig` fields verified on docs.rs; the `BufferSize::{Default, Fixed}` variant list was not re-verified on its own page.
