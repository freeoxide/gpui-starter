# 07 Canvas Shaders Plan

## Goal

A "Shader Canvas" sidebar page runs four WGSL fragment shaders — animated
gradient, raymarched SDF scene, image warp, mouse-reactive ripple — inside the
canvas element from plan 01, with a slider-driven uniform per demo, a play/pause
control, and a live FPS overlay. Shaders render through the offscreen wgpu
device from plan 11's bridge (`WgpuEngine`), read back, and blit into the
canvas document as image shapes; the page works on macOS, Windows, and Linux,
and shows a capability-reason panel on wasm.

This revision supersedes the earlier 07 draft on disk: that draft implemented a
private wgpu runtime because no bridge existed. Plan 11 has since landed with a
public bridge, so this plan consumes it and drops the duplicated device and
readback code (Research Digest "Corrections" lists everything that changed and
why).

## Dependencies

- `docs/plans/01-canvas-foundation.md` — Interface Contract items consumed:
  - `Canvas::new(id, Entity<CanvasState>)` element and the observe requirement
    ("the mounting view must `cx.observe(&state, |_, _, cx| cx.notify())`",
    01 Interface Contract "Consumption notes").
  - `CanvasState::{new, reset_view, set_tool, set_builtin_tool, add_layer,
    add_shape, edit_document, transform}` and `BuiltinTool::Pan`.
  - `CanvasDocument::{add_layer, add_shape, update_shape, clear}` (reached
    through `CanvasState::edit_document(cx, |d| ...)`, the contract's notified
    edit path) and `CanvasShape::Image`/`CanvasShape::image`.
  - `CanvasTool` trait (`pointer_down`/`pointer_move`/`pointer_up`, `id`,
    `cursor`), `CanvasPointerEvent`, `ToolContext`, and the pub(crate)
    `dispatch_tool_event` test seam.
- `docs/plans/11-wgpu-embed.md` — Interface Contract items consumed:
  - `WgpuEngine` (native-only): `new() -> anyhow::Result<Self>` (async),
    `device()`, `queue()`, and `read_texture_bytes(&wgpu::Texture, u32, u32)
    -> anyhow::Result<Vec<u8>>` (blocking, background thread only,
    tightly-packed rows in the texture's component order).
  - `frame::bgra_to_render_image(width, height, Vec<u8>) -> Arc<RenderImage>`.
  - `frame::FpsTracker` (`new`, `push(clock::Instant)`, `fps() -> u32`).
  - Consumption-note obligations: render into an `Rgba8Unorm` texture with
    `RENDER_ATTACHMENT | COPY_SRC`; read through `read_texture_bytes`; every
    displayed frame must eventually reach `window.drop_image`, and dependents
    that paint through their own surface run their own drain (this plan's
    ticker, Phase 6).
  - Ordering constraint: plan 11 must be implemented before this plan's
    Phase 4. Phases 1-3 and the page seam do not reference the bridge. Every
    step that imports it opens with a `test -d src/features/wgpu` check.
- This feature does not touch plan 11's `wgpu_embed` capability; it registers
  its own `canvas_shaders` entry.
- Survey input: `docs/plans/_survey/seams.md` (sections 1, 4, 5, 6, 7, 8, 9)
  and `docs/plans/_survey/ecosystem.md` (sections a, c, d, e, g). Every
  load-bearing gpui/gpui-kit/wgpu claim below was re-verified against the
  crate sources on disk this session (2026-10-12); the four WGSL programs were
  additionally parsed with naga 29.0.4 in a scratch binary (Research Digest
  "WGSL validation").

## Research Digest

Path key (crate sources read this session):

| Short | Path |
|---|---|
| `GPUI` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-0.3.8/src` |
| `WGPU29` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/wgpu-29.0.4/src` |
| `WTYPES` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/wgpu-types-29.0.4/src` |
| `GB` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-base-0.7.1/src` |
| `GC` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-component-0.7.1/src` |
| `KIT` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-kit-0.7.1/src` |
| `NAGA` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/naga-29.0.4` |
| `REPO` | `/home/thragg/fo/gpui-starter` |

### No runtime shader API in gpui-pre 0.3.8 (the spec's central question)

The snapshot does **not** expose shader registration or painting to
applications, so the spec's fallback applies: blit through plan 11's wgpu
bridge. Evidence, checked by grep this session:

- The workspace comment is accurate: `REPO/Cargo.toml:12` notes gpui-kit
  enables `runtime_shaders` on gpui-pre-platform. The feature chain is
  `gpui-pre-platform-0.3.8/Cargo.toml:43` → `gpui-pre-macos-0.3.8/Cargo.toml:41`
  → `gpui-pre-apple-0.3.8/Cargo.toml:37`, where it is an **empty feature**
  (`runtime_shaders = []`).
- Its only effect is swapping how the *built-in* Metal library is created:
  `#[cfg(feature = "runtime_shaders")] const SHADERS_SOURCE_FILE` vs the
  precompiled `.metallib` (`gpui-pre-apple-0.3.8/src/metal_renderer.rs:30-34`),
  consumed by `new_library_with_source(&SHADERS_SOURCE_FILE, ...)` at
  `metal_renderer.rs:250-254`. No user shader hook, no uniform registration,
  no paint call.
- `grep -n "shader" GPUI/gpui.rs` returns zero hits — the crate root exports
  nothing shader-related. `grep -rn "create_shader\|register_shader\|ShaderId\|paint_shader"`
  across `gpui-pre-0.3.8/src`, `gpui-pre-apple-0.3.8/src`,
  `gpui-pre-wgpu-0.3.8/src` hits only the wgpu renderer's own built-in WGSL
  modules (`gpui-pre-wgpu-0.3.8/src/wgpu_renderer.rs:694,700`).

### Blit path facts (what the bridge gives this feature)

- `RenderImage` doc pins the byte format: **BGRA**
  (`GPUI/assets.rs:42` "A cached and processed image, in BGRA format").
  `RenderImage::new(impl Into<SmallVec<[Frame; 1]>>)` allocates a fresh
  `ImageId` from a global atomic counter (`GPUI/assets.rs:61-70`); `Frame` is
  `image::Frame` imported privately by gpui (`GPUI/assets.rs:4`), which is why
  the bridge (plan 11) owns the `image` dependency and the `RenderImage`
  construction helpers — this feature never builds a `RenderImage` itself.
- `window.paint_image(bounds, image_bounds, corner_radii, data:
  Arc<RenderImage>, frame_index, grayscale)` tiles through the sprite atlas
  keyed by `(ImageId, frame_index)` (`GPUI/window.rs:4919-4960`). Plan 01's
  canvas `Image` painter calls this; this feature only swaps the `Arc` inside
  the shape.
- The atlas has **no eviction**: `get_or_insert_with` inserts into
  `tiles_by_key` and only `remove` deletes (`GPUI/platform.rs:1692-1697,
  1748`). One new `RenderImage` per animation frame grows the atlas
  unboundedly unless old images are dropped explicitly.
- `window.drop_image(Arc<RenderImage>) -> Result<()>` removes every frame of
  an image from the atlas (`GPUI/window.rs:5035-5048`); it takes `&mut Window`
  and carries no paint-phase assert (read this session). Called during the
  ticker's paint, this bounds the atlas at a few tiles — exactly the drain
  obligation plan 11's consumption notes place on dependents.

### wgpu 29.0.4 API shapes used by `runtime.rs`

`REPO/Cargo.lock` carries a single `wgpu 29.0.4` (pulled by gpui-pre-wgpu)
and a single `naga 29.0.4`; plan 11's `wgpu =29.0.4` pin reuses that copy.
Signatures verified in the 29.0.4 sources:

- `Instance::new(desc: InstanceDescriptor)` takes the descriptor **by value**
  (`WGPU29/api/instance.rs:63`). **There is no `InstanceDescriptor::default()`**:
  the struct derives only `Debug` (`WTYPES/instance.rs:26-27`) and an
  exhaustive grep for `impl Default for InstanceDescriptor` across
  wgpu-29.0.4 and wgpu-types-29.0.4 returns nothing. This feature never
  constructs one — `WgpuEngine::new()` (plan 11) owns instance/adapter/device
  creation; the erratum is recorded because the previous 07 draft would not
  have compiled.
- `adapter.request_device(&DeviceDescriptor<'_>) -> impl Future<Output =
  Result<(Device, Queue), RequestDeviceError>>` (`WGPU29/api/adapter.rs:58-64`)
  — engine-side, listed for context.
- `Queue::write_buffer(&Buffer, offset, &[u8])` (`WGPU29/api/queue.rs:157`);
  `Queue::write_texture(TexelCopyTextureInfo<'_>, data: &[u8],
  TexelCopyBufferLayout, size: Extent3d)` (`WGPU29/api/queue.rs:234-240`).
  `TexelCopyBufferLayout { offset, bytes_per_row: Option<u32>,
  rows_per_image: Option<u32> }` (`WTYPES/texture.rs:795-827`); its doc notes
  `write_texture` (unlike `copy_texture_to_buffer`) has **no** 256-byte
  row-alignment requirement.
- `Device::create_shader_module(ShaderModuleDescriptor)` — `ShaderSource`
  enum at `WGPU29/api/shader_module.rs:185`, descriptor at `:223`. Invalid
  WGSL panics through the uncaptured-error handler; the naga tests in Phase 2
  are the compile gate.
- `RenderPipelineDescriptor` fields include `multiview_mask: Option<NonZeroU32>`
  (`WGPU29/api/render_pipeline.rs:199,238` — spelled `multiview_mask`, not
  `multiview`). `VertexState { module, entry_point: Option<&str>,
  compilation_options, buffers: &[VertexBufferLayout] }` (`:95-116`);
  `FragmentState { module, entry_point, compilation_options,
  targets: &[Option<ColorTargetState>] }` (`:127-151`).
- `BindGroupLayoutDescriptor { label, entries: &[BindGroupLayoutEntry] }`
  (`WGPU29/api/bind_group_layout.rs:38-43`);
  `BindGroupLayoutEntry { binding: u32, visibility: ShaderStages, ty:
  BindingType, count: Option<NonZeroU32> }` (`WTYPES/binding.rs:323-358`).
- `RenderPassDescriptor` color attachments are
  `&[Option<RenderPassColorAttachment>]`;
  `RenderPassColorAttachment { view, depth_slice: Option<u32>,
  resolve_target, ops }` (`WGPU29/api/render_pass.rs:603-614`) — set
  `depth_slice: None`.
- Sampler: gpui's own renderer uses
  `device.create_sampler(&wgpu::SamplerDescriptor { label, mag_filter:
  Linear, min_filter: Linear, ..Default::default() })`
  (`gpui-pre-wgpu-0.3.8/src/wgpu_renderer.rs:1238-1243`); the defaults cover
  `ClampToEdge` addressing. Same shape here.
- `wgpu::COPY_BYTES_PER_ROW_ALIGNMENT = 256` (`WTYPES/lib.rs:169`) — padding
  math lives inside `WgpuEngine::read_texture_bytes` (plan 11); this feature
  needs none of it.

### Frame loop, threads, clocks, spawn shapes

- `Window::on_next_frame(callback: impl FnOnce(&mut Window, &mut App))` runs
  directly after the current frame renders (`GPUI/window.rs:2650-2657`) — the
  safe point to kick the next render without mutating entities mid-paint.
- `App::spawn<AsyncFn, R>(f: AsyncFn) -> Task<R>` where
  `AsyncFn: AsyncFnOnce(&mut AsyncApp) -> R` (`GPUI/app.rs:2177-2182`) —
  **one-argument** async closure; used from the `&mut App` inside
  `on_next_frame`. By contrast `Context::spawn` takes a **two-argument**
  closure `AsyncFnOnce(WeakEntity<T>, &mut AsyncApp)` (`GPUI/app/context.rs:230-236`)
  — used from the page's methods, where the weak handle comes for free. The
  previous draft wrote one-arg closures against `Context::spawn`; that arity
  mismatch is a compile error (Correction 5 below).
- `AsyncApp: AppContext` (`GPUI/app/async_context.rs:36`) with an inherent
  `background_executor()` (`:159`); `BackgroundExecutor::timer(Duration) ->
  Task<()>` (`GPUI/executor.rs:183`). Wgpu futures are `WasmNotSend` =
  `Send` on native, and `WgpuEngine` is `Send + Sync` (plan 11 contract), so
  `Arc<WgpuEngine>` and `Arc<ShaderRuntime>` cross into background tasks.
- `WeakEntity::update<C: AppContext>(&self, cx: &mut C, impl FnOnce(&mut T,
  &mut Context<T>) -> R) -> Result<R>` — `Err` on a released entity, never a
  panic (`GPUI/app/entity_map.rs:803-818`); `WeakEntity::upgrade()` at `:792`;
  `Entity::downgrade() -> WeakEntity<T>` at `:470`.
- Clocks: `crate::platform::clock::Instant` (native `std`, wasm `web_time`;
  `REPO/src/platform/clock.rs:1-8`, also exports `Duration`) — the repo rule
  against `std::time` in shared code (seams survey §6) applies because
  `state.rs` compiles on wasm.
- `App::reduce_motion() -> bool` (`GPUI/app.rs:1222`). This page is
  user-played demo content (video-player class per the
  `request_animation_frame` doc, `GPUI/window.rs:2670-2674`), not chrome
  decoration: it starts **paused** when `reduce_motion()` is true and runs
  after an explicit Play.

### Kit components used

- `Slider::new(&Entity<SliderState>)` (`GC/slider.rs:96`); `Slider`
  implements `Styled` (`GC/slider.rs:139`) and `RenderOnce` (`:145`) — it has
  **no `Sizable` impl, so no `.small()`**; sizing goes through `Styled`, the
  repo pattern being `Slider::new(&state).w(rems(12.5))`
  (`REPO/src/gallery/sections/theme/brush.rs:353-356`).
- `SliderState::new()` with consuming builders `.min(f32)`, `.max(f32)`,
  `.default_value(impl Into<SliderValue>)`, reader `.value() -> SliderValue`
  (`GB/slider.rs:198-296`); `SliderValue::Single(f32)` with `.start()` for the
  scalar (`GB/slider.rs:29-33`, repo read at `brush.rs:98`); slider events are
  `SliderEvent::{Change(SliderValue), Release(SliderValue)}` on
  `EventEmitter<SliderEvent> for SliderState` (`GB/slider.rs:15-20, 867`).
  Imports: `gpui_kit::component::slider::{Slider, SliderState}` — the
  component crate re-exports the base types (`GC/slider.rs:4`).
- `Button::new(id).label(...).outline().on_click(...)` (`GC/button/button.rs:371,332,447`);
  `.primary()` is a default method of the `ButtonVariants` trait
  (`GC/button/button.rs:48-53`, `impl … for Button` at `:573`) — the repo uses
  it widely (`REPO/src/gallery/sections/buttons/button.rs:92`).
  `ButtonGroup::new(id).children(impl IntoIterator<Item = Button>)`
  (`GC/button/button_group.rs:44,67`).
- `Context::subscribe(&entity, |&mut T, Entity<T2>, &Evt, &mut Context<T>)`
  (`GPUI/app/context.rs:94-108`).
- The `canvas(prepaint, paint)` helper builds a one-off painting element; the
  paint closure receives `(Bounds<Pixels>, T, &mut Window, &mut App)`
  (`GPUI/elements/canvas.rs:9-22`), and `impl<T> Styled for Canvas<T>`
  (`GPUI/elements/canvas.rs:91`) carries a style
  refinement. Exported through `pub use elements::*`
  (`GPUI/gpui.rs:111`) and the kit glob (`KIT/lib.rs:97`), so `canvas()` is
  reachable as `gpui_kit::canvas` — no name collision with plan 01's `Canvas`
  struct. Used for the frame ticker.
- Repo page conventions from `REPO/src/features/pages/notifications.rs`:
  imports `gpui_kit::component::{...}` + `gpui_kit::{prelude::*, *}` with
  `use crate::accessibility::A11yExt as _;` (`notifications.rs:1-5`), heading
  via `.a11y(Role::Heading, title).aria_level(1).text_xl().font_weight(FontWeight::BOLD)`
  (`notifications.rs:48-58`), page container `v_flex().min_h_full().p_6().gap_4()`.
- Sidebar icon: `IconName::Palette` — verified in the component enum the app
  registers (`gpui-kit-assets-0.7.1/default-icons.txt` + the generated
  `component::IconName` in `target/debug/build/gpui-kit-assets-*/out/
  icon_name.rs`; the default 106 is the only set the app's asset source
  carries). The earlier pick `Orbit` (and the fallbacks `WandSparkles`/
  `Activity`) exist only as svgs in the FULL `gpui_kit::assets::IconName`
  catalog — the svg being on disk does not put the variant in the component
  enum or the bytes in the default asset source, the same trap plans 04/06/12
  hit. `Palette` reads as "visual effects" for a shader playground and is
  unambiguous in the subset.

### Capability + i18n seams

- `CapabilityStatus` has pub fields `supported/enabled/degraded/reason/
  last_error` and constructors `supported_enabled`, `degraded(err)`,
  `error(err)` (`REPO/src/runtime/capabilities.rs:5-50`).
- `crate::capabilities::set(name, status, cx: &mut App)` takes `&mut App`
  **directly** (`capabilities.rs:64`) — not a generic context. From async
  code it must run inside a `WeakEntity::update`/`Context` closure (both
  deref to `App`, `GPUI/app/context.rs:26-38`), never on `&mut AsyncApp`.
  Wasm "hidden with capability reason" needs a manual struct literal with
  `supported: false`; precedent at
  `REPO/src/platform/process/single_instance.rs:647-655`. Startup list
  `DEFAULT_ENABLED_CAPABILITIES` at `REPO/src/app/init.rs:95-108`.
- i18n facade: `crate::i18n::localize("key")` and
  `crate::i18n::localize_with_args("key", &[("name", value)])` with `%{name}`
  placeholders (`REPO/src/services/i18n.rs:3-13`). Catalog grammar and the
  both-locales lockstep rule per seams survey §5; the scanner resolves only
  literal first arguments, so `localize(self.title_key())` nav keys ride on
  the lockstep test alone.
- Page views reach `AppRoot` through `crate::views`
  (`REPO/src/lib.rs:45` re-exports `features::pages as views`;
  `src/shell/root/app_root/state.rs:6-11` imports them from there).

### WGSL validation (run this session)

The four programs in Phase 2 were parsed with naga 29.0.4 in a scratch
binary under `/tmp/wgslcheck` (`cargo run --offline`, dep
`naga = { version = "=29.0.4", features = ["wgsl-in"] }`; `parse_str` at
`NAGA/src/front/wgsl/mod.rs:85`, feature at `NAGA/Cargo.toml:83`):

```text
gradient: OK (2 entry points)
raymarch: OK (2 entry points)
warp: OK (2 entry points)
ripple: OK (2 entry points)
```

The run caught one real error: `target` is a reserved WGSL keyword
(`name 'target' is a reserved keyword`); the shader now uses `cam_target`.
This is exactly the test Phase 2 wires into `demos.test.rs`, so regressions
of this class fail CI without a GPU.

### Corrections vs the previous 07 draft on disk

1. **Private runtime → plan 11 bridge.** The draft's `runtime.rs` built its
   own instance/adapter/device and readback stack because no bridge existed
   ("plan 11's wgpu bridge does not exist", its Open Questions). Plan 11 now
   ships `WgpuEngine` + `read_texture_bytes` + image helpers + `FpsTracker`
   publicly; this revision consumes them and deletes the duplicated code
   (plan 11's own risk 5 expected exactly this extraction).
2. **`InstanceDescriptor::default()` does not exist** in wgpu 29.0.4
   (`WTYPES/instance.rs:26-27`, exhaustive grep; plan 11's correction 1).
   The draft's Step 4.1 would not have compiled. Moot now — the engine
   constructs it inside plan 11.
3. **CPU swizzle → GPU-side swizzle.** The draft swizzled RGBA→BGRA on the
   CPU after readback. Plan 11's correction 2 shows the copy-free form: write
   `b,g,r,a` component order into the `Rgba8Unorm` target so readback bytes
   are already BGRA, then `bgra_to_render_image`. The four fragment shaders
   end with `return vec4<f32>(col.b, col.g, col.r, 1.0);` and the naga test
   asserts the marker so it cannot be dropped silently.
4. **Icon verification method fixed twice.** The interim revision claimed
   `wand-sparkles.svg` was absent — a false negative from piping `ls | grep`
   through `head`. Re-checked without `head`: the full-catalog svgs all
   exist. But svg existence proves nothing about reachability: the app
   registers the default 106 only, so `Orbit` (and the `WandSparkles`/
   `Activity` alternatives) render blank on native even through the assets
   enum. Final choice: `IconName::Palette`, verified in
   `default-icons.txt` (icon policy: INDEX §4).
5. **`Slider` has no `.small()`** — it implements only `Styled`/`RenderOnce`
   (`GC/slider.rs:139,145`). The draft's `Slider::new(..).small()` was an
   invented API. Sizing now follows the repo pattern `.w(rems(12.5))`.
6. **`Context::spawn` arity.** The draft spawned `async move |cx| ...` from a
   `Context`; the real bound is `AsyncFnOnce(WeakEntity<T>, &mut AsyncApp)`
   (`GPUI/app/context.rs:230-236`). The frame task now spawns from
   `on_next_frame`'s `&mut App` via `App::spawn` (one-arg,
   `GPUI/app.rs:2177-2182`), and the init task uses the two-arg
   `Context::spawn` form.
7. **`capabilities::set` needs `&mut App`** (`capabilities.rs:64`); all calls
   now route through entity-update closures.
8. **No `image` dependency.** The warp source pattern is a plain `Vec<u8>`
   fed to `Queue::write_texture`, and every `RenderImage` construction goes
   through the bridge's `bgra_to_render_image`. The draft's `image` dep line
   is dropped.

### Crate picks

| Crate | Version | Why |
|---|---|---|
| `wgpu` | `=29.0.4`, default features, native-only table | `runtime.rs` names wgpu types directly (textures, pipelines, bind groups). Exact pin reuses the lock's single wgpu copy (pulled by gpui-pre-wgpu); any other version compiles a second full wgpu/naga stack. Plan 11 adds the identical line; both plans guard with grep-first. |
| `naga` (dev-dep) | `=29.0.4`, feature `wgsl-in` | GPU-free validation of the four WGSL sources (proven this session, above). Same version as the lock's naga; dev-deps never reach the wasm lib check. Same pick as plan 11. |
| (dropped) `image` | — | Not needed: no `image::Frame`/`RgbaImage` construction happens in this feature (Corrections item 8). |

## Architecture

Entities and ownership (all inside `src/features/pages/shaders/`; the page
itself is owned by `AppRoot`):

- `Entity<CanvasState>` (plan 01) — the visible canvas. Its document holds
  exactly one `CanvasShape::Image` covering doc rect `(0,0)-(W,H)` where
  `W×H` is the demo's render resolution. Active tool is `PanTool`, except on
  the ripple demo where it is `RippleTool`.
- `Entity<ShaderState>` — playback state: current demo, playing flag, uniform
  parameters (`time`, `delta`, `mouse` uv, `slider`), in-flight flag, FPS
  ring (plan 11's `FpsTracker`), the displayed `Arc<RenderImage>` mirror, a
  bounded `stale_frames` queue (cap 4), and the runtime error string.
  Platform-neutral (compiles on wasm; the runtime never touches it there).
- `Arc<WgpuEngine>` (plan 11, native only) — headless device/queue plus the
  blocking readback. Created by the page's init task; never stores this
  feature's state.
- `Arc<ShaderRuntime>` (native only) — this feature's pipelines, bind groups,
  per-demo render targets, and the 32-byte uniform buffer, all built once in
  `ShaderRuntime::new(Arc<WgpuEngine>)` (fixed per-demo resolutions; no
  reconfiguration path). `render_frame(&self, ...)` takes `&self` — encoding,
  submission, and readback never mutate the stored handles, so the `Arc`
  shares across background tasks without locks.
- `Entity<SliderState>` — the uniform slider; the page subscribes to
  `SliderEvent::Change` and writes the value into `ShaderState`. Rebuilt (new
  entity, resubscribed) whenever the active demo changes its slider spec.

Data flow per animation frame (native):

1. Paint: the page renders `Canvas::new("shader-canvas", canvas)` plus an
   absolutely-positioned full-size `canvas()` ticker inside the canvas region.
   The ticker's paint closure (a) drains `stale_frames` through
   `window.drop_image` — the atlas-eviction obligation from plan 11's
   consumption notes — and (b) when playing, no frame in flight, runtime
   ready, and no error, registers `window.on_next_frame(kick)`.
2. `on_next_frame` (post-render, safe to mutate): re-checks eligibility
   through the weak entities, `mark_frame_started(now)`, snapshots
   `ShaderParams`, and `App::spawn`s the frame task.
3. Frame task (main-thread async): paces on a background-executor timer to
   the next 1/60 s boundary, re-checks playing, then
   `background_executor().spawn(runtime.render_frame(spec, params))`:
   `queue.write_buffer` uniforms → render pass (fullscreen triangle,
   `draw(0..3)`, fragments write BGRA byte order) → submit →
   `WgpuEngine::read_texture_bytes` (its own copy/submit/map/poll; queue
   order guarantees the copy sees the render) →
   `bgra_to_render_image`. All blocking waits live on the background thread;
   the UI thread never polls.
4. Completion (back on `&mut AsyncApp`): the canvas entity's document swaps
   the `Arc<RenderImage>` inside the image shape via `edit_document` +
   `update_shape` (a missing shape means the demo switched mid-flight; the
   next install re-creates it), then `ShaderState::frame_done(now, image)`
   moves the previous image into `stale_frames`, advances `time`, pushes the
   FPS stamp, clears the in-flight flag, and notifies once. Notify → page
   re-render → canvas repaint → ticker drain + kick: the loop is
   self-sustaining while the page paints and halts with no frame in flight
   when it stops painting.
5. Pointer (ripple demo): plan 01's router dispatches into
   `RippleTool::pointer_down/pointer_move` inside the `CanvasState` lease; the
   tool converts the doc position to uv over the image shape's doc bounds and
   updates `ShaderState` through a `WeakEntity` — a different entity than the
   leased one, so the entity-map double-lease rule
   (`GPUI/app/entity_map.rs:160-165`) is not violated. `set_mouse` does not
   notify; the uniform is consumed by the next render task.

Failure path: `WgpuEngine::new()` or `render_frame` errors set
`ShaderState::runtime_error` and flip the `canvas_shaders` capability to
`degraded`/`error` (always inside an entity-update closure — set takes
`&mut App`), the loop stops, and the page shows an error panel with a Retry
button that re-runs the init task; success resets the capability to
`supported_enabled`.

Wasm: `runtime.rs` and the ticker/frame-task code are
`#[cfg(not(target_family = "wasm"))]`; the page renders a capability-reason
panel instead. `demos.rs`, `state.rs`, `tool.rs` compile everywhere (pure
data + `clock::Instant`).

Module tree:

```text
src/features/pages/shaders/
├── mod.rs        page seam: header comment + removal recipe, ShaderCanvasPage
│                 (layout, toolbar, slider wiring, runtime init task, ticker
│                 mount, error/wasm panels), module decls + CAPABILITY_SHADERS
├── demos.rs      DemoId, SliderSpec, DemoSpec, DEMOS registry, ShaderParams
│                 uniform encode, the four WGSL programs, warp_source_bytes
├── state.rs      ShaderState entity (playback, uniforms, fps, stale frames,
│                 in-flight, error) — platform-neutral
├── runtime.rs    #[cfg(not(wasm))] ShaderRuntime over Arc<WgpuEngine>:
│                 pipelines/bind groups/targets; render_frame -> Arc<RenderImage>
├── tool.rs       RippleTool: plan-01 CanvasTool feeding mouse uv
└── *.test.rs     sibling test modules (demos, state, runtime, tool)
```

## Interface Contract

Leaf feature: nothing else consumes it. The complete public surface:

```rust
// src/features/pages/shaders/mod.rs
mod demos;
mod state;
#[cfg(not(target_family = "wasm"))]
mod runtime;
mod tool;

/// Capability-registry name registered at startup ("canvas_shaders").
/// Reached from init.rs as `crate::views::CAPABILITY_SHADERS` (the page
/// module is private; features/pages/mod.rs re-exports the const).
pub const CAPABILITY_SHADERS: &str = "canvas_shaders";

// ShaderCanvasPage is defined in mod.rs itself; no further re-exports.
pub struct ShaderCanvasPage { /* private fields */ }
impl ShaderCanvasPage {
    /// Constructs the page, mounts its entities, and (native only) starts
    /// the wgpu engine init task. Callers never block.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self;
}
```

Also part of the contract (observable surface, not importable API):

- Sidebar page variant `Page::ShaderCanvas` with `host() == "shader-canvas"`
  (deep link `gpui-starter://shader-canvas`), appended last in `all()` at
  land time (no digit shortcut: bindings cover only `Page::all()[..9]`,
  seams survey §1 row 8 note).
- Capability registry name `canvas_shaders`: `supported_enabled` on native
  via `DEFAULT_ENABLED_CAPABILITIES`; `degraded(err)`/`error(err)` on runtime
  failure; `supported: false` with a reason on wasm.
- i18n keys listed in the i18n section, present in both locale files.
- Nothing from `demos`/`state`/`runtime`/`tool` is exported past the page
  module. If a later feature wants shader effects, it should consume plan
  11's bridge directly and keep its own pipelines — this module is a page,
  not a library.

## Module Layout

| File | Purpose |
|---|---|
| `src/features/pages/shaders/mod.rs` | Page seam + `ShaderCanvasPage` render/layout, toolbar, slider subscription, `install_demo`, runtime init task, frame ticker mount, error and wasm panels, `CAPABILITY_SHADERS` |
| `src/features/pages/shaders/demos.rs` | `DemoId`, `SliderSpec`, `DemoSpec`, `DEMOS` array with the four WGSL programs and resolutions, `ShaderParams` + `to_bytes` uniform encode, `warp_source_bytes()` |
| `src/features/pages/shaders/state.rs` | `ShaderState` entity: playback, uniforms, frame lifecycle, stale-frame queue, error state |
| `src/features/pages/shaders/runtime.rs` | Native `ShaderRuntime` over `Arc<WgpuEngine>`: per-demo pipelines/bind groups/targets, `render_frame` with bridge readback |
| `src/features/pages/shaders/tool.rs` | `RippleTool` implementing plan 01's `CanvasTool` |
| `src/features/pages/shaders/demos.test.rs` | naga validation of all WGSL sources, uniform byte layout, spec table checks |
| `src/features/pages/shaders/state.test.rs` | Playback/uniform/stale-frame lifecycle (entity context) |
| `src/features/pages/shaders/runtime.test.rs` | Native GPU smoke test (ignored by default) |
| `src/features/pages/shaders/tool.test.rs` | Ripple uv mapping through plan 01's dispatch seam |

## Seam Touches

Every edit outside the new module:

| # | File | Change | Why minimal |
|---|---|---|---|
| 1 | `Cargo.toml` `[target.'cfg(not(target_family = "wasm"))'.dependencies]` (table at line 311) | add `wgpu = { version = "=29.0.4" }` **only if absent** (grep first — plan 11 adds the identical line; duplicate TOML keys are an error) with a one-line comment naming the exact-pin reason | Only table native-only deps may live in (seams survey §3/§6); version already in the lock |
| 2 | `Cargo.toml` `[dev-dependencies]` (line 212) | add `naga = { version = "=29.0.4", features = ["wgsl-in"] }` only if absent (same grep-first rule vs plan 11) | Test-only WGSL validation |
| 3 | `Cargo.lock` | regenerate via `cargo check -p gpui-starter` and commit together with the manifest (seams survey §9 lock rule) | Lock must match the manifest |
| 4 | `src/features/pages/mod.rs` | `mod shaders;` + `pub use shaders::{ShaderCanvasPage, CAPABILITY_SHADERS};` | Standard page registration; the const must be re-exported because every sibling `mod` here is private (page types are re-exported only), and `src/app/init.rs` references the const via `crate::views::CAPABILITY_SHADERS` — a `pub` item left inside the private module is E0603 there, and only under the wasm cfg, so native `cargo check` would stay green while Step 1.3's `just wasm-check` fails |
| 5 | `src/shell/sidebar.rs` | `ShaderCanvas` variant (enum at :5-16), `title()` arm `"Shader Canvas"`, `title_key()` arm `nav_shader_canvas`, `host()` arm `shader-canvas`, `icon()` arm `IconName::Palette`, `all()` entry appended last | Exhaustive matches force all arms; last position avoids re-binding digit keys |
| 6 | `src/shell/route.rs` | `Page::ShaderCanvas.host(),` in `VALID_HOSTS` (:10-21) | Deep-link validation |
| 7 | `src/shell/root/app_root/state.rs` | import `ShaderCanvasPage` via `crate::views` (state.rs:6-11 pattern), field `shader_canvas_page: Entity<ShaderCanvasPage>` (:16-36 block), construction in `AppRoot::new` (:50-62 block), `unchecked_active_page_view` arm (:209-223) | Page mounting |
| 8 | `tests/e2e_navigation.rs` | read the current count assertion (:13-19), bump by one, update the comment, add `Page::ShaderCanvas` to the expected array | Exhaustive-count test; the absolute number depends on landing order (plans 02-11 also add pages), so it is read, not hardcoded |
| 9 | `locales/en.yml`, `locales/zh-CN.yml` | all keys from the i18n table, both files, after the `nav_*` block (`locales/en.yml:53-62`) | Lockstep rule (seams survey §5) |
| 10 | `src/app/init.rs` | `"canvas_shaders"` in `DEFAULT_ENABLED_CAPABILITIES` (:95-108) + a `#[cfg(target_family = "wasm")]` block after the loop setting the unsupported status | Capability surface |

Deliberate non-touches: no command-palette entry (sidebar + deep link reach
it), no gallery section, no `src/ui/canvas/` edits (plan 01 is consumed
strictly through its Interface Contract; the atlas drain lives in this
feature's ticker, not in the canvas painter), no `src/features/wgpu/` edits
(plan 11's bridge is consumed read-only), no `[workspace.dependencies]`
change, no new locale files, no `image` dependency.

### Removal Recipe

`mod.rs` opens with this header, mirroring `src/gallery/mod.rs:1-7`:

```rust
//! Removable shader-canvas page. `ShaderCanvasPage` is the only public
//! surface; the wgpu pipelines, demos, and tool stay private to this module.
//! Removal = delete this directory and revert its ten seam touches: the
//! wgpu/naga lines in Cargo.toml plus the regenerated Cargo.lock (only if
//! plan 11's bridge is not keeping them), the `mod shaders;` line and the
//! `pub use shaders::{ShaderCanvasPage, CAPABILITY_SHADERS};` re-export in
//! features/pages/mod.rs, the Page::ShaderCanvas arms and all() entry in
//! shell/sidebar.rs, the VALID_HOSTS entry in shell/route.rs, the AppRoot
//! wiring in shell/root/app_root/state.rs, the page count in
//! tests/e2e_navigation.rs, the shader_* and nav_shader_canvas keys in both
//! locale files, and the canvas_shaders capability lines in app/init.rs.
```

Ordered removal steps:

1. Revert seams 4-10 (pages mod registration, sidebar arms, route host,
   AppRoot wiring, nav count, locale keys, capability lines).
2. If `grep -rn "features::wgpu" src/` still hits (plan 11 landed), keep the
   `wgpu`/`naga` lines; otherwise revert seams 1-2 and run
   `cargo check -p gpui-starter` so `Cargo.lock` drops the now-unused direct
   entries (they may remain transitively via gpui-pre-wgpu — correct);
   commit lock + manifest together.
3. `rm -r src/features/pages/shaders/`.
4. `grep -rn "ShaderCanvas\|canvas_shaders\|shader_" src/ tests/ locales/` —
   zero hits remaining.
5. Run the Verification gates; they must pass.

## Implementation Phases

Preconditions: plan 01 has landed (`test -d src/ui/canvas && grep -n "pub
mod canvas" src/ui/mod.rs`) — every phase depends on its contract. Phase 4
and later additionally require plan 11's bridge on disk
(`test -d src/features/wgpu && grep -n "pub use engine::WgpuEngine"
src/features/wgpu/mod.rs`); if that check fails, stop and land plan 11 first
rather than reimplementing a device stack. Plans 02-11 may have landed other
pages first; every seam edit below reads current file state before writing.

## Phase 1: Dependencies, page seam, wasm panel

### Step 1.1: Cargo manifest and lock

- Files: `Cargo.toml`, `Cargo.lock`.
- Build: run `grep -n '^wgpu\|^naga' Cargo.toml` first. Under
  `[target.'cfg(not(target_family = "wasm"))'.dependencies]` (line 311) add —
  only if the line is still missing —
  `wgpu = { version = "=29.0.4" }` (comment: "exact pin shares the lock's
  wgpu with gpui-pre-wgpu; any other version compiles a second stack").
  Under `[dev-dependencies]` (line 212) add, if missing,
  `naga = { version = "=29.0.4", features = ["wgsl-in"] }` (comment:
  "GPU-free WGSL validation for the demo shaders; matches the lock's naga").
  Run `cargo check -p gpui-starter` to rewrite the lock; `git diff
  Cargo.lock` must show only entries moving from transitive to direct (no
  version bumps — both are already locked).
- Tests: none.
- Done-check: `cargo check -p gpui-starter` passes;
  `grep -c '^wgpu = ' Cargo.toml` is 1 and the line sits under the not-wasm
  table; `grep -c '^naga = ' Cargo.toml` is 1 under `[dev-dependencies]`.

### Step 1.2: Module scaffold and page seam

- Files: `src/features/pages/shaders/mod.rs` (header comment + stub),
  `src/features/pages/mod.rs`, `src/shell/sidebar.rs`, `src/shell/route.rs`,
  `src/shell/root/app_root/state.rs`, `tests/e2e_navigation.rs`,
  `locales/en.yml`, `locales/zh-CN.yml`, `src/app/init.rs`.
- Build:
  - `shaders/mod.rs`: the removal-recipe header (Seam Touches), then a
    minimal `pub struct ShaderCanvasPage { _subscriptions: Vec<Subscription> }`
    with `pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self`
    (no-op body returning `Self { _subscriptions: Vec::new() }`, mirroring
    `notifications.rs:12-23`) and a `Render` impl: `v_flex().min_h_full().
    p_6().gap_4()` with a level-1 heading
    `div().id("shader-title").a11y(Role::Heading, title).aria_level(1).
    text_xl().font_weight(FontWeight::BOLD).child(title)` where `title =
    crate::i18n::localize("shader_title")` (`notifications.rs:48-58` pattern,
    `use crate::accessibility::A11yExt as _;`). Declare `pub const
    CAPABILITY_SHADERS: &str = "canvas_shaders";`.
  - `features/pages/mod.rs`: `mod shaders;` (alphabetical slot after
    `settings`) + `pub use shaders::{ShaderCanvasPage, CAPABILITY_SHADERS};`
    — both symbols, because every sibling module here is private
    (the current file re-exports page types only,
    `src/features/pages/mod.rs:1-25`), and `init.rs` below reaches the const
    through `crate::views` (`lib.rs:45` re-export).
  - `sidebar.rs`: `ShaderCanvas` variant appended last in the enum and in
    `all()`; `title() => "Shader Canvas"`; `title_key() => "nav_shader_canvas"`;
    `host() => "shader-canvas"`; `icon() => IconName::Palette`.
  - `route.rs`: `Page::ShaderCanvas.host(),` appended to `VALID_HOSTS`.
  - `app_root/state.rs`: add `ShaderCanvasPage` to the `crate::views` import
    list (:6-11), field `shader_canvas_page: Entity<ShaderCanvasPage>`,
    construct `cx.new(|cx| ShaderCanvasPage::new(window, cx))` in
    `AppRoot::new`, arm `Page::ShaderCanvas => self.shader_canvas_page.
    clone().into()` in `unchecked_active_page_view`.
  - `tests/e2e_navigation.rs`: read the current count assertion (10 today),
    bump by one, update the trailing comment, add `Page::ShaderCanvas` to
    the expected array.
  - Locale files: every key from the i18n table below, both files, placed
    after the `nav_*` block.
  - `init.rs`: add `"canvas_shaders"` to `DEFAULT_ENABLED_CAPABILITIES`
    (:95-108); after the loop add:

    ```rust
    #[cfg(target_family = "wasm")]
    crate::capabilities::set(
        crate::views::CAPABILITY_SHADERS,
        crate::capabilities::CapabilityStatus {
            supported: false,
            enabled: false,
            degraded: true,
            reason: Some("WGSL shader rendering needs an offscreen wgpu device; the web build ships no bridge.".into()),
            last_error: None,
        },
        cx,
    );
    ```

    `crate::views::CAPABILITY_SHADERS`, not
    `crate::features::pages::shaders::…`: the module is private (Step above),
    so the private path is E0603 — and only under this wasm cfg, which is why
    the path must be checked by `just wasm-check`, not native `cargo check`.

    (struct literal is legal: fields are pub, `capabilities.rs:5-11`; wasm
    literal precedent `single_instance.rs:647-655`.)
- Tests: existing `tests/qa_localization.rs` (lockstep) and
  `tests/e2e_navigation.rs` (count) cover the seam.
- Done-check: `cargo test --test e2e_navigation && cargo test --test
  qa_localization` pass; `cargo check -p gpui-starter` passes.

### Step 1.3: Wasm degradation panel

- Files: `src/features/pages/shaders/mod.rs`.
- Build: split `render` into a `#[cfg(target_family = "wasm")]` branch — the
  heading plus a muted paragraph
  `div().text_sm().text_color(cx.theme().muted_foreground).child(
  crate::i18n::localize("shader_unavailable_wasm"))` inside
  `.border_1().border_color(cx.theme().border).rounded_lg().p_4()` — and a
  `#[cfg(not(target_family = "wasm"))]` branch that for now returns the same
  scaffold minus the panel (filled in Phase 5). The cfg split is established
  here so later phases only touch the native branch.
- Tests: none (wasm rendering is not unit-testable; compile coverage below).
- Done-check: `just wasm-check` passes (`cargo +nightly check --target
  wasm32-unknown-unknown -p gpui-starter --lib`, `justfile:24-25`).

## Phase 2: Demo registry, uniforms, WGSL

### Step 2.1: Types and uniform encode

- Files: `src/features/pages/shaders/demos.rs`,
  `src/features/pages/shaders/demos.test.rs` (wired from `demos.rs` with
  `#[cfg(test)] #[path = "demos.test.rs"] mod demos_test;`).
- Build:
  - `#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub(in crate::features::pages::shaders) enum DemoId { Gradient,
    Raymarch, Warp, Ripple }` with `pub(in ...) fn ix(self) -> usize`
    (exhaustive match, declaration order).
  - `pub(in ...) struct SliderSpec { pub min: f32, pub max: f32, pub
    default: f32 }` (record-like, `#[non_exhaustive]` unnecessary inside the
    module; plain pub fields per the coding guides' record exception).
  - `pub(in ...) struct DemoSpec { pub id: DemoId, pub wgsl: &'static str,
    pub width: u32, pub height: u32, pub slider: SliderSpec, pub
    uses_texture: bool }`.
  - `pub(in ...) const DEMOS: [DemoSpec; 4]`: Gradient 512×512 slider
    `0.1..=2.` default `0.5`; Raymarch 384×384 slider `0.05..=1.` default
    `0.25`; Warp 512×512 slider `0..=1` default `0.35` `uses_texture: true`;
    Ripple 512×512 slider `0..=1` default `0.5`.
  - `pub(in ...) fn demo(id: DemoId) -> &'static DemoSpec` — exhaustive
    match over the four ids into `&DEMOS[ix]`.
  - `#[derive(Clone, Copy, Debug, PartialEq)]
    pub(in ...) struct ShaderParams { pub time: f32, pub delta: f32, pub
    slider: f32, pub resolution: (f32, f32), pub mouse: (f32, f32) }` with
    `pub(in ...) fn to_bytes(self) -> [u8; 32]` writing, in order, `time`
    @0, `delta` @4, `slider` @8, zero `f32` pad @12, `resolution.0/.1`
    @16/20, `mouse.0/.1` @24/28 — each via `f32::to_le_bytes` (uniform
    buffers are little-endian). The layout matches the WGSL struct
    (`f32, f32, f32, f32, vec2<f32>, vec2<f32>`): every `vec2` sits at an
    8-byte-aligned offset (16, 24) and the total is a multiple of 16.
- Tests (`demos.test.rs`): `to_bytes` offsets (bytes 0..4 ==
  `1.5f32.to_le_bytes()`, resolution lands at 16, mouse at 24, pad is zero);
  `demo(id).id == id` for all four; exactly one spec has `uses_texture`;
  all resolutions non-zero and Raymarch is 384×384 (perf pin).
- Done-check: `cargo test -p gpui-starter --lib shaders::demos` passes.

### Step 2.2: The four WGSL programs and the warp source

- Files: same.
- Build:
  - Shared WGSL prefix. `concat!` accepts string literals only — not a
    `const WGSL_PREFIX` name — so write four self-contained consts, each
    embedding the prefix text verbatim before its fragment (the prefix block
    below is duplicated four times in `demos.rs`; the naga test parses each
    const as-is):

    ```wgsl
    struct Params {
        time: f32,
        delta: f32,
        slider: f32,
        _pad0: f32,
        resolution: vec2<f32>,
        mouse: vec2<f32>,
    };
    @group(0) @binding(0) var<uniform> params: Params;

    @vertex
    fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
        var pos = array<vec2<f32>, 3>(
            vec2<f32>(-1.0, -1.0),
            vec2<f32>(3.0, -1.0),
            vec2<f32>(-1.0, 3.0));
        return vec4<f32>(pos[vi], 0.0, 1.0);
    }
    ```

  - `GRADIENT_WGSL` appends the fragment below; `RAYMARCH_WGSL`,
    `WARP_WGSL`, `RIPPLE_WGSL` follow. Every fragment computes a color into
    `var col: vec3<f32>` and ends with the BGRA readback swizzle — the
    component order written to the `Rgba8Unorm` target is `b, g, r`, so the
    readback bytes are already BGRA for `RenderImage` (Corrections item 3):

    ```wgsl
    return vec4<f32>(col.b, col.g, col.r, 1.0);
    ```

  - Gradient fragment:

    ```wgsl
    @fragment
    fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
        let uv = frag.xy / params.resolution;
        let t = params.time * params.slider;
        let v = 0.5 + 0.5 * vec2<f32>(
            sin(uv.x * 6.28318 + t),
            cos(uv.y * 6.28318 + t * 0.7));
        let w = 0.5 + 0.5 * sin((uv.x + uv.y) * 12.56636 + t * 1.3);
        var col = vec3<f32>(v.x * w, v.y * (1.0 - w), 0.5 + 0.5 * sin(t + uv.x * 3.14159));
        return vec4<f32>(col.b, col.g, col.r, 1.0);
    }
    ```

  - Raymarch fragment (`cam_target`, not `target` — `target` is a reserved
    WGSL keyword, caught by the naga run this session):

    ```wgsl
    fn sdf(p: vec3<f32>) -> f32 {
        let sphere = length(p - vec3<f32>(-0.6, 0.4, 0.0)) - 0.5;
        var q = abs(p - vec3<f32>(0.6, 0.35, 0.0)) - vec3<f32>(0.35, 0.35, 0.35);
        let box_d = length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
        let plane = p.y + 0.4;
        return min(min(sphere, box_d), plane);
    }

    @fragment
    fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
        let uv = (frag.xy / params.resolution) * 2.0 - vec2<f32>(1.0);
        let angle = params.time * params.slider;
        let cam = vec3<f32>(sin(angle) * 3.0, 1.2, cos(angle) * 3.0);
        let cam_target = vec3<f32>(0.0, 0.2, 0.0);
        let f = normalize(cam_target - cam);
        let r = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), f));
        let u = cross(f, r);
        let dir = normalize(f * 1.6 + r * uv.x + u * uv.y);
        var p = cam;
        var dist = 0.0;
        var hit = false;
        for (var i = 0; i < 64; i++) {
            let d = sdf(p);
            if (d < 0.001) {
                hit = true;
                break;
            }
            p = p + dir * d;
            dist = dist + d;
        }
        if (!hit) {
            return vec4<f32>(vec3<f32>(0.10, 0.12, 0.16), 1.0);
        }
        let n = normalize(vec3<f32>(
            sdf(p + vec3<f32>(0.001, 0.0, 0.0)) - sdf(p - vec3<f32>(0.001, 0.0, 0.0)),
            sdf(p + vec3<f32>(0.0, 0.001, 0.0)) - sdf(p - vec3<f32>(0.0, 0.001, 0.0)),
            sdf(p + vec3<f32>(0.0, 0.0, 0.001)) - sdf(p - vec3<f32>(0.0, 0.0, 0.001))));
        let light_dir = normalize(vec3<f32>(0.6, 1.0, 0.4));
        let diff = max(dot(n, light_dir), 0.0);
        let ao = 1.0 / (1.0 + dist * 0.35);
        var col = select(vec3<f32>(0.55, 0.55, 0.58), vec3<f32>(0.86, 0.30, 0.35), p.y > 0.0);
        col = col * (0.25 + 0.75 * diff) * ao;
        return vec4<f32>(col.b, col.g, col.r, 1.0);
    }
    ```

  - Warp fragment (own bindings after the prefix; binding 0 is the shared
    uniform):

    ```wgsl
    @group(0) @binding(1) var src_tex: texture_2d<f32>;
    @group(0) @binding(2) var src_sampler: sampler;

    @fragment
    fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
        let uv = frag.xy / params.resolution;
        let w = vec2<f32>(
            sin(uv.y * 10.0 + params.time * 1.5),
            cos(uv.x * 8.0 + params.time)) * 0.08 * params.slider;
        let uv2 = clamp(uv + w, vec2<f32>(0.0), vec2<f32>(1.0));
        var col = textureSample(src_tex, src_sampler, uv2).rgb;
        return vec4<f32>(col.b, col.g, col.r, 1.0);
    }
    ```

  - Ripple fragment:

    ```wgsl
    @fragment
    fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
        let uv = frag.xy / params.resolution;
        let d = distance(uv, params.mouse);
        let freq = mix(20.0, 90.0, clamp(params.slider, 0.0, 1.0));
        let wave = sin(d * freq - params.time * 6.0);
        let atten = exp(-d * 3.0);
        let intensity = (0.5 + 0.5 * wave) * atten;
        var col = vec3<f32>(intensity, 0.15 + 0.55 * atten, 1.0 - intensity * 0.85);
        return vec4<f32>(col.b, col.g, col.r, 1.0);
    }
    ```

  - `pub(in ...) fn warp_source_bytes() -> Vec<u8>` — 512×512 RGBA rows,
    `512 * 512 * 4` bytes, alpha 255: 32-px checker; even tiles a radial
    gradient (`(255 - c, c, 96)` with `c = radial * 255` from the tile
    center), odd tiles a linear blend (`(24, fx * 255, fy * 255)` from the
    tile-local fractions). Pure byte writes, no `image` types.
- Tests (`demos.test.rs` additions): for every `DEMOS` entry,
  `naga::front::wgsl::parse_str(spec.wgsl)` is `Ok` and the module has two
  entry points (`vs_main`, `fs_main`); each source contains the readback
  swizzle marker `vec4<f32>(col.b, col.g, col.r` (drops are caught here, not
  on the GPU); no source contains the reserved identifier `let target =`
  (regression pin for the naga catch); `warp_source_bytes().len() ==
  512 * 512 * 4` and the alpha byte of the first, middle, and last pixel is
  255.
- Done-check: `cargo test -p gpui-starter --lib shaders::demos` passes,
  including the four naga validations.

## Phase 3: `ShaderState` entity

### Step 3.1: State core

- Files: `src/features/pages/shaders/state.rs`, `state.test.rs`.
- Build (private fields; platform-neutral; `FpsTracker` and clock from the
  bridge and platform modules):

  ```rust
  pub(in crate::features::pages::shaders) struct ShaderState {
      demo: DemoId,
      playing: bool,
      time: f32,
      delta: f32,                   // last measured frame delta, seconds
      slider: f32,
      mouse: (f32, f32),            // uv in [0,1]^2, default (0.5, 0.5)
      frame_in_flight: bool,
      last_frame_started_at: Option<crate::platform::clock::Instant>,
      fps: crate::features::wgpu::FpsTracker,
      displayed: Option<Arc<RenderImage>>,
      stale_frames: Vec<Arc<RenderImage>>,   // cap 4
      runtime_error: Option<String>,
      frames_rendered: u64,
  }
  ```

  Methods (mutators take `&mut Context<Self>`; one `cx.notify()` per
  coherent change per the coding guides): `new(demo: DemoId, cx) -> Self`
  (`playing = !cx.reduce_motion()`, `GPUI/app.rs:1222`; slider from
  `demo(demo).slider.default`; `delta = 1.0 / 60.0`; no notify in a
  constructor), `demo()`, `set_demo(id, cx)` (reset `time = 0.`, `mouse` to
  `(0.5, 0.5)`, `slider` to the new spec default, clear error, notify),
  `is_playing()`, `toggle_playing(cx)` (also clears `frame_in_flight` when
  pausing, notify), `slider()`, `set_slider(v, cx)` (clamped to the active
  spec's `[min, max]`, notify), `mouse()`, `set_mouse(uv, cx)` (clamped to
  `[0,1]`, **no notify** — consumed by the next render task; also the reason
  it is safe to call from paint-adjacent tool dispatch),
  `is_frame_in_flight()`, `mark_frame_started(now, cx)` (sets in-flight +
  `last_frame_started_at`, no notify), `frame_done(now, image:
  Arc<RenderImage>, cx)` (compute `delta = clamp(now - last_start, 0..
  0.25s)` and store it, `time += delta`, push the old `displayed` into
  `stale_frames` capped at 4, set `displayed`, `fps.push(now)`,
  `frames_rendered += 1`, clear in-flight, one `cx.notify()`),
  `take_stale_frames() -> Vec<Arc<RenderImage>>` (swap out, **no notify** —
  called from the ticker's paint), `displayed()`, `fps() -> u32`,
  `runtime_error() -> Option<&str>`, `frame_failed(err: String, cx)`
  (clears in-flight, stores error, notify), `clear_error(cx)`,
  `pacing_wait(now) -> Duration` (`FRAME_INTERVAL.saturating_sub(now -
  last_frame_started_at.unwrap_or(now))`, `const FRAME_INTERVAL: Duration =
  Duration::from_millis(1000 / 60)` — `Duration` from
  `crate::platform::clock`), `params_snapshot(spec: &DemoSpec) ->
  ShaderParams` (resolution from the spec, pure read).
- Tests (`state.test.rs`, `#[gpui_kit::test]` with `cx.new`, subscription
  counting via `cx.subscribe` on a helper entity or an `cx.observe` counter):
  `set_demo` resets `time`/`mouse`/`slider` and clears the error;
  `set_slider` clamps to the spec; `toggle_playing` flips and clears
  in-flight when pausing; `frame_done` moves the old `displayed` into
  `stale_frames`, advances `time` by the measured delta, and notifies
  exactly once per call; `frame_failed` stops the in-flight flag and stores
  the message; `pacing_wait` returns zero when the last start is older than
  the interval and the full remainder otherwise; `take_stale_frames` drains
  and leaves the queue empty; `set_mouse` clamps and does not notify.
- Done-check: `cargo test -p gpui-starter --lib shaders::state` passes.

## Phase 4: Native shader runtime (over plan 11's bridge)

Precondition for both steps: `test -d src/features/wgpu && grep -n "pub use
engine::WgpuEngine" src/features/wgpu/mod.rs` succeed. If not, land plan 11
first; do not substitute a private device stack.

### Step 4.1: `ShaderRuntime` construction

- Files: `src/features/pages/shaders/runtime.rs` (module declared in
  `mod.rs` under `#[cfg(not(target_family = "wasm"))]`), `runtime.test.rs`
  (test module cfg-gated the same way).
- Build:
  - `pub(in crate::features::pages::shaders) struct ShaderRuntime` with
    private fields: `engine: Arc<crate::features::wgpu::WgpuEngine>`,
    `uniform: wgpu::Buffer` (32 bytes, `UNIFORM | COPY_DST`), `pipelines:
    [wgpu::RenderPipeline; 4]`, `bind_groups: [wgpu::BindGroup; 4]`,
    `targets: [RenderTarget; 4]` where `RenderTarget { texture:
    wgpu::Texture, view: wgpu::TextureView }` (one pair per demo at its spec
    resolution; readback buffers are created per call inside the bridge's
    `read_texture_bytes`, so none are retained here).
  - `pub(in ...) fn new(engine: Arc<WgpuEngine>) -> Self` (synchronous and
    infallible — wgpu's `create_*` calls report through the uncaptured
    error handler, and the naga tests have already pinned the shaders):
    1. Uniform buffer: `engine.device().create_buffer(&wgpu::BufferDescriptor
       { label: Some("shader-canvas-uniform"), size: 32, usage:
       wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
       mapped_at_creation: false })`.
    2. Bind group layouts: layout A (Gradient, Raymarch, Ripple) — one
       entry, binding 0 uniform, `visibility: wgpu::ShaderStages::FRAGMENT`,
       `ty: wgpu::BindingType::Buffer(wgpu::BufferBindingLayout { ty:
       wgpu::BufferBindingType::Uniform, has_dynamic_offset: false,
       min_binding_size: None })`, `count: None`
       (`WTYPES/binding.rs:323-358`). Layout B (Warp) — the same binding 0
       plus binding 1 texture (`BindingType::Texture(
       wgpu::TextureBindingLayout { sample_type:
       wgpu::TextureSampleType::Float { filterable: true },
       view_dimension: wgpu::TextureViewDimension::D2, multisampled: false })`)
       and binding 2 sampler (`BindingType::Sampler(
       wgpu::SamplerBindingLayout { ty:
       wgpu::SamplerBindingType::Filtering })`).
    3. Per demo: shader module
       (`device.create_shader_module(wgpu::ShaderModuleDescriptor { label:
       Some("shader-canvas demo"), source:
       wgpu::ShaderSource::Wgsl(spec.wgsl.into()) })` — `ShaderSource` at
       `WGPU29/api/shader_module.rs:185`), pipeline layout from the demo's
       bind group layout, then
       `device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
       label: Some("shader-canvas pipeline"), layout: Some(&pipeline_layout),
       vertex: wgpu::VertexState { module: &module, entry_point:
       Some("vs_main"), compilation_options: Default::default(), buffers:
       &[] }, primitive: wgpu::PrimitiveState::default(), /* TriangleList */
       depth_stencil: None, multisample: Default::default(), fragment:
       Some(wgpu::FragmentState { module: &module, entry_point:
       Some("fs_main"), compilation_options: Default::default(), targets:
       &[Some(wgpu::ColorTargetState { format:
       wgpu::TextureFormat::Rgba8Unorm, blend: None, write_mask:
       wgpu::ColorWrites::ALL })] }), multiview_mask: None, cache: None })`
       (`multiview_mask` spelling per `WGPU29/api/render_pipeline.rs:238`).
    4. Per demo target: `create_texture` at the spec size
       (`Extent3d { width, height, depth_or_array_layers: 1 }`, format
       `Rgba8Unorm`, usage `RENDER_ATTACHMENT | COPY_SRC` — the exact usage
       plan 11's consumption notes require) and `texture.create_view(
       &wgpu::TextureViewDescriptor::default())`.
    5. Warp extras: source texture `Rgba8Unorm` 512×512, usage
       `TEXTURE_BINDING | COPY_DST`; upload once via
       `engine.queue().write_texture(wgpu::TexelCopyTextureInfo { texture:
       &warp_texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect:
       wgpu::TextureAspect::All }, &demos::warp_source_bytes(),
       wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(512 *
       4), rows_per_image: Some(512) }, size)` (`WGPU29/api/queue.rs:234-240`;
       `write_texture` has no 256-alignment requirement,
       `WTYPES/texture.rs:810-817`); sampler
       `device.create_sampler(&wgpu::SamplerDescriptor { label: Some(
       "shader-canvas sampler"), mag_filter: wgpu::FilterMode::Linear,
       min_filter: wgpu::FilterMode::Linear, ..Default::default() })` — the
       gpui-renderer shape (`gpui-pre-wgpu-0.3.8/src/wgpu_renderer.rs:1238-1243`),
       defaults carry `ClampToEdge` addressing.
    6. Bind groups: `device.create_bind_group(&wgpu::BindGroupDescriptor {
       layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource:
       wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &uniform,
       offset: 0, size: None })] })`; the warp group adds binding 1
       (`BindingResource::TextureView(&warp_view)`) and binding 2
       (`BindingResource::Sampler(&sampler)`).
- Tests (`runtime.test.rs`): none yet (construction needs a device; covered
  by the ignored smoke in Step 4.2).
- Done-check: `cargo check -p gpui-starter` passes.

### Step 4.2: `render_frame` and the GPU smoke test

- Files: `src/features/pages/shaders/runtime.rs`, `runtime.test.rs`.
- Build:
  - `pub(in ...) fn render_frame(&self, spec: &DemoSpec, params:
    ShaderParams) -> anyhow::Result<Arc<RenderImage>>` (synchronous;
    called only from a background thread; `&self` — nothing here mutates the
    stored handles):
    1. `self.engine.queue().write_buffer(&self.uniform, 0,
       &params.to_bytes())`.
    2. `let target = &self.targets[spec.id.ix()];` then encode one render
       pass: `encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
       label: None, color_attachments: &[Some(
       wgpu::RenderPassColorAttachment { view: &target.view, depth_slice:
       None, resolve_target: None, ops: wgpu::Operations { load:
       wgpu::LoadOp::Clear(wgpu::Color::BLACK), store:
       wgpu::StoreOp::Store } })], depth_stencil_attachment: None,
       timestamp_writes: None, occlusion_query_set: None })`
       (`WGPU29/api/render_pass.rs:603-655`); `set_pipeline`,
       `set_bind_group(0, .., &[])`, `pass.draw(0..3, 0..1)` — the
       fullscreen triangle covers the target, so the clear color never
       shows.
    3. `self.engine.queue().submit(Some(encoder.finish()))`. Queue order
       guarantees the bridge's copy submission (inside
       `read_texture_bytes`) observes this render.
    4. `let bgra = self.engine.read_texture_bytes(&target.texture,
       spec.width, spec.height).context("reading shader frame back")?;`
       (bridge method: blocking, tightly-packed rows, texture component
       order — BGRA here because the fragments write `b,g,r`).
    5. `Ok(crate::features::wgpu::bgra_to_render_image(spec.width,
       spec.height, bgra))`.
    Error strings carry the failing step via `anyhow::Context` so the page
    panel is actionable (`anyhow` is a workspace dependency,
    `Cargo.toml:24`).
  - `runtime.test.rs`: one `#[gpui_kit::test]` async test marked
    `#[ignore = "needs a working wgpu adapter (GPU or lavapipe); run with --ignored locally"]`:
    `WgpuEngine::new().await`, build the runtime, `render_frame` one
    Gradient frame with `ShaderParams { time: 0.5, .. }`, then through
    `RenderImage`'s readers (`as_bytes(frame_index)` at `GPUI/assets.rs:74`,
    `size(frame_index)` at `:81`): assert `as_bytes(0).len() ==
    512 * 512 * 4` and that the center pixel's bytes differ from the
    top-left corner's (non-uniform content). CI has no GPU guarantee; the
    ignore keeps
    `cargo test --workspace` green.
- Done-check: `cargo clippy -p gpui-starter --all-features` reports nothing
  new for `runtime.rs`; `cargo test -p gpui-starter --lib shaders::runtime
  -- --ignored` passes on this machine (adapter present).

## Phase 5: Page UI (native)

### Step 5.1: Page structure, toolbar, slider, `install_demo`

- Files: `src/features/pages/shaders/mod.rs`.
- Build:
  - `ShaderCanvasPage` fields:
    `canvas: Entity<CanvasState>`, `shader: Entity<ShaderState>`,
    `slider: Entity<SliderState>`,
    `#[cfg(not(target_family = "wasm"))] runtime: Option<Arc<ShaderRuntime>>`,
    `image_shape: Option<ShapeId>`,
    `_subscriptions: Vec<Subscription>`.
  - `wire_subscriptions(&mut self, cx: &mut Context<Self>)`: rebuild
    `_subscriptions` with (a) `cx.observe(&self.canvas, |_, _, cx|
    cx.notify())` — plan 01's documented mount requirement, (b)
    `cx.observe(&self.shader, |_, _, cx| cx.notify())`, (c)
    `cx.subscribe(&self.slider, |this, _, event, cx| if let
    SliderEvent::Change(value) = event { if let SliderValue::Single(v) =
    value { this.shader.update(cx, |s, cx| s.set_slider(*v, cx)); } })`
    (`GPUI/app/context.rs:94-108`; event shapes at `GB/slider.rs:15-33`).
  - `new(window, cx)`: create `CanvasState` (`cx.new(|_| CanvasState::new())`),
    `ShaderState::new(DemoId::Gradient, cx)`, and the slider
    (`cx.new(|_| SliderState::new().min(spec.slider.min).max(
    spec.slider.max).default_value(spec.slider.default))` — consuming
    builders, `GB/slider.rs:213-266`); wire subscriptions; on native,
    call `self.start_runtime(cx)` (Step 5.2) and detach.
  - `install_demo(&mut self, cx: &mut Context<Self>)`: read the active demo
    from `self.shader.read(cx).demo()`; rebuild the document —
    `self.canvas.update(cx, |s, cx| { let id = s.edit_document(cx, |d| {
    d.clear(); let layer = d.add_layer(); d.add_shape(layer,
    CanvasShape::image(Bounds { origin: point(px(0.), px(0.)), size: size(
    px(spec.width as f32), px(spec.height as f32)) }, blank_image()) });
    s.reset_view(cx); id })` storing the `Option<ShapeId>` in
    `self.image_shape`, where `blank_image()` = the bridge's
    `crate::features::wgpu::bgra_to_render_image(1, 1, vec![0, 0, 0, 0])`
    (a 1×1 transparent stand-in stretched over the doc rect); set the tool —
    ripple gets `Box::new(RippleTool::new(self.shader.downgrade(), spec.
    width as f32, spec.height as f32))` via `set_tool`, everything else
    `set_builtin_tool(BuiltinTool::Pan)` (`Entity::downgrade` at
    `GPUI/app/entity_map.rs:470`); rebuild the slider entity from the new
    spec and re-run `wire_subscriptions` (rebuilding the Vec drops the old
    subscriptions).
  - Render (native branch): `v_flex().min_h_full().gap_4().p_6()`:
    1. Header row `h_flex().w_full().flex_wrap().justify_between().
       items_center()`: the level-1 heading (Step 1.2) and a right
       `h_flex().gap_2().flex_shrink_0()` with one outline `Button`
       (`id "shader-toggle-play"`) labeled `localize(if playing {
       "shader_pause" } else { "shader_play" })`, toggling
       `self.shader.update(cx, |s, cx| s.toggle_playing(cx))`.
    2. Demo selector: `ButtonGroup::new("shader-demos").children(DEMOS.
       iter().map(|spec| Button::new(button_id(spec.id)).
       label(localize(label_key)).when(current == spec.id, |b| b.primary()).
       on_click(...)))` — active demo `.primary()` for a persistent selected
       state distinct from hover (design guides "Interaction states");
       click → `shader.update(set_demo)` + `install_demo(cx)`. Two private
       exhaustive-match helpers in `mod.rs`: `button_id(DemoId) -> &'static
       str` (`"shader-demo-gradient"` …; plain string ids — the `ElementId`
       `From` impls also cover tuples such as `(&'static str, usize)`
       (`GPUI/window.rs:7514-7580`), strings are used because the demo set
       is fixed and the ids read as diagnostics) and `label_key(DemoId) ->
       &'static str` mapping to `shader_demo_*`.
    3. Slider row `h_flex().gap_3().items_center()`: a `text_sm` label from
       the per-demo `shader_param_*` key, `Slider::new(&self.slider).
       w(rems(12.5))` (Styled sizing — no `.small()`, Corrections item 5;
       `brush.rs:353-356` pattern), and a `text_sm` numeric readout
       `format!("{:.2}", self.slider.read(cx).value().start())`
       (`SliderValue::start`, `brush.rs:98` pattern).
    4. Canvas region: `div().relative().flex_1().min_h_0().rounded(
       cx.theme().radius).overflow_hidden().border_1().border_color(
       cx.theme().border)` with children, in paint order: the canvas
       (`Canvas::new("shader-canvas", self.canvas.clone())` — fills the
       region per plan 01), the frame ticker (Phase 6, absolute full-size),
       and the FPS overlay `div().absolute().top(rems(0.5)).right(rems(
       0.5)).px_2().py_1().rounded_md().bg(cx.theme().popover).border_1().
       border_color(cx.theme().border).text_xs().text_color(cx.theme().
       muted_foreground)` showing `localize_with_args("shader_fps",
       &[("fps", &fps.to_string())])`, or `localize("shader_waiting")`
       before the first frame (`frames_rendered == 0`). The position
       builders are `Styled` methods generated by
       `gpui_macros::position_style_methods!()`
       (`GPUI/styled.rs:22,30`); in-repo precedent: `.top(rems(-0.3125))` at
       `src/gallery/sections/tables/list.rs:240` and the absolute-overlay
       idiom `.absolute().bottom(rems(-0.75)).left_0().right_0()` at
       `src/gallery/sections/layout/collapsible.rs:326-328`.
    5. Error state: when `runtime_error()` is `Some`, replace item 4's
       children with a bordered panel — `shader_error_title`, the error
       string in `div().text_sm().text_color(cx.theme().danger)`, and an
       outline Retry `Button` re-running `start_runtime(cx)`.
- Tests: none new (layout); Phases 6-7 cover behavior.
- Done-check: `cargo check -p gpui-starter` passes; manual `cargo run`
  optional here (formal QA in Verification).

### Step 5.2: Runtime init task

- Files: `src/features/pages/shaders/mod.rs`.
- Build: `start_runtime(&mut self, cx: &mut Context<Self>)`:
  `self.runtime = None;` clear the error; then
  `cx.spawn(async move |this, cx| { ... }).detach()` — the two-argument
  `Context::spawn` closure (`GPUI/app/context.rs:230-236`), `this: 
  WeakEntity<ShaderCanvasPage>`. Body:
  1. `let result = cx.background_executor().spawn(async move {
     let engine = std::sync::Arc::new(WgpuEngine::new().await?);
     anyhow::Ok(std::sync::Arc::new(ShaderRuntime::new(engine.clone())))
     }).await;` — engine request and pipeline construction both off the UI
     thread (`WgpuEngine::new` is async and `Send`; `ShaderRuntime::new` is
     plain CPU work riding the same background task).
  2. `let _ = this.update(cx, |page, cx| match result { Ok(runtime) => {
     page.runtime = Some(runtime); crate::capabilities::set(
     CAPABILITY_SHADERS, CapabilityStatus::supported_enabled(), cx);
     page.install_demo(cx); cx.notify(); } Err(err) => {
     crate::capabilities::set(CAPABILITY_SHADERS,
     CapabilityStatus::degraded(err.to_string()), cx); page.shader.update(
     cx, |s, cx| s.frame_failed(err.to_string(), cx)); cx.notify(); } });`
     — the capability writes run inside the update closure because
     `capabilities::set` takes `&mut App` (`capabilities.rs:64`) and
     `Context<Self>` derefs to `App` (`GPUI/app/context.rs:26-38`).
     Weak-entity update after teardown is an `Err`, not a panic
     (`GPUI/app/entity_map.rs:803-818`).
  Called once from `new` (after the entities exist) and from Retry.
- Tests: none (threaded init; the error path's state shape is covered by
  Step 3.1's `frame_failed` test).
- Done-check: `cargo check -p gpui-starter` passes; capability flips to
  degraded when the adapter is absent (manual: `WGPU_BACKEND=gl cargo run`
  on a headless box exercises the error panel).

## Phase 6: Frame loop and atlas drain

### Step 6.1: Ticker element

- Files: `src/features/pages/shaders/mod.rs` (private `render_ticker` fn).
- Build: a private helper (native render branch only) returning
  `gpui_kit::canvas(move |_, _, _| (), move |_, _, window, cx| { ... })`
  styled `.absolute().top_0().left_0().size_full()` so its bounds track the
  canvas region (`Canvas<T>: Styled`, `GPUI/elements/canvas.rs:91`; closure
  shapes at `:9-22`; absolute/full-size is the plan-11 ticker shape — the
  element inserts no hitbox, so it never intercepts pointer input). Paint
  closure body:
  1. Drain stale images: `for image in shader.update(cx, |s, _|
     s.take_stale_frames()) { let _ = window.drop_image(image); }`
     (`GPUI/window.rs:5035-5048`; `take_stale_frames` does not notify, so
     this paint-phase mutation is safe).
  2. Kick: read `shader.read(cx)`; if `!playing || frame_in_flight ||
     runtime.is_none() || error.is_some()` return. Else capture
     `WeakEntity<ShaderState>`, `WeakEntity<CanvasState>`,
     `WeakEntity<ShaderCanvasPage>`, the `Arc<ShaderRuntime>` clone, the
     `image_shape: ShapeId`, and the `&'static DemoSpec`, and register
     `window.on_next_frame(move |_, cx| { ... })`
     (`GPUI/window.rs:2650-2657`).
  3. `on_next_frame` body: `let Some(shader_ent) = shader_weak.upgrade()
     else { return };` re-check eligibility from `shader_ent.read(cx)`; if
     eligible: `let now = clock::Instant::now();` `shader_ent.update(cx,
     |s, cx| s.mark_frame_started(now, cx));` `let params =
     shader_ent.read(cx).params_snapshot(spec);` then
     `cx.spawn(async move |cx| { frame_task(...).await }).detach()` —
     `App::spawn`, one-argument closure (`GPUI/app.rs:2177-2182`).
  The closure set is rebuilt each frame by construction (the ticker element
  is re-created every render), which matches GPUI's clear-on-render listener
  semantics.
- Tests: none here (Step 6.2 covers the state machine).
- Done-check: `cargo check -p gpui-starter` passes.

### Step 6.2: Frame task body

- Files: `src/features/pages/shaders/mod.rs` (private `async fn
  frame_task(cx: &mut AsyncApp, ...)` invoked from the kick).
- Build:
  1. `let Some(shader) = shader_weak.upgrade() else { return };`
  2. Pacing: `let wait = shader.update(cx, |s, _| s.pacing_wait(
     clock::Instant::now()));` `cx.background_executor().timer(wait).await;`
     (`GPUI/executor.rs:183`). Re-check playing after the wait; bail
     without touching the in-flight flag (pausing already cleared it,
     Step 3.1).
  3. `let params = shader.update(cx, |s, _| s.params_snapshot(spec));`
  4. `let frame = cx.background_executor().spawn(async move {
     runtime.render_frame(spec, params) }).await;`
  5. On `Ok(image)`: if the canvas is alive, swap the shape —
     `let _ = canvas_weak.update(cx, |s, cx| s.edit_document(cx, |d| {
     d.update_shape(shape_id, |shape| if let CanvasShape::Image { image:
     slot, .. } = shape { *slot = image.clone(); }); })` (`update_shape`
     returns `Option` — `None` means the demo switched mid-flight; the next
     `install_demo` re-created the shape) — then `shader.update(cx, |s, cx|
     s.frame_done(clock::Instant::now(), image, cx));`.
  6. On `Err(err)`: route through the page weak entity so the capability
     write gets a `&mut App` — `let _ = page_weak.update(cx, |_, cx| {
     crate::capabilities::set(CAPABILITY_SHADERS,
     CapabilityStatus::degraded(err.to_string()), cx); });` then
     `shader.update(cx, |s, cx| s.frame_failed(err.to_string(), cx));`.
- Tests (`state.test.rs` addition, entity-level): `mark_frame_started` then
  `toggle_playing` (pause) clears the in-flight flag; a subsequent
  `frame_done` on the stale task's completion path leaves the entity
  consistent (in-flight false, exactly one stale push).
- Done-check: `cargo test -p gpui-starter --lib shaders` passes; manual run
  shows continuous animation with the FPS overlay holding ~60 and GPU
  process memory flat over a minute (atlas drain works).

## Phase 7: Ripple tool

### Step 7.1: `RippleTool`

- Files: `src/features/pages/shaders/tool.rs`, wired from `mod.rs` (private
  module, no cfg — all dependencies are wasm-clean; it is simply never
  installed on wasm), `tool.test.rs`.
- Build:
  - `struct RippleTool { shader: WeakEntity<ShaderState>, dims: (f32, f32) }`
    — the image shape's doc-space dimensions, captured at install.
  - `impl CanvasTool for RippleTool` (plan 01 contract): `id() ->
    "shader-ripple"` (diagnostics identifier, never UI copy), `cursor() ->
    CursorStyle::Crosshair` (precision manipulation), `pointer_down` and
    `pointer_move` both map `event.doc_position` to uv — `let uv =
    ((p.x.as_f32() / self.dims.0).clamp(0., 1.), (p.y.as_f32() /
    self.dims.1).clamp(0., 1.))` — then `let _ = self.shader.update(cx,
    |s, cx| s.set_mouse(uv, cx));`. The image shape's doc bounds start at
    `(0,0)`, so doc position divides directly by the dimensions. Updating
    `ShaderState` here is legal even though the tool runs inside the
    `CanvasState` lease: it is a different entity, and the entity map only
    panics on re-leasing the same entity
    (`GPUI/app/entity_map.rs:160-165`). `set_mouse` does not notify
    (Step 3.1), so no repaint storm while the pointer moves. `scroll`
    keeps the default `false` (wheel pans/zooms per plan 01's router
    rules).
- Tests (`tool.test.rs`, `#[gpui_kit::test]`): construct
  `Entity<CanvasState>` + `Entity<ShaderState>`, install the tool via
  `canvas.update(cx, |s, cx| s.set_tool(Box::new(RippleTool { shader:
  shader_entity.downgrade(), dims: (512., 512.) }), cx))`, then drive
  `s.dispatch_tool_event(cx, |tool, s, cx| tool.pointer_move(&ev, &ctx, s,
  cx))` (plan 01's pub(crate) dispatch seam) with a `CanvasPointerEvent`
  whose `doc_position` is `(128., 384.)`; assert `ShaderState::mouse() ==
  (0.25, 0.75)`. Add an out-of-bounds position asserting the clamp to
  `[0, 1]`.
- Done-check: `cargo test -p gpui-starter --lib shaders` passes; manual
  run: ripple demo waves track the cursor over the canvas; click recenters.

## Phase 8: Freeze and gates

### Step 8.1: Re-export and doc surface

- Files: `src/features/pages/shaders/mod.rs`.
- Build: final module header matches the Removal Recipe; the module's `pub`
  surface is exactly `CAPABILITY_SHADERS` + `ShaderCanvasPage` (both live in
  `mod.rs`, so there is no `pub use` to audit); doc comment on
  `ShaderCanvasPage::new` states that the page owns its entities and starts
  the engine init task on native.
- Tests: existing.
- Done-check: `grep -rn "pub use\|pub fn\|pub struct\|pub const" 
  src/features/pages/shaders/` shows only the contract items; `grep -rn
  "use gpui::" src/features/pages/shaders/` returns nothing (kit facade
  rule, `tests/qa_migration.rs:386-405`).

### Step 8.2: Full gates

- Run, in order: `cargo fmt --all -- --check`, then `cargo clippy
  --workspace --all-features`, then `cargo test --workspace --all-features`,
  then `just wasm-check`.
- Done-check: all four pass with zero new warnings; `cargo test -p
  gpui-starter --lib shaders` green; the `#[ignore]`d GPU test passes
  locally with `-- --ignored`.

## Platform & Wasm Gating

- cfg gates: `runtime.rs` is declared under
  `#[cfg(not(target_family = "wasm"))]` in `mod.rs`; the ticker, frame
  task, and native render branch sit inside the `#[cfg(not(...))]` arm of
  `ShaderCanvasPage::render`; the wasm arm renders the degradation panel
  (Step 1.3). `demos.rs`, `state.rs`, `tool.rs` compile on every target;
  `state.rs` touches only `crate::platform::clock::{Duration, Instant}`
  (`src/platform/clock.rs:1-8`) — no `std::time` in wasm-reachable code.
- Native-only dependencies: `wgpu =29.0.4` under
  `[target.'cfg(not(target_family = "wasm"))'.dependencies]`
  (`Cargo.toml:311`). `naga =29.0.4/wgsl-in` is a dev-dependency;
  `cargo check --lib` for wasm does not build dev-deps, so it never reaches
  the wasm gate. No `image` dependency is added (Corrections item 8).
- Capability registry (`canvas_shaders`):
  - native: `supported_enabled` at startup
    (`DEFAULT_ENABLED_CAPABILITIES`); `degraded(err)` when the engine
    request or a frame fails — the page then shows the error panel with
    Retry, which re-runs init and resets the capability to
    `supported_enabled` on success.
  - wasm: `supported: false, enabled: false, degraded: true` with the
    reason literal set in `init.rs` — "hidden with capability reason": the
    shader surface is replaced by the localized panel, while the page,
    sidebar entry, and deep link stay present so routing behaves
    identically across platforms.
- Platform behavior matrix: macOS/Windows/Linux — full page; wgpu selects
  metal/dx12/vulkan/gles by default backend enumeration inside plan 11's
  engine (a single wgpu instance is shared through the exact version pin).
  wasm — panel only, no wgpu, no animation loop. The ignored GPU test
  documents that CI boxes without adapters are expected (`.github/workflows/
  ci.yml` has no GPU job, survey §7).
- `just wasm-check` must keep passing: the lib includes the page module and
  its wasm-clean halves only.

## i18n

Catalog keys added to `locales/en.yml` and `locales/zh-CN.yml` in lockstep
(facade `crate::i18n::localize` / `localize_with_args`, seams survey §5;
flat top-level scalars, quoted, `[a-z0-9_]` names):

| key | en | zh-CN |
|---|---|---|
| `nav_shader_canvas` | "Shader Canvas" | "着色器画布" |
| `shader_title` | "Shader Canvas" | "着色器画布" |
| `shader_demo_gradient` | "Gradient" | "渐变" |
| `shader_demo_raymarch` | "Raymarch" | "光线步进" |
| `shader_demo_warp` | "Image warp" | "图像扭曲" |
| `shader_demo_ripple` | "Ripple" | "波纹" |
| `shader_param_gradient` | "Color speed" | "色彩流速" |
| `shader_param_raymarch` | "Orbit speed" | "环绕速度" |
| `shader_param_warp` | "Warp amount" | "扭曲强度" |
| `shader_param_ripple` | "Wave frequency" | "波纹频率" |
| `shader_play` | "Play" | "播放" |
| `shader_pause` | "Pause" | "暂停" |
| `shader_fps` | "FPS: %{fps}" | "帧率：%{fps}" |
| `shader_waiting` | "Waiting for first frame" | "等待首帧" |
| `shader_error_title` | "Shader renderer failed" | "着色器渲染失败" |
| `shader_retry` | "Retry" | "重试" |
| `shader_unavailable_wasm` | "WGSL shader rendering draws through an offscreen wgpu device, which the web build does not provide." | "WGSL 着色器渲染依赖离屏 wgpu 设备，Web 构建暂不提供。" |

Notes: the per-demo param labels switch with the active demo; the FPS value
formats through `%{fps}` (`localize_with_args`,
`src/services/i18n.rs:7-13`); short labels carry no terminal punctuation
and the Chinese strings use full-width punctuation only inside sentences
(design guides, "Capitalization, punctuation, and symbols"). Tool id
`"shader-ripple"` and `DemoId` variants are identifiers, never localized.
The localization scanner sees only literal first arguments, so the indirect
`localize(self.title_key())` nav key rides on the lockstep test
(`tests/qa_localization.rs:188-198`), the survey §5 blind spot.

## Design Compliance

- Layout (design guides "Choose a stable shell", "Responsive desktop
  windows"): single-workspace page — control rows on top, canvas region
  `flex_1().min_h_0()` taking the surplus. The canvas is the primary object
  and gets the dominant area; controls are chrome. Window narrowing wraps
  the header row (`flex_wrap`, `notifications.rs:42-43` pattern) and never
  hides the play control (no hover-only actions).
- Hierarchy ("Visual language"): one level-1 heading, one ButtonGroup
  (segmented demo choice with persistent selected state via `.primary()`),
  one slider row, one stats overlay. No cards-in-cards; the error panel is
  a single bordered region, not an Alert stack.
- Tokens ("Color and themes"): overlay surface `cx.theme().popover` +
  `border`; FPS text `muted_foreground`; error text `danger`; canvas frame
  `border` + `cx.theme().radius`; heading default foreground. No raw
  hex/rgb/hsla in page code. The shader pixels are raster data whose color
  is the content — exempt per the guides' "audited data/raster content".
- Spacing/density ("Radius, spacing, and density", "Spatial grammar"):
  page padding `p_6`, section gap `gap_4` (separate sections), header gap
  `gap_2` (closely related controls), slider row `gap_3`; text sizes via
  `text_sm`/`text_xs` rem helpers; slider width via `w(rems(12.5))`. The
  only geometry literals are the overlay's two `rems(0.5)` insets — rem
  scale, documented as the overlay's optical offset.
- Interaction states: kit Button/ButtonGroup/Slider own hover, pressed,
  focus-visible, and disabled treatments; the active demo button stays
  visually distinct (primary) — "Selected/checked: persistent state
  distinct from hover". Retry is an ordinary outline button (not primary —
  it is not a decision-area commit). Play/Pause label reflects state, so
  playback is never state-by-color-only.
- Motion ("Motion", "Zoom, base font, and rem"): the animated shaders are
  content the user explicitly plays (video-player class,
  `GPUI/window.rs:2670-2674` doc), and the loop honors `App::reduce_motion()`
  by starting paused; no decorative chrome animation is added. FPS numbers
  and error text change without animation.
- A11y ("Accessibility checklist"): the heading announces the page; every
  control is a labeled kit component (keyboard path: Tab to Play/demo
  buttons/slider, Enter to activate); the FPS overlay is supplementary
  `text_xs` muted status text, not the sole carrier of state (the play
  button label carries playback state). Known limitation, recorded in
  Risks: plan 01's canvas paints no accessibility node in v1, so shader
  pixels are invisible to AT; the toolbar and overlay carry the semantic
  content.
- Pointer conventions: the ripple tool uses `CursorStyle::Crosshair`
  (precision manipulation), other demos keep the pan tool's hand cursors —
  cursors reinforce the active manipulation, they do not decorate. Default
  arrow on all buttons.

## Verification

Repo gates, in this order (AGENTS.md; first three mandatory before "done"):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

Feature-targeted automation (all wired in the phases):

```sh
cargo test -p gpui-starter --lib shaders            # demos (naga), state, tool
cargo test --test e2e_navigation                    # page count bumped
cargo test --test qa_localization                   # key lockstep
cargo test -p gpui-starter --lib shaders::runtime -- --ignored   # GPU smoke, local only
```

Manual QA on `cargo run` (scratch tree, not committed):

1. Sidebar shows "Shader Canvas" last with the palette icon; deep link
   `gpui-starter://shader-canvas` opens it.
2. Gradient demo animates; FPS overlay climbs to ~60 and holds; GPU memory
   stays flat over 60 s (atlas drain verified).
3. Raymarch renders the sphere/box scene with the camera orbiting; slider
   changes orbit speed.
4. Warp demo distorts the checker/radial pattern; slider sweeps from none
   to full warp.
5. Ripple demo: waves track the cursor over the canvas; click also
   recenters; slider changes ring frequency.
6. Pause freezes frames and drops CPU/GPU usage to idle; Play resumes.
7. Canvas pan (drag) and ctrl/cmd+wheel zoom still work under every demo
   (plan 01 router untouched); the image shape scales with zoom.
8. Light and dark themes: overlay legible on both; no raw-color artifacts.
9. Reduce-motion (set the OS flag): page opens paused; playing works after
   pressing Play.
10. Error path: run with an unusable backend override (e.g. `WGPU_BACKEND=gl`
    headless); the page shows the error panel, Retry recovers once a
    backend is available; the diagnostics page lists `canvas_shaders`
    degraded.
11. Wasm build (`just wasm-check` compile-only here): the browser build
    renders the capability-reason panel; diagnostics lists `canvas_shaders`
    unsupported.

Acceptance criteria: all four repo gates pass; the feature-targeted tests
pass including the four naga shader validations; the manual checklist is
confirmed on a scratch tree or waived with a note naming which items were
checked and how.

## Risks & Open Questions

Ranked risks:

1. **Landing-order coupling on plan 11.** Phases 4-6 import
   `crate::features::wgpu::{WgpuEngine, bgra_to_render_image, FpsTracker}`;
   if plan 11 has not landed, those phases cannot compile. The plan states
   the `test -d` precondition at every entry point instead of carrying a
   fallback device stack (the spec names the bridge as the fallback; a
   second private runtime would undo the dedup plan 11's risk 5 expects).
   If sequencing forces 07 first, land plan 11's `engine.rs`/`frame.rs`
   subset ahead of its page rather than forking the bridge.
2. **Readback blit cost.** Every frame crosses GPU→CPU (`read_texture_bytes`
   ≈ 1 MB at 512², ~60 MB/s at 60 fps) and re-uploads through the sprite
   atlas. The BGRA swizzle is free (GPU-side in the fragment shaders). This
   is the only supported path — no surface/texture interop exists
   (`GPUI/platform.rs:1144` trait object unreachable, survey section g). If
   raymarch at 384² misses pacing on low-end GPUs, lower its `DemoSpec`
   resolution before touching the loop; the 384 pin in `demos.test.rs`
   forces a conscious change.
3. **Contract churn in plans 01 and 11.** Phases 5-7 consume plan 01's
   canvas contract verbatim and Phase 4 consumes `WgpuEngine`'s signatures.
   If either shifts (`update_shape`, `read_texture_bytes` return shape,
   `FpsTracker` API), the matching phase needs an edit; both dependency
   plans document their contracts as load-bearing for dependents, and plan
   08's line-number citations already constrain 01's contract block layout.
4. **Atlas growth if the ticker stops painting while frames keep landing.**
   No LRU in the atlas (`GPUI/window.rs:4948-4960`); `stale_frames` is
   capped at 4 and drained on every ticker paint; the ticker is mounted
   whenever the canvas region is, so paint cycles coincide. If a future
   refactor mounts the canvas without the ticker, the cap bounds the leak
   at four tiles — visible in GPU memory, not unbounded.
5. **wgpu version coupling.** The `=29.0.4` pin must track whatever
   gpui-pre-wgpu resolves to; a future gpui bump that moves wgpu to 30.x
   breaks the pin loudly (two copies or a resolution error), which is the
   intended failure mode. `tests/qa_migration.rs:131-146` guards stray
   copies.
6. **Frame loop liveness.** The loop is paint-driven (notify → paint →
   `on_next_frame` → spawn). A hidden/minimized window stops painting and
   the loop halts; any repaint (focus, resize, route return) restarts it
   because the ticker's kick condition is re-evaluated on every paint. No
   frame is left in flight across the halt: pausing clears the flag, and a
   completed task's weak-entity updates no-op after teardown.
7. **GPU tests are `#[ignore]`d.** CI runners have no guaranteed adapter;
   shader correctness is pinned by the naga parse tests (which caught a
   reserved-keyword error during planning — Research Digest) plus local
   `--ignored` runs. Visual verification stays manual (checklist items
   2-5).
8. **Canvas a11y gap** (plan 01 v1 paints no accessibility node): shader
   content is invisible to assistive technology; only toolbar/FPS semantics
   exist. Tracked as plan 01's follow-up, not fixable here.

Open questions (each with what was tried):

- **Implementation order of plans 02-11.** Each adds a sidebar page and
  bumps `e2e_navigation`; absolute counts/positions are therefore written
  relative ("append last at land time", "read the count, bump by one").
  Tried: `ls docs/plans/` (01-11, 16, `_survey` on disk); no implementation
  order document exists. This plan additionally gates Phases 4-6 on plan
  11's bridge being on disk (risk 1).
- **Adapter-less CI.** Not verifiable from here whether GitHub runners
  expose a lavapipe device; the GPU test is `#[ignore]`d so CI outcome does
  not depend on it. Tried: grep for "gpu" across `.github/workflows/ci.yml`
  — zero hits (via survey §7; not re-run this session beyond reading the
  survey).
- **Icon variant reachability — resolved.** The earlier draft picked
  `Orbit` after verifying only that `orbit.svg` exists in the full assets
  catalog; that is not sufficient, because the component enum the app's
  sidebar uses is generated from the DEFAULT 106 only
  (`default-icons.txt`; generated enum verified in
  `target/debug/build/gpui-kit-assets-*/out/icon_name.rs`), and the app
  registers the default asset source. Revision: `IconName::Palette`,
  verified present in the default subset. The residual compile-time check
  (only compilation proves the generated name) still applies to any future
  icon swap.
- **`ShaderRuntime::new` infallibility.** wgpu's `create_*` calls report
  through the uncaptured error handler (panic), not `Result`; the naga
  tests pin the shader sources and the descriptors are fixed constants, so
  no `Result` wrapper was added. If a backend rejects a descriptor at
  runtime (risk shape plan 11 also carries), the recovery is the panic
  traceback, not a UI state. Not exercised on every backend during
  planning.
- **No repo gates were run during planning** (doc-only scope, matching the
  survey and plan 01/11 conventions). The Verification section states the
  exact commands for the implementer. The one executable check run this
  session is the naga 29.0.4 parse of the four WGSL sources in a scratch
  binary (command and output in Research Digest "WGSL validation").
- **Em-dash density left as-is** (review note: ~105 occurrences vs 12 in
  plan 01). A mechanical rewrite of that many sentences in a frozen
  technical plan risks altering statements, so the reformat is deferred to
  a prose-only pass after implementation lands; no banned words are present
  (grep: zero hits for delve/seamless/leverage).
