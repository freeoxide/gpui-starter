# 16 Waveform Foundation Plan

## Goal

A shared waveform visualization widget lives under `src/ui/waveform/` with no page of its own. An audio feature moves one `SampleProducer` to its audio thread and pushes interleaved f32 frames into it; the UI side drains a lock-free SPSC ring on a 30 Hz tick, feeds a min/max/RMS decimation pyramid, and repaints a custom Element drawing a scrolling oscilloscope (envelope or line mode by zoom) or a log-frequency spectrum-bar view, device-pixel sharp, themed from kit tokens, with pause/resume and a configurable time window. Plans 18, 19, and optionally 13 consume the Interface Contract below.

## Dependencies

No plan dependencies. This is a foundation; plans 18, 19, and 13 are not on disk (`ls docs/plans/` shows 01 through 11 only, checked this session), so nothing of theirs is consumed. Inputs used:

- `docs/plans/_survey/seams.md` sections 2 (ui widget seam), 6 (wasm), 7 (tests), 8 (gates), 9 (rules), re-verified where cited below.
- `docs/plans/_survey/ecosystem.md` sections a (Element), d (timer), h (rustfft) as starting points; every load-bearing claim re-verified against crate sources or docs.rs this session.
- `docs/plans/01-canvas-foundation.md` for element conventions (Element impl shape, `Size::full()` layout idiom, `IntoElement` pair, observe contract, sibling tests). Plan 01 is not implemented yet (`src/ui/mod.rs:1` still reads `pub mod widgets;` only), so this plan follows its documented conventions rather than consuming code; both modules land as siblings under `src/ui/`.

## Research Digest

Path key: `GPUI` = `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-0.3.8/src`, `GB` = `.../gpui-base-0.7.1/src`, `COMP` = `.../gpui-component-0.7.1/src`, `REPO` = `/home/thragg/fo/gpui-starter`.

Element lifecycle (re-verified this session):

- `Element` trait: associated types `RequestLayoutState`/`PrepaintState: 'static`, `id() -> Option<ElementId>`, `source_location()` with no default body (`GPUI/element.rs:53-72`). Supertrait `IntoElement` requires a paired `impl IntoElement` (`GPUI/element.rs:147-159`); `AnyElement` sets the precedent of `type Element = Self` (`GPUI/element.rs:717`).
- Full-container layout idiom: `Style { size: Size::full(), ..Default::default() }` passed to `window.request_layout(style, None, cx)` (`GB/plot/element.rs:74-84`).
- Paint primitives: `window.paint_quad(quad)` (`GPUI/window.rs:4550`), free fn `fill(bounds, background) -> PaintQuad` (`GPUI/window.rs:7694`), `window.paint_path(path, color)` (`GPUI/window.rs:4621`), `PathBuilder` with `stroke(width)`, `move_to`, `line_to`, `build() -> Result<Path<Pixels>>` (`GPUI/path_builder.rs:88-98, 125-152, 244`).
- Clipping: `window.with_content_mask(Some(ContentMask { bounds }), f)` (`GPUI/window.rs:4029`).
- Device pixel ratio: `window.scale_factor() -> f32` (`GPUI/window.rs:2973`); `DevicePixels(pub i32)` (`GPUI/geometry.rs:2994`). `Bounds` has pub `origin`/`size` fields (`GPUI/geometry.rs:723-727`). `point()`/`size()`/`px()` constructors flow through the kit glob (repo usage: `src/gallery/sections/charts/fps.rs:87`, `src/ui/widgets/virtual_list.rs:10`).
- `ElementId` converts from `&'static str`, `usize`, `String`, tuples and more (`GPUI/window.rs:7514-7568`).
- Geometry arithmetic: `Pixels` derives `Add, AddAssign, Sub, SubAssign, Neg` (`GPUI/geometry.rs:2672-2685`) with `Mul<f32>`/`Mul<usize>` (`:2719-2741`) and `Pixels / Pixels -> f32` (`:2690-2694`); the field `Pixels.0` is `pub(crate)` (`:2689`), so conversions go through `as_f32()`. `Bounds` has `top()/bottom()/left()/right()` (`:1296-1323`) and `center() -> Point<T>` (`:1001`); there is no `center_y()`.

Theme tokens for data visualization (verified in `COMP`):

- `Theme` carries `chart_1: Hsla` (`COMP/theme/theme_color.rs:133`), `chart_2: Hsla` (`:135`), `chart_grid: Hsla` (`:147`), plus `primary`, `background` (kit standard). `ActiveTheme for App` gives `cx.theme()` in paint's `&mut App` (`COMP/theme/mod.rs:44-50`). Repo precedent: charts read `cx.theme().chart_2`/`chart_4` (`src/gallery/sections/charts/system_monitor.rs:305,314`) and layer token colors with `opacity()` (`system_monitor.rs:291`).

Timer and entity patterns (re-verified):

- `cx.background_executor().timer(Duration) -> Task<()>` (`GPUI/executor.rs:179-186`, scheduler trait fn at `gpui-pre-scheduler-0.3.8/src/scheduler.rs:130`, platform impl at `GPUI/platform_scheduler.rs:68`). Wasm evidence: the gallery's `SystemMonitorSection` runs `cx.spawn` + `background_executor().timer(INTERVAL)` (`src/gallery/sections/charts/system_monitor.rs:217-230`) and the gallery compiles into the wasm lib (seams section 6).
- Weak-entity spawn loop with `this.update(cx, ...).is_err()` break, exactly as `system_monitor.rs:217-230`.
- Tests: `#[gpui_kit::test]` with `cx: &TestAppContext`, `use gpui_kit::TestAppContext;`, `cx.executor().advance_clock(Duration)` and `cx.executor().run_until_parked()` (`src/state/config_store.test.rs:1-4, 280-295`). `TestAppContext::add_window` (`GPUI/app/test_context.rs:279-296`).

Crate picks (both new; neither is in `Cargo.lock` today, verified with `grep -E '^name = "(rustfft|rtrb|ringbuf|realfft|symphonia)"' Cargo.lock` returning no hits for them):

- `rtrb = "0.4"` (0.4.0, updated 2026-08-17, MIT OR Apache-2.0). Purpose-built real-time SPSC ring buffer; `#![cfg_attr(not(feature = "std"), no_std)]` so it compiles on wasm32-unknown-unknown; zero dependencies. Verified API (docs.rs, fetched 2026-10-12): `RingBuffer::new(capacity) -> (Producer<T>, Consumer<T>)`; `Producer::push_partial_slice(&mut self, &[T]) -> (&[T] copied, &[T] remainder)` for `T: Copy`, never fails; `Consumer::pop_partial_slice(&mut self, &mut [T]) -> (&mut [T] filled, &mut [T] remainder)`; `Producer`/`Consumer` are `Send` (not `Sync`) when `T: Send`; a dropped Consumer makes `Producer::is_abandoned()` true. Chosen over hand-rolling: a correct SPSC ring needs `UnsafeCell` plus `Acquire`/`Release` ordering discipline; rtrb is the audited version of exactly that, and its partial-slice API matches the audio-callback contract (never block, push what fits). Chosen over `ringbuf` (larger, more modes than needed).
- `rustfft = "6.4"` (6.4.1, MIT OR Apache-2.0; release date unverified, see Open Questions). Verified API (docs.rs, fetched 2026-10-12): `FftPlanner::<f32>::new()`, `plan_fft_forward(len: usize) -> Arc<dyn Fft<f32>>` (planner may be dropped afterwards); `Fft::process(&self, &mut [Complex<f32>])` allocates scratch internally, `process_with_scratch(&self, buffer, scratch)` reuses a caller buffer sized by `get_inplace_scratch_len()`; buffer length must be `>= len` and a multiple of it; output unnormalized; `Complex` comes from the re-exported `num_complex`. SIMD auto-selects AVX/SSE4.1/NEON on native; `wasm_simd` is a separate off-by-default feature, so wasm32 compiles the scalar path. Chosen over hand-rolled radix-2: one 1024-point transform at a 30 Hz tick costs tens of microseconds either way, so hand-rolling only saves compile time in a tree that already compiles gpui and reqwest, while taking on bit-reversal and twiddle-factor correctness risk. Chosen over `realfft`: real-input packing saves half the work, which is irrelevant at this rate, and adds a second crate.

Both go in the common `[dependencies]` table: rtrb is no_std by default and rustfft is pure Rust with per-target `cfg` intrinsics, so neither needs the not-wasm table (seams section 6 rules).

Repo constraints (from seams, spot-verified): kit facade only, no `use gpui::` (`tests/qa_migration.rs:386-405`); sibling test wiring `#[cfg(test)] #[path = "x.test.rs"] mod x_test;` (`src/ui/widgets/virtual_list.rs:90-92`); module pattern `use gpui_kit::{prelude::*, *};` (`virtual_list.rs:1-6`); Cargo.lock must be committed with manifest changes (`tests/qa_migration.rs` lock helpers at `:17-30`, rule cited by seams at `:167-186`); `std::time::Duration` is identical on wasm (web-time re-exports the core type; `src/platform/clock.rs:6,9` re-exports it under both cfgs), so `Duration` in the public API needs no clock shim, and the widget never reads a clock at all.

## Architecture

Ownership model (kit coding guides: state in the narrowest owner; entity-backed system, value-like element to the caller):

- `SampleProducer` is the only object that crosses onto an audio thread. It owns the rtrb `Producer` half plus an `Arc<AtomicU64>` drop counter shared with the UI side. It is `Send` and single-owner (rtrb halves are `Send`, not `Sync`).
- `WaveformState` is the only retained UI state, an `Entity<WaveformState>` owned by the mounting view (plans 18/19). It owns the rtrb `Consumer` half, one `ChannelHistory` per channel (raw ring plus min/max/RMS pyramid), a `SpectrumAnalyzer` (FFT plan, scratch, and input buffers), the pause/window/mode flags, and the tick `Task<()>`.
- `Waveform` is a value-like element built each render (`Waveform::new(id, entity)`); its `WaveformElement` implements `Element`, fills its parent, and paints from `state.read(cx)`. No hitbox, no pointer listeners, no keyed element state: v1 is a pure display.
- Data flow: audio callback pushes interleaved f32 frames (never blocks; overflow counted as dropped) into the ring. A tick loop spawned at `attach_source` fires every 33 ms, calls `WaveformState::refresh`, which drains the ring into a scratch buffer, deinterleaves into the per-channel histories (raw ring append plus pyramid aggregation), recomputes the spectrum from the newest 1024-frame mixdown, and notifies once. Paint reads state only: it queries the pyramid for the configured window and column count, snaps geometry to device pixels, and emits quads/paths.
- Pause: the tick keeps draining (so the ring never backs up) but discards; the display and spectrum freeze. Resume continues from the current stream position, leaving a gap.
- Redraw: `refresh` notifies only when it integrated new data. The mounting view must `cx.observe(&state, |_, _, cx| cx.notify())` (same contract as plan 01's Canvas).

Decimation pyramid (per channel): a raw `VecDeque<f32>` ring capped at 16384 samples serves line-mode zoom, FFT input, and provisional-entry computation. Level k aggregates `8^(k+1)` raw samples per entry (`MinMaxRms { min, max, rms }`), built by chaining accumulators: raw feeds level 0 in groups of 8, each completed level-k entry feeds level k+1's accumulator in raw-sample units. Level factors are capped at the raw capacity so an incomplete final group always lies inside the raw ring; queries append that group (min/max/rms scanned from raw) as a provisional newest entry, so zoomed-out views lag at most one group. Level capacities are `ceil(retention_frames / factor)`, so the levels cover the retention window and evict from the front in lockstep with time. Queries pick the raw ring whenever the window fits inside it, otherwise the finest level whose factor covers samples-per-column, and map entries to columns with floor division so coverage is gapless in both directions.

Module tree:

```text
src/ui/waveform/
├── mod.rs        seam: header comment + module decls + re-exports
├── ring.rs       SPSC channel wrapper over rtrb: SampleProducer/SampleConsumer
├── history.rs    ChannelHistory: raw ring + min/max/rms pyramid + column query
├── spectrum.rs   SpectrumAnalyzer (rustfft plan/scratch) + SpectrumFrame
├── state.rs      WaveformState entity: attach/pause/window/mode, tick, refresh
├── paint.rs      pure geometry: device snapping, time grid, envelope/bars/line
├── element.rs    Waveform/WaveformElement: Element impl + painters
└── *.test.rs     sibling test modules per repo convention
```

## Interface Contract

Exact pub surface. Consumers (plans 18, 19, optionally 13) import from `gpui_starter::ui::waveform`. Behavioral structs keep private fields with builders/readers; record-like data stays plain. Everything that may grow is `#[non_exhaustive]`.

```rust
// src/ui/waveform/mod.rs — re-exports every item below.
use std::time::Duration;
use gpui_kit::*;

pub const MIN_WINDOW: Duration = Duration::from_millis(50);
pub const DEFAULT_WINDOW: Duration = Duration::from_secs(5);
pub const DEFAULT_RETENTION: Duration = Duration::from_secs(10);
pub const MIN_RETENTION: Duration = Duration::from_secs(1);
pub const MAX_RETENTION: Duration = Duration::from_secs(30);
/// Bar count of every `SpectrumFrame` this version produces.
pub const SPECTRUM_BARS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum WaveformMode { #[default] Oscilloscope, Spectrum }

/// Producer half of the sample channel. `Send`, single-owner (move it onto
/// the audio thread; it is not `Sync`). Samples are interleaved f32 frames
/// normalized to ±1.0. Pushing never blocks: frames that do not fit are
/// dropped and counted.
pub struct SampleProducer { /* private */ }
impl SampleProducer {
    /// Pushes whole interleaved frames; a trailing partial frame is rejected.
    /// Returns the number of whole frames copied; the uncopied remainder
    /// (including any partial frame) is counted in `dropped_samples()`.
    pub fn push_interleaved(&mut self, samples: &[f32]) -> usize;
    pub fn channels(&self) -> usize;            // 1 or 2
    pub fn sample_rate(&self) -> u32;
    pub fn capacity_frames(&self) -> usize;
    pub fn dropped_samples(&self) -> u64;
    /// False once the UI side detached or the state entity was dropped.
    pub fn is_attached(&self) -> bool;
}

/// Retained waveform state. Mount with `cx.new(|_| WaveformState::new())`,
/// render with `Waveform::new(id, entity)`, and observe:
/// `cx.observe(&state, |_, _, cx| cx.notify())`. All mutators end with one
/// `cx.notify()`.
pub struct WaveformState { /* fields private */ }
impl Default for WaveformState;   // -> Self::new()

impl WaveformState {
    pub fn new() -> Self;

    /// Creates the sample channel, starts the 33 ms refresh tick, and returns
    /// the producer for the audio thread. `channels` must be 1 or 2 (panics
    /// otherwise); `retention` is clamped to [MIN_RETENTION, MAX_RETENTION].
    /// Replaces any previous source (the old producer sees `is_attached() ==
    /// false`).
    pub fn attach_source(
        &mut self,
        sample_rate: u32,
        channels: usize,
        retention: Duration,
        cx: &mut Context<Self>,
    ) -> SampleProducer;

    /// Drops the channel, cancels the tick, clears history and spectrum.
    pub fn detach_source(&mut self, cx: &mut Context<Self>);

    pub fn has_source(&self) -> bool;
    pub fn sample_rate(&self) -> Option<u32>;
    pub fn channels(&self) -> usize;           // 0 while detached

    /// Paused: the tick keeps draining and discarding, display freezes.
    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>);
    pub fn is_paused(&self) -> bool;

    /// Clamped to [MIN_WINDOW, retention]. Only changes the query range.
    pub fn set_window(&mut self, window: Duration, cx: &mut Context<Self>);
    pub fn window(&self) -> Duration;
    pub fn retention(&self) -> Duration;       // DEFAULT_RETENTION while detached

    pub fn set_mode(&mut self, mode: WaveformMode, cx: &mut Context<Self>);
    pub fn mode(&self) -> WaveformMode;

    /// Samples the producer could not push since attach. Diagnostics readout.
    pub fn dropped_samples(&self) -> u64;

    /// Drains the ring, integrates history, recomputes the spectrum, notifies
    /// once if unpaused data arrived. The internal tick calls this; tests and
    /// embedders may call it directly to force a synchronous update.
    pub fn refresh(&mut self, cx: &mut Context<Self>);

    /// Latest spectrum over the 1024-frame mixdown; None before first data.
    pub fn spectrum(&self) -> Option<&SpectrumFrame>;
}

/// One computed spectrum. `bars()` holds normalized 0.0..=1.0 heights mapped
/// from [MIN_DB, 0] dB, log-spaced from max(20 Hz, fft bin width) to Nyquist.
pub struct SpectrumFrame { /* private */ }
impl SpectrumFrame {
    pub fn bars(&self) -> &[f32];              // len == SPECTRUM_BARS
    pub fn fft_size(&self) -> usize;           // 1024 in this version
    pub fn sample_rate(&self) -> u32;
    /// Inclusive Hz range bar `ix` aggregates; None when out of range.
    pub fn bar_band(&self, ix: usize) -> Option<(f32, f32)>;
}

/// Mounts the waveform. Fills its parent. Non-interactive: no hitbox, no
/// pointer handling, no a11y node in v1.
pub struct Waveform { /* private */ }
impl Waveform {
    pub fn new(id: impl Into<ElementId>, state: Entity<WaveformState>) -> Self;
    /// Overrides the theme colorway for every lane and the spectrum bars.
    /// Default colorway: mono lane and bars use `theme.primary`; stereo lanes
    /// use `theme.chart_1` and `theme.chart_2`.
    pub fn accent(mut self, color: impl Into<Hsla>) -> Self;
}
impl IntoElement for Waveform { type Element = WaveformElement; /* into_element() constructs it */ }
pub struct WaveformElement { /* private; build via `Waveform` */ }
impl IntoElement for WaveformElement { type Element = Self; /* required by the Element supertrait */ }
// impl Element for WaveformElement:
//   RequestLayoutState = (), PrepaintState = (), id() -> Some(id),
//   request_layout -> Size::full() style, prepaint -> (), paint -> painters.

// pub(crate) internals (not exported past ui::waveform):
//   ring.rs:        fn sample_channel(...) -> (SampleProducer, SampleConsumer)
//   history.rs:     ChannelHistory, MinMaxRms, ColumnAgg, LEVEL_STRIDE, RAW_RING_SAMPLES
//   spectrum.rs:    SpectrumAnalyzer (plan + scratch + input buffers), MIN_DB
//   paint.rs:       pure geometry helpers
//   state.rs:       pub(crate) enum WaveformSamples { Line(Vec<f32>), Envelope(Vec<ColumnAgg>) }
//                   pub(crate) fn channel_view(&self, ch: usize, columns: usize) -> WaveformSamples
//                     // computes window_samples from window()+sample_rate(), returns Line when
//                     // window_samples <= columns, else Envelope from ChannelHistory::query
//                     // (which picks raw or level internally); element.rs only draws
```

Consumption notes that are part of the contract:

- Mount recipe: create the entity, keep it in the view, `cx.observe(&state, |_, _, cx| cx.notify())` once, render `Waveform::new("player-waveform", state.clone())`.
- `attach_source` returns the only producer; clone it into the audio callback via move. If plans 18/19 need multiple feeder threads they must serialize pushes themselves (rtrb halves are single-owner).
- Values are expected normalized to ±1.0 (cpal f32 convention); display clamps to that range. Gain staging is the consumer's job.
- The tick loop lives inside the state and stops when the entity drops; consumers never manage it.
- No `EventEmitter` in v1: every observable change (pause, window, mode) is consumer-driven, so there is nothing an owner needs to hear back. If 18/19 need overflow notifications, add an event in a follow-up contract edit rather than polling `dropped_samples`.

## Module Layout

| File | Purpose |
|---|---|
| `src/ui/waveform/mod.rs` | Seam: header comment with removal recipe, `mod` decls, `pub use` re-exports |
| `src/ui/waveform/ring.rs` | `sample_channel`, `SampleProducer`, `SampleConsumer`; frame alignment and drop accounting over rtrb 0.4 |
| `src/ui/waveform/history.rs` | `ChannelHistory` (raw ring, `Level`, accumulator chain, eviction), `MinMaxRms`, `ColumnAgg`, column query |
| `src/ui/waveform/spectrum.rs` | `SpectrumAnalyzer` (rustfft plan, scratch reuse, log bands, dB mapping), `SpectrumFrame` |
| `src/ui/waveform/state.rs` | `WaveformState` entity, constants, attach/detach, pause/window/mode, `refresh`, tick task |
| `src/ui/waveform/paint.rs` | Pure geometry: device snapping, nice time steps, gridline positions, envelope/rms/line/bar geometry |
| `src/ui/waveform/element.rs` | `Waveform`/`WaveformElement`: `Element` impl, grid/lane/mode painters, token colorways |
| `src/ui/waveform/*.test.rs` | Sibling test modules (`#[cfg(test)] #[path = ...]`) for ring, history, spectrum, state, paint, element |

## Seam Touches

Two edits outside the new module:

- `src/ui/mod.rs` (currently `pub mod widgets;`, `src/ui/mod.rs:1`): add `pub mod waveform;`. One line; the widget is consumed through `gpui_starter::ui::waveform` like `ui::widgets` (seams section 2).
- `Cargo.toml` `[dependencies]`: add `rtrb = "0.4"` and `rustfft = "6.4"`. Then run `cargo check --workspace` to regenerate `Cargo.lock` and commit the lock with the manifest change (`tests/qa_migration.rs:167-186` rule). Both crates are wasm-safe, so neither goes under a target table.

Deliberate non-touches: no `src/features/pages/` entry, no `src/shell/sidebar.rs` variant (page count test `tests/e2e_navigation.rs:14-19` untouched), no gallery section, no `locales/` keys, no capability-registry entry (no platform variance to report; see Platform & Wasm Gating), no commands.

### Removal Recipe

`mod.rs` opens with (mirroring `src/gallery/mod.rs:1-7`):

```rust
//! Removable shared waveform widget library. `ui::waveform` is the only
//! public surface; features mount it as a child element, nothing routes to
//! it. Removal = delete this directory, remove the `pub mod waveform;` line
//! from src/ui/mod.rs (sibling module lines stay), and remove the
//! rtrb/rustfft entries plus regenerated Cargo.lock in Cargo.toml.
```

Ordered removal steps:

1. Remove the `pub mod waveform;` line from `src/ui/mod.rs`; leave every other line (including plan 01's `pub mod canvas;` if it has landed) untouched.
2. `rm -r src/ui/waveform/`.
3. Remove the `rtrb` and `rustfft` lines from `Cargo.toml`, run `cargo check --workspace`, commit the regenerated `Cargo.lock`.
4. Grep `ui::waveform` across `src/` and `tests/`: hits remaining are dependent features (plans 18/19/13); removal is only complete when those are removed or stubbed first.
5. Run the gates in Verification; they must pass with zero references left.

## Implementation Phases

## Phase 1: Sample channel

### Step 1.1: Dependencies and scaffold

- Files: `Cargo.toml`, `src/ui/mod.rs`, `src/ui/waveform/mod.rs`.
- Build: add `rtrb = "0.4"` and `rustfft = "6.4"` to `[dependencies]` (common table, after `rust-embed = "8"` to keep the list tidy); add `pub mod waveform;` to `src/ui/mod.rs`; create `mod.rs` with the removal-recipe header, `mod ring;` and a temporary empty re-export of `ring` items.
- Tests: none.
- Done-check: `cargo check --workspace` passes; `git diff Cargo.lock | grep -E 'rtrb|rustfft|num-complex|num-traits'` shows the new packages added.

### Step 1.2: Ring wrapper

- Files: `src/ui/waveform/ring.rs`, `src/ui/waveform/ring.test.rs` (wired from `ring.rs` with the sibling pattern).
- Build: `pub(crate) fn sample_channel(sample_rate: u32, channels: usize) -> (SampleProducer, SampleConsumer)` calling `rtrb::RingBuffer::new(SOURCE_RING_FRAMES * channels)` (const `SOURCE_RING_FRAMES: usize = 1 << 14`). `SampleProducer { producer: rtrb::Producer<f32>, dropped: Arc<AtomicU64>, sample_rate, channels }` with the contract methods: `push_interleaved` truncates to whole frames (`usable = samples.len() - samples.len() % channels`), calls `push_partial_slice`, adds `samples.len() - copied` to `dropped` with `fetch_add`, returns `copied / channels`; `is_attached` is `!producer.is_abandoned()`. `SampleConsumer` (pub(crate)) mirrors it: `drain_into(&mut self, out: &mut Vec<f32>)` that `pop_partial_slice`s into a chunked read (loop a fixed 4096-slot stack buffer, extend `out`) and `dropped()` reading the same `Arc`.
- Tests: `fn _assert_send() { fn assert_send<T: Send>() {} assert_send::<SampleProducer>(); }` compile check; push/pop roundtrip of interleaved stereo frames preserves order and length; partial trailing frame is rejected (returns frames floor, remainder counted in `dropped_samples`); overflow beyond capacity increments `dropped_samples` by the exact uncopied count and keeps earlier data intact; after dropping the consumer, `is_attached()` is false.
- Done-check: `cargo test -p gpui-starter --lib waveform::ring` passes.

## Phase 2: Decimation history

### Step 2.1: Pyramid append and eviction

- Files: `src/ui/waveform/history.rs`, `src/ui/waveform/history.test.rs`.
- Build: `#[derive(Clone, Copy, Debug)] pub(crate) struct MinMaxRms { min: f32, max: f32, rms: f32 }`; consts `LEVEL_STRIDE: usize = 8`, `RAW_RING_SAMPLES: usize = 1 << 14`; `pub(crate) struct ChannelHistory { raw: VecDeque<f32>, levels: Vec<Level>, acc: Acc, total_pushed: u64 }` and `struct Level { entries: VecDeque<MinMaxRms>, factor: usize, cap: usize, acc: Acc }` where `Acc { min: f32, max: f32, sq_sum: f32, count: usize }` starts at `{f32::INFINITY, f32::NEG_INFINITY, 0., 0}`. The accumulators are emit machinery only; queries never read them (the provisional entry is computed from the raw ring, below). Constructor `ChannelHistory::new(retention_frames: usize)` builds levels with `factor = 8, 64, 512, ...` while `factor <= retention_frames.min(RAW_RING_SAMPLES)`, `cap = retention_frames.div_ceil(factor)`. The factor cap matters: it guarantees a level's incomplete final group always fits inside the raw ring, so the provisional entry can be computed exactly from raw data. `push(&mut self, s: f32)`: `total_pushed += 1`; append to `raw`, evict front over `RAW_RING_SAMPLES`; feed the channel `acc` (`min`, `max`, `sq_sum += s * s`, `count += 1`); when `count == LEVEL_STRIDE`, push `MinMaxRms { min, max, rms: (sq_sum / LEVEL_STRIDE as f32).sqrt() }` into `levels[0]` (evict front over its `cap`), reset, then feed that entry into `levels[0].acc` (`sq_sum += rms * rms * factor_0 as f32`, `count += factor_0`); cascade: whenever `levels[k].acc.count == levels[k + 1].factor`, emit into `levels[k + 1]`, reset, feed `levels[k + 1].acc`. The last level's accumulator is never fed (no consumer) and stays at its init values. Readers `len_raw()`, `level_count()`, and `provisional(&self, level_ix: usize) -> Option<MinMaxRms>`: `let n = (total_pushed % factor as u64) as usize;` None when `n == 0`, otherwise scan the newest `n` raw samples (always available because `factor <= RAW_RING_SAMPLES`) and return their min/max/rms.
- Tests: push 28 ascending samples: `raw.len() == 28`, level 0 has 3 complete entries and the channel accumulator's `count == 4`; entry 0 is `{min: 0., max: 7., rms: (140. / 8.).sqrt()}`; push to 64: level 1 (factor 64) holds 1 entry `{min: 0., max: 63., rms: (sum_of_squares(0..64) / 64.).sqrt()}`; eviction: construct with `retention_frames = 64`, push `RAW_RING_SAMPLES + 200` samples (16,584 — pushing only 200 can never evict a 16,384-cap ring), level-0 `len() <= 8` and `raw.len() == RAW_RING_SAMPLES`; the ring holds exactly the newest 16,384 samples; rms chain consistency: level-1 entry rms equals the direct rms of its 64 raw samples; provisional: after 28 ramp samples, `provisional(0)` is `Some({min: 24., max: 27., rms: ((576. + 625. + 676. + 729.) / 4.).sqrt()})` and after 32 samples it is `None` (group boundary).
- Done-check: `cargo test -p gpui-starter --lib waveform::history` passes.

### Step 2.2: Column query

- Files: same.
- Build: `#[derive(Clone, Copy, Debug)] pub(crate) struct ColumnAgg { pub min: f32, pub max: f32, pub rms: f32, pub populated: bool }`; `query(&self, window_samples: usize, columns: usize) -> Vec<ColumnAgg>` returning `columns` entries, index 0 oldest (left) and the last index newest (right). Selection: if `window_samples <= self.raw.len()` use the raw path (scan the last `m = window_samples` samples; for each column c the sample range is `[m * c / columns, m * (c + 1) / columns)` over that suffix, empty range means unpopulated; aggregate min/max/rms over the range). Otherwise use the finest level with `factor >= window_samples.div_ceil(columns)`, falling back to the coarsest level; append `provisional(level_ix)` (Step 2.1) as the newest entry; `n = min(available entries, window_samples.div_ceil(factor))`; when `columns >= n`, entry e covers columns `e * columns / n .. (e + 1) * columns / n`; when `columns < n`, column c covers entries `c * n / columns .. (c + 1) * n / columns`; aggregate `min` of mins, `max` of maxes, `rms = (mean of rms_i^2).sqrt()`; ranges that fall outside `[0, n)` are unpopulated. Also `last_samples(&self, n: usize, out: &mut Vec<f32>)` copying the newest `min(n, raw.len())` samples (FFT input).
- Tests (all with ramp input `i as f32`):
  - Raw path: push 100 samples, query window 100 columns 10: column 9 covers samples 90..99 so `min == 90.`, `max == 99.`; query window 100 columns 200: even-indexed columns are unpopulated and odd-indexed columns are populated (integer ranges `[m*c/columns, m*(c+1)/columns)` are empty exactly when both floors agree, which happens on even c here; empty-range rule).
  - Level path with provisional entry: `ChannelHistory::new(retention_frames = RAW_RING_SAMPLES + 800)` (18384 so the 4096 level exists), push 17184 samples (raw ring retains the newest 16384), query window 17184 columns 10. The window exceeds raw capacity, `spc = 1718.4`, so the factor-4096 level runs with 4 complete entries plus the provisional fifth covering the incomplete group (the newest 800 raw samples, `total_pushed % 4096 == 800`); all 10 columns are populated (2 columns per entry, gapless), the newest column reports `min == 16384.`, `max == 17183.` (provisional entry), and column 0 reports `min == 0.`, `max == 4095.`.
  - Level-0 boundary: `ChannelHistory::new(retention_frames = RAW_RING_SAMPLES + 8)`, push `RAW_RING_SAMPLES + 4` = 16388 samples, query window 16388 columns 2049 (`spc = 7.999` picks the factor-8 level; 2048 complete entries plus a provisional covering the newest 4 samples). Column 2048 (newest) reports `max == 16387.` (provisional), column 2047 reports `max == 16383.` (last complete group 16376..16383), and column 0 reports `max == 7.`.
- Done-check: `cargo test -p gpui-starter --lib waveform::history` passes.

## Phase 3: Spectrum analyzer

### Step 3.1: Analyzer and frame

- Files: `src/ui/waveform/spectrum.rs`, `src/ui/waveform/spectrum.test.rs`.
- Build: `const FFT_SIZE: usize = 1024; pub(crate) const MIN_DB: f32 = -60.;`; `pub(crate) struct SpectrumAnalyzer { fft: Arc<dyn Fft<f32>>, scratch: Vec<Complex<f32>>, buffer: Vec<Complex<f32>> }`; `new()` plans once via `FftPlanner::new().plan_fft_forward(FFT_SIZE)` and sizes `scratch` with `fft.get_inplace_scratch_len()`; `analyze(&mut self, mixdown: &[f32], sample_rate: u32) -> SpectrumFrame`: zero-pad the front of `buffer` when `mixdown.len() < FFT_SIZE` (copy the newest `min(len, FFT_SIZE)` samples into the tail), run `fft.process_with_scratch(&mut buffer, &mut scratch)`, magnitude per bin `k in 1..FFT_SIZE / 2` as `(re * re + im * im).sqrt() / (FFT_SIZE as f32 / 2.)`, then fill `SPECTRUM_BARS` bars: band edges `f_lo = 20f32.max(sample_rate as f32 / FFT_SIZE as f32)` to `f_hi = sample_rate as f32 / 2.`, edge_i = `f_lo * (f_hi / f_lo).powf(i as f32 / SPECTRUM_BARS as f32)`, bins `ceil(edge_i / bin_hz) .. max(that + 1, ceil(edge_{i+1} / bin_hz))`, bar value = max magnitude in band, `db = 20. * (value + 1e-9).log10()`, normalized `(db - MIN_DB) / -MIN_DB` clamped to `0..=1`. `SpectrumFrame` per the contract with `bar_band` recomputing the same edges.
- Tests: 440 Hz unit sine at 48 kHz (generate 1024 samples with `(2π * 440 * i / 48000).sin()`): the bar with max height satisfies `band.0 <= 440. && 440. <= band.1`; silence produces all-zero bars; a 0.5-amplitude sine yields its peak bar strictly lower than a 1.0-amplitude sine's peak bar at the same frequency; `bar_band(0)` starts at `max(20, 48000 / 1024)` and `bar_band(SPECTRUM_BARS - 1)` ends at 24000; `bar_band(SPECTRUM_BARS)` is None.
- Done-check: `cargo test -p gpui-starter --lib waveform::spectrum` passes.

## Phase 4: WaveformState entity

### Step 4.1: Core state and refresh

- Files: `src/ui/waveform/state.rs`, `src/ui/waveform/state.test.rs`.
- Build: constants `MIN_WINDOW`, `DEFAULT_WINDOW`, `DEFAULT_RETENTION`, `MIN_RETENTION`, `MAX_RETENTION`, `REFRESH_INTERVAL: Duration = Duration::from_millis(33)`; `pub struct WaveformState { source: Option<SourceAttachment>, channels: Vec<ChannelHistory>, paused: bool, window: Duration, mode: WaveformMode, retention: Duration, spectrum: Option<SpectrumFrame>, analyzer: Option<SpectrumAnalyzer>, tick: Option<Task<()>>, drain_buf: Vec<f32>, mixdown: Vec<f32> }` with `SourceAttachment { consumer: SampleConsumer, sample_rate: u32, channels: usize }`. `new()`/`Default` with `window = DEFAULT_WINDOW`, `retention = DEFAULT_RETENTION`. Methods per the contract: `attach_source` (validate `channels == 1 || channels == 2` with `assert!`, `sample_rate > 0`; drop old `tick` and source; clamp retention; build `ChannelHistory::new(retention_frames)` per channel where `retention_frames = (retention.as_secs_f32() * sample_rate as f32).ceil() as usize`; spawn the tick per Step 4.2; notify), `detach_source`, readers, `set_paused`, `set_window` (clamp to `[MIN_WINDOW, self.retention]`), `set_mode`, `dropped_samples` (source consumer read). `refresh(&mut self, cx)`: return early with no source; `drain_buf.clear(); consumer.drain_into(&mut drain_buf)`; return without notify when empty; if paused, return (data discarded by not integrating); else deinterleave `drain_buf` into `channels[i].push(sample)`; rebuild `mixdown` via `last_samples` (mono: channel 0; stereo: mean of the two, over the common suffix); `spectrum = Some(analyzer.get_or_insert_with(SpectrumAnalyzer::new).analyze(&mixdown, sample_rate))`; `cx.notify()` once.
- Tests (`#[gpui_kit::test]`, 48 kHz stereo sources throughout; `refresh` driven via `entity.update(cx, |s, cx| s.refresh(cx))`):
  - Attach returns a producer whose `sample_rate()`/`channels()` match the call.
  - Integration: push 480 interleaved frames of a 440 Hz sine, `refresh`, assert `channels() == 2` and `spectrum().is_some()` with the peak bar in the band containing 440 Hz (the spectrum requires a mixdown, which requires integration; reuses Step 3.1's band logic).
  - Pause: with the sine flowing, pause, push 200 frames of silence, `refresh` twice, assert the spectrum bars are still non-zero (frozen pre-pause frame; drained silence was discarded); unpause, push 1100 frames of silence, `refresh`, assert all bars are zero (a new frame integrated). 1100 is not arbitrary: after it, each channel's raw ring holds 480 sine + 1100 silence = 1580 samples, so the 1024-sample mixdown window is entirely post-sine silence; with only 200 post-unpause frames the window would still contain the 480 sine samples and the bars could not be zero.
  - Window clamp: `set_window(1ms)` yields `window() == MIN_WINDOW`; `set_window(999s)` yields `window() == retention()`.
  - Detach: `detach_source` leaves `has_source() == false`, `channels() == 0`, `spectrum().is_none()`; the producer reports `is_attached() == false`.
  - Overflow: fresh source, push 20000 interleaved stereo frames at once (ring holds 16384 frames); exactly the samples of 3616 frames are dropped, and `state.dropped_samples() == producer.dropped_samples()`.
- Done-check: `cargo test -p gpui-starter --lib waveform::state` passes.

### Step 4.2: Tick loop

- Files: same.
- Build: in `attach_source`, after building the attachment: `self.tick = Some(cx.spawn(async move |this, cx| { loop { cx.background_executor().timer(REFRESH_INTERVAL).await; if this.update(cx, |state, cx| state.refresh(cx)).is_err() { break; } } }))`: the `system_monitor.rs:217-230` shape, except the task is stored, not detached, so dropping it (detach, re-attach, or entity drop) cancels the loop. `detach_source` and re-`attach_source` drop the previous task first.
- Tests: `#[gpui_kit::test]` with a state entity created via `cx.update(|cx| cx.new(|_| WaveformState::new()))`, attach a source, push samples, `cx.executor().advance_clock(2 * REFRESH_INTERVAL)` then `cx.executor().run_until_parked()`, assert `spectrum().is_some()` without any manual `refresh` call (proves the timer fired and the loop survives multiple ticks). The empty-drain early return in `refresh` (no notify when nothing arrived) is verified by code review plus the Step 4.1 pause tests.
- Done-check: `cargo test -p gpui-starter --lib waveform::state` passes.

## Phase 5: Paint geometry

### Step 5.1: Pure helpers

- Files: `src/ui/waveform/paint.rs`, `src/ui/waveform/paint.test.rs`.
- Build (all `pub(crate)`, pure):
  - `snap_to_device(v: f32, scale: f32) -> f32` = `(v * scale).round() / scale`.
  - `nice_time_step(window_secs: f32, target_lines: usize) -> f32`: smallest value from `{1., 2., 5.} × 10^k, k in -3..=2` such that `window_secs / step <= target_lines` (default target 6), falling back to the largest candidate.
  - `time_gridline_xs(bounds: Bounds<Pixels>, window_secs: f32, step: f32, scale: f32) -> Vec<Pixels>`: x positions at `right - k * step / window_secs * width` for `k = 1..`, snapped, while `x > left`.
  - `amplitude_y(value: f32, lane: Bounds<Pixels>) -> Pixels`: `lane.center().y - value.clamp(-1., 1.) * (lane.size.height * 0.5)` (`Pixels` `Add`/`Sub` derived, `f32 * Pixels` and `Pixels * f32` exist; `GPUI/geometry.rs:2672-2741`).
  - `envelope_bounds(agg: &ColumnAgg, col_ix: usize, columns: usize, lane: Bounds<Pixels>, scale: f32) -> Option<(Bounds<Pixels>, Bounds<Pixels>)>`: peak quad (y from `amplitude_y(max)` to `amplitude_y(min)`, floored to `1. / scale` tall) and rms quad (`amplitude_y(rms)` to `amplitude_y(-rms)`, same floor); x span `[left + col_ix * w / columns, left + (col_ix + 1) * w / columns)` snapped on both edges; None when `!agg.populated`.
  - `oscilloscope_line_points(samples: &[f32], window_samples: usize, lane: Bounds<Pixels>, scale: f32) -> Vec<Point<Pixels>>`: newest at the right; `x_i = left + width * ((window - m) + i) as f32 / window as f32` where `m = samples.len()`, snapped; `y_i = amplitude_y(samples[i], lane)`; single sample returns one point.
  - `bar_bounds(height_norm: f32, bar_ix: usize, bars: usize, lane: Bounds<Pixels>, scale: f32) -> Bounds<Pixels>`: x span `[left + bar_ix * w / bars, left + (bar_ix + 1) * w / bars)` inset by `1. / scale` on each side, snapped; height `height_norm.clamp(0., 1.) * lane.size.height` rising from `lane.bottom()`.
- Tests: `snap_to_device` at scale 2.0 maps 10.25 to 10.5 and 10.24 to 10.0; `nice_time_step(5., 6) == 1.` and `nice_time_step(0.05, 6) == 0.01`; gridline xs are strictly inside bounds, descending from the right, and spaced within a pixel of `step / window * width`; envelope bounds of a known agg at scale 1.0 and 2.0 have exact expected edges and the 1-device-px floor kicks in for flat aggs; line points place the last sample at the right edge and honor `window_samples > m` by left-insetting; bar bounds at bars=4 tile the lane with `2. / scale` total gap and never overlap.
- Done-check: `cargo test -p gpui-starter --lib waveform::paint` passes.

## Phase 6: Element

### Step 6.1: WaveformElement

- Files: `src/ui/waveform/element.rs`, extend `mod.rs` re-exports.
- Build: `Waveform { id: ElementId, state: Entity<WaveformState>, accent: Option<Hsla> }` with `new` and `accent`; `WaveformElement` mirroring the fields; `impl IntoElement for Waveform { type Element = WaveformElement; }` and `impl IntoElement for WaveformElement { type Element = Self; }` (`GPUI/element.rs:147-159`); `impl Element for WaveformElement` with `RequestLayoutState = ()`, `PrepaintState = ()`, `id() -> Some(self.id.clone())`, `source_location() -> None`, `request_layout` returning `window.request_layout(Style { size: Size::full(), ..Default::default() }, None, cx)` (`GB/plot/element.rs:74-84`), `prepaint -> ()`, `paint(bounds, ...)`:
  - `let state = self.state.read(cx);` then `window.with_content_mask(Some(ContentMask { bounds }), ...)` around everything (`GPUI/window.rs:4029`).
  - Background `fill(bounds, cx.theme().background)` (`GPUI/window.rs:7694`).
  - `let scale = window.scale_factor();` (`GPUI/window.rs:2973`); columns = `((bounds.size.width.as_f32() * scale).round() as usize).max(1)` (`Pixels.0` is `pub(crate)`, `GPUI/geometry.rs:2689`).
  - Lane layout: detached state draws one idle lane; attached mono draws one lane; attached stereo splits the bounds into two equal-height lanes.
  - Colorway: `let mono_color = self.accent.unwrap_or(cx.theme().primary);` stereo lanes `self.accent.unwrap_or(cx.theme().chart_1)` / `unwrap_or(cx.theme().chart_2)` (`COMP/theme/theme_color.rs:133,135`).
  - Grid per lane: center amplitude line and `time_gridline_xs` lines, each `fill` of height or width `1. / scale`, color `cx.theme().chart_grid` (`COMP/theme/theme_color.rs:147`).
  - `WaveformMode::Oscilloscope`: per lane, `state.channel_view(ch, columns)` (pub(crate), defined in state.rs); `WaveformSamples::Line(samples)` strokes a `PathBuilder` polyline (`stroke(px(1.))`, `move_to`/`line_to` from `oscilloscope_line_points`, `GPUI/path_builder.rs:88-98, 125-152`) in the lane color via `window.paint_path`, painting only `if let Ok(path) = builder.build()`; `WaveformSamples::Envelope(cols)` paints, per populated column, the peak quad in `color.opacity(0.45)` and the rms quad in `color.opacity(0.9)` on top (`system_monitor.rs:291` opacity precedent). Detached or dataless lanes draw only the grid and center line. Stereo lanes are separated by a one-device-px divider in `chart_grid`.
  - `WaveformMode::Spectrum`: grid, then baseline hairline at the lane bottom in `chart_grid`, then `state.spectrum()` bars as `bar_bounds` quads in the mono color (no extra opacity).
  - No hitbox, no mouse listeners, no keyed state; the element registers nothing that survives the frame.
- Tests: none yet (Step 6.2).
- Done-check: `cargo check -p gpui-starter` passes.

### Step 6.2: Window integration test

- Files: `src/ui/waveform/element.test.rs`, wired from `element.rs`.
- Build (`#[gpui_kit::test]`): a `WaveformHost` view holding `Entity<WaveformState>` with the contract's observe wiring; each variant below builds its own state, mutates it, then calls `cx.add_window(|window, cx| WaveformHost::new(state, window, cx))` (`GPUI/app/test_context.rs:279-296`; `App::open_window` draws once before returning, `GPUI/app.rs:1431-1455`), so every variant's element paint runs during the window's first draw with no reliance on redraw-after-notify. Variants: detached idle (never attached); mono attached with 2048 frames of a 440 Hz sine and a `refresh`; stereo attached likewise, in both `WaveformMode`s; stereo paused after pushing data (frozen display draws); a 50 ms window (line mode when the test window's width in device pixels exceeds `window_samples`). After each `add_window`, assert the state readers still report the expected `mode()`/`window()`/`channels()` and that nothing panicked. Pixel output is manual QA.
- Done-check: `cargo test -p gpui-starter --lib waveform::element` passes.

### Step 6.3: Re-export freeze and module header

- Files: `src/ui/waveform/mod.rs`.
- Build: final re-export list exactly matching the Interface Contract; doc comments on `SampleProducer`, `WaveformState`, `Waveform`, and `SpectrumFrame` stating the ownership, observe, and frame-alignment rules; module header per the Removal Recipe.
- Tests: existing `tests/qa_migration.rs` must stay green (no `use gpui::` crept in).
- Done-check: `cargo test --workspace --all-features` passes and `grep -rn "use gpui::" src/ui/waveform/` returns nothing.

## Platform & Wasm Gating

- No `cfg` gates in `src/ui/waveform/`. Every API used is platform-neutral: quads, paths, content masks, `scale_factor`, timers, entities. The widget contains no file, socket, OS-thread, or clock-read code; the only `Duration` values are constants and API parameters (`std::time::Duration` is the same type on wasm, `src/platform/clock.rs:6,9`).
- Dependencies: `rtrb` and `rustfft` go in the common `[dependencies]` table, not the `[target.'cfg(not(target_family = "wasm"))'.dependencies]` table. rtrb is `no_std` by default (`#![cfg_attr(not(feature = "std"), no_std)]`, docs.rs source). rustfft compiles on wasm32 via its scalar path; its `wasm_simd` feature is optional and off (docs.rs feature list). Residual risk that default native SIMD features fail the wasm check is tracked in Risks with the fallback (move to `default-features = false` plus per-target feature tables).
- Capability-registry entry: none. The widget has no platform variance, no init step, and no runtime failure mode of its own; audio device access is the consumer's concern (plans 18/19). A registry entry would be noise on the diagnostics page (seams section 4 lists entries for features with real platform state).
- `just wasm-check` (`cargo +nightly check --target wasm32-unknown-unknown -p gpui-starter --lib`, `justfile:24-25`) must pass: the lib gains the module and both crates.
- Wasm behavior: fully functional. The tick timer works on the web executor (gallery precedent `system_monitor.rs:217-230` compiles into the wasm lib); a wasm consumer feeds samples from JS via whatever bridge plan 18/19 builds.

## i18n

No user-facing strings originate in this feature. The widget paints data and grid geometry only; all chrome (play/pause buttons, mode switch, window presets, "dropped samples" readouts) is owned by the mounting feature, which adds its own keys to `locales/en.yml` and `locales/zh-CN.yml` in lockstep (seams section 5; key-set equality enforced by `tests/qa_localization.rs:188-198`).

| key | en | zh-CN |
|---|---|---|
| (none) | | |

## Design Compliance

- Tokens: grid and baseline from `cx.theme().chart_grid`, mono/spectrum default from `cx.theme().primary`, stereo lanes from `chart_1`/`chart_2` (design guides, Color and themes; token fields at `COMP/theme/theme_color.rs:133,135,147`). No raw hex, `rgb()`, or `hsla()` constructors; `.accent()` accepts a caller color the same way charts accept `.stroke()` (kit precedent), so the raw-color prohibition stays the caller's responsibility.
- Fixed-pixel exceptions, all in the guide's audited classes: gridlines and baseline are one device pixel (`1. / scale`, the hairline class), envelope/rms quads floor at one device pixel, bar gaps are one device pixel, the line-mode stroke is `px(1.)`. Envelope widths, bar widths, and line x positions are data raster geometry (measured runtime geometry), not layout; the element itself uses no rem-based sizing because it fills its parent (`Size::full()`, the PlotElement idiom) and owns no text or chrome.
- Opacity layering on tokens (0.45 peak envelope, 0.9 rms) follows the repo's chart precedent (`system_monitor.rs:291`).
- Density and components: the widget ships no controls and no chrome; consumers compose buttons, tabs, and readouts from kit components. Zoom (time window) is data zoom, not interface zoom; it never touches `rem_size`, and nothing in the element caches resolved layout, so rem changes cannot stale it.
- Interaction states: the element is a non-interactive display; pause/resume and mode are API states whose visible affordances (pressed toggle, selected tab) are the consumer's components. No hover, focus, or pressed states exist to design because nothing is clickable; this matches the design guide's requirement that state be visible where interaction exists.
- Motion: the scroll is data-driven redraw, not a transition; there is no decorative animation, so reduced-motion needs no handling.
- A11y: v1 paints no accessibility node (same posture as plan 01's canvas). The waveform is a live raster whose content is not expressible as a static tree; consumers provide labeled controls and text readouts (for example a "dropped samples" status) around it. Documented limitation, tracked in Risks.

## Verification

Gates (run in this order; the first three are the repo's mandatory set, AGENTS.md):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

Feature-specific automated coverage (wired in the phases above):

```sh
cargo test -p gpui-starter --lib waveform
```

covering frame alignment and drop accounting, pyramid values and eviction, gapless column queries with the provisional entry, sine-band spectrum placement, pause/window/mode semantics, the timer-driven tick (`advance_clock`), and a real-window render across modes.

Mechanical checks: `grep -rn "use gpui::" src/ui/waveform/` returns nothing; `git diff Cargo.lock` in the implementation commit contains the rtrb/rustfft additions (qa_migration lock rule).

Manual QA (scratch working tree, never committed): mount `Waveform::new("smoke", state)` inside any existing page view, attach a 48 kHz stereo source, and push a 440 Hz sine from a `std::thread` holding the producer:

1. Scrolling envelope tracks the tone; zooming the window below roughly `columns / sample_rate` seconds switches to the stroked line trace with no popping between modes.
2. Pause freezes the display and spectrum; resume shows a gap then continues; `dropped_samples()` stays 0 during sustained playback.
3. Window presets (50 ms, 1 s, 5 s, 30 s) change the time span and gridline labels scale sensibly (1-2-5 steps).
4. Spectrum mode peaks in the bar band containing 440 Hz (cross-check with `SpectrumFrame::bar_band`).
5. On a 2x display, gridlines are one physical pixel, envelope columns tile with no seams or moiré, and the line trace does not blur between adjacent x positions.
6. Light and dark themes both render legibly (grid visible on both, lanes distinguishable in stereo).
7. Flooding the producer with a large burst makes `dropped_samples()` grow and the display continue without stutter or panic.

Acceptance criteria: all four gates pass; the targeted test run passes; the manual checklist is confirmed on the scratch tree or waived with a note naming which items were checked and how.

## Risks & Open Questions

Ranked:

1. **Dependent plans are absent.** Plans 18, 19, and 13 do not exist on disk (tried `ls docs/plans/`: files 01 through 11 plus `_survey/`). The Interface Contract covers exactly the spec's list (producer, ring, pyramid, element, modes, pause, window) and nothing more; if a dependent needs per-channel spectra, gain control, scrollback navigation, or peak-hold bars, it extends this contract in a follow-up edit rather than working around it.
2. **rustfft default features on wasm32 are unverified by compilation.** Evidence: pure Rust implementation, per-target `cfg` intrinsics, optional `wasm_simd` feature implies wasm targeting (docs.rs, fetched 2026-10-12). Not run: `cargo check --target wasm32-unknown-unknown` with the new deps (doc-only planning scope). Fallback if `just wasm-check` fails: `rustfft = { version = "6.4", default-features = false }` in the common table plus `avx`/`sse`/`neon` re-enabled under the not-wasm target table.
3. **rtrb 0.4 renames the survey's type.** The survey ecosystem notes describe the 0.3 `Rtrb` constructor; 0.4.0 uses `RingBuffer::new` (docs.rs, fetched 2026-10-12). This plan is written against the verified 0.4 API; the implementer pins `rtrb = "0.4"` and must not copy 0.3 snippets.
4. **30 Hz timer smoothness.** A 33 ms tick redraws steppier than a display-locked `request_animation_frame` loop. Accepted for v1: it matches the repo's loop precedent, is deterministic under `advance_clock`, and stops cleanly. An rAF-driven variant is a follow-up if 18/19 report visible stepping.
5. **FFT runs on the UI thread.** A 1024-point transform at 30 Hz is tens of microseconds (rustfft SIMD on native); the tick also does O(new samples) pyramid work. If a consumer feeds very high sample rates and the frame budget suffers, the fix is moving `analyze` onto `background_executor` with a stale-result revision check, not an API change.
6. **Provisional-entry scan cost.** Each pyramid query may scan up to `factor` newest raw samples (≤ 16384) to build the provisional entry. That is a bounded linear scan per channel per frame; if profiling ever shows it, the fix is caching the provisional per refresh tick (it only changes when samples arrive), not an API change.
7. **Memory at max retention.** Worst case (stereo, 48 kHz, 30 s): raw rings 2 × 16384 × 4 B ≈ 131 KB plus per-channel level storage ≈ 2.5 MB (12 B per entry over 1.8 M frames / 8, / 64, ...), ≈ 5.2 MB total. Bounded by `MAX_RETENTION`; default 10 s lands near 1.7 MB.
8. **No a11y node.** Screen readers see nothing of the waveform until consumers add labeled surfaces; same accepted limitation as plan 01, revisit with 18/19's actual chrome.

Open questions (each with what was tried):

- **Amplitude convention with 18/19.** The widget assumes ±1.0 normalized f32 (cpal's documented f32 convention per the survey's audio section). Tried: no plan files to confirm against. If a dependent captures int-scaled floats, either it normalizes before push or a later contract edit adds a gain factor.
- **Whether plan 13 needs offline rendering.** A file-decoding consumer may want to push a whole buffer at once rather than stream; the current ring handles it (bounded capacity, drop accounting) but a dedicated batch API with seek would be cleaner. Tried: no plan 13 on disk; deferred.
- **Kit `test-support` module** (`ElementSnapshot` et al.) stays unavailable (survey Open Questions; the repo enables only gpui's `test-support` via the dev-dep, `Cargo.toml:216`). Pixel-level assertions in Step 6.2 are therefore manual QA; the automated test proves rendering does not panic and state stays consistent.
- **No gates were run during planning** (doc-only scope). The Verification section states the exact commands for the implementer; external-crate claims rest on docs.rs fetches from 2026-10-12 cited in the Research Digest.
- **rustfft 6.4.1 release date is unresolved.** The survey records 2025-09-18 (`docs/plans/_survey/ecosystem.md`), the docs.rs page fetch during planning showed 2026-10-04, and crates.io was unreachable from this sandbox to adjudicate. The date is dropped from this plan; the version pin `rustfft = "6.4"` is unaffected either way. Related skipped review note: rtrb `push_partial_slice`'s `(copied, remainder)` tuple order was verified from the crate's `chunks.rs` doc source during planning (first slice = "The part that has been copied into the ring buffer"), so no change was made; if an implementer finds the order flipped at compile time, the Step 1.2 `copied` binding takes element 0 either way.
