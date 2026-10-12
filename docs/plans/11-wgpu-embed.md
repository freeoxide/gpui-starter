# 11 WGPU Embed Plan

## Goal

A "WGPU" sidebar page renders three GPU demos through an offscreen wgpu device —
a compute-shader particle system (default 1M particles), a textured 3D rotating
mesh with a depth buffer, and the same mesh through a post-process pass, with
each frame read back on a background thread and displayed as a gpui image at a
paced ~60 fps. The wgpu bridge (`WgpuEngine` + pure frame helpers) is a
reusable crate-internal foundation other features import; the page works on
macOS, Windows, and Linux, and shows a capability-reason panel on wasm.

## Dependencies

None at build time; this is a foundation plan. `docs/plans/07-canvas-shaders.md`
consumes this plan's Interface Contract (its Dependencies section, 07:33-47:
`WgpuEngine::new/device/queue/read_texture_bytes`, `frame::
bgra_to_render_image`, `frame::FpsTracker`, plus the drain obligation) and
sequences plan 11 before its own Phase 4 — the bridge's pub surface is
load-bearing for 07. Inputs read: `docs/plans/_survey/seams.md` (sections 1,
3, 4, 5, 6, 7, 8, 9), `docs/plans/_survey/ecosystem.md` (sections a, c, d, e,
g), and plan 07's current revision (which builds on this bridge, 07:289-299).
Every load-bearing claim was re-verified against the crate sources on disk
this session (2026-10-12); two survey/07-draft facts were corrected (see
Research Digest "Corrections").

## Research Digest

Path key (crate sources read this session):

| Short | Path |
| --- | --- |
| `GPUI` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-0.3.8/src` |
| `WGPU29` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/wgpu-29.0.4/src` |
| `WTYPES` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/wgpu-types-29.0.4/src` |
| `GPWGPU` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-wgpu-0.3.8/src` |
| `KIT` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-kit-0.7.1/src` |
| `REPO` | `/home/thragg/fo/gpui-starter` |

### Image upload path (the spec's central question)

- `RenderImage` doc pins the byte format: **BGRA**
  (`GPUI/assets.rs:42` "A cached and processed image, in BGRA format").
  `RenderImage::new(data: impl Into<SmallVec<[Frame; 1]>>)` allocates a fresh
  `ImageId` from a global atomic counter (`GPUI/assets.rs:61-70`).
- `Frame` is `image::Frame` (`GPUI/assets.rs:4` imports it privately, gpui
  does **not** re-export the `image` crate; grep for `pub use image` in
  `GPUI/gpui.rs` returns nothing). `image::Frame::new(buffer: RgbaImage)` at
  `image-0.25.10/src/animation.rs:88`;
  `ImageBuffer::from_raw(width, height, buf) -> Option` at
  `image-0.25.10/src/images/buffer.rs:680`. The app therefore needs its own
  `image 0.25` dependency (no codec features), lock already carries 0.25.10.
- `[Frame; 1]` satisfies the `Into<SmallVec<[Frame; 1]>>` bound: smallvec
  1.15.2 has `impl<A: Array> From<A> for SmallVec<A>`
  (`smallvec-1.15.2/src/lib.rs:2070`); `[T; N]: Array`. So
  `RenderImage::new([frame])` type-checks directly, no `SmallVec::from_vec`
  detour.
- Display path: `Arc<RenderImage>` converts into
  `ImageSource::Render` (`GPUI/elements/img.rs:107-110`; enum at `:42-51`);
  `img(source: impl Into<ImageSource>) -> Img` (`img.rs:201-211`). The `Img`
  element paints via
  `window.paint_image(bounds, object_fit_bounds, corner_radii, data, frame_index, grayscale)`
  (`img.rs:496-504`; `paint_image` at `GPUI/window.rs:4919-4927`). `object_fit`
  is a `StyledImage` builder (`img.rs:159`); `ObjectFit::{Fill, Contain, Cover,
  ScaleDown, None}` (`GPUI/style.rs:29-40`).
- Atlas has **no eviction**: `get_or_insert_with` inserts tiles keyed by
  `(ImageId, frame_index)` and nothing removes them
  (`GPUI/window.rs:4948-4960`; plan 07's digest, confirmed by grep, no LRU in
  this snapshot). One new `RenderImage` per frame therefore grows the atlas
  unboundedly unless the old image is dropped explicitly.
- `window.drop_image(Arc<RenderImage>) -> Result<()>` removes every atlas tile
  of the image; no paint-phase assert (verified read,
  `GPUI/window.rs:5037-5048`).

### No GPU-surface share with gpui internals (why out of scope)

- The spec's "(blade)" backend does not exist in this tree: `grep -i blade`
  over `Cargo.lock` and `ls ~/.cargo/registry/src/*/ | grep -i blade` both
  return nothing (run this session). This family's renderers are Metal
  (gpui-pre-apple) and wgpu 29.0.4 (gpui-pre-wgpu), internal crates with no
  public device/queue/texture API (grep for `create_shader|register_shader|`
  `paint_shader` across `GPUI/`, `GPWGPU/` hits only the renderer's own WGSL
  modules at `GPWGPU/wgpu_renderer.rs:694-700`).
- `PlatformWindow: HasWindowHandle + HasDisplayHandle`
  (`GPUI/platform.rs:1144`), but the trait object is unreachable from app code
  (survey section g; `Window::platform_window` is `pub(crate)`,
  `GPUI/window.rs:1170`). Even on Linux, where both sides would use wgpu, an
  app `wgpu::Instance` is a distinct object from gpui's internal one and wgpu
  exposes no external-memory import path here. The only sanctioned crossing is
  CPU bytes through `RenderImage`.
- `window.paint_surface(bounds, CVPixelBuffer)` exists but is macOS/iOS-only
  (`#[cfg]` at `GPUI/window.rs:5020-5021`) and takes a `CVPixelBuffer`; wgpu
  has no zero-copy export to one in this dependency set. Not pursued.

### wgpu 29.0.4 (the version already in the lock)

`Cargo.lock` pins `wgpu 29.0.4` via gpui-pre-wgpu
(`GPWGPU/Cargo.toml:145-146` dependency table); `naga 29.0.4` is the single
lock copy; `pollster 0.4.0` is already locked (wgpu 29's own dep). Signatures
verified in the 29.0.4 sources:

- `Instance::new(desc: InstanceDescriptor)` **by value**
  (`WGPU29/api/instance.rs:63`). `InstanceDescriptor` fields: `backends`,
  `flags`, `memory_budget_thresholds`, `backend_options`, `display`
  (`WTYPES/instance.rs:27-56`). Native pattern to copy field-for-field:
  `GPWGPU/wgpu_context.rs:473-481`.
- `instance.request_adapter(&RequestAdapterOptions) -> impl Future<Output =
  Result<Adapter, RequestAdapterError>>` (`WGPU29/api/instance.rs:167-171`).
  Options fields `power_preference`, `force_fallback_adapter`,
  `compatible_surface` (`WTYPES/adapter.rs:36-46`); `compatible_surface: None`
  is the headless case (the doc's WebGL2 warning does not apply, this runtime
  is native-only).
- `adapter.request_device(&DeviceDescriptor) -> impl Future<Output =
  Result<(Device, Queue), RequestDeviceError>>` (`WGPU29/api/adapter.rs:58-64`).
- `Device::poll(poll_type: PollType) -> Result<PollStatus, PollError>`
  (`WGPU29/api/device.rs:95`); `PollType::{Wait { submission_index, timeout },
  Poll}` with const `PollType::wait_indefinitely()`
  (`WTYPES/lib.rs:229-247, 257-267`).
- `Buffer::slice(bounds: impl RangeBounds<BufferAddress>) -> BufferSlice`
  (`WGPU29/api/buffer.rs:312`, `impl Buffer` at `:233`) then
  `BufferSlice::map_async(mode: MapMode, callback)` (`:566`, `impl<'a>
  BufferSlice` at `:503`). The 3-param convenience
  `Buffer::map_async(mode, bounds, callback)` sits at `:390` and just
  delegates to `self.slice(bounds).map_async(...)`; `MapMode` at `:864`. The
  gpui readback calls `slice(..).map_async(...)`, this plan does the same.
- `encoder.copy_texture_to_buffer(...)` with `TexelCopyBufferInfo`
  (`WGPU29/api/command_encoder.rs:45,146`).
- `wgpu::COPY_BYTES_PER_ROW_ALIGNMENT = 256` (`WGPU29/lib.rs:154` re-export of
  `WTYPES/lib.rs:169`); readback rows must be padded to it.
- `wgpu::util::DeviceExt::create_buffer_init(&BufferInitDescriptor)` where
  `BufferInitDescriptor { label, contents: &[u8], usage }`
  (`WGPU29/src/util/device.rs:7-20`), plain bytes, no bytemuck needed.
- `TextureUsages::{COPY_SRC = 1<<0, COPY_DST = 1<<1, RENDER_ATTACHMENT =
  1<<4}` and `BufferUsages::COPY_SRC = 1<<2` (`WTYPES/texture.rs:178-222`).
- `BufferDescriptor` has `mapped_at_creation` (`WTYPES/buffer.rs:31`).
- `TextureFormat::Depth24Plus` (`WTYPES/texture/format.rs:246`),
  `TextureFormat::Rgba8Unorm` (format.rs, same enum).
- The readback pattern to mirror is gpui's own headless path:
  copy_texture_to_buffer → `queue.submit` → `map_async(Read)` over an mpsc →
  `device.poll(PollType::Wait { submission_index, timeout })` →
  `get_mapped_range` → `chunks_exact(padded_bytes_per_row)` row unpad
  (`GPWGPU/wgpu_renderer.rs:2470-2510`).

### Corrections to prior documents (both re-verified this session)

1. **`InstanceDescriptor::default()` does not exist.** wgpu-types-29.0.4 has
   only `#[derive(Debug)]` on the struct (`WTYPES/instance.rs:26`) and no
   manual `Default` impl anywhere in wgpu-types-29.0.4 or wgpu-29.0.4 (grep
   `impl Default for InstanceDescriptor`: zero hits; plan 07's superseded
   draft used the nonexistent default in its Step 4.1 — its current revision
   avoids the trap by going through this bridge's `WgpuEngine`, 07:296-299).
   The inherent constructor `InstanceDescriptor::
   new_without_display_handle()` covers the same ground
   (`WTYPES/instance.rs:66-74`, all-default fields); this plan instead
   constructs the descriptor explicitly to set `Backends::all()`, copying
   `GPWGPU/wgpu_context.rs:473-481`.
2. Survey section (e) says the RGBA→BGRA swap is "manual ... in a copy". A
   copy-free alternative exists and this plan uses it: swizzle in the final
   GPU pass (write `b,g,r,a` component order into an `Rgba8Unorm` target; the
   format names the component order, and the memory bytes are then BGRA). The
   CPU-side `rgba_to_bgra` helper is still shipped for bridge consumers that
   render RGBA.

### Frame loop, threads, clocks

- `Window::on_next_frame(callback: impl FnOnce(&mut Window, &mut App))` runs
  directly after the current frame renders and wakes the platform frame source
  (`GPUI/window.rs:2650-2658`). `Window::request_animation_frame()` is built
  on it (`:2670-2674`, doc names "a video player" as the use case and says
  decorative-motion callers should check `App::reduce_motion`).
- `App::reduce_motion() -> bool` (`GPUI/app.rs:1222`).
- `cx.background_executor().timer(Duration) -> Task<()>`
  (`GPUI/executor.rs:183`); `BackgroundExecutor::spawn` takes `Send + Sync`
  futures, wgpu `Device`/`Queue` are `Send + Sync` on native (wgpu 29's
  `send_sync` cfg, `WGPU29/api/device.rs:23` and the whole API surface being
  thread-safe handles), so `Arc<WgpuEngine>` crosses into background tasks.
- Clocks: `crate::platform::clock::Instant` (native `std`, wasm `web_time`,
  `REPO/src/platform/clock.rs:1-8`), required because `state.rs` compiles on
  wasm.
- Frame-loop skeleton (paint-driven ticker → `on_next_frame` kick → paced
  `cx.spawn` task → background render → weak-entity update → single notify):
  this plan specifies it in Phase 7, and plan 07's canvas-shaders ticker uses
  the same shape on top of this bridge — one skeleton, two consumers, both
  against this repo's threading rules (blocking work on the background
  executor, weak entities in tasks).

### Kit components and repo seams

- `canvas(prepaint, paint)` closures receive `(Bounds<Pixels>, &mut Window,
  &mut App)`; `Canvas<T>: Styled` (`GPUI/elements/canvas.rs:9-22, 91`).
- `ButtonGroup::new(id)` takes `impl Into<ElementId>`, children are added
  with `.child(Button)`, and one group-level `on_click(Fn(&Vec<usize>, &mut
  Window, &mut App))` receives the clicked/selected indices — the component's
  own doc example builds exactly the selector shape this page needs
  (`gpui-component-0.7.1/src/button/button_group.rs:18-24, 61-68, 103-121`).
  `Button` has `new(id)` (`button.rs:241`), `label` (`:371`), `outline`
  (`:332`), `selected(bool)` (`:543`), `primary()` via `ButtonVariants`
  (`:48-52`), `on_click(&ClickEvent, &mut Window, &mut App)` (`:447`).
- Page seam per `seams.md` §1 (15 rows), all files re-read this session:
  `src/shell/sidebar.rs:4-105`, `src/shell/route.rs:10-21`,
  `src/shell/root/app_root/state.rs:22-56,211-223`,
  `src/features/pages/mod.rs:1-25`, `tests/e2e_navigation.rs:14-19` (count
  currently 10), `locales/en.yml:53-62` nav block (zh-CN same lines).
- Capability registry: `CapabilityStatus` pub fields + constructors
  (`src/runtime/capabilities.rs:6-50`); wasm "hidden with capability reason"
  literal precedent at `src/platform/process/single_instance.rs:647-655` and
  `src/services/accessibility.rs:135-143`; startup loop
  `DEFAULT_ENABLED_CAPABILITIES` at `src/app/init.rs:95-108`; the diagnostics
  page renders whatever registers (`src/features/pages/diagnostics/rows.rs:25`
  reads `capabilities::snapshot(cx)`).
- i18n facade `crate::i18n::localize` / `localize_with_args` with `%{name}`
  placeholders (`src/services/i18n.rs:3-13`).
- Sidebar icon: `gpu.svg` exists in
  `gpui-kit-assets-0.7.1/default-icons.txt` — the default 106 is the only
  set the app registers; `gpu.svg` exists in the full catalog but its bytes
  are not registered natively, so the page icon is `IconName::Cpu`
  (verified default; compute semantics).
- Feature-module precedent for a non-page feature dir: `src/features/mod.rs:1-3`
  (`pub mod command_palette; pub mod pages; mod palette;`); a page mounted from
  outside `features/pages/` has precedent too (`src/gallery` →
  `state.rs:3,31,56`). This plan uses the first: `src/features/wgpu/` with the
  page inside, so the bridge has a stable import path
  (`crate::features::wgpu::...`).

### Crate picks

| Crate | Version | Why |
| --- | --- | --- |
| `wgpu` | `=29.0.4`, default features, native-only table | Exact pin reuses the lock's single wgpu copy (pulled by gpui-pre-wgpu); any other version compiles a second full wgpu/naga stack into the binary. Default features (metal/vulkan/dx12/gles) match gpui's own usage. |
| `image` | `0.25`, `default-features = false`, native-only table | Names `image::Frame`/`RgbaImage` needed to build `RenderImage` frames; gpui does not re-export the crate. 0.25.10 already locked; no codec features (pixels are procedural). Already declared png-only in the Linux and Windows tables (`REPO/Cargo.toml:266, 278`) but not macOS's and not the not-wasm table — the new entry is what covers macOS; the same crate may appear in several target tables (features union), so the existing lines stay. |
| `naga` (dev-dep) | `=29.0.4`, feature `wgsl-in` | GPU-free WGSL validation in unit tests, same version as the lock's naga. Dev-deps never reach the wasm lib check. Same pick plan 07 made. |

## Architecture

Entities and ownership (all inside `src/features/wgpu/`; `WgpuPage` is owned by
`AppRoot`):

- `Entity<WgpuPageState>`, platform-neutral playback state: active demo,
  playing flag, particle target count, quantized render size (pending +
  applied), frame-in-flight flag, `FpsTracker`, engine-ms EMA, mirror of the
  displayed `Arc<RenderImage>`, bounded stale-frame queue (cap 4), runtime
  error string. Compiles on wasm; never touched there at runtime.
- `Arc<WgpuEngine>` (native only), the bridge: wgpu `Device`, `Queue`, and an
  adapter label. `Send + Sync`; cloned into background render tasks. Created
  lazily by the page's init task; never stores pipeline state (demos own
  theirs).
- `Arc<SceneRenderer>` (native only, page-private), per-demo wgpu pipelines,
  bind groups, scene/depth/readback textures, particle buffers. Rebuilt by
  `reconfigure` when the demo, particle count, or quantized size changes;
  `render(job)` returns tightly-packed BGRA bytes. Reached only from
  background tasks; mutation serialized by the single-frame-in-flight rule.
- `WgpuPage`, owns the state entity, the optional engine/renderer `Arc`s,
  mounts the ticker element, toolbar, stats row, error and wasm panels.

Data flow per frame (native), the skeleton from the digest above:

1. Paint: page renders toolbar, stats, the `img()` surface (current
   `displayed` image, `ObjectFit::Contain`), and an absolutely-positioned
   full-size `canvas()` ticker. The ticker's paint closure: (a) drains
   `stale_frames` through `window.drop_image` (atlas fix), (b) reads its own
   bounds, the viewport size, and updates the state's pending size when it
   changed, (c) if playing, no frame in flight, renderer ready, no error,
   registers `window.on_next_frame(kick)`.
2. `on_next_frame` (post-render, safe to mutate): re-checks, marks
   `frame_in_flight = true`, `cx.spawn`s the frame task.
3. Frame task (main-thread async): paces on a background timer to the next
   1/60 s boundary, snapshots `RenderJob`, then
   `background_executor().spawn(renderer.render(job))`: write uniforms →
   compute pass (particles) → scene pass (depth for mesh demos) → output pass
   (identity-or-post fragment, writes BGRA byte order) →
   `copy_texture_to_buffer` → `map_async` + `poll(Wait)` → row unpad. All
   blocking waits live on the background thread.
4. Completion (back on `&mut AsyncApp`): `bgra_to_render_image` builds the
   `Arc<RenderImage>`, `state.frame_done(now, image)` swaps `displayed`,
   pushes the previous image into `stale_frames`, advances the clock, pushes
   the fps stamp, clears in-flight, notifies once. Notify → repaint → ticker
   → next kick: the loop is self-sustaining while the page paints and halts
   with no frame in flight when it stops painting.
5. Resize: the ticker's bounds are converted to physical pixels
   (`window.scale_factor()`, `GPUI/window.rs:2973`), quantized to multiples
   of 16, clamped to `[64, 4096]`; a change marks the renderer dirty and the
   next frame's task calls `renderer.reconfigure` before rendering. Targets
   are never recreated mid-drag more than once per quantized step.

Failure path: engine/renderer init errors set `runtime_error`, flip the
`wgpu_embed` capability to `degraded`/`error`, stop the loop, and render an
error panel with Retry (re-runs the init task and resets the capability on
success).

Wasm: `engine.rs`, `demos.rs`, and the ticker are
`#[cfg(not(target_family = "wasm"))]`; the page renders the wasm panel instead.
`frame.rs`, `math.rs`, `state.rs` compile everywhere, except the two
RenderImage builders inside `frame.rs`, which are cfg-gated native-only (they
construct `image::Frame`; the `image` dependency is not-wasm-table-only, so
an ungated import would fail `just wasm-check`).

Module tree:

```text
src/features/wgpu/
├── mod.rs        feature seam: header comment + removal recipe, module decls,
│                 pub re-exports (WgpuPage, WgpuEngine, frame helpers)
├── page.rs       WgpuPage: toolbar, demo/count selectors, stats row, img
│                 surface, ticker mount, error + wasm panels, init task
├── state.rs      WgpuPageState entity, platform-neutral
├── frame.rs      padded_bytes_per_row, rgba_to_bgra, rgba/bgra → RenderImage,
│                 FpsTracker, pure, wasm-clean
├── math.rs       mat4 (column-major encode), perspective, rotations, sphere
│                 mesh + vertex bytes, particle init, pure, wasm-clean
├── engine.rs     #[cfg(not(wasm))] WgpuEngine: headless device/queue +
│                 read_texture_bytes readback
├── demos.rs      #[cfg(not(wasm))] DemoId, DemoSpec, SceneRenderer
│                 (particles compute+draw, mesh+depth, output passes), WGSL
└── *.test.rs     sibling test modules (frame, math, state; naga tests in
                  demos.test.rs; #[ignore]d GPU smokes in engine/demos tests)
```

## Interface Contract

Exact pub surface consumers get. Dependents import from
`crate::features::wgpu` (same crate) or `gpui_starter::features::wgpu`
(cross-crate tests). Everything not listed stays private to the module.

```rust
// src/features/wgpu/mod.rs
mod demos;
#[cfg(not(target_family = "wasm"))]
mod engine;
mod frame;
mod math;
mod page;
mod state;

/// Capability-registry name registered at startup ("wgpu_embed").
pub const CAPABILITY_WGPU: &str = "wgpu_embed";

pub use page::WgpuPage;

// Pure helpers, compile on every target, including wasm.
pub use frame::{padded_bytes_per_row, rgba_to_bgra, FpsTracker};

// RenderImage builders are native-only: they construct image::Frame, and the
// image dependency lives in the not-wasm table (repo precedent: every
// existing `image::` use is cfg'd, src/app/assets.rs:23,30,41).
#[cfg(not(target_family = "wasm"))]
pub use frame::{bgra_to_render_image, rgba_to_render_image};

// Native-only bridge.
#[cfg(not(target_family = "wasm"))]
pub use engine::WgpuEngine;

impl WgpuPage {
    /// Constructs the page; mounts the state entity; on native, defers wgpu
    /// engine creation to a background init task (callers never block).
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self;
}

/// Headless wgpu device shared by this feature and future consumers.
/// `Send + Sync`; all handles are clones of the same underlying device.
#[cfg(not(target_family = "wasm"))]
pub struct WgpuEngine { /* private fields */ }
#[cfg(not(target_family = "wasm"))]
impl WgpuEngine {
    /// Requests a surface-less adapter + device with default limits.
    /// Async; poll on any executor. Errors name the failing step.
    pub async fn new() -> anyhow::Result<Self>;
    pub fn device(&self) -> &wgpu::Device;
    pub fn queue(&self) -> &wgpu::Queue;
    /// Human-readable adapter/backend label for diagnostics.
    pub fn adapter_label(&self) -> &str;
    /// Reads a COPY_SRC-capable 2D texture back as tightly-packed rows
    /// (width*4 bytes per row) in the texture's own component order.
    /// BLOCKING (submits, maps, polls to completion), call from a
    /// background thread only.
    pub fn read_texture_bytes(
        &self,
        texture: &wgpu::Texture,
        width: u32,
        height: u32,
    ) -> anyhow::Result<Vec<u8>>;
}

// frame.rs, pure functions, no wgpu types (wasm-clean).
pub fn padded_bytes_per_row(width: u32) -> usize; // 256-aligned (wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
pub fn rgba_to_bgra(padded_rows: &[u8], width: u32, height: u32) -> Vec<u8>; // unpad + swizzle
pub fn rgba_to_render_image(width: u32, height: u32, rgba: Vec<u8>) -> Arc<RenderImage>;
pub fn bgra_to_render_image(width: u32, height: u32, bgra: Vec<u8>) -> Arc<RenderImage>;
pub struct FpsTracker { /* private */ }
impl FpsTracker {
    pub fn new() -> Self;
    pub fn push(&mut self, now: crate::platform::clock::Instant);
    pub fn fps(&self) -> u32; // stamps within the last 750 ms, scaled to 1 s
}
```

Observable surface that is part of the contract without being importable API:

- Sidebar page variant `Page::Wgpu` with `host() == "wgpu"` (deep link
  `gpui-starter://wgpu`), appended last in `all()` (no digit shortcut:
  bindings cover only `all()[..9]`, `seams.md` §1 row 8 note).
- Capability registry name `wgpu_embed`: `supported_enabled` on native at
  startup; `degraded(err)`/`error(err)` on runtime failure; `supported: false`
  with reason on wasm.
- i18n keys in the i18n section, both locales in lockstep.
- Demo identifiers (for future diagnostics use):
  `DemoId::{Particles, Mesh, MeshPost}` are `pub(crate)` from `demos.rs`, not
  part of the external contract.

Consumption notes for dependents: build your own pipelines against
`WgpuEngine::device()/queue()`; render into an `Rgba8Unorm` texture with
`RENDER_ATTACHMENT | COPY_SRC`; read it with `read_texture_bytes`; wrap with
`rgba_to_render_image` (or swizzle in your own final shader and use
`bgra_to_render_image`). Every displayed frame must eventually reach
`window.drop_image`, dependents that paint through their own `img()` must run
their own drain (mirror `page.rs`'s ticker).

## Module Layout

| File | Purpose |
| --- | --- |
| `src/features/wgpu/mod.rs` | Feature seam: removal-recipe header, module decls, `pub` re-exports, `CAPABILITY_WGPU` |
| `src/features/wgpu/page.rs` | `WgpuPage`: toolbar (play/pause, demo selector, count selector), stats row, `img` surface + ticker, error/wasm panels, background init + frame tasks |
| `src/features/wgpu/state.rs` | `WgpuPageState` entity: playback, sizing, in-flight, fps, stale frames, error |
| `src/features/wgpu/frame.rs` | Row padding, RGBA→BGRA swizzle, `RenderImage` construction, `FpsTracker`, pure |
| `src/features/wgpu/math.rs` | Column-major `Mat4` encode, perspective, rotations, UV-sphere mesh + vertex/index bytes, deterministic particle init, pure |
| `src/features/wgpu/engine.rs` | Native `WgpuEngine`: headless adapter/device/queue, `read_texture_bytes` |
| `src/features/wgpu/demos.rs` | Native demo definitions + `SceneRenderer`: particle compute/draw pipelines, mesh pipeline with `Depth24Plus`, output passes (identity + post), WGSL sources |
| `src/features/wgpu/frame.test.rs` | Padding, swizzle, image round-trip, fps window |
| `src/features/wgpu/math.test.rs` | Matrix encodes, perspective near/far mapping, mesh counts/bounds, particle determinism |
| `src/features/wgpu/state.test.rs` | Playback/sizing/in-flight/stale lifecycle (entity context) |
| `src/features/wgpu/engine.test.rs` | `#[ignore]`d GPU smoke: clear + readback round-trip |
| `src/features/wgpu/demos.test.rs` | naga validation of every WGSL source + spec invariants + `#[ignore]`d GPU demo render |

## Seam Touches

Every edit outside the new module:

| # | File | Change | Why minimal |
|---|---|---|---|
| 1 | `Cargo.toml` `[target.'cfg(not(target_family = "wasm"))'.dependencies]` (table at line 310) | add `wgpu = { version = "=29.0.4" }` and `image = { version = "0.25", default-features = false }` **only if absent** (grep first: plan 07 adds the same `wgpu` line under this table, its seam 1; the `image` line is this plan's alone — 07 leaves the image dependency to the bridge, 07:102-103. Duplicate keys within one table are an error) | Only table native-only deps may live in; versions already in the lock |
| 2 | `Cargo.toml` `[dev-dependencies]` (line 212) | add `naga = { version = "=29.0.4", features = ["wgsl-in"] }` only if absent | Test-only WGSL validation; same pick as plan 07 |
| 3 | `Cargo.lock` | regenerate via `cargo check -p gpui-starter` and commit together with the manifest (`seams.md` §9 lock rule) | Lock must match the manifest |
| 4 | `src/features/mod.rs:1-3` | `pub mod wgpu;` | Module root for the bridge + page |
| 5 | `src/shell/sidebar.rs` | `Wgpu` variant (enum at :4-16), `title()` arm `"WGPU"`, `title_key()` arm `"nav_wgpu"`, `host()` arm `"wgpu"`, `icon()` arm `IconName::Cpu`, `all()` entry appended last | Exhaustive matches force all arms; last position avoids re-binding digit keys |
| 6 | `src/shell/route.rs:10-21` | `Page::Wgpu.host(),` in `VALID_HOSTS` | Deep-link validation |
| 7 | `src/shell/root/app_root/state.rs` | import `crate::features::wgpu::WgpuPage`, field `wgpu_page: Entity<WgpuPage>` (:22-31 block), construction `cx.new(|cx| WgpuPage::new(window, cx))` (:47-56 block), `unchecked_active_page_view` arm (:211-223) | Page mounting |
| 8 | `tests/e2e_navigation.rs:14-19` | bump the page-count assertion by one (do **not** hardcode an absolute number here, plans 05/06/07/09 also bump it; read the current value first) and add `Page::Wgpu` to the expected list | Exhaustive-count test |
| 9 | `locales/en.yml` + `locales/zh-CN.yml` | every key from the i18n table, both files, after the `nav_*` block | Lockstep rule (`seams.md` §5) |
| 10 | `src/app/init.rs:95-108` | `"wgpu_embed"` in `DEFAULT_ENABLED_CAPABILITIES`; after the loop add a `#[cfg(target_family = "wasm")]` block setting the unsupported literal | Capability surface |

Deliberate non-touches: no command-palette entry (sidebar + deep link reach
it), no gallery section, no `src/ui/` edits, no `[workspace.dependencies]`
change, no new locale files, no changes to plan 07's module (it imports this
bridge per its own Dependencies section, 07:33-47; its shaders page and
`canvas_shaders` capability are its own seams).

### Removal Recipe

`mod.rs` opens with this header, mirroring `src/gallery/mod.rs:1-7`:

```rust
//! Removable wgpu bridge + demo page. `WgpuPage`, `WgpuEngine`, and the
//! frame helpers are the only public surface. Removal = delete this
//! directory and revert its ten seam touches: the wgpu/image/naga lines in
//! Cargo.toml plus the regenerated Cargo.lock (only if no other feature
//! still needs them), the `pub mod wgpu;` line in features/mod.rs, the
//! Page::Wgpu arms and all() entry in shell/sidebar.rs, the VALID_HOSTS
//! entry in shell/route.rs, the AppRoot wiring in shell/root/app_root/
//! state.rs, the page count in tests/e2e_navigation.rs, the wgpu_* and
//! nav_wgpu keys in both locale files, and the wgpu_embed capability lines
//! in app/init.rs.
```

Ordered removal steps:

1. Revert seams 4-10 (features mod, sidebar arms, route host, AppRoot wiring,
   nav count, locale keys, capability lines).
2. If no other feature imports `crate::features::wgpu`, revert seams 1-2 and
   run `cargo check -p gpui-starter` so `Cargo.lock` drops the now-unused
   direct-dep entries (they may remain transitively via gpui-pre-wgpu, that
   is correct); commit lock + manifest together.
3. `rm -r src/features/wgpu/`.
4. `grep -rn "Wgpu\|wgpu_embed\|wgpu_" src/ tests/ locales/`, zero hits
   remaining (transitive `wgpu` crate names in `Cargo.lock` are not hits).
5. Run the Verification gates; they must pass.

## Implementation Phases

Precondition: none (foundation plan). Note before Step 1.1: plans 05/06/07/09
may have landed other pages, every seam edit below is written relative to
current file state, with a read-first check.

## Phase 1: Manifest, feature seam, page scaffold, wasm panel

### Step 1.1: Cargo manifest and lock

- Files: `Cargo.toml`, `Cargo.lock`.
- Build: run `grep -n '^wgpu\|^image\|^naga' Cargo.toml` first — `image`
  already appears in the Linux and Windows tables (`Cargo.toml:266, 278`,
  png-only); those are NOT the not-wasm table and must not be touched. Under
  `[target.'cfg(not(target_family = "wasm"))'.dependencies]` (line 310) add —
  only if that table itself lacks the line:
  `wgpu = { version = "=29.0.4" }` (comment: "exact pin shares the lock's
  single wgpu with gpui-pre-wgpu; any other version compiles a second stack")
  and `image = { version = "0.25", default-features = false }` (comment:
  "names image::Frame/RgbaImage for RenderImage frames; pixels are
  procedural, no codecs; macOS has no other image entry"). Under
  `[dev-dependencies]` (line 212) add, if missing,
  `naga = { version = "=29.0.4", features = ["wgsl-in"] }` (comment:
  "GPU-free WGSL validation; matches the lock's naga"). Run
  `cargo check -p gpui-starter` to rewrite the lock; `git diff Cargo.lock`
  must show only entries moving from transitive to direct (no version bumps).
- Tests: none.
- Done-check: `cargo check -p gpui-starter` passes;
  `grep -c '^wgpu = ' Cargo.toml` is 1 and it sits under the not-wasm table;
  `sed -n '/cfg(not(target_family = "wasm")).dependencies/,/^\[/p' Cargo.toml
  | grep -c '^image = '` is 1.

### Step 1.2: Module scaffold and page seam

- Files: `src/features/wgpu/mod.rs`, `src/features/mod.rs`,
  `src/shell/sidebar.rs`, `src/shell/route.rs`,
  `src/shell/root/app_root/state.rs`, `tests/e2e_navigation.rs`,
  `locales/en.yml`, `locales/zh-CN.yml`, `src/app/init.rs`.
- Build:
  - `mod.rs`: the removal-recipe header (above), module decls
    (`mod demos;` …, create each file empty or with a stub in later steps;
    declare only `mod frame; mod math; mod page; mod state;` now, add the
    cfg'd `engine`/`demos` decls in Phases 4-5), `pub const
    CAPABILITY_WGPU: &str = "wgpu_embed";`, and `pub use page::WgpuPage;`.
  - `page.rs`: minimal `pub struct WgpuPage { _subscriptions:
    Vec<Subscription> }` with `pub fn new(_: &mut Window, _: &mut
    Context<Self>) -> Self { Self { _subscriptions: Vec::new() } }` and a
    `Render` impl: `v_flex().min_h_full().p_6().gap_4()` with a level-1
    heading `div().id("wgpu-title").a11y(Role::Heading,
    crate::i18n::localize("wgpu_title")).aria_level(1).text_xl().font_weight(FontWeight::BOLD)`
    (heading pattern from `src/features/pages/notifications.rs:48-58`;
    `Role` arrives via the `gpui_kit` glob, `GPUI/gpui.rs:99`).
  - `features/mod.rs`: `pub mod wgpu;` (keep alphabetical order with
    `command_palette`/`pages`/`palette`).
  - `sidebar.rs`: `Wgpu` variant appended last in the enum and in `all()`;
    `title() => "WGPU"`; `title_key() => "nav_wgpu"`; `host() => "wgpu"`;
    `icon() => IconName::Cpu`.
  - `route.rs`: `Page::Wgpu.host(),` appended to `VALID_HOSTS`.
  - `app_root/state.rs`: import `crate::features::wgpu::WgpuPage`, field
    `wgpu_page: Entity<WgpuPage>`, construct `cx.new(|cx|
    WgpuPage::new(window, cx))`, arm `Page::Wgpu =>
    self.wgpu_page.clone().into()`.
  - `tests/e2e_navigation.rs`: read the current count assertion, bump by one,
    update the comment, add `Page::Wgpu` to the expected array.
  - Locale files: all keys from the i18n table, both files, placed after the
    `nav_*` block (`locales/en.yml:53-62`).
  - `init.rs`: add `"wgpu_embed"` to `DEFAULT_ENABLED_CAPABILITIES`
    (`src/app/init.rs:95-108`); after the loop:

    ```rust
    #[cfg(target_family = "wasm")]
    crate::capabilities::set(
        crate::features::wgpu::CAPABILITY_WGPU,
        crate::capabilities::CapabilityStatus {
            supported: false,
            enabled: false,
            degraded: false,
            reason: Some("Offscreen wgpu rendering is native-only; the web build ships no bridge.".into()),
            last_error: None,
        },
        cx,
    );
    ```

    (struct literal is legal: fields are pub,
    `src/runtime/capabilities.rs:6-12`. `degraded: false` follows the repo's
    wasm convention — unsupported is not degraded — stated in the comment at
    `src/services/accessibility.rs:134-136` and matched by both precedents,
    `src/services/accessibility.rs:135-143` and
    `src/platform/process/single_instance.rs:647-655`).
- Tests: existing `qa_localization` (lockstep) and `e2e_navigation` (count)
  cover the seam.
- Done-check: `cargo test --test e2e_navigation && cargo test --test
  qa_localization` pass; `cargo check -p gpui-starter` passes.

### Step 1.3: Wasm degradation panel

- Files: `src/features/wgpu/page.rs`.
- Build: split `render` into a `#[cfg(target_family = "wasm")]` branch, the
  heading plus a muted paragraph
  `div().text_sm().text_color(cx.theme().muted_foreground)
  .child(crate::i18n::localize("wgpu_unavailable_wasm"))` inside
  `.border_1().border_color(cx.theme().border).rounded_lg().p_4()`, and a
  `#[cfg(not(target_family = "wasm"))]` branch that for now returns the same
  scaffold minus the panel (filled in Phase 6). The cfg split is established
  here so later phases only touch the native branch.
- Tests: none (wasm rendering is not unit-testable; compile coverage below).
- Done-check: `just wasm-check` passes (`cargo +nightly check --target
  wasm32-unknown-unknown -p gpui-starter --lib`, `justfile:24-25`).

## Phase 2: Pure frame helpers

### Step 2.1: `frame.rs`

- Files: `src/features/wgpu/frame.rs`, `frame.test.rs` (wired from
  `frame.rs` with `#[cfg(test)] #[path = "frame.test.rs"] mod frame_test;`).
- Build (all `pub` items from the Interface Contract; private visibility
  otherwise):
  - `pub fn padded_bytes_per_row(width: u32) -> usize` =
    `(width as usize * 4 + 255) & !255`, literal 256 with the comment
    "wgpu::COPY_BYTES_PER_ROW_ALIGNMENT, spelled out so this module stays
    wasm-clean" (`WGPU29/lib.rs:154` value 256).
  - `pub fn rgba_to_bgra(padded_rows: &[u8], width: u32, height: u32) ->
    Vec<u8>`: `Vec::with_capacity(width*height*4)`; per row
    (`chunks_exact(padded_bytes_per_row(width))`, take `height` rows), per
    4-byte chunk push `chunk[2], chunk[1], chunk[0], chunk[3]`.
  - `#[cfg(not(target_family = "wasm"))] pub fn bgra_to_render_image(width:
    u32, height: u32, bgra: Vec<u8>) -> Arc<RenderImage>`: debug-assert
    `bgra.len() == width as usize * height as usize * 4` (release:
    truncate/expect via `image::RgbaImage::from_raw` returning None → panic
    message "renderer returned width*height*4 bytes"), then
    `Arc::new(RenderImage::new([image::Frame::new(buf)]))` (`Frame::new` at
    `image-0.25.10/src/animation.rs:88`; `[Frame;1] -> SmallVec` via
    `smallvec-1.15.2/src/lib.rs:2070`). The cfg gate is required: `image`
    sits only in the not-wasm table, so an ungated `use image::` fails
    `just wasm-check`.
  - `#[cfg(not(target_family = "wasm"))] pub fn
    rgba_to_render_image(...)`: `rgba_to_bgra` then
    `bgra_to_render_image`.
  - `pub struct FpsTracker { stamps: VecDeque<crate::platform::clock::Instant>
    }` with `new()`, `push(now)` dropping stamps older than 750 ms, `fps() ->
    u32` = `(stamps.len() as f32 / 0.75).round() as u32`.
- Tests (`frame.test.rs`): `padded_bytes_per_row(512) == 2048` and
  `(513) == 2304` (2052 payload bytes → 2304, the next 256 multiple; 2080
  would not even be 256-aligned); `rgba_to_bgra` on a 2-row padded input
  produces the exact swapped bytes with padding dropped;
  `#[cfg(not(target_family = "wasm"))]` tests: `bgra_to_render_image(2,1,..)`
  → `size(0)` is `(2,1)` and `as_bytes(0)` matches input;
  `rgba_to_render_image` swaps channel 0/2 relative to input; `FpsTracker`
  empty → 0, and `n` pushes at the same `now` → expected scaled value.
- Done-check: `cargo test -p gpui-starter --lib features::wgpu::frame` passes.

## Phase 3: Pure math and geometry

### Step 3.1: `math.rs`

- Files: `src/features/wgpu/math.rs`, `math.test.rs` (same sibling wiring).
- Build (all private to the feature except where noted; `pub(in
  crate::features::wgpu)` on items used by `demos.rs`):
  - `pub(in ..) type Mat4 = [f32; 16]`, column-major (index `c*4 + r`).
  - `pub(in ..) fn mat4_identity() -> Mat4`,
    `fn mat4_mul(a: &Mat4, b: &Mat4) -> Mat4` (b applied first:
    `out[c*4+r] = Σ a[k*4+r]*b[c*4+k]`),
    `fn mat4_rotate_y(rad: f32) -> Mat4`, `fn mat4_rotate_x(rad: f32) -> Mat4`,
    `fn mat4_translate(x, y, z: f32) -> Mat4`.
  - `pub(in ..) fn perspective(fov_y_rad: f32, aspect: f32, near: f32, far:
    f32) -> Mat4`, wgpu NDC z ∈ [0,1]: `f = 1/tan(fov/2)`;
    `m[0]=f/aspect, m[5]=f, m[10]=far/(near-far), m[11]=-1,
    m[14]=near*far/(near-far)`, all else 0.
  - `pub(in ..) const MESH_VERTEX_STRIDE: usize = 32;`, vertex layout
    `[px,py,pz, nx,ny,nz, u,v]` as f32 bytes.
  - `pub(in ..) fn sphere_mesh(lat: usize, lon: usize) -> (Vec<u8>,
    Vec<u32>)`, UV sphere radius 1.0: vertices `(lat+1)*lon` + 2 poles;
    positions `sin(polar)*cos(azimuth)`-style with poles at y=±1; normals =
    positions; uv = `(azimuth/2π, polar/π)` (poles: `v` 0 and 1, `u` 0);
    indices per quad → two triangles, pole rows degenerate quads collapse to
    one triangle each. Encode vertices via `f32::to_ne_bytes` per component.
  - `pub(in ..) fn checker_texture(size: u32) -> Vec<u8>`, 256 default;
    RGBA bytes, 8×8 checker of `(230,230,235,255)`/`(38,38,46,255)` with a
    `(200,70,60,255)` diagonal stripe where `(x+y) % 64 < 8`.
  - `pub(in ..) fn particle_init(count: u32) -> (Vec<u8>, Vec<u8>)` —
    deterministic hash `h(i) = fract(sin(i as f32 * 127.1) * 43758.5453)`
    (three offsets for independent channels); positions on a shell radius
    `0.9 + 0.4*h1(i)` at golden-spiral direction
    (`y = 1 - 2*(i+0.5)/count`, `r = sqrt(1-y²)`,
    `θ = i * 2.399963`), `pos.w = 1.5 + h2(i)` (point size); velocities
    tangential `t = normalize(cross(up, p)) * (0.2 + 0.3*h3(i))`, `vel.w = 0`.
    Returns (positions bytes, velocities bytes) as `[f32;4]` rows.
- Tests (`math.test.rs`): identity/mul/rotate numerics against hand-computed
  values; `perspective` maps a point at `z=-near` to ndc z `0.0` and `z=-far`
  to `1.0` (within 1e-5), and flips w sign correctly; `sphere_mesh(16, 24)`
  vertex bytes length `== vertex_count * 32`, every index `< vertex_count`,
    index count `== 6*lat*lon - 2*lon` (quad collapse accounted);
  `checker_texture(256).len() == 256*256*4`; `particle_init(1024)` is
    deterministic (two calls byte-equal) and all `pos.w` ∈ [1.5, 2.5].
- Done-check: `cargo test -p gpui-starter --lib features::wgpu::math` passes.

## Phase 4: Native wgpu engine

### Step 4.1: `WgpuEngine`

- Files: `src/features/wgpu/engine.rs` (module declared in `mod.rs` under
  `#[cfg(not(target_family = "wasm"))]`), `engine.test.rs` (test module also
  cfg-gated).
- Build:
  - `pub struct WgpuEngine { device: wgpu::Device, queue: wgpu::Queue,
    adapter_label: String }` (private fields; readers per contract).
  - `pub async fn new() -> anyhow::Result<Self>`:

    ```rust
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        flags: wgpu::InstanceFlags::default(),
        backend_options: wgpu::BackendOptions::default(),
        memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        display: None,
    });
    ```

    (field-for-field the gpui-native pattern, `GPWGPU/wgpu_context.rs:473-481`;
    `display: None` = headless. There is **no** `InstanceDescriptor::default()`
    in 29.0.4, `WTYPES/instance.rs:26` derives only `Debug`.) Then
    `instance.request_adapter(&wgpu::RequestAdapterOptions {
    power_preference: wgpu::PowerPreference::HighPerformance,
    force_fallback_adapter: false, compatible_surface: None }).await`
    (`WGPU29/api/instance.rs:167-171`) mapping the error through
    `anyhow::Context` with "requesting wgpu adapter"; then
    `adapter.request_device(&wgpu::DeviceDescriptor::default()).await`
    (`WGPU29/api/adapter.rs:58-64`) with "requesting wgpu device". Build
    `adapter_label` from `adapter.get_info()` (`"{} ({})"` name + backend).
  - `pub fn read_texture_bytes(&self, texture: &wgpu::Texture, width: u32,
    height: u32) -> anyhow::Result<Vec<u8>>`, the gpui readback pattern
    (`GPWGPU/wgpu_renderer.rs:2470-2510`):
    1. `let padded = crate::features::wgpu::frame::padded_bytes_per_row(width);`
    2. readback buffer `size = padded * height`, usage `MAP_READ | COPY_DST`,
       `mapped_at_creation: false`.
    3. `let mut encoder = self.device.create_command_encoder(...);
       encoder.copy_texture_to_buffer(texture.as_image_copy(),
       wgpu::TexelCopyBufferInfo { buffer: &buffer, layout:
       wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(padded as
       u32), rows_per_image: Some(height) } }, wgpu::Extent3d { width, height,
       depth_or_array_layers: 1 });`
       — `TexelCopyBufferInfo` is the type (alias at
       `WGPU29/api/command_encoder.rs:44-45`; there is no `TexelCopyBuffer`,
       grep `TexelCopyBuffer[^IL]` over wgpu-29.0.4/src returns nothing);
       struct fields `buffer`/`layout` at `WTYPES/texture.rs:837-844`;
       `copy_texture_to_buffer` at `WGPU29/api/command_encoder.rs:146`.
    4. `let submission = self.queue.submit(Some(encoder.finish()));`
    5. mpsc + `buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
       let _ = tx.send(r); })` (`slice` at `WGPU29/api/buffer.rs:312`,
       `BufferSlice::map_async` at `:566`), then
       `self.device.poll(wgpu::PollType::Wait {
       submission_index: Some(submission), timeout: Some(Duration::from_secs(
       30)) })` and `rx.recv_timeout(..)`, both errors mapped with context.
    6. `get_mapped_range(..)`, copy `width*4` bytes per row into a
       `Vec::with_capacity(width*height*4)` skipping padding, `buffer.unmap()`.
- Tests (`engine.test.rs`, cfg-gated): one `#[gpui_kit::test]` async test,
  `#[ignore = "needs a wgpu adapter (GPU or lavapipe); run with --ignored
  locally"]`: create the engine, create an 8×8 `Rgba8Unorm` texture with
  `RENDER_ATTACHMENT | COPY_SRC`, submit a clear-only pass with
  `LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 })`, call
  `read_texture_bytes`, assert `len() == 8*8*4` and every pixel row starts
  `[0, 0, 255, 255]` (Rgba8Unorm byte order).
- Done-check: `cargo check -p gpui-starter` passes;
  `cargo test -p gpui-starter --lib features::wgpu::engine -- --ignored`
  passes on this machine.

## Phase 5: Demo scene renderer

### Step 5.1: Demo definitions and WGSL sources

- Files: `src/features/wgpu/demos.rs`, `demos.test.rs` (both cfg-gated).
- Build:
  - `pub(crate) enum DemoId { Particles, Mesh, MeshPost }` (Copy, Clone,
    Debug, PartialEq, Eq, Hash).
  - `const PARTICLES_WGSL: &str = ...`, `const MESH_WGSL: &str = ...`,
    `const OUTPUT_IDENTITY_WGSL: &str = ...`, `const OUTPUT_POST_WGSL: &str =
    ...`, exact sources below; every source embeds `include_str!`-free plain
    `const` strings.
  - Shared WGSL prelude (prepended to each const where needed):

    ```wgsl
    struct FrameParams {
        mvp: mat4x4<f32>,
        model: mat4x4<f32>,
        light_dir: vec4<f32>,   // xyz dir, w unused
        misc: vec4<f32>,        // x: dt, y: elapsed, z: point_scale, w: unused
    };
    @group(0) @binding(0) var<uniform> params: FrameParams;
    ```

  - `PARTICLES_WGSL`, compute stage:

    ```wgsl
    struct Particle { pos: vec4<f32>, vel: vec4<f32> };
    @group(0) @binding(1) var<storage, read_write> particles: array<Particle>;
    @group(0) @binding(2) var<uniform> attractor: vec4<f32>; // xyz, w strength

    @compute @workgroup_size(64)
    fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
        let i = gid.x;
        if (i >= arrayLength(&particles)) { return; }
        var p = particles[i];
        let dt = params.misc.x;
        let to_a = attractor.xyz - p.pos.xyz;
        let dist = max(length(to_a), 0.05);
        let pull = normalize(to_a) * (attractor.w / (dist * dist));
        p.vel.xyz = p.vel.xyz + (pull - p.vel.xyz * 0.35) * dt;
        p.pos.xyz = p.pos.xyz + p.vel.xyz * dt;
        let r = length(p.pos.xyz);
        if (r > 1.8) {
            let n = p.pos.xyz / r;
            p.pos.xyz = n * 1.8;
            p.vel.xyz = reflect(p.vel.xyz, n) * 0.9;
        }
        particles[i] = p;
    }
    ```

    plus the draw stage appended in the same const (separate module strings
    are built from one const by splitting on a marker, or simpler: two consts
    `PARTICLES_COMPUTE_WGSL` / `PARTICLES_DRAW_WGSL`; take the two-const
    form):

    ```wgsl
    struct Particle { pos: vec4<f32>, vel: vec4<f32> };
    @group(0) @binding(1) var<storage, read> particles_draw: array<Particle>;

    struct VOut {
        @builtin(position) pos: vec4<f32>,
        @location(0) speed: f32,
    };

    @vertex
    fn vs_main(@builtin(instance_index) ii: u32,
               @builtin(vertex_index) vi: u32) -> VOut {
        var corners = array<vec2<f32>, 4>(
            vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0),
            vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0));
        let p = particles_draw[ii];
        let clip = params.mvp * vec4<f32>(p.pos.xyz, 1.0);
        let ndc = clip.xyz / clip.w;
        let size = p.pos.w * params.misc.z / max(ndc.z, 0.1);
        var out: VOut;
        let offset = corners[vi] * size;
        out.pos = vec4<f32>((ndc.xy + offset) * clip.w, clip.z, clip.w);
        out.speed = length(p.vel.xyz);
        return out;
    }

    @fragment
    fn fs_main(in: VOut) -> @location(0) vec4<f32> {
        let s = clamp(in.speed * 0.8, 0.0, 1.0);
        let col = mix(vec3<f32>(0.15, 0.45, 0.95), vec3<f32>(0.85, 0.95, 1.0), s);
        return vec4<f32>(col, 0.9);
    }
    ```

    Draw topology `TriangleStrip`, 4 vertices, instance count = particle
    count. The positions buffer is bound twice (read-write for compute,
    read-only for vertex) via two bind groups on the same buffer, legal in
    wgpu when the usages don't overlap within a pass.
  - `MESH_WGSL`:

    ```wgsl
    struct VIn {
        @location(0) pos: vec3<f32>,
        @location(1) normal: vec3<f32>,
        @location(2) uv: vec2<f32>,
    };
    struct VOut {
        @builtin(position) pos: vec4<f32>,
        @location(0) normal: vec3<f32>,
        @location(1) uv: vec2<f32>,
    };
    @vertex
    fn vs_main(in: VIn) -> VOut {
        var out: VOut;
        let world = params.model * vec4<f32>(in.pos, 1.0);
        out.pos = params.mvp * vec4<f32>(in.pos, 1.0);
        out.normal = (params.model * vec4<f32>(in.normal, 0.0)).xyz;
        out.uv = in.uv;
        return out;
    }
    @group(0) @binding(1) var mesh_tex: texture_2d<f32>;
    @group(0) @binding(2) var mesh_sampler: sampler;
    @fragment
    fn fs_main(in: VOut) -> @location(0) vec4<f32> {
        let n = normalize(in.normal);
        let lambert = max(dot(n, normalize(params.light_dir.xyz)), 0.0);
        let albedo = textureSample(mesh_tex, mesh_sampler, in.uv).rgb;
        let col = albedo * (0.25 + 0.75 * lambert);
        return vec4<f32>(col, 1.0);
    }
    ```

    Depth-stencil: `Depth24Plus`, compare `Less`, write enabled.
  - Output pass (fullscreen triangle; vertex shared by both consts):

    ```wgsl
    @vertex
    fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
        var pos = array<vec2<f32>, 3>(
            vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
        return vec4<f32>(pos[vi], 0.0, 1.0);
    }
    @group(0) @binding(1) var scene_tex: texture_2d<f32>;
    @group(0) @binding(2) var scene_sampler: sampler;
    ```

    `OUTPUT_IDENTITY_WGSL` fragment:

    ```wgsl
    @fragment
    fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
        let uv = frag.xy / vec2<f32>(textureDimensions(scene_tex, 0));
        let c = textureSample(scene_tex, scene_sampler, uv);
        // write BGRA byte order into the Rgba8Unorm target (gpui RenderImage)
        return vec4<f32>(c.b, c.g, c.r, c.a);
    }
    ```

    `OUTPUT_POST_WGSL` fragment:

    ```wgsl
    @fragment
    fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
        let dims = vec2<f32>(textureDimensions(scene_tex, 0));
        let uv = frag.xy / dims;
        let d = distance(uv, vec2<f32>(0.5));
        let pulse = 0.5 + 0.5 * sin(params.misc.y * 1.5);
        let amt = (0.002 + 0.004 * pulse) * d;
        let dir = normalize(uv - vec2<f32>(0.5) + vec2<f32>(1e-5));
        let r = textureSample(scene_tex, scene_sampler, uv + dir * amt).r;
        let g = textureSample(scene_tex, scene_sampler, uv).g;
        let b = textureSample(scene_tex, scene_sampler, uv - dir * amt).b;
        let vign = smoothstep(0.95, 0.35, d);
        let c = vec3<f32>(r, g, b) * mix(0.55, 1.0, vign);
        return vec4<f32>(c.b, c.g, c.r, 1.0);
    }
    ```

- Tests (`demos.test.rs`): `naga::front::wgsl::parse_str(src)` is `Ok` for
  every const (particles compute + draw, mesh, both output consts); each
  const contains its `@compute`/`@vertex`/`@fragment` entry points; the two
  output consts end with the bgra swizzle line (string contains
  `vec4<f32>(c.b, c.g, c.r`).
- Done-check: `cargo test -p gpui-starter --lib features::wgpu::demos`
  passes including all naga parses.

### Step 5.2: `SceneRenderer`

- Files: `src/features/wgpu/demos.rs`.
- Build (all `pub(in crate::features::wgpu)` or private):
  - `struct DemoSpec { uses_particles: bool, uses_mesh: bool, post: bool }`
    with `const SPECS: [(DemoId, DemoSpec); 3]`, Particles
    `{uses_particles: true, ..false}`, Mesh `{uses_mesh: true, ..false}`,
    MeshPost `{uses_mesh: true, post: true, ..}`.
  - `pub(in ..) struct RenderJob { demo: DemoId, particle_count: u32, dt:
    f32, elapsed: f32, width: u32, height: u32 }` (Copy).
  - `pub(in ..) struct SceneRenderer` private fields: `engine:
    Arc<WgpuEngine>`, `frame_params: wgpu::Buffer` (uniform, 160 bytes:
    layout `mvp(64) model(64) light(16) misc(16)` matches the WGSL
    `FrameParams` struct), `attractor: wgpu::Buffer` (16 bytes),
    particles pipelines + buffers (positions, velocities, sizes: `Vec<u32>`
    of supported counts), mesh pipeline + vertex/index buffers + texture +
    sampler, output pipelines (identity, post) + scene sampler, and
    `Targets { width, height, scene: wgpu::Texture, scene_view, depth:
    Option<wgpu::Texture>, out: wgpu::Texture, out_view }`.
  - `pub(in ..) fn new(engine: Arc<WgpuEngine>, initial: &RenderJob) ->
    anyhow::Result<Self>`:
    - frame-params bind group layout: binding 0 uniform, visibility
      `VERTEX | FRAGMENT | COMPUTE`.
    - particles compute: layout adds binding 1 storage `read_write`
      (`BufferBindingLayout { ty: Storage, has_dynamic_offset: false,
      min_binding_size: None }`), binding 2 uniform; compute pipeline
      entry `cs_main`, workgroup 64 (in WGSL).
    - particles draw: layout binding 1 storage **read-only** (`ty: Storage,
      read_only: true` equivalent via `BufferBindingType::Storage {
      read_only: true }`), visibility `VERTEX`; render pipeline `TriangleStrip`,
      no depth, blend standard alpha (`ColorTargetState { format:
      Rgba8Unorm, blend: Some(BlendState::ALPHA_BLENDING), write_mask:
      ALL }`).
    - mesh: vertex buffer layout `[Float32x3, Float32x3, Float32x2]` stride
      `MESH_VERTEX_STRIDE`, topology `TriangleList` with the index buffer;
      depth-stencil `Depth24Plus` compare `Less`, depth_write true; `write_mask
      ALL`, no blend.
    - mesh texture: `Rgba8Unorm | COPY_DST | TEXTURE_BINDING`, 256×256,
      uploaded once via `queue.write_texture` from `math::checker_texture(256)`
      (`bytes_per_row: Some(256*4)`).
    - output pipelines: fullscreen triangle, `TriangleList`, no depth, no
      blend; targets `[Rgba8Unorm]`.
    - buffers initialized with `DeviceExt::create_buffer_init`
      (`BufferInitDescriptor { contents: &[u8], usage }`,
      `WGPU29/src/util/device.rs:7-20`); particle positions/velocities from
      `math::particle_init` per supported count (see `set_particle_count`).
    - `reconfigure(initial)` builds `Targets` for `initial` size.
  - `pub(in ..) fn set_particle_count(&mut self, count: u32)`: if the count
    differs, rebuild positions/velocities buffers via `particle_init` (only
    the first call per count; buffer per current count, old dropped).
  - `pub(in ..) fn reconfigure(&mut self, job_width: u32, job_height: u32)`:
    no-op when equal; else recreate `Targets` (scene texture `RENDER_ATTACHMENT
    | TEXTURE_BINDING`, depth `Depth24Plus` `RENDER_ATTACHMENT` when mesh,
    out texture `RENDER_ATTACHMENT | COPY_SRC`).
  - `pub(in ..) fn render(&mut self, job: RenderJob) -> anyhow::Result<Vec<u8>>`
    (blocking; background thread only):
    1. `reconfigure(job.width, job.height)`; `set_particle_count` when
       particles.
    2. CPU-side matrices via `math`: `view = translate(0,0,-3)`; `model =
       rotate_y(elapsed*0.5) * rotate_x(0.5)`; `proj = perspective(PI/4,
       width/height, 0.1, 100)`; `mvp = proj * view * model`; light
       `(0.6, 1.0, 0.4, 0)`; misc `(dt, elapsed, 0.02, 0)` (point scale).
       Attractor: `(sin(elapsed*0.7)*0.5, sin(elapsed*1.1)*0.3,
       cos(elapsed*0.7)*0.5, 2.2)`. Write both buffers
       (`queue.write_buffer`).
    3. Encode: Particles, compute pass `dispatch_workgroups(ceil(n/64))`,
       then scene pass (clear `Color{r:0.02,g:0.02,b:0.04,a:1.0}`, draw
       `0..4` verts, `0..n` instances). Mesh, scene pass with depth
       attachment (depth clear `1.0`), `draw_indexed(0..index_count, 0, 0..1)`.
    4. Output pass to `out_view` sampling `scene_view`: identity pipeline,
       or post pipeline (its own bind group needs the frame-params uniform
       binding 0 + scene texture/sampler 1/2).
    5. `engine.read_texture_bytes(&targets.out, job.width, job.height)` →
       BGRA bytes.
- Tests (`demos.test.rs` additions): `SPECS` table invariants (exactly one
  particles demo, two mesh demos, post only on MeshPost); a `#[ignore]`d GPU
  test rendering one 64×64 frame per demo (Particles at count 4096) asserting
  `len() == 64*64*4` and that the four corners + center bytes differ from a
  cleared frame (non-uniform content), and that MeshPost output differs from
  Mesh output byte-wise on the same `elapsed`.
- Done-check: `cargo test -p gpui-starter --lib features::wgpu::demos`
  passes; `-- --ignored` demo render passes locally.

## Phase 6: State entity and page UI

### Step 6.1: `WgpuPageState`

- Files: `src/features/wgpu/state.rs`, `state.test.rs`.
- Build (private fields, platform-neutral; uses only `clock::Instant` for
  time):

  ```rust
  pub(in crate::features::wgpu) struct WgpuPageState {
      demo_ix: usize,                     // 0=Particles, 1=Mesh, 2=MeshPost
      playing: bool,
      particle_count: u32,                 // 1_000_000 default
      pending_size: (u32, u32),            // quantized physical px, (0,0) = unknown
      applied_size: (u32, u32),
      frame_in_flight: bool,
      last_frame_started_at: Option<clock::Instant>,
      fps: FpsTracker,
      engine_ms_ema: f32,
      displayed: Option<Arc<RenderImage>>,
      stale_frames: Vec<Arc<RenderImage>>, // cap 4
      runtime_error: Option<String>,
      frames_rendered: u64,
  }
  ```

  `demo_ix`, not a `DemoId` field: `demos.rs` is native-only, so `state.rs`
  (wasm-clean) cannot hold `DemoId`. The native side maps the index through a
  `DemoId::from_ix(usize)` helper on `SceneRenderer`; all state methods
  take/return `usize` for the demo.
  Methods (notify rules per coding guides, one notify per coherent change):
  `new(cx) -> Self` (`playing = !cx.reduce_motion()`,
  `GPUI/app.rs:1222`; pending/applied `(0,0)`), `demo_ix()`,
  `set_demo_ix(ix, cx)` (resets `frames_rendered`, clears error, notify),
  `is_playing()`, `toggle_playing(cx)` (also clears `frame_in_flight` when
  pausing, notify), `particle_count()`, `set_particle_count(n, cx)` (only
  the three constants `100_000 | 500_000 | 1_000_000`, notify),
  `viewport_changed(physical: (u32, u32), cx)`, quantize to
  `((w + 15) / 16) * 16` clamped `[64, 4096]` per axis, set `pending_size`
  only when the quantized value changed (no notify, consumed by the next
  frame task), `pending_size()`, `size_changed_on_gpu(size, …)` sets
  `applied_size` (no notify), `is_frame_in_flight()`,
  `mark_frame_started(now, cx)` (no notify), `frame_done(now, image, ms, cx)`
  (push old `displayed` into `stale_frames` cap 4, set `displayed`, fps
  push, `engine_ms_ema = mix(ema, ms, 0.2)`, `frames_rendered += 1`, clear
  in-flight, one notify), `take_stale_frames()`, `displayed()`, `fps()`,
  `engine_ms()`, `frames_rendered()`, `runtime_error()`,
  `frame_failed(err, cx)` (clear in-flight, set error, notify),
  `clear_error(cx)`, `pacing_wait(now) -> Duration`
  (`FRAME_INTERVAL.saturating_sub(now - last_start)`,
  `const FRAME_INTERVAL: Duration = Duration::from_millis(1000 / 60)`).
- Tests (`state.test.rs`, `#[gpui_kit::test]` with `cx.new`): quantization
  table (`viewport_changed((100, 100)) → (112, 112)`; `(4095, 20) →
  (4096, 64)`; `(8, 8) → (64, 64)`); pausing while in-flight clears the flag;
  `frame_done` moves the old image to stale, notifies exactly once
  (observer count), advances the EMA; `take_stale_frames` drains; error set
  by `frame_failed` clears via `set_demo_ix`/`clear_error`.
- Done-check: `cargo test -p gpui-starter --lib features::wgpu::state` passes.

### Step 6.2: Page UI (native branch)

- Files: `src/features/wgpu/page.rs`.
- Build:
  - `WgpuPage` fields: `state: Entity<WgpuPageState>`,
    `#[cfg(not(target_family = "wasm"))] engine: Option<Arc<WgpuEngine>>`,
    `#[cfg(not(target_family = "wasm"))] renderer: Option<Arc<Mutex<SceneRenderer>>>`
    (`std::sync::Mutex`, the single-in-flight rule makes contention
    impossible; wgpu types are not `Sync`-mutably shareable otherwise),
    `_subscriptions: Vec<Subscription>`.
  - `new`: create state (`cx.new(|cx| WgpuPageState::new(cx))`); on native,
    spawn the init task (Step 7.1 pattern, called `start_engine`) and detach.
  - Render (native branch): `v_flex().min_h_full().p_6().gap_4()` with:
    1. Header row: `h_flex().w_full().flex_wrap().justify_between().
       items_center()`, the level-1 heading (from Step 1.2) and a right
       `h_flex().gap_2()` with one outline `Button` `("wgpu-toggle-play")`
       labeled `localize(playing ? "wgpu_pause" : "wgpu_play")`.
2. Demo selector, the component's own selection idiom (doc example at
   `gpui-component-0.7.1/src/button/button_group.rs:103-121`):
   `ButtonGroup::new("wgpu-demos")` with three children
   `Button::new("wgpu-demo-particles").label(localize("wgpu_demo_particles")).selected(demo_ix == 0)`
   (likewise mesh/mesh-post, `selected(demo_ix == 1/2)`) and one group-level
   `.on_click(cx.listener(|this, clicks: &Vec<usize>, _, cx| { if let
   Some(&ix) = clicks.first() { this.state.update(cx, |s, cx| s.set_demo_ix(ix, cx)); } }))`.
   `.selected(bool)` is the kit's persistent selected state, distinct from
   hover (`button.rs:543`).
3. Count selector, only when `demo_ix == 0`, same shape:
   `ButtonGroup::new("wgpu-count").outline()` with children
   `wgpu_particles_100k`/`wgpu_particles_500k`/`wgpu_particles_1m`,
   `.selected(count == that constant)`, group `on_click` mapping index →
   100_000/500_000/1_000_000 into `set_particle_count`.
    4. Viewport: `div().relative().flex_1().min_h_0().rounded(cx.theme().
       radius_lg).overflow_hidden().border_1().border_color(cx.theme().
       border)` containing: (a) the frame image —
       `state.displayed()` → `img(image.clone()).size_full().object_fit(
       ObjectFit::Contain)` (`img.rs:159,201`; `GPUI/style.rs:29-40`), or a
       centered muted `wgpu_waiting` placeholder when `None`; (b) the stats
       overlay, `div().absolute().top(rems(0.5)).left(rems(0.5)).px_2().
       py_1().rounded_md().bg(cx.theme().popover).border_1().border_color(
       cx.theme().border).text_xs().text_color(cx.theme().muted_foreground)`
       showing `localize_with_args("wgpu_fps", &[("fps", …)])`, engine ms
       (`wgpu_ms`), and `engine.as_ref().map(|e| e.adapter_label())`; (c)
       the ticker (Phase 7). The image is content, so it also gets
       `.role(Role::Image)` + `.id("wgpu-frame")` with an
       `.aria_label(localize("wgpu_surface_label"))` if the div role API
       requires a child text, otherwise `Role::Image` on the img wrapper
       div with the label suffices (accesskit `Role::Image` exists,
       `accesskit-0.24.1/src/lib.rs:68`; `role()` builder at
       `GPUI/elements/div.rs:1310`; repo usage
       `src/ui/widgets/virtual_list.rs:56`).
    5. Error state: when `runtime_error()` is Some, replace items 4's
       children with a bordered panel (`wgpu_error_title`, the error in
       `div().text_sm().text_color(cx.theme().danger)`, outline Retry
       button → `start_engine`).
  - Handlers: play toggle → `state.update(cx, |s, cx| s.toggle_playing(cx))`;
    demo buttons → `set_demo_ix` + renderer-dirty; count buttons →
    `set_particle_count` + renderer-dirty (renderer rebuild happens in the
    frame task via `set_particle_count`/`reconfigure`, so "dirty" is just the
    state change).
- Tests: none new (layout); Phase 7 covers behavior.
- Done-check: `cargo check -p gpui-starter` passes; manual `cargo run`
  optional (formal QA in Verification).

## Phase 7: Frame loop, resize, init task

### Step 7.1: Engine init task and renderer creation

- Files: `src/features/wgpu/page.rs`.
- Build: `start_engine(&mut self, cx: &mut Context<Self>)`:
  `self.engine = None; self.renderer = None;` clear error; capture
  `weak = cx.weak_entity()` (for state updates) and a `WeakEntity<Self>`
  via `cx.weak_entity()`; `cx.spawn(async move |cx| { let result =
  cx.background_executor().spawn(async move { WgpuEngine::new().await
  }).await; match result { Ok(engine) => { let engine = Arc::new(engine);
  cx.update(|cx| { // via weak entity update on Self
  ... self.engine = Some(engine.clone()); // renderer deferred: needs a
  nonzero pending size; created on first frame
  }) } Err(err) => { crate::capabilities::set(CAPABILITY_WGPU,
  CapabilityStatus::degraded(err.to_string()), cx); state.frame_failed(...) }
  } }).detach()`. On success set capability back to
  `CapabilityStatus::supported_enabled()`. First render task creates the
  `SceneRenderer` with `Arc::new(Mutex::new(SceneRenderer::new(engine,
  &job)?))` when `renderer.is_none()`.
- Tests: none (threaded init; error path covered by state tests).
- Done-check: `cargo check -p gpui-starter` passes.

### Step 7.2: Ticker element (drain, resize, kick)

- Files: `src/features/wgpu/page.rs` (private `render_ticker`).
- Build: helper returning
  `canvas(move |_, _, _| (), move |bounds, _, window, cx| { ... })`
  (`GPUI/elements/canvas.rs:9-22`) styled `.absolute().top_0().left_0().
  size_full()` so its bounds are the viewport's (Styled impl at
  `canvas.rs:91`). Paint closure body (native branch only, the helper is
  inside the cfg'd render arm):
  1. Drain: `for image in state.update(cx, |s, _| s.take_stale_frames()) {
     let _ = window.drop_image(image); }` (`GPUI/window.rs:5037-5048`).
  2. Resize: `let f = window.scale_factor() as f32;`
     (`GPUI/window.rs:2973`); `let phys = ((bounds.size.width.0 * f) as
     u32, (bounds.size.height.0 * f) as u32);` guard non-zero;
     `state.update(cx, |s, cx| s.viewport_changed(phys, cx));`
  3. Kick: read `state.read(cx)`; if `!playing || frame_in_flight ||
     engine.is_none() || error.is_some() || pending_size == (0,0)` return.
     Else register `window.on_next_frame(move |_, cx| { ... })`
     (`GPUI/window.rs:2650-2658`) capturing `WeakEntity<WgpuPageState>`,
     `WeakEntity<WgpuPage>`, demo ix, particle count, engine Arc clone.
  4. `on_next_frame` body: re-check eligibility via the weak entities; if
     eligible: `let now = clock::Instant::now();` `state.update(cx, |s, cx|
     s.mark_frame_started(now, cx));` then `cx.spawn(frame_task).detach()`
     (Step 7.3).
- Tests: resize quantization covered in Step 6.1; loop covered manually.
- Done-check: `cargo check -p gpui-starter` passes.

### Step 7.3: Frame task (pacing, render, completion)

- Files: `src/features/wgpu/page.rs`.
- Build: `async fn frame_task(...)` spawned from the kick:
  1. Pacing: `let wait = state_weak.read(&cx).pacing_wait(clock::Instant::
     now()); cx.background_executor().timer(wait).await;`
     (`GPUI/executor.rs:183`). Re-check playing after the wait; bail
     without touching in-flight if paused (pause clears the flag, Step
     6.1).
  2. Snapshot: `let (demo_ix, count, (w, h)) = state.read(&cx).(demo_ix(),
     particle_count(), pending_size()); let job = RenderJob { demo:
     DemoId::from_ix(demo_ix), particle_count: count, dt: 1/60 clamp
     measured, elapsed: measured since page mount (state holds a
     `started_at: clock::Instant`), width: w, height: h };`
  3. Render on the background executor:
     `cx.background_executor().spawn(async move { let t0 =
     clock::Instant::now(); let renderer = page_weak... // get or create
     Arc<Mutex<SceneRenderer>>; let guard = renderer.lock().expect(
     "wgpu renderer mutex"); guard.render(job).map(|bytes| (bytes,
     t0.elapsed())) }).await`, creating the renderer here on first frame
     (Step 7.1 note); a poisoned/creation failure maps to the error path.
  4. Completion: `let image = bgra_to_render_image(w, h, bytes);` then
     `state_weak.update(cx, |s, cx| s.frame_done(clock::Instant::now(),
     image, ms.as_secs_f32() * 1000.0, cx));` and
     `state.update(cx, |s, cx| s.size_changed_on_gpu((w, h), cx))`.
  5. On `Err(err)`: `crate::capabilities::set(CAPABILITY_WGPU,
     CapabilityStatus::degraded(err.to_string()), cx);` then
     `state.frame_failed(err, cx)`.
- Tests (`state.test.rs` addition): pausing mid-flight then completing the
  frame leaves the entity consistent (in-flight false, no stale double
  push).
- Done-check: `cargo test -p gpui-starter --lib features::wgpu` passes;
  manual run animates all three demos; GPU process memory flat over 60 s
  (drain works).

## Phase 8: Freeze and gates

### Step 8.1: Re-export and doc surface

- Files: `src/features/wgpu/mod.rs`.
- Build: module header matches the Removal Recipe; re-exports exactly the
  Interface Contract list; doc comments on `WgpuEngine::read_texture_bytes`
  (blocking, background-only) and the frame helpers (BGRA contract, padding)
  stating preconditions in plain sentences.
- Tests: existing.
- Done-check: `grep -rn "pub use" src/features/wgpu/` shows only the
  contract items; `grep -rn "use gpui::\|use wgpu::" src/features/wgpu/`
  shows no `gpui::` imports (kit facade rule, `tests/qa_migration.rs:386-405`)
  and `wgpu::` only inside cfg'd native modules.

### Step 8.2: Full gates

- Run, in order: `cargo fmt --all -- --check`; `cargo clippy --workspace
  --all-features`; `cargo test --workspace --all-features`;
  `just wasm-check`.
- Done-check: all four pass with zero new warnings;
  `cargo test -p gpui-starter --lib features::wgpu` green; the `#[ignore]`d
  GPU tests pass locally with `-- --ignored`.

## Platform & Wasm Gating

- cfg gates: `engine.rs` and `demos.rs` modules declared under
  `#[cfg(not(target_family = "wasm"))]` in `mod.rs`; the native render branch
  (ticker, viewport, error panel) sits inside a
  `#[cfg(not(target_family = "wasm"))]` arm of `WgpuPage::render`; the wasm
  arm renders the degradation panel (Step 1.3). `frame.rs`, `math.rs`,
  `state.rs`, `page.rs` compile on every target, with one exception:
  `bgra_to_render_image`/`rgba_to_render_image` in `frame.rs` (and their
  re-exports and tests) are `#[cfg(not(target_family = "wasm"))]` — they
  construct `image::Frame` and the `image` dependency exists only in the
  not-wasm table; ungated, `just wasm-check` fails (repo precedent for the
  gating: `src/app/assets.rs:23,30,41`). `state.rs` touches only
  `crate::platform::clock::Instant` (`src/platform/clock.rs:1-8`), no
  `std::time` in wasm-reachable code.
- Native-only deps live in `[target.'cfg(not(target_family = "wasm"))'.
  dependencies]` (`Cargo.toml:310`): `wgpu =29.0.4`, `image 0.25`
  (default-features off). `naga =29.0.4/wgsl-in` is a dev-dependency;
  `cargo check --lib` for wasm does not build dev-deps, so it never reaches
  the wasm gate.
- Capability matrix (`wgpu_embed`):
  - native: `supported_enabled` at startup; `degraded(err)` on
    adapter/device failure or a frame error (Retry resets to
    `supported_enabled` on success).
  - wasm: `supported: false, enabled: false, degraded: false` (repo wasm
    convention, `src/services/accessibility.rs:134-136`) with the reason
    literal in `init.rs`, "hidden with capability reason": the live surface
    is replaced by the localized panel; the page, sidebar entry, and deep
    link stay present so routing behaves identically across platforms.
- Platform behavior: macOS/Windows/Linux run the full page; wgpu selects
    metal/dx12/vulkan/gles by its default backend enumeration
    (`Backends::all()`). CI lanes on Linux/Windows compile it; the local
    machine (Linux) is the primary dev target. GPU-dependent tests are
    `#[ignore]`d because CI runners have no guaranteed adapter
    (`.github/workflows/ci.yml` has no GPU job, survey §7).
- `just wasm-check` must keep passing: the lib includes the page module and
  its wasm-clean halves only.

## i18n

Keys added to `locales/en.yml` and `locales/zh-CN.yml` in lockstep (facade
`crate::i18n::localize` / `localize_with_args`, `seams.md` §5; flat
top-level scalars, quoted, `[a-z0-9_]` names):

| key | en | zh-CN |
|---|---|---|
| `nav_wgpu` | "WGPU" | "WGPU" |
| `wgpu_title` | "WGPU" | "WGPU" |
| `wgpu_demo_particles` | "Particles" | "粒子" |
| `wgpu_demo_mesh` | "Mesh" | "网格" |
| `wgpu_demo_mesh_post` | "Mesh + Post" | "网格 + 后处理" |
| `wgpu_particles_100k` | "100K" | "10 万" |
| `wgpu_particles_500k` | "500K" | "50 万" |
| `wgpu_particles_1m` | "1M" | "100 万" |
| `wgpu_play` | "Play" | "播放" |
| `wgpu_pause` | "Pause" | "暂停" |
| `wgpu_fps` | "FPS: %{fps}" | "帧率：%{fps}" |
| `wgpu_ms` | "Frame: %{ms} ms" | "帧耗时：%{ms} 毫秒" |
| `wgpu_waiting` | "Waiting for first frame" | "等待首帧" |
| `wgpu_surface_label` | "Live wgpu render output" | "wgpu 实时渲染画面" |
| `wgpu_error_title` | "WGPU renderer failed" | "WGPU 渲染器初始化失败" |
| `wgpu_retry` | "Retry" | "重试" |
| `wgpu_unavailable_wasm` | "These demos render through an offscreen wgpu device, which the web build does not provide." | "这些演示依赖离屏 wgpu 设备渲染，Web 构建暂不提供。" |

Notes: FPS/ms format through `%{…}` placeholders
(`src/services/i18n.rs:7-13`); short control labels carry no terminal
punctuation; zh uses full-width punctuation only inside sentences; the
scanner only sees literal first arguments, so the indirect
`localize(self.title_key())` nav key (`nav_wgpu`) is covered by the manual
lockstep rule, not the scanner (`seams.md` §5 blind spot).

## Design Compliance

- Layout (design guides "Choose a stable shell", "Responsive desktop
  windows"): single-workspace page, control rows on top, viewport
  `flex_1().min_h_0()` taking all surplus; the render surface is the primary
  object and dominates. Header row wraps (`flex_wrap`) on narrow windows; no
  hover-only actions; the count selector hides only when the particles demo
  is inactive, and it is a mode switch, not a command (the demo buttons
  remain the path to it).
- Hierarchy ("Visual language"): one level-1 heading, one ButtonGroup per
  decision (demo, count), one stats overlay. No cards-in-cards; the error
  panel is a single bordered region, not an Alert stack.
- Tokens ("Color and themes"): overlay `cx.theme().popover` + `border` +
  `muted_foreground`; error `danger`; viewport frame `border` +
  `theme.radius_lg`. No raw hex/rgb/hsla in page code, the GPU pixels are
  raster data whose color is the content, exempt per "audited data/raster
  content".
- Density/spacing ("Radius, spacing, and density", "Spatial grammar"): page
  `p_6`, section gap `gap_4` (separate sections), header `gap_2` (closely
  related controls); text via `text_sm`/`text_xs` rem helpers. The only
  geometry literals are the overlay's two `rems(0.5)` insets and the physical
  quantization constant (16 px, a raster boundary, documented exception).
- Interaction states: kit `Button`/`ButtonGroup` own hover, pressed,
  focus-visible, disabled; the active demo/count child carries
  `.selected(true)` ("Selected/checked: persistent state distinct from
  hover", `button.rs:543`); Play/Pause label reflects state so the demo is
  never state-by-color-only.
- Motion ("Motion", "Zoom, base font, and rem"): the animation is user-played
  content (video-player class per `GPUI/window.rs:2668-2674` doc); the loop
  starts paused when `App::reduce_motion()` is true and runs after an
  explicit Play. No decorative chrome animation.
- A11y ("Accessibility checklist"): heading announces the page; every control
  is a labeled kit component (keyboard path: Tab to Play/demo buttons, Enter
  to activate); the live surface exposes `Role::Image` with a label
  (`GPUI/elements/div.rs:1310`, `accesskit-0.24.1/src/lib.rs:68`); the stats
  overlay is supplementary muted text, not the sole carrier of state.
- Pointer conventions: default arrow cursors throughout; no custom cursors
  (no pointer manipulation in v1, the attractor is automatic).

## Verification

Repo gates, in this order (AGENTS.md; first three mandatory before "done"):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

Feature-targeted automation (wired in the phases):

```sh
cargo test -p gpui-starter --lib features::wgpu   # frame, math, state, demos (naga)
cargo test --test e2e_navigation                   # page count bumped
cargo test --test qa_localization                  # key lockstep
cargo test -p gpui-starter --lib features::wgpu -- --ignored  # GPU smokes, local only
```

Manual QA on `cargo run` (scratch tree, not committed):

1. Sidebar shows "WGPU" last with the gpu icon; deep link
   `gpui-starter://wgpu` opens it.
2. Particles demo at 1M: fps overlay reads ≥ 55 on the dev machine; frame ms
   stays under ~16 ms; particles orbit the moving attractor and reflect off
   the containment sphere.
3. Count buttons 100K/500K/1M switch without a stall longer than one frame
   task (buffers rebuild off the UI thread, UI stays responsive while
   dragging another window).
4. Mesh demo: textured sphere rotates, correct depth ordering (no faces
   bleeding through), texture stripes visible.
5. Mesh + Post: same scene with vignette and pulsing chromatic fringing at
   the edges; output visibly differs from the plain Mesh demo.
6. Resize: drag the window edge, the render re-targets in 16 px steps with
   no stretched aspect (Contain letterbox during transients), no leak (GPU
   memory flat across 30 s of continuous resize).
7. Pause freezes frames and drops CPU/GPU to idle; Play resumes.
8. Light and dark themes: overlay legible in both; no raw-color artifacts.
9. Reduce-motion (OS flag): page opens paused; Play works.
10. Error path: run with an unusable backend (e.g. `WGPU_BACKEND=gl` on a
    headless box), error panel appears, diagnostics page lists `wgpu_embed`
    degraded, Retry recovers once a backend is available.
11. Wasm (`just wasm-check` compile-only here): the browser build renders the
    capability-reason panel; diagnostics lists `wgpu_embed` unsupported.

Acceptance criteria: all four repo gates pass; the targeted tests pass
including every naga WGSL validation; the manual checklist is confirmed on a
scratch tree or waived with a note naming which items were checked and how.

## Risks & Open Questions

Ranked risks:

1. **CPU readback cost is the architecture's floor.** Every frame crosses
   GPU→CPU (`map` + padded-row copy ≈ 3.7 MB at 1280×720; ≈ 220 MB/s at
   60 fps) and re-uploads through the sprite atlas. This is the only
   supported path, no surface/texture interop exists (see Research Digest
   "No GPU-surface share"). The BGRA swizzle is free (GPU-side in the output
   pass); if a demo misses pacing, lower its resolution or the particle
   count before touching the loop.
2. **1M particles at 60 fps is a hardware target, not a guarantee.** 1M
   instanced quads = 2M triangles plus a 1M-item compute pass per frame;
   integrated GPUs may fall short. The count selector (100K default tier
   available) is the documented degradation; the fps overlay makes the miss
   visible. Not measured during planning (no GPU run, see Open Questions).
3. **Atlas growth if the ticker stops painting while frames land.** No LRU
   in the atlas (`GPUI/window.rs:4948-4960`); `stale_frames` is capped at 4
   and drained on every ticker paint; the ticker is mounted whenever the
   viewport is, so paint cycles coincide. Same bounded-leak shape as plan 07.
4. **Storage buffer in the vertex stage.** The particles draw reads a
   read-only storage buffer in `VERTEX`, valid WebGPU and accepted by naga
   (the demos test parses it), but the GL backend historically had the
   tightest limits. Fallback if a backend rejects it: after the compute
   pass, `copy_buffer_to_buffer` positions into a `VERTEX`-usage buffer and
   bind that (one extra 16 MB copy at 1M; still off the UI thread). Decide
   only if a real backend fails, naga + local Vulkan run pass in the happy
   path.
5. **wgpu version coupling + plan 07 coexistence.** The `=29.0.4` pin must
   track gpui-pre-wgpu's resolution; a gpui bump to wgpu 30 breaks it loudly
   (intended failure mode; `tests/qa_migration.rs:131-146` guards stray
   copies). Plan 07 (canvas-shaders) consumes this bridge (`WgpuEngine`,
   `bgra_to_render_image`, `FpsTracker`, 07:33-47), so the same pin governs
   both plans and any change to the bridge's pub surface must be checked
   against 07's import list. One wgpu instance, one owner: the engine,
   readback, and image helpers live here only.
6. **`InstanceDescriptor::default()` does not exist** in wgpu-types-29.0.4
   (`WTYPES/instance.rs:26`; exhaustive grep). This plan constructs the
   descriptor explicitly; the trap is recorded because any future direct
   `wgpu::Instance::new` caller can fall into it (07's superseded draft did).
7. **Resize churn.** Continuous window dragging recreates textures once per
   16-px quantized step; at 4K that is ~240 steps corner-to-corner, each a
   few texture allocations on the background thread. Bounded by the clamp
   [64, 4096] and single-in-flight pacing.
8. **macOS/Windows compile not exercised here.** Planning ran on Linux;
   wgpu 29 default features cover metal/dx12, and CI's check lanes compile
   both, but no macOS/Windows execution happened during planning.
9. **GPU tests are `#[ignore]`d.** CI runners have no guaranteed adapter;
   shader correctness is pinned by naga parse tests plus local `--ignored`
   runs; visual checks stay manual (checklist items 2-5).

Open questions (each with what was tried):

- **1M@60fps on the target dev machine.** Not run: planning is doc-only and
  no `cargo run`/GPU benchmark was executed this session. The manual QA item
  2 is the acceptance gate; the count selector is the fallback.
- **Landing order vs plans 05/06/07/09.** Each adds a sidebar page and bumps
  `e2e_navigation`; absolute counts/positions were therefore written
  relative ("append last", "bump by one"). Tried: `ls docs/plans/`, plans
  01-10 exist; no implementation order document is on disk.
- **Icon variant reachability — resolved.** The draft picked `Gpu` after
  verifying only that `gpu.svg` exists in the full catalog; the app registers
  the default 106 (`default-icons.txt` + generated component enum), so `Gpu`
  compiles only via the assets enum and renders blank on native. Revision:
  `IconName::Cpu`, verified in the default subset. Compilation remains the
  arbiter for any future swap.
- **`.aria_label` on the Role::Image wrapper.** `role()` is verified
  (`GPUI/elements/div.rs:1310`) and repo a11y usage exists
  (`src/ui/widgets/virtual_list.rs:56` uses `Role::List`); the exact label
  builder for a non-text node was not verified against an in-repo example
  (`src/features/pages/notifications.rs:50-53` uses `a11y_live`). If no
  builder fits, wrap the img in a `div().role(Role::Image)` carrying a
  visually-hidden text child, the kit's text node then carries the label.
- **Whether the GL/wgpu fallback in risk 4 is ever needed.** naga 29.0.4
  parses the storage-in-vertex WGSL (test-pinned); no GL-backend device was
  available to exercise it here (`WGPU_BACKEND=gl` smoke was not run).
