# 19 Audio Playback Plan

## Goal

An Audio page loads a WAV/FLAC/MP3/OGG file (or a runtime-generated sine-sweep or chord demo, no bundled audio), precomputes a full-file waveform overview once, and plays it through cpal with play/pause/stop, seek scrubbing over the overview, a live output-level monitor (plan 16's widget, oscilloscope/spectrum toggle), an output-device selector, a volume slider, and a space-bar transport binding. The decode + output stack lives in `src/services/audio/` so plan 13's music player (and the unimplemented plan 18) can consume the same engine; on wasm the page renders with an explicit capability reason and no playback.

## Dependencies

- **docs/plans/16-waveform-foundation.md** (contract only — `src/ui/mod.rs:1` currently reads `pub mod widgets;`, so plan 16 is NOT implemented yet; verified this session). Consumed Interface Contract items (`16-waveform-foundation.md:79-206`):
  - `WaveformState::{attach_source, detach_source, set_mode, mode, refresh}` (`:129-135`, `:138`, `:153-154`, `:162` — `refresh` included for engine-test forcing) — `attach_source(sample_rate, channels, retention, cx) -> SampleProducer`.
  - `SampleProducer::{push_interleaved, channels, sample_rate, is_attached}` (`:100-112`) — `Send`, single-owner, moved into the cpal callback.
  - `WaveformMode::{Oscilloscope, Spectrum}` (`:92-94`) and the `Waveform::new(id, entity)` element (`:181-188`) with the observe recipe (`:208-210`).
  - Constants `DEFAULT_RETENTION` (`:86`).
- **docs/plans/18-microphone.md** (peer, on disk this session): 18 owns the CAPTURE side — input-device enumeration, input stream, macOS permission, `LevelMeter`, its own page (`18-microphone.md:3-10`). This plan owns DECODE + OUTPUT. Reconciliation: the shared audio stack is (a) the single `cpal = "0.18"` dependency in the not-wasm table — whichever plan lands second must not duplicate the Cargo.toml key (see Seam Touches row 2), (b) plan 16's `WaveformState`/`SampleProducer` as the common visualization contract both consume. Plan 19 does NOT consume 18's capture contract; 18's Interface Contract explicitly names input-device reuse as plan 19's option (`18-microphone.md:275-277`), which this plan declines in favor of the output-only surface below. 18 also lands the `libasound2-dev` CI dependency this plan needs (`18-microphone.md:414-416`).
- **docs/plans/13-music-player.md** (peer, not consumed): its `PlayerEngine` is a simulated clock with an optional rodio Phase 6 (`13-music-player.md:370-386`); integration point documented in Interface Contract below. Its line 189 explicitly defers real-sample wiring ("Dependent plans needing real samples (e.g. plan 16) extend this contract later").

## Research Digest

Path key: `GPUI` = `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-0.3.8/src`, `BASE` = `.../gpui-base-0.7.1/src`, `COMP` = `.../gpui-component-0.7.1/src`, `REPO` = `/home/thragg/fo/gpui-starter`, `SYMPH` = `/tmp/symph/symphonia-0.6.1` + `/tmp/symph/symphonia-core-0.6.1` (crates.io tarballs downloaded and read this session — authoritative for 0.6.1), `CPAL` = `/tmp/cpal-0.18.2` (crate tarball present on disk this session; the same copy plan 18 cites, `18-microphone.md:36`). hound claims re-verified against docs.rs fetches this session (2026-10-12).

**cpal 0.18.2** (crate source at `CPAL/src/{traits.rs,lib.rs}`, read this session):

- `HostTrait::devices() -> Result<Self::Devices, Error>` (`traits.rs:62`), provided `output_devices()` (`:119`) and `default_output_device() -> Option<Self::Device>` (`:97`); not dyn-compatible.
- `DeviceTrait::description() -> Result<DeviceDescription, Error>` (`traits.rs:153`); "For the device name as a string, use `device.to_string()`" (`:145`; Display supertrait per plan 18's verification at `18-microphone.md:53`). `id() -> Result<DeviceId, Error>` (`:165`, stable, `Display`+`FromStr` per `18-microphone.md:61-64`).
- `DeviceTrait::supported_output_configs() -> Result<Self::SupportedOutputConfigs, Error>` (`traits.rs:205`), `default_output_config()` (`:231`). `SupportedStreamConfigRange` accessors: `channels` (`lib.rs:681`), `min_sample_rate` (`:685`), `max_sample_rate` (`:689`), `sample_format` (`:697`), `with_sample_rate` (`:708`), `try_with_sample_rate -> Option` (`:717`), `with_max_sample_rate` (`:732`), `contains_rate` (`:742`).
- `StreamConfig { pub channels: ChannelCount, pub sample_rate: SampleRate, pub buffer_size: BufferSize }` (`lib.rs:452-456`), `BufferSize::{Default, Fixed(FrameCount)}` (`:388-392`). `SampleFormat` is `#[non_exhaustive]` with 15 variants including `F32` (docs.rs `enum.SampleFormat.html`, fetched this session; match with wildcard).
- `DeviceTrait::build_output_stream<T: SizedSample>(config, D: FnMut(&mut [T], &OutputCallbackInfo) + Send, E: FnMut(Error) + Send, timeout: Option<Duration>) -> Result<Self::Stream, Error>` (`traits.rs:318-331`). Doc: the output slice is "pre-filled with silence, so a callback that writes fewer samples than the slice holds leaves the remainder silent rather than stale" (`traits.rs:295`) — this is the end-of-file path.
- `StreamTrait::{play, pause}(&self) -> Result<(), Error>` (`traits.rs:451`, `:469`); streams are created paused (crate docs).
- No per-device volume in cpal; gain is applied in the callback (survey `ecosystem.md` §h, confirmed by trait docs listing only play/pause as control).
- cpal is NOT in `Cargo.lock` today (`grep -c '^name = "cpal"' Cargo.lock` → 0, run this session); plan 18's seam also adds it, so exactly one of the two plans introduces the entry and the lock dedupes to a single cpal.
- On Linux cpal pulls the `alsa` crate (links libasound); plan 18 already carries the fix — `libasound2-dev` added to `scripts/install-linux-deps.sh`, which every CI Linux lane runs (`18-microphone.md:414-416`; script's `PKGS` array at `scripts/install-linux-deps.sh:14`).

**hound 3.5.1** (docs.rs `struct.WavReader.html`, `struct.WavWriter.html`): `WavReader::new(R: Read)` / `open(path)`, `.spec() -> WavSpec`, `.duration()` (samples), `.samples::<S>()` interleaved iterator (S must match `spec().sample_format`; 24-bit int reads as `i32`). `WavSpec { channels, sample_rate, bits_per_sample, sample_format }`, `SampleFormat::{Int, Float}` (survey `ecosystem.md` Step 3 row, confirmed by docs). `WavWriter<W: Write + Seek>::new(W, WavSpec) -> Result<Self>`, `.write_sample(s)`, `.finalize()`; `Cursor<Vec<u8>>` satisfies `Write + Seek` for in-memory demo generation.

**symphonia 0.6.1** (tarball sources — the 0.5-era API in blog posts is stale; 0.6 reorganized modules):

- `symphonia::default::{get_codecs() -> &'static CodecRegistry, get_probe() -> &'static Probe}` (`SYMPH/symphonia-0.6.1/src/lib.rs:215,225`); `pub use symphonia_core as core` (`lib.rs:303`). Note: upstream GETTING_STARTED.md still shows a stale `core::probe::Hint` import; the real path is `core::formats::probe::Hint` (`SYMPH/symphonia-core-0.6.1/src/formats/probe.rs:261`).
- `Hint::new()` + `.with_extension(&str) -> &mut Self` (`probe.rs:261,273`); `MediaSourceStream::new(Box<dyn MediaSource>, Default::default())`; `MediaSource` is pre-implemented for `std::fs::File` and `std::io::Cursor` (GETTING_STARTED.md @ tag v0.6.0, fetched this session).
- `Probe::probe(&self, &Hint, MediaSourceStream, FormatOptions, MetadataOptions) -> Result<Box<dyn FormatReader>>` (`probe.rs:429`).
- `FormatReader` trait (`core/src/formats/mod.rs:551`): `tracks() -> &[Track]` (`:594`), provided `default_track(TrackType) -> Option<&Track>` (`:630`), `next_packet() -> Result<Option<Packet>>` (`:646`, `Ok(None)` = end of stream). `Track { pub id: u32, pub codec_params: Option<CodecParameters>, ... }` (`:235-245`). `Packet { pub track_id: u32, ... }` (`core/src/packet.rs:50`).
- `CodecParameters::audio() -> Option<&AudioCodecParameters>` (`core/src/codecs/mod.rs:99`); `AudioCodecParameters { codec, sample_rate: Option<u32>, channels: Option<Channels>, ... }` (`core/src/codecs/audio.rs:78-105`); `Channels::count() -> usize` (`core/src/audio/channels.rs:310`).
- `CodecRegistry::make_audio_decoder(&AudioCodecParameters, &AudioDecoderOptions) -> Result<Box<dyn AudioDecoder>>` (`core/src/codecs/registry.rs:330`); `AudioDecoder::decode(&mut self, &Packet) -> Result<GenericAudioBufferRef<'_>>` (`core/src/codecs/audio.rs:279`).
- f32 extraction: `GenericAudioBufferRef::copy_to_vec_interleaved::<Sout: ConvertibleSample>(&mut Vec<Sout>)` (`core/src/audio/generic.rs:487`), plus `spec() -> &AudioSpec` (`:423`) with `rate()`/`channels()` (`core/src/audio/mod.rs:64,69`). There is NO `SampleBuffer` in 0.6.1 (grep over core sources → no hits); `copy_to_vec_interleaved::<f32>` replaces it.
- Features (`SYMPH/symphonia-0.6.1/Cargo.toml.orig [features]`): `default = ["opt-simd","all-meta","adpcm","flac","mkv","ogg","pcm","vorbis","wav"]`; opt-in `aac`, `alac`, `mp3`, `isomp4`, `aiff`, `caf`, ... This plan pins `default-features = false, features = ["wav","flac","mp3","ogg","vorbis","aac","isomp4","alac"]` (wav included so the symphonia path is unit-testable with generated WAV bytes; hound remains the .wav fast path). License MPL-2.0 — flagged in Risks.

**Repo seams** (verified this session):

- Page seam table: `docs/plans/_survey/seams.md` §1 rows 1–15 against `REPO/src/shell/sidebar.rs:4-105` (variant, `title` `:19-31`, `title_key` `:36-48`, `host` `:57-69`, `icon` `:77-89`, `all` `:92-105`), `src/shell/route.rs:10-21` (`VALID_HOSTS`), `src/shell/root/app_root/state.rs:46-56` (construct) + `:211-223` (`unchecked_active_page_view`), `src/features/pages/mod.rs:1-25`, `tests/e2e_navigation.rs:14-19` (count assert = 10). Digit-binding gotcha: `NavigateToPage(usize)` is positional (`seams.md` §1 row 8; binding loop `state.rs:144-166` binds only i < 9) — `Audio` is appended at the END of `all()`.
- `pick_file`: native `src/services/desktop_actions.rs:194-196` (`rfd::FileDialog::new().pick_file()`), wasm `:199-201` returns `None`. Call precedent `src/features/pages/settings/dev_sections/runtime_sections.rs:57-59`.
- Actions: `gpui_kit::actions!` macro (`src/app/actions.rs:1-12`); feature-local action derive precedent `src/gallery/sections/buttons/toolbar.rs:25-27` (`#[derive(Action, ...)] #[action(namespace = ..., no_json)]`). `KeyBinding::new(keystrokes, action, context: Option<&str>)` (`GPUI/keymap/binding.rs:33`); `div().key_context(...)` (`GPUI/elements/div.rs:830`); `track_focus` (`:788`); `FocusHandle::tab_stop(bool)` (`GPUI/window.rs:593`); keystroke `"space"` parses (`GPUI/platform/keystroke.rs:249`).
- Drag/scrub pattern (in-tree precedent, kit slider): stateful div `.on_mouse_down(MouseButton::Left, window.listener_for(&entity, ...))` + `.on_drag(Payload(entity_id), |drag,_,_,cx| { cx.stop_propagation(); cx.new(|_| drag.clone()) })` + `.on_drag_move(window.listener_for(...))` (`BASE/slider.rs:676-716`); drag payload renders `Empty` (`:430-436`). `window.listener_for(&Entity<T>, f)` (`GPUI/window.rs:6707`). `MouseDownEvent { button, position, ... }` (`GPUI/interactive.rs:148-160`).
- Painting inside a div: `canvas(prepaint, paint)` free fn, paint closure gets `(Bounds<Pixels>, T, &mut Window, &mut App)` (`GPUI/elements/canvas.rs:10-22`).
- Components: `Switch::new(id).checked(bool).label(...).on_click(|&bool,...)` / `on_change` (`COMP/switch.rs:33-127`); `SelectState::new(items: SearchableVec/Vec, Some(IndexPath::default().row(ix)), window, cx)` + `Select::new(&state).placeholder(...)` (`src/gallery/sections/buttons/toolbar.rs:42-49,223-227`); `SelectEvent::Confirm(Option<Value>)` subscription (`src/gallery/sections/theme/colors.rs:151-163`, enum at `COMP/select.rs:77-80`); `SliderState` + `SliderEvent::{Change,Release}` with `value.start()` (`src/gallery/sections/controls/slider.rs:100-130`); `Button::{label, icon, tooltip, outline, ghost, disabled, on_click, accessibility_label}` (verified by plan 13 against `COMP/button/button.rs:371-525`; re-verified usage in `toolbar.rs:82-87`); `Empty` for empty states — NOT with string args: `EmptyHeader::title(EmptyTitle)` / `.description(EmptyDescription)` take typed builders with no `From<str>` impl (`COMP/empty.rs:114-121`; `grep "impl From" COMP/empty.rs` → no hits), the working form is `EmptyTitle::new().child("No projects yet")` / `EmptyDescription::new().child(...)` (repo precedent `src/gallery/sections/loading/empty.rs:110-124`); `Badge` is count/dot/icon-only — `BadgeVariant::{Number, Dot, Icon(Box<Icon>)}`, private, no neutral or text variant (`COMP/badge.rs:6-11`, renders children plus overlay at `badge.rs:105-160`) — status text uses a plain `div` (notifications-page pattern), not Badge. `Alert` for inline errors (`COMP/alert.rs`, per design guides' overlay table).
- Icons: the component enum the app registers carries ONLY the default 106
  (`gpui-kit-assets-0.7.1/default-icons.txt`); full-catalog svgs
  (`audio-lines`, `volume-2`, `volume-x`, `file-up`, `audio-waveform`) exist
  on disk but their bytes are not registered natively — assets-enum variants
  compile yet render blank. Verified defaults used: `Play`, `Pause`,
  `Square`, `RotateCw`, `FolderOpen`, `Settings2` (volume stand-in — no
  speaker glyph in the defaults), `Close`.
- a11y: repo `A11yExt::{a11y(Role, label), a11y_live(Live)}` (`src/services/accessibility.rs:23-41`); `aria_value(SharedString)` (`GPUI/elements/div.rs:1426`), `aria_min_numeric_value(f64)` (`:1440`), `aria_max_numeric_value(f64)` (`:1445`), `on_a11y_action(AccessibleAction::Click, ...)` (`GPUI/elements/div.rs:1503`; usage `src/shell/root/app_root/render.rs:342`).
- Capabilities: `CapabilityStatus` has pub fields (`src/runtime/capabilities.rs:6-13`) — a `supported: false` entry is a struct literal; `set(name, status, cx)` (`:64-79`); runtime-services init block `src/app/init.rs:202-263`.
- Page layout/a11y precedent: `src/features/pages/notifications.rs:37-58` (v_flex p_6 gap_4, heading with `Role::Heading` + `aria_level(1)`).
- Async: `cx.background_spawn` / `cx.spawn(async move |this, cx| ...)` weak-entity pattern (`src/gallery/sections/charts/system_monitor.rs:217-230`); `#[gpui_kit::test]` + `advance_clock`/`run_until_parked` (`src/state/config_store.test.rs:280-295`).
- Lock rule: Cargo.lock committed with manifest changes (`tests/qa_migration.rs:167-186`); kit facade only, no `use gpui::` (`tests/qa_migration.rs:386-405`).
- Clocks: `std::time::Duration` is wasm-safe (`src/platform/clock.rs:6,9`); no `Instant::now()` anywhere in this feature (timers go through `background_executor().timer`).

Crate picks (all new):

| Crate | Version | Table | Why |
|---|---|---|---|
| `cpal` | 0.18.2 | `[target.'cfg(not(target_family = "wasm"))'.dependencies]` | Spec names it; device enumeration + typed f32 output streams; wasm backend exists but is not wired (see Platform & Wasm Gating) |
| `hound` | 3.5 | `[dependencies]` | Spec names it for WAV; `WavWriter` also generates the demo assets |
| `symphonia` | 0.6.1, `default-features = false`, features above | `[dependencies]` | Spec names it for compressed formats; pure Rust, compiles on wasm32 |

## Architecture

Entities and ownership (kit coding guides: narrowest owner, one retained state per concern):

- `AudioEngine` (`Entity`, `src/services/audio/engine.rs`) — the only playback state. Fields: `load: Option<LoadState>` where `LoadState { meta: AudioMeta, render: Arc<Vec<f32>> /* interleaved at output rate, 1–2 ch, normalized ±1 */, render_rate: u32, render_channels: usize, peaks: Arc<WaveformPeaks>, transport: Arc<SharedTransport> }`; `status: Option<PlaybackStatus>`; `volume: f32`; `devices: Vec<OutputDevice>`; `selected_device: Option<String>`; `meter: Option<Entity<crate::ui::waveform::WaveformState>>`; `load_generation: u64`; `loading: Option<String>` (name being decoded); `last_error: Option<String>`; tick `Task<()>`; native-only `session: Option<output::OutputSession>`. Every mutator ends with exactly one `cx.notify()`.
- `AudioPage` (`Entity`, `src/features/pages/audio.rs`) — view state only: `engine: Entity<AudioEngine>`, `waveform: Entity<WaveformState>` (plan 16), `volume_slider: Entity<SliderState>`, `device_select: Entity<SelectState<Vec<String>>>`, `focus_handle: FocusHandle`, subscriptions. Owns no playback state.
- `SharedTransport` (plain `Arc`, `transport.rs`) — the audio-callback-side state: `cursor: AtomicU64` (next render frame to output), `playing: AtomicBool`, `volume_bits: AtomicU32` (`f32::to_bits`), `eof: AtomicBool`. Written by the engine (seek/volume/eof-flag), read+advanced by the cpal callback. No locks; the callback never blocks.
- `WaveformPeaks` (`peaks.rs`) — immutable 2048-column min/max/RMS decimation of `render`, computed once per load in the background together with decoding.

Data flow: page action → `engine.update(...)` → engine mutates `SharedTransport`/rebuilds the native `OutputSession` → cpal callback fills output frames from `render` starting at `cursor`, applies `volume`, pushes the post-gain interleaved frames into the plan 16 `SampleProducer` (moved into the callback at build; never blocks, overflow counted by the widget), advances `cursor`, sets `eof` at end (remaining buffer stays silent — cpal pre-fills). A 33 ms tick inside the engine reads the atomics, updates cached `position`/`status`, and notifies once; the page (`cx.observe(&engine)`) re-renders the playhead, time readout, and transport state. Background decode: `load_bytes`/`load_path` set `PlaybackStatus::Loading` and spawn `background_spawn(decode + peaks + channel-adapt)`, guarded by `load_generation`; the result lands through `this.update(...)` and stale generations are dropped.

Playback mapping: `position = cursor / render_rate`; `duration = render_frames / render_rate`. Pause = `stream.pause()` (cursor frozen). Seek = `cursor.store(frame)` (callback picks it up next fill; works while paused for playhead-only updates). Device switch = remember position seconds, resample `render` to the new device rate, rebuild the session, seek to the saved time, resume if it was playing. Output config: pick an f32 `SupportedStreamConfigRange` (prefer `channels == 2`, else any `channels >= 2`, else mono), rate = `try_with_sample_rate(file_rate)` else clamp into `[min,max]`; `StreamConfig { channels: out_channels, sample_rate, buffer_size: Default }`.

Space-bar: `actions!(audio_page, [TogglePlayback])` + `cx.bind_keys([KeyBinding::new("space", TogglePlayback, Some("AudioPage"))])` in `AudioPage::new`; the page root and the overview are `track_focus(&self.focus_handle)` (one handle, `tab_stop(true)`) under a `.key_context("AudioPage")` root, so space works whenever focus is inside the page (Tab reaches the handle; clicking the overview focuses it via gpui's focusing mouse-down). A focused kit Button may consume space itself (standard control activation) — accepted, noted in Risks.

Plan 13 integration point (spec requirement): `services::audio` is importable without the page (`crate::audio::AudioEngine`). Plan 13's optional Phase 6 backend (`13-music-player.md:370-386`, rodio) can be replaced by this engine: `PlayerEngine` keeps its simulated clock as source of truth and forwards transport calls (`toggle_playback`, `seek_ms`, `set_volume`, `next`/`previous` → `load_bytes` of the real track) into an owned `Entity<AudioEngine>` inside `music/audio.rs`; metering for the NowPlaying Bar is `AudioEngine::attach_meter(&WaveformState)`. Nothing in plan 13's current contract changes shape; this engine is the "dependent plans needing real samples" extension that plan 13 line 189 anticipates.

Module tree:

```text
src/services/audio/
├── mod.rs          seam: header + mod decls + re-exports + initialize() capability
├── transport.rs    SharedTransport atomics (cursor/playing/volume/eof)
├── peaks.rs        WaveformPeaks: fixed-column min/max/rms decimation
├── decode.rs       decode_bytes: hound (.wav) / symphonia (compressed) → DecodedAudio
├── demo.rs         sine-sweep + chord WAV generators (hound WavWriter → Vec<u8>)
├── output.rs       native-only: device enumeration, config pick, OutputSession (cpal stream + callback)
├── engine.rs       AudioEngine entity: load/transport/seek/volume/device/tick/attach_meter
└── *.test.rs       sibling tests per repo convention

src/features/pages/
└── audio.rs        AudioPage: load row, overview+scrub, transport row, live monitor
```

## Interface Contract

Exact pub surface re-exported from `src/services/audio/mod.rs` (crate path `gpui_starter::audio` via the lib.rs services re-export). Behavioral structs keep private fields with readers; record types stay plain. Growable types are `#[non_exhaustive]`.

```rust
use std::{path::PathBuf, sync::Arc, time::Duration};
use gpui_kit::*;

/// Columns in every WaveformPeaks this version produces.
pub const PEAK_COLUMNS: usize = 2048;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AudioError {
    #[error("decode failed: {0}")] Decode(String),
    #[error("no decodable audio track")] NoAudioTrack,
    #[error("no output device")] NoOutputDevice,
    #[error("device unsupported: {0}")] UnsupportedDevice(String),
    #[error("output failed: {0}")] Output(String),
    #[error("playback unavailable on this platform")] Unavailable,
}

/// Fully decoded, normalized audio. Samples are interleaved f32 in ±1.0,
/// 1 or 2 channels. `samples` is owned by the value (moved into the engine's
/// render buffer, not retained).
pub struct DecodedAudio { /* private */ }
impl DecodedAudio {
    pub fn source_name(&self) -> &str;
    pub fn sample_rate(&self) -> u32;
    pub fn channels(&self) -> usize;      // 1 or 2 (sources with more are downmixed)
    pub fn frames(&self) -> usize;
    pub fn duration(&self) -> Duration;
    pub fn samples(&self) -> &[f32];      // len == frames * channels
}

/// Synchronous decode. `name` supplies the extension hint (".wav" routes to
/// hound, everything else to symphonia). Runs on the caller's thread; the
/// engine wraps it in a background task.
pub fn decode_bytes(name: &str, bytes: Vec<u8>) -> Result<DecodedAudio, AudioError>;

#[derive(Clone, Copy, Debug)]
pub struct PeakColumn { pub min: f32, pub max: f32, pub rms: f32 }

pub struct WaveformPeaks { /* private */ }
impl WaveformPeaks {
    /// Min/max/rms decimation to exactly PEAK_COLUMNS columns over `samples`.
    pub fn from_interleaved(samples: &[f32], channels: usize) -> Self;
    pub fn peak(&self, ix: usize) -> Option<PeakColumn>;  // None out of range
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputDevice { pub name: String, pub is_default: bool }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaybackStatus { Loading, Stopped, Playing, Paused }

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct AudioMeta {
    pub source_name: String,
    pub sample_rate: u32,
    pub channels: usize,
    pub duration: Duration,
}

/// Playback engine. All mutators end with exactly one `cx.notify()`;
/// consumers re-render via `cx.observe(&engine, |_, _, cx| cx.notify())`.
pub struct AudioEngine { /* private */ }
impl AudioEngine {
    pub fn new(cx: &mut Context<Self>) -> Self;

    /// Async load: sets Loading, decodes + builds peaks off-thread, then
    /// prepares the render buffer (native: channel-adapt + resample to the
    /// selected device rate). Replaces any previous load. Decode failures
    /// land in `last_error()` and leave the previous load intact.
    pub fn load_bytes(&mut self, name: &str, bytes: Vec<u8>, cx: &mut Context<Self>);
    /// Native only; wasm callers get `last_error()` = Unavailable.
    #[cfg(not(target_family = "wasm"))]
    pub fn load_path(&mut self, path: PathBuf, cx: &mut Context<Self>);
    pub fn unload(&mut self, cx: &mut Context<Self>);

    pub fn toggle_play(&mut self, cx: &mut Context<Self>); // Stopped at EOF restarts from 0
    pub fn stop(&mut self, cx: &mut Context<Self>);        // pause + cursor 0 + Stopped
    pub fn seek_fraction(&mut self, fraction: f32, cx: &mut Context<Self>); // clamped 0..=1
    pub fn set_volume(&mut self, volume: f32, cx: &mut Context<Self>);      // clamped 0..=1

    pub fn status(&self) -> Option<PlaybackStatus>;   // None = unloaded
    pub fn position(&self) -> Duration;               // 0 unloaded
    pub fn duration(&self) -> Option<Duration>;
    pub fn progress(&self) -> f32;                    // position/duration, 0 unloaded
    pub fn volume(&self) -> f32;
    pub fn meta(&self) -> Option<AudioMeta>;
    pub fn peaks(&self) -> Option<Arc<WaveformPeaks>>;
    pub fn last_error(&self) -> Option<String>;       // read once per render; cleared on next successful action

    pub fn devices(&self) -> Vec<OutputDevice>;       // empty on wasm
    pub fn selected_device(&self) -> Option<String>;
    pub fn refresh_devices(&mut self, cx: &mut Context<Self>); // native enumerate; wasm no-op
    pub fn select_device(&mut self, name: &str, cx: &mut Context<Self>); // rebuild at saved position

    /// Live metering: on the next successful load (or immediately when a
    /// load exists), attaches plan 16's widget via
    /// `WaveformState::attach_source(rate, channels, DEFAULT_RETENTION)`
    /// and feeds the returned producer from the output callback.
    pub fn attach_meter(&mut self, meter: &Entity<crate::ui::waveform::WaveformState>, cx: &mut Context<Self>);
}

/// Capability registration ("audio_playback"); call once from app init.
pub fn initialize(cx: &mut App);
```

Page surface (`src/features/pages/mod.rs`): `pub use audio::AudioPage;` — `AudioPage::new(window: &mut Window, cx: &mut Context<Self>) -> Self`.

Consumption notes that are part of the contract:

- Observe recipe: `cx.observe(&engine, |_, _, cx| cx.notify())` once in the consumer's constructor.
- The tick task lives inside the engine and dies with the entity; tests drive it with `cx.executor().advance_clock`.
- `attach_meter` before the first `load_*` is the normal order (the page does it in its constructor); a late attach re-attaches on the current load.
- Plan 13 (music player) consumes `AudioEngine`, `decode_bytes`, and `OutputDevice`; plan 18 (microphone, on disk) keeps its own capture stack — 18's contract offers input-device reuse to plan 19 (`18-microphone.md:275-277`), which this plan does not take; the output-only surface above is the boundary. `output.rs` internals stay `pub(crate)`.
- No `EventEmitter`: every observable change is engine-driven; owners observe.

## Module Layout

| File | Purpose |
|---|---|
| `src/services/audio/mod.rs` | Seam: removal-recipe header, `mod` decls, `pub use` re-exports, `initialize()` capability registration |
| `src/services/audio/transport.rs` | `SharedTransport`: Arc'd atomics shared between engine and audio callback |
| `src/services/audio/peaks.rs` | `WaveformPeaks`, `PeakColumn`, `PEAK_COLUMNS`, fixed-column min/max/rms decimation |
| `src/services/audio/decode.rs` | `AudioError`, `DecodedAudio`, `decode_bytes` (hound + symphonia paths), channel downmix to ≤2 |
| `src/services/audio/demo.rs` | `sweep_wav(...)`, `chords_wav(...)`: runtime WAV generation via hound (no bundled assets) |
| `src/services/audio/output.rs` | `#[cfg(not(target_family = "wasm"))]`: device enumeration, f32 config pick, `OutputSession` (stream build + fill callback + pure `fill_buffer`) |
| `src/services/audio/engine.rs` | `AudioEngine`, `PlaybackStatus`, `AudioMeta`, `OutputDevice`, tick loop, background load pipeline, `attach_meter` |
| `src/services/audio/{transport,peaks,decode,demo,output,engine}.test.rs` | Sibling tests (`#[cfg(test)] #[path = ...]`) |
| `src/features/pages/audio.rs` | `AudioPage`: load row, overview + scrub, transport row, device select, spectrum switch, live monitor |
| `src/features/pages/audio.test.rs` | Sibling render-smoke test |

## Seam Touches

Every edit outside the new modules:

| # | File | Change |
|---|---|---|
| 1 | `Cargo.toml` `[dependencies]` | `hound = "3.5"`, `symphonia = { version = "0.6.1", default-features = false, features = ["wav","flac","mp3","ogg","vorbis","aac","isomp4","alac"] }` |
| 2 | `Cargo.toml` `[target.'cfg(not(target_family = "wasm"))'.dependencies]` | `cpal = "0.18"` — CONDITIONAL: plan 18's seam row 1 adds the identical entry (`18-microphone.md:405-408`); if 18 landed first, this row is a no-op (a duplicate TOML key would be an error). hound/symphonia in the common `[dependencies]` are unconditional (row 1) |
| 2b | `scripts/install-linux-deps.sh:14` | CONDITIONAL: add `libasound2-dev` to `PKGS` — only if plan 18 has NOT landed (its row 2 does the same, `18-microphone.md:414-416`); the script already runs in every CI Linux lane |
| 3 | `Cargo.lock` | Regenerated with the manifest change and committed (`tests/qa_migration.rs:167-186`); resolves to ONE cpal regardless of which plan introduced it |
| 4 | `src/services/mod.rs` | `pub mod audio;` |
| 5 | `src/lib.rs:57` | Add `audio` to the `pub use services::{...}` list (gives `crate::audio`) |
| 6 | `src/features/pages/mod.rs` | `mod audio;` + `pub use audio::AudioPage;` |
| 7 | `src/shell/sidebar.rs:5-16` | `Audio` variant appended AFTER `Gallery` (end of enum; digit bindings unshifted — seams §1 row 8) |
| 8 | `src/shell/sidebar.rs:19-31` | `title()` arm `"Audio"` |
| 9 | `src/shell/sidebar.rs:36-48` | `title_key()` arm `"nav_audio"` |
| 10 | `src/shell/sidebar.rs:57-69` | `host()` arm `"audio"` |
| 11 | `src/shell/sidebar.rs:77-89` | `icon()` arm `IconName::Play` (verified default; `AudioLines` is full-catalog and renders blank on native — Research Digest icons bullet) |
| 12 | `src/shell/sidebar.rs:92-105` | `all()` entry appended after `Page::Gallery` |
| 13 | `src/shell/route.rs:10-21` | `Page::Audio.host(),` in `VALID_HOSTS` |
| 14 | `src/shell/root/app_root/state.rs:17-38` | Field `audio_page: Entity<AudioPage>` |
| 15 | `src/shell/root/app_root/state.rs:46-56` | `let audio_page = cx.new(|cx| AudioPage::new(window, cx));` + import at `:6-9` |
| 16 | `src/shell/root/app_root/state.rs:211-223` | `unchecked_active_page_view` arm `Page::Audio => self.audio_page.clone().into()` |
| 17 | `tests/e2e_navigation.rs:14-19` | Count assert → current value + 1 (plan 18's row 10 makes it 10 → 11, so 19 lands as 11 → 12 if 18 went first, else 10 → 11; read the assert, add 1); add `Page::Audio` to the `expected` array |
| 18 | `locales/en.yml` + `locales/zh-CN.yml` | All `audio_*` + `nav_audio` keys below, both files in lockstep (`tests/qa_localization.rs:188-198`) |
| 19 | `src/app/init.rs:202-263` | `crate::audio::initialize(cx);` inside the `runtime_services_init` block (after `crate::storage::initialize(cx);` line) |

Deliberate non-touches: no `AppConfig` field (volume/device/queue are session state; schema pinned by `tests/snapshot_tests.rs:26,57`); no command-palette entry (launcher integration is plan 13's territory); no `src/ui/` changes (the overview painter is feature-local; the live monitor is plan 16's widget); no sub-routes; no gallery section; `Cargo.toml` `[patch.crates-io]` untouched (`tests/qa_migration.rs:238-267`).

### Removal Recipe

`src/services/audio/mod.rs` opens with (mirroring `src/gallery/mod.rs:1-7`):

```rust
//! Removable audio playback service. `AudioEngine` is the shared decode +
//! output stack; the Audio page is its only consumer today. Removal = delete
//! this directory and revert its seam touches: the `pub mod audio;` line in
//! services/mod.rs, the `audio` name in lib.rs's services re-export, the
//! AudioPage module/pub use in features/pages/mod.rs, the Page::Audio arms
//! and all() entry in shell/sidebar.rs, the VALID_HOSTS entry in
//! shell/route.rs, the AppRoot wiring in shell/root/app_root/state.rs, the
//! page count in tests/e2e_navigation.rs, the audio_* and nav_audio keys in
//! both locale files, the initialize call in app/init.rs, and the
//! hound/symphonia/cpal entries plus regenerated Cargo.lock in Cargo.toml.
//! Consumers of the Interface Contract (plans 13/18) must be ported first.
```

Ordered removal steps:

1. Revert seam rows 19, 18, 17, 16, 15, 14, 13, 12–7, 6, 5, 4, 2b, 2, 1 (reverse order), each to its current content. Rows 2 and 2b only revert what this plan actually added: if plan 18 introduced the `cpal` line or the `libasound2-dev` entry, they stay.
2. `rm -r src/services/audio/ src/features/pages/audio.rs src/features/pages/audio.test.rs`.
3. Remove the `hound`, `symphonia` lines from `[dependencies]`; remove `cpal` from the not-wasm table only if plan 18 did not put it there; run `cargo check --workspace`; commit the regenerated `Cargo.lock`.
4. `grep -rn "services::audio\|crate::audio\|AudioPage\|ui::waveform" src/ tests/` — remaining hits are dependents (plans 13/16/18 surfaces); removal is only complete when they are removed or stubbed first.
5. Run the gates in Verification; they must pass with zero references left.

## Implementation Phases

## Phase 1: Dependencies and module scaffold

### Step 1.1: Cargo manifest + lock

- Files: `Cargo.toml`, `Cargo.lock`, conditionally `scripts/install-linux-deps.sh`.
- Build: add `hound = "3.5"` and the `symphonia` line to `[dependencies]` (after `rust-embed = "8"`); add `cpal = "0.18"` to the `[target.'cfg(not(target_family = "wasm"))'.dependencies]` table (after `arboard`) ONLY if the line is not already there (plan 18 adds the identical entry; a duplicate TOML key is a hard error). If plan 18 has not landed, also add `libasound2-dev` to `PKGS` in `scripts/install-linux-deps.sh:14` (cpal's ALSA backend needs the headers; the script runs in every CI Linux lane). Run `cargo check --workspace` to regenerate the lock.
- Tests: none.
- Done-check: `cargo check --workspace` passes; `git diff Cargo.lock | grep -E '^\+name = "(cpal|hound|symphonia|symphonia-core|alsa)"'` shows the new packages (the `\+` anchor is required — plain `^name` never matches diff lines), and `just wasm-check` still passes — none of the three lands in the wasm table.

### Step 1.2: services module skeleton

- Files: `src/services/mod.rs` (one line), `src/lib.rs` (one name in the services re-export), `src/services/audio/mod.rs`.
- Build: `pub mod audio;` in services/mod.rs; add `audio` to lib.rs's `pub use services::{...}`; create `src/services/audio/mod.rs` with the removal-recipe header, `mod transport;` and a temporary empty re-export.
- Tests: none.
- Done-check: `cargo check -p gpui-starter` passes.

## Phase 2: Pure playback core

### Step 2.1: SharedTransport

- Files: `src/services/audio/transport.rs`, `transport.test.rs` (wired `#[cfg(test)] #[path = "transport.test.rs"] mod transport_test;`).
- Build: `pub(crate) struct SharedTransport { cursor: AtomicU64, playing: AtomicBool, volume_bits: AtomicU32, eof: AtomicBool }` with `new(volume: f32)`, `cursor_frames(&self) -> u64`, `set_cursor(&self, frames: u64)`, `is_playing(&self) -> bool`, `set_playing(&self, bool)`, `volume(&self) -> f32` (`f32::from_bits(load(Relaxed))`), `set_volume(&self, f32)` (`to_bits`), `take_eof(&self) -> bool` (swap false), `set_eof(&self)`. All `Ordering::Relaxed` (single producer/consumer per field, no tearing at 32/64-bit widths).
- Tests: volume roundtrip at 0.0, 0.5, 1.0 exact bit equality; cursor store/load; eof set→take true→take false; playing toggle.
- Done-check: `cargo test -p gpui-starter --lib services::audio::transport` passes.

### Step 2.2: WaveformPeaks

- Files: `src/services/audio/peaks.rs`, `peaks.test.rs`, `mod.rs` re-exports.
- Build: `pub const PEAK_COLUMNS: usize = 2048;`; `PeakColumn { pub min, max, rms: f32 }`; `WaveformPeaks { columns: Vec<PeakColumn> }`; `from_interleaved(samples: &[f32], channels: usize) -> Self`: frames = `samples.len() / channels.max(1)`; column c covers frames `[frames * c / PEAK_COLUMNS, frames * (c+1) / PEAK_COLUMNS)`; per column scan every channel's samples in the frame range for min/max, accumulate `sq_sum` across all channels and `count`, `rms = sqrt(sq_sum/count)`; empty ranges → `{min: 0., max: 0., rms: 0.}`; zero-length input → all-zero columns. `peak(ix) -> Option<PeakColumn>` (`samples()` not needed — engine holds the Arc).
- Tests: ramp `i as f32 / 1000.` mono, 1000 frames → column 2047 is `{min: 0.999, max: 1.0, rms: ...}` (compute expected directly), column 0 is `{0.0, ~0.0005, ...}`; stereo interleave `[l, r, ...]` uses both channels (column with l=-1., r=1. gives min -1., max 1.); every column populated when frames >= PEAK_COLUMNS; empty input → 2048 zero columns; `peak(PEAK_COLUMNS)` is None.
- Done-check: `cargo test -p gpui-starter --lib services::audio::peaks` passes.

## Phase 3: Decode and demo assets

### Step 3.1: Demo generators

- Files: `src/services/audio/demo.rs`, `demo.test.rs`.
- Build: `pub(crate) fn sweep_wav(seconds: u32) -> Vec<u8>` — 44_100 Hz stereo 16-bit; frequency sweeps 20 Hz → 20 kHz exponentially over the duration; per frame `s = 0.5 * sin(2π · phase)` with `phase += freq / 44100.` (phase accumulated in f64 to avoid drift); both channels identical. `pub(crate) fn chords_wav(seconds: u32) -> Vec<u8>` — 44_100 Hz stereo; chord progression of 2-second blocks cycling [C major (261.63, 329.63, 392.00), A minor (220.00, 261.63, 329.63), F major (174.61, 220.00, 261.63), G major (196.00, 246.94, 293.66)] Hz; each frame `s = 0.4 * mean(sin(2π f_i t))`, stereo-duplicated. Both write via `hound::WavWriter::new(std::io::Cursor::new(Vec::new()), WavSpec { channels: 2, sample_rate: 44_100, bits_per_sample: 16, sample_format: hound::SampleFormat::Int })`, `write_sample(i16)` per sample, `finalize()`, return the inner Vec (`.into_inner()` after finalize; if the API returns Result, map errors to `expect("in-memory wav write cannot fail")`).
- Tests: for each generator: re-read with `hound::WavReader::new(Cursor)` → `spec().sample_rate == 44100`, `channels == 2`, `duration() == seconds * 44100`; all samples within ±0.6·32768; sweep: first 100 samples correlate with a 20 Hz sine (zero crossings ≈ 2 per 4410 samples), last 100 with a high frequency (many zero crossings); chords: exactly `seconds` divisible by 2 blocks, non-silent throughout.
- Done-check: `cargo test -p gpui-starter --lib services::audio::demo` passes.

### Step 3.2: hound decode path

- Files: `src/services/audio/decode.rs`, `decode.test.rs`.
- Build: `AudioError` (thiserror, variants per the contract); `DecodedAudio { source_name: String, sample_rate: u32, channels: usize, samples: Vec<f32> }` + readers + `frames()` (`samples.len() / channels`) + `duration()` (`Duration::from_secs_f32(frames as f32 / rate as f32)`); `pub fn decode_bytes(name: &str, bytes: Vec<u8>) -> Result<DecodedAudio, AudioError>`: extension = `Path::new(name).extension()` lowercased; `"wav"` → `decode_wav(&bytes)`; else `decode_symphonia(bytes, ext)`. `decode_wav`: `hound::WavReader::new(Cursor::new(&bytes[..]))` (map err → `AudioError::Decode`); match `(spec().sample_format, spec().bits_per_sample)`: `(Float, 32)` → `samples::<f32>()` clamped to ±1; `(Int, 16)` → `samples::<i16>()` / 32_768.; `(Int, 24)` → `samples::<i32>()` / 8_388_608.; `(Int, 32)` → `samples::<i32>()` / 2_147_483_648.; anything else → `AudioError::Decode("unsupported wav sample format")`. `adapt_channels(&mut samples, &mut channels)`: >2 → stereo downmix (L = mean of channels `0..n/2`, R = mean of `n/2..n`), 0 → `AudioError::NoAudioTrack`; applied in both paths. Empty samples → `NoAudioTrack`.
- Tests: roundtrip — `decode_bytes("sweep.wav", sweep_wav(1))` → 44_100 Hz, 2 ch, 44_100 frames, `duration() == 1s`; i16 normalization: hand-build a minimal 1-sample mono i16 WAV with hound (value 16_384) → decoded 0.5; float WAV (write f32 via `WavSpec { sample_format: Float, bits_per_sample: 32 }`) passes through; garbage bytes (`b"not audio"`) → `Err(Decode(_))`; truncated valid WAV (first 44 bytes only) → Err.
- Done-check: `cargo test -p gpui-starter --lib services::audio::decode` passes.

### Step 3.3: symphonia decode path

- Files: same.
- Build: `fn decode_symphonia(bytes: Vec<u8>, ext: Option<String>) -> Result<DecodedAudio, AudioError>`: `MediaSourceStream::new(Box::new(std::io::Cursor::new(bytes)), Default::default())`; `Hint::new()` + `with_extension` when ext present; `get_probe().probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())` → `Box<dyn FormatReader>`; `let track = format.default_track(TrackType::Audio).ok_or(NoAudioTrack)?;` (import `symphonia::core::formats::TrackType`); `let params = track.codec_params.as_ref().and_then(|p| p.audio()).ok_or(NoAudioTrack)?;` (also derive `sample_rate`/`channels` from `params.sample_rate`/`params.channels` as `Option` fallbacks); `let mut decoder = get_codecs().make_audio_decoder(params, &AudioDecoderOptions::default())` (map err → Decode); loop `match format.next_packet()`: `Ok(Some(packet))` — skip `packet.track_id != track.id`; `decoded = decoder.decode(&packet)?` (map symphonia error: `Error::DecodeError`/`Unsupported` → `AudioError::Decode`/`NoAudioTrack`, IoError UnexpectedEof → break); first iteration captures `spec = *decoded.spec()`; `let mut block = Vec::new(); decoded.copy_to_vec_interleaved::<f32>(&mut block); samples.extend(block)`. `Ok(None)` → break. Then clamp ±1, `adapt_channels`, build `DecodedAudio`. `use symphonia::core::{audio::Channels?, codecs::audio::AudioDecoderOptions, formats::{FormatOptions, FormatReader, TrackType}, formats::probe::Hint, io::MediaSourceStream, meta::MetadataOptions}` — note `Hint` lives at `core::formats::probe` (`SYMPH/symphonia-core-0.6.1/src/formats/probe.rs:261`); the `core::probe::Hint` path in upstream docs is stale.
- Tests: `decode_symphonia(sweep_wav(1).to_vec(), Some("wav"))` (feature "wav" enabled for exactly this) → same numbers as the hound path test in 3.2; `decode_symphonia(garbage, None)` → Err; `decode_bytes("x.mp3", garbage)` routes to symphonia (Err, not the wav path); `decode_bytes("x.flac", garbage)` same.
- Done-check: `cargo test -p gpui-starter --lib services::audio::decode` passes.

## Phase 4: Engine skeleton (no cpal)

### Step 4.1: Entity, load pipeline, tick

- Files: `src/services/audio/engine.rs`, `engine.test.rs`, `mod.rs` re-exports.
- Build: structs per the contract (`PlaybackStatus`, `AudioMeta`, `OutputDevice`, `LoadState` as in Architecture); `AudioEngine::new` sets `volume: 0.8`, spawns the 33 ms tick (`cx.spawn(async move |this, cx| loop { cx.background_executor().timer(Duration::from_millis(33)).await; if this.update(cx, |e, cx| e.refresh_from_transport(cx)).is_err() { break; } })`, stored not detached). `refresh_from_transport`: if load Some: `let cursor = transport.cursor_frames();` recompute `position_secs`; `if transport.take_eof() { status = Stopped; }` sync `status` Playing↔Paused from `transport.is_playing()`; notify only when something changed (compare cached f32 position rounded to ms + status). `load_bytes`: `self.load_generation += 1; gen = ...; self.loading = Some(name.into()); self.last_error = None; status = Some(Loading);` then `let decode = cx.background_spawn(async move { decode_bytes(&name, bytes).map(|d| (d, Arc::new(WaveformPeaks::from_interleaved(d.samples(), d.channels())))) });` and `cx.spawn(async move |this, cx| { let result = decode.await; this.update(cx, |e, cx| e.finish_load(gen, result, cx)).ok(); }).detach();`. `finish_load`: if `gen != self.load_generation` → drop; map Err → `last_error = Some(...)` + `status = None if no previous load` + `loading = None` + notify; Ok → `prepare_render(decoded)` then build `LoadState` (transport volume = self.volume), `status = Some(Stopped)`, `loading = None`, refresh meter (Step 4.2), notify. `prepare_render` native-only body in Phase 6 (Phase 4 stub: `render = Arc::new(decoded.samples().to_vec())`, `render_rate = decoded.sample_rate()`, `render_channels = decoded.channels()`; wasm keeps this stub permanently). Readers per contract; `position()` = `cursor / render_rate` via `Duration::from_secs_f32`; `progress()` guarded. `stop`, `seek_fraction` (clamp; `transport.set_cursor(frame)`; notify), `set_volume` (clamp; `transport.set_volume`; notify), `unload` (drop load, `status = None`, detach meter via `meter.update(|s,cx| s.detach_source(cx))` if attached, notify). `devices()` → `self.devices.clone()` (empty until Phase 6 enumeration), `refresh_devices`/`select_device` no-op in Phase 4. `toggle_play` in Phase 4: wasm-final body — no session → `last_error = Some(AudioError::Unavailable.to_string())`, notify.
- Tests (`#[gpui_kit::test]`, `cx.update(|cx| cx.new(|cx| AudioEngine::new(cx)))`): `load_bytes("sweep.wav", demo::sweep_wav(1))` → `cx.run_until_parked()` (background executor drains) → `status() == Some(Stopped)`, `duration() == Some(1s)`, `peaks().is_some()`; `seek_fraction(0.5)` → `position() == 500ms`; `seek_fraction(2.0)` clamps to 1.0; `set_volume(1.5)` clamps to 1.0; `unload()` → all readers None/0; garbage load with a good load present → previous `duration()` intact and `last_error().is_some()`; generation guard: start load A (garbage name), immediately start load B (good) → `run_until_parked` → status Stopped from B (B wins regardless of completion order).
- Done-check: `cargo test -p gpui-starter --lib services::audio::engine` passes.

### Step 4.2: Meter attach

- Files: same.
- Build: `attach_meter(&mut self, meter: &Entity<WaveformState>, cx)`: `self.meter = Some(meter.clone());` then if `load` Some → `self.reconnect_meter(cx)`. `reconnect_meter` (private): `let state = self.meter.clone(); let (rate, ch) = (load.render_rate, load.render_channels); let producer = state.update(cx, |s, cx| s.attach_source(rate, ch, DEFAULT_RETENTION, cx));` store `load.producer = Some(producer)` (field `producer: Option<SampleProducer>` on LoadState). `finish_load` and `unload` call `detach` on the old load: `self.meter.update(|s, cx| s.detach_source(cx))` before replacing. This step hard-depends on plan 16 being implemented (`crate::ui::waveform`); if `src/ui/waveform/` is absent, STOP and land plan 16 first (its Phases 1–4 suffice for `attach_source`; the element is needed only in Phase 7 of this plan).
- Tests: extend 4.1's suite — create `Entity<WaveformState>` (plan 16), `attach_meter`, load demo bytes, `run_until_parked` → `state.read(cx).has_source() && sample_rate() == Some(44_100)` (Phase 4 stub rate); `unload()` → `has_source() == false`.
- Done-check: `cargo test -p gpui-starter --lib services::audio::engine` passes.

## Phase 5: Native output (cpal)

### Step 5.1: Enumeration and config pick

- Files: `src/services/audio/output.rs` (entire file inside `#![cfg(not(target_family = "wasm"))]`-equivalent: guard via `#![cfg]` is crate-level — instead put `#[cfg(not(target_family = "wasm"))]` on every item, matching the repo's per-item cfg style), `output.test.rs` (same cfg).
- Build: `use cpal::traits::{DeviceTrait, HostTrait};`. `pub(crate) fn enumerate() -> Vec<OutputDevice>`: `cpal::default_host()`, collect `output_devices()` mapped to `OutputDevice { name: device.to_string(), is_default: false }`, then mark the one equal to `default_output_device().to_string()` (None → empty vec). `pub(crate) struct StreamPlan { pub config: cpal::StreamConfig, pub sample_rate: u32, pub channels: u16 }`; `pub(crate) fn plan_stream(device: &cpal::Device, source_rate: u32) -> Result<StreamPlan, AudioError>`: iterate `device.supported_output_configs()`; candidates with `sample_format() == cpal::SampleFormat::F32` (match wildcard `_` for the non-exhaustive arm is impossible for equality — compare via `matches!(fmt, cpal::SampleFormat::F32)`); pick first with `channels() == 2`, else first with `channels() >= 2` clamped to 2 in the config, else first with `channels() == 1`; none → `UnsupportedDevice("no f32 output config")`, no device configs at all → `NoOutputDevice`. Rate: `if range.contains_rate(SampleRate(source_rate)) { range.try_with_sample_rate(SampleRate(source_rate)) } else if source_rate < min { try_with(min) } else { Some(with_max_sample_rate()) }` → unwrap the Option into `UnsupportedDevice` on None. Build `StreamPlan { config: StreamConfig { channels: out_ch, sample_rate, buffer_size: BufferSize::Default }, .. }`.
- Tests (pure, no cpal types needed): extract `fn pick_rate(min: u32, max: u32, src: u32) -> u32` (contains → src; below min → min; else max) and test 44.1k in [44.1k, 192k] → 44.1k; 22.05k below 44.1k → 44.1k; 96k in range → 96k; above max → max. The iterator-filtering logic is exercised by code review + Step 6 device-guarded tests.
- Done-check: `cargo check -p gpui-starter` passes; `cargo test -p gpui-starter --lib services::audio::output` passes.

### Step 5.2: Fill callback (pure) and OutputSession

- Files: same.
- Build: `pub(crate) fn fill_buffer(src: &[f32], channels: usize, transport: &SharedTransport, out: &mut [f32], tap: &mut Vec<f32>) -> bool` (returns eof): `let cursor = transport.cursor_frames() as usize; let total = src.len() / channels.max(1); let frames_out = out.len() / channels; let avail = total.saturating_sub(cursor); let take = frames_out.min(avail); let vol = transport.volume(); let start = cursor * channels; for i in 0..take * channels { out[i] = src[start + i] * vol; } for i in take * channels..out.len() { out[i] = 0.; } tap.clear(); tap.extend_from_slice(&out[..take * channels]); transport.set_cursor((cursor + take) as u64); take < frames_out && avail <= frames_out` (eof when the source ran dry this fill; the rest stays silent per cpal's pre-filled buffer). `pub(crate) struct OutputSession { stream: cpal::Stream }`; `pub(crate) fn build_session(device: &cpal::Device, render: Arc<Vec<f32>>, render_channels: usize, plan: StreamPlan, transport: Arc<SharedTransport>, producer: crate::ui::waveform::SampleProducer) -> Result<OutputSession, AudioError>`: clone render/Arc/producer into `device.build_output_stream::<f32>(&plan.config, move |data: &mut [f32], _| { let mut tap = Vec::with_capacity(data.len()); if fill_buffer(&render, render_channels, &transport, data, &mut tap) { transport.set_playing(false); transport.set_eof(); } producer.push_interleaved(&tap); }, |err| tracing::warn!(target: "gpui_starter::audio", error = %err, "cpal stream error"), None)` — map err → `AudioError::Output`; `OutputSession { stream }` with `pub(crate) fn play(&self) -> Result<(), AudioError>` and `pause` (`stream.play()/pause()` mapped through `Output`).
- Tests: `fill_buffer` — mono src `[0.5; 100]`, transport cursor 0 volume 1.0, out len 10 → out all 0.5, cursor 10, tap len 10, eof false; volume 0.5 → 0.25; cursor 95 out len 10 → 5 copied + 5 zeros, eof true; stereo interleaving preserved (`src = [1., -1.]` repeated → out alternates). Session build itself is device-dependent — covered by the guarded engine tests (6.1) and manual QA.
- Done-check: `cargo test -p gpui-starter --lib services::audio::output` passes.

### Step 5.3: Resample (pure)

- Files: same.
- Build: `pub(crate) fn resample_linear(src: &[f32], channels: usize, from: u32, to: u32) -> Vec<f32>`: identity when `from == to`; frames_out = `round(frames_in * to / from)`; per output frame f: `t = f * from / to` (f64), `i = floor`, frac; per channel lerp `a + (b - a) * frac` (last frame clamps). Also `pub(crate) fn adapt_to_channels(src: &[f32], ch: usize, out_ch: usize) -> Vec<f32>`: mono→stereo duplicate, stereo→mono average, identity otherwise.
- Tests: identity no-op (ptr-equal content, len equal); 1 Hz DC mono 44100→48000 → len 48000, all values ≈ DC; mono sine 440 Hz 44100→48000: peak of resampled within 5% of source peak; stereo channel independence (L const 1., ramp R → output keeps separation); stereo→mono average.
- Done-check: `cargo test -p gpui-starter --lib services::audio::output` passes.

## Phase 6: Engine native playback

### Step 6.1: Session management, transport, device switch

- Files: `src/services/audio/engine.rs`, `engine.test.rs`.
- Build (all `#[cfg(not(target_family = "wasm"))]` unless noted): replace `prepare_render` stub — pick device (`self.device()` = selected name → re-enumerate and match by name, else `default_output_device()`); no device → `AudioError::NoOutputDevice` into `last_error` (load still succeeds so the overview renders); `plan = output::plan_stream(&device, decoded.sample_rate())?`; `samples = adapt_to_channels(decoded.samples(), ch, plan.channels)`; `samples = resample_linear(&samples, plan.channels, decoded.sample_rate(), plan.sample_rate)`; store render/rate/channels; `connect_session(cx)`. `connect_session`: if load + device + `load.producer` Some → `build_session(...)` → `session = Some(...)`; `transport.set_playing(false)`; if a play was pending (`pending_play` bool) → `session.play()`, `transport.set_playing(true)`, status Playing. `toggle_play` native: no load → no-op; `Loading` → no-op; Playing → `session.pause()`, playing false, status Paused; Paused → `session.play()`, playing true, status Playing; Stopped → if cursor >= frames → seek 0 → play (status Playing); no session → set `pending_play` and `connect_session`. `stop`: pause + cursor 0 + status Stopped. `refresh_devices` native: `self.devices = output::enumerate();` notify. `select_device(name)`: if name == selected → no-op; save `pos = position_secs()`, `was_playing`; unload session; `selected_device = Some(name)`; rebuild render from `load.original`? — rebuilding requires re-decoding (source samples were transformed in place). Fix: keep `decoded: Arc<DecodedAudio>` in LoadState (do NOT drop after prepare; memory noted in Risks), re-run `prepare_render` on switch, `transport.set_cursor(pos * new_rate)`, if was_playing → play. `load_path(path, cx)`: `self.load_bytes(&file_name_from(&path), std::fs::read(path) mapped in background, cx)` — spawn the read inside the same background task: restructure `load_bytes` to take a `background_spawn` closure; simplest: `load_path` sets Loading + spawns `cx.background_spawn(async move { std::fs::read(&path).map_err(|e| AudioError::Decode(e.to_string())).and_then(|b| decode_bytes(name, b)) ... })` duplicating finish_load's tail via a shared private `spawn_decode(task, cx)` helper.
- Tests: device-independent (skip stream assertions when `cpal::default_host().default_output_device().is_none()` — guard each test with early return; document in the test comment): with a device (local dev machines): load demo → `toggle_play` → status Playing, then poll for the playhead with a BOUNDED loop — `for _ in 0..50 { cx.executor().advance_clock(Duration::from_millis(33)); cx.executor().run_until_parked(); if position() > 0 { break; } }` — because `advance_clock`/`run_until_parked` drive the engine tick deterministically but NOT the real cpal callback thread; the loop yields to it instead of assuming one tick is enough; assert the loop broke (position > 0). `toggle_play` → Paused and position frozen across another poll round; seek while playing jumps position; `stop` → position 0 + Stopped; volume 0 → (manual QA audible; automated: `transport`-level covered in 5.2). Always-run: `refresh_devices` on wasm/no-device returns without panic (cfg'd native — on wasm the method is the no-op body); `select_device` with unknown name keeps the current selection.
- Done-check: `cargo test -p gpui-starter --lib services::audio::engine` passes on this machine; device-present assertions pass locally (report if this host has no device).

## Phase 7: Audio page and shell seam

### Step 7.1: Page scaffold + full page seam

- Files: `src/features/pages/audio.rs`, `audio.test.rs`, `src/features/pages/mod.rs`, `src/shell/sidebar.rs`, `src/shell/route.rs`, `src/shell/root/app_root/state.rs`, `tests/e2e_navigation.rs`, `locales/en.yml`, `locales/zh-CN.yml`.
- Build: `AudioPage { engine: Entity<AudioEngine>, waveform: Entity<WaveformState>, volume_slider: Entity<SliderState>, device_select: Entity<SelectState<Vec<String>>>, spectrum: bool, focus_handle: FocusHandle, _subscriptions: Vec<Subscription> }`. `new`: `engine = cx.new(|cx| AudioEngine::new(cx));` `cx.observe(&engine, |_, _, cx| cx.notify())`; `waveform = cx.new(|_| WaveformState::new()); cx.observe(&waveform, ...)` (contract `16:208-210`); `engine.update(|e, cx| { e.attach_meter(&waveform, cx); e.refresh_devices(cx); })`; `volume_slider = cx.new(|_| SliderState::new().min(0.).max(1.).step(0.01).default_value(engine.volume()))`; subscribe `SliderEvent::Change/Release` → `engine.update(set_volume)` (pattern `controls/slider.rs:122-130`); `device_select = cx.new(|cx| SelectState::new(engine.devices().names(), Some(IndexPath::default()), window, cx))` + subscribe `SelectEvent::Confirm` → `engine.update(select_device)` (pattern `theme/colors.rs:151-163`); `actions!(audio_page, [TogglePlayback]);` at module top (`gpui_kit::actions`, precedent `app/actions.rs:1-12`); `cx.bind_keys([KeyBinding::new("space", TogglePlayback, Some("AudioPage"))]);`; `focus_handle: cx.focus_handle().tab_stop(true)`. `Render`: `v_flex().id("audio-page").size_full().p_6().gap_4().key_context("AudioPage").track_focus(&self.focus_handle).on_action(cx.listener(|this, _: &TogglePlayback, _, cx| { this.engine.update(|e, cx| e.toggle_play(cx)); }))` + header (heading `Role::Heading` `aria_level(1)` + meta line, pattern `notifications.rs:48-58`). All seam rows 4–18 from Seam Touches land in this step (both locale files, all keys from the i18n table).
- Tests: `audio.test.rs` render smoke (`#[gpui_kit::test]` + `cx.add_window` per plan 16 Step 6.2 pattern, `GPUI/app/test_context.rs:279-296`): page renders unloaded without panic; heading text non-empty. `tests/e2e_navigation.rs` passes with the new count.
- Done-check: `cargo test -p gpui-starter --lib features::pages::audio && cargo test --test e2e_navigation` pass.

### Step 7.2: Load row and empty state

- Files: `audio.rs`.
- Build: `render_load_row(cx)`: native — `Button::new("audio-open").outline().icon(IconName::FolderOpen).label(localize("audio_open_file")).disabled(loading)` → on click `pick_file` (`desktop_actions.rs:194`; `None` = cancelled → no-op) → `engine.update(load_path)`; demo buttons `Button::new("audio-demo-sweep").ghost().label(localize("audio_demo_sweep"))` → `engine.update(load_bytes("sine-sweep.wav", demo::sweep_wav(10)))`, chords likewise. Wasm: the row is replaced by the capability notice (7.6) — wrap the row body in `#[cfg(not(target_family = "wasm"))]` helper `render_native_controls`. Unloaded main area: `Empty::new().header(EmptyHeader::new().title(EmptyTitle::new().child(localize("audio_empty_title"))).description(EmptyDescription::new().child(localize("audio_empty_description"))))` — `title`/`description` take the typed `EmptyTitle`/`EmptyDescription` builders, not strings (`COMP/empty.rs:114-121`, repo precedent `src/gallery/sections/loading/empty.rs:110-124`). Meta line when loaded: `localize_with_args("audio_meta_format", &[("channels",...), ("rate",...), ("duration", ...)])` — channels/rate as numbers via `format!` into the placeholder values (placeholders are strings; pre-format).
- Tests: render smoke now asserts the empty-state title appears (localize returns the en string in test locale) and that buttons are disabled while `Loading` (drive a garbage load, `run_until_parked`? — garbage fails fast; instead assert disabled at rest with a load present is N/A; keep: disabled=false unloaded).
- Done-check: `cargo test -p gpui-starter --lib features::pages::audio` passes.

### Step 7.3: Waveform overview with playhead and scrubbing

- Files: `audio.rs`.
- Build:
  - Page field `overview_bounds: std::cell::RefCell<Option<Bounds<Pixels>>>`, written every frame by the peaks canvas prepaint (prepaint runs before the paint phase where the div registers its listeners, so same-frame listeners always read fresh geometry; document the frame-scoped caveat in one comment line).
  - `fn fraction_at(x: Pixels, bounds: Bounds<Pixels>) -> f32` (private, pure): `((x - bounds.left()) / bounds.size.width).clamp(0., 1.)`.
  - `fn format_time(d: Duration) -> String` (private, pure): `m:ss`.
  - `fn paint_peaks(bounds: Bounds<Pixels>, peaks: &WaveformPeaks, progress: f32, window: &mut Window, cx: &App)` (private, pure-ish painter): `let scale = window.scale_factor();` `let width_px = (bounds.size.width * scale).round().max(1.) as usize;` for each device-pixel column `x in 0..width_px`: fraction = `x as f32 / width_px as f32`; `let col = peaks.peak((fraction * PEAK_COLUMNS as f32) as usize)`; amplitude = `col.map(|c| c.max.abs().max(c.min.abs())).unwrap_or(0.).min(1.)`; y span `center ± amp * height / 2`, each edge snapped via `(v * scale).round() / scale`, floored at `1. / scale` tall; `window.paint_quad(fill(quad_bounds, color))` with color `cx.theme().primary.opacity(0.9)` when `fraction <= progress` else `cx.theme().muted_foreground.opacity(0.45)`; then a playhead hairline: width `1. / scale`, x at `bounds.left() + fraction_at… (progress * bounds.size.width)` snapped, color `cx.theme().foreground`.
  - `render_overview`: `div().id("audio-overview").relative().h_24().w_full().rounded(cx.theme().radius).border_1().border_color(cx.theme().border).track_focus(&self.focus_handle).a11y(Role::Slider, localize("audio_seek_label")).aria_min_numeric_value(0.).aria_max_numeric_value(1.).aria_value(format_time(position)).on_a11y_action(AccessibleAction::Click, { let engine = self.engine.clone(); move |_, _, cx| { engine.update(cx, |e, cx| e.toggle_play(cx)); } })` with two absolutely-stacked children: (1) `canvas(move |bounds, _, _, _| *self.overview_bounds.borrow_mut() = Some(bounds), move |bounds, _, window, cx| if let Some(peaks) = engine_peaks { paint_peaks(bounds, &peaks, progress, window, cx) })` — `canvas(prepaint, paint)` (`GPUI/elements/canvas.rs:10-22`), capturing clones of `Entity<AudioEngine>` (read `peaks().clone()` and `progress()` inside the closures via `engine.read(cx)`), and `Empty` when unloaded (border-only box); (2) scrub layer cloned from the kit slider (`BASE/slider.rs:676-716`): `div().id("audio-scrub").absolute().size_full().on_mouse_down(MouseButton::Left, { let engine = self.engine.clone(); let bounds = &self.overview_bounds; move |e: &MouseDownEvent, _, cx| { if let Some(b) = *bounds.borrow() { let f = fraction_at(e.position.x, b); engine.update(cx, |eng, cx| eng.seek_fraction(f, cx)); } } }).on_drag(DragSeek(entity_id), { ... same seek body using the captured bounds refcell via a captured page WeakEntity … })` — concretely: define `struct DragSeek(EntityId);` with `impl Render { Empty }` (`BASE/slider.rs:430-436`), and register `.on_drag(DragSeek(self.engine.entity_id()), |drag, _, _, cx| { cx.stop_propagation(); cx.new(|_| drag.clone()) })` (`Entity::entity_id()` per `BASE/slider.rs:497`) plus `.on_drag_move(window.listener_for(&self.engine, move |engine, e: &DragMoveEvent<DragSeek>, _, cx| { if let Some(b) = *bounds_cell.borrow() { engine.seek_fraction(fraction_at(e.event.position.x, b), cx); } }))` — `bounds_cell` is an `Rc<RefCell<Option<Bounds<Pixels>>>>` clone of the same cell the prepaint writes (switch the page field to `Rc<RefCell<…>>` so both closures share it). `on_mouse_down` seeks immediately (jump-to-click); drag continues scrubbing.
- Tests: `format_time(0) == "0:00"`, `format_time(65s) == "1:05"`, `format_time(600s) == "10:00"`; `fraction_at` clamps outside bounds to 0/1 and maps the midpoint to 0.5. Render smoke: loaded page (engine.load_bytes demo + run_until_parked) renders overview without panic; overview_bounds cell is Some after a frame.
- Done-check: `cargo test -p gpui-starter --lib features::pages::audio` passes.

### Step 7.4: Transport row

- Files: `audio.rs`.
- Build: `render_transport(cx)`: `h_flex().gap_2().items_center().flex_wrap()` containing: play/pause `Button::new("audio-toggle").ghost().icon(if playing {IconName::Pause} else {IconName::Play}).accessibility_label(localize(playing ? "audio_pause" : "audio_play")).tooltip(...).disabled(no load || loading)` → toggle; stop `Button::new("audio-stop").ghost().icon(IconName::Square).accessibility_label(localize("audio_stop")).disabled(...)`; time readout `div().text_sm().child(format!("{}/{}", format_time(position), format_time(duration)))` with `a11y_live(accesskit::Live::Polite)` (`accesskit::` path per repo precedent `src/features/pages/notifications.rs:52`); status text (`audio_status_*` label) as a plain `div().text_sm().text_color(cx.theme().muted_foreground)` — NOT a Badge: `BadgeVariant` is `{Number, Dot, Icon(Box<Icon>)}` only, no neutral/text variant (`COMP/badge.rs:6-11`); `Icon::new(IconName::Settings2)` (volume stand-in — no speaker glyph in the default 106) + `Slider::new(&self.volume_slider).horizontal().w_32()`; device `Select::new(&self.device_select).placeholder(localize("audio_device")).w_48()` preceded by a refresh ghost icon button (`IconName::RotateCw`, verified in `toolbar.rs:269`); `Switch::new("audio-spectrum").checked(self.spectrum).label(localize("audio_spectrum")).on_click(cx.listener(|this, checked, _, cx| { this.spectrum = *checked; let mode = if *checked { WaveformMode::Spectrum } else { WaveformMode::Oscilloscope }; this.waveform.update(cx, |s, cx| s.set_mode(mode, cx)); cx.notify(); }))` (`COMP/switch.rs:56,79`). Device list sync: when engine notifies and `devices()` names differ from the SelectState items, rebuild items in place (SelectState has no runtime set_items — recreate the entity is disruptive; instead keep page-side `device_names: Vec<String>` and only rebuild the SelectState when the list actually changed, via a new `cx.new` stored into the page field; note as a documented re-create). Unloaded → all transport controls disabled; device Select disabled on wasm.
- Tests: render smoke asserts play button disabled unloaded; after engine.load_bytes + run_until_parked the play button is enabled (assert via state readers the page uses, not DOM querying: keep to engine readers + no-panic render).
- Done-check: `cargo test -p gpui-starter --lib features::pages::audio` passes.

### Step 7.5: Live monitor

- Files: `audio.rs`.
- Build: `render_live(cx)`: section title (`audio_live_title`, `Role::Heading` `aria_level(2));` loaded + attached → `Waveform::new("audio-live", self.waveform.clone()).accent(cx.theme().primary)` sized `h_40().w_full()` inside a bordered `group_box`-free container (plain `div().border_1().rounded(cx.theme().radius)`; no nested cards per design guides); unloaded → compact `Empty` with `EmptyHeader::new().title(EmptyTitle::new().child(localize("audio_empty_title")))` (typed builders as in Step 7.2). The observe wiring from 7.1 drives redraws; plan 16's internal tick notifies the state, the page observer re-renders.
- Tests: render smoke with load — live monitor present (no panic); toggle spectrum switch → `waveform.read(cx).mode() == Spectrum`.
- Done-check: `cargo test -p gpui-starter --lib features::pages::audio` passes.

### Step 7.6: Error and unsupported states

- Files: `audio.rs`, `src/services/audio/mod.rs`.
- Build: engine `last_error` Some → `Alert::new(...)` warning variant with `localize_with_args("audio_error_decode"/"audio_error_output", &[("error", ...)])` above the overview; wasm / capability `supported == false` → replace the native content area with `render_unsupported()`: `Empty::new().header(EmptyHeader::new().title(EmptyTitle::new().child(localize("audio_unsupported_title"))).description(EmptyDescription::new().child(localize("audio_unsupported_description"))))` (typed builders as in Step 7.2), read from `crate::capabilities::snapshot(cx).get("audio_playback")`. Toggle play on wasm surfaces `audio_error_output` via the same Alert (engine sets Unavailable).
- Tests: render smoke — with capability absent (test app has no initialize call) the page still renders (guard: treat missing entry as unknown → render content on native, notice on wasm).
- Done-check: `cargo test -p gpui-starter --lib features::pages::audio` passes.

## Phase 8: Capability, wasm, gates

### Step 8.1: Capability registration

- Files: `src/services/audio/mod.rs`, `src/app/init.rs`.
- Build: `pub fn initialize(cx: &mut App)` — wasm: `capabilities::set("audio_playback", CapabilityStatus { supported: false, enabled: false, degraded: false, reason: Some("cpal output requires a native host".into()), last_error: None }, cx)` (pub fields, `capabilities.rs:6-13`); native: enumerate once — devices non-empty → `supported_enabled()`, else `degraded("no output device found")`. Add `crate::audio::initialize(cx);` in `init.rs` runtime_services block (after `crate::storage::initialize(cx);`).
- Tests: `initialize` in a `#[gpui_kit::test]` app → `snapshot(cx)["audio_playback"]` present with expected platform fields.
- Done-check: `cargo test -p gpui-starter --lib services::audio` passes.

### Step 8.2: Wasm pass

- Files: none new.
- Build: run `just wasm-check` (`cargo +nightly check --target wasm32-unknown-unknown -p gpui-starter --lib`, `justfile:24-25`); fix any cfg leakage (symphonia/hound compile on wasm32 — pure Rust; cpal is excluded by the target table).
- Done-check: `just wasm-check` passes.

### Step 8.3: Full gates

- Files: none.
- Build: run the four repo gates in order (Verification).
- Done-check: all four pass; `grep -rn "use gpui::" src/services/audio/ src/features/pages/audio.rs` returns nothing.

## Platform & Wasm Gating

- `src/services/audio/output.rs`: every item `#[cfg(not(target_family = "wasm"))]`; `engine.rs` has four cfg points (`session` field, `prepare_render` real body, `load_path`, native bodies of `toggle_play`/`refresh_devices`/`select_device`) with wasm fallbacks that surface `AudioError::Unavailable` through `last_error()`. `transport.rs`, `peaks.rs`, `decode.rs`, `demo.rs`, and the page module compile everywhere.
- Dependencies: `cpal` in `[target.'cfg(not(target_family = "wasm"))'.dependencies]` (`Cargo.toml:310` table). `hound` and `symphonia` in the common table — both pure Rust and wasm32-safe (symphonia README: "100% safe Rust"; no `cpal`-style platform backend). If wasm binary size becomes a concern, the fallback is moving both to the not-wasm table plus a `decode_bytes` wasm stub returning `Unavailable` (noted in Risks).
- cpal's own wasm backend (`wasm-bindgen` feature) is deliberately NOT wired: it would need web-sys feature additions and an audioworklet story outside this plan's scope. The page therefore degrades rather than half-plays.
- Capability registry: `"audio_playback"` — native `supported_enabled`/`degraded("no output device found")`; wasm `supported: false` with reason (the "hidden with capability reason" posture: the page stays navigable and renders the notice instead of vanishing from the sidebar, because `Page` is not platform-gated in this repo).
- Platform behaviors: macOS/Windows/Linux identical through cpal's default host; Linux links ALSA (`alsa` crate enters the graph via cpal) — `libasound2-dev` is required to build/test on Linux; plan 18's `scripts/install-linux-deps.sh` row carries it for CI and this plan only adds it when landing first (`18-microphone.md:414-416`). `desktop_actions::pick_file` is native-only (`desktop_actions.rs:194-201`); the open-file button is cfg'd out on wasm. `just wasm-check` must stay green.
- No `std::time::Instant` anywhere; all timers via `background_executor().timer` (wasm-safe, `system_monitor.rs:217-230` precedent).

## i18n

Catalog facade: `crate::i18n::localize` / `localize_with_args` (`src/services/i18n.rs:3-13`); keys must land in BOTH `locales/en.yml` and `locales/zh-CN.yml` in lockstep (`tests/qa_localization.rs:188-198`). Grammar: `snake_case:` double-quoted scalars, one nesting level max.

| key | en | zh-CN |
|---|---|---|
| `nav_audio` | "Audio" | "音频" |
| `audio_title` | "Audio Playback" | "音频播放" |
| `audio_subtitle` | "Load a file or a demo tone, scrub the waveform, play through the selected device" | "加载文件或演示音频，拖动波形定位，通过所选设备播放" |
| `audio_open_file` | "Open file…" | "打开文件…" |
| `audio_demo_sweep` | "Sine sweep" | "正弦扫频" |
| `audio_demo_chords` | "Chords" | "和弦" |
| `audio_empty_title` | "No audio loaded" | "未加载音频" |
| `audio_empty_description` | "Open a WAV, FLAC, MP3, or OGG file, or generate a demo tone." | "打开 WAV、FLAC、MP3 或 OGG 文件，或生成演示音频。" |
| `audio_play` | "Play" | "播放" |
| `audio_pause` | "Pause" | "暂停" |
| `audio_stop` | "Stop" | "停止" |
| `audio_volume` | "Volume" | "音量" |
| `audio_seek_label` | "Seek" | "定位" |
| `audio_device` | "Output device" | "输出设备" |
| `audio_device_refresh` | "Refresh devices" | "刷新设备" |
| `audio_spectrum` | "Spectrum" | "频谱" |
| `audio_live_title` | "Live output" | "实时输出" |
| `audio_status_playing` | "Playing" | "正在播放" |
| `audio_status_paused` | "Paused" | "已暂停" |
| `audio_status_stopped` | "Stopped" | "已停止" |
| `audio_status_loading` | "Loading…" | "正在加载…" |
| `audio_unsupported_title` | "Playback unavailable on this platform" | "此平台不支持播放" |
| `audio_unsupported_description` | "Audio output needs the native cpal backend; the web build shows this page read-only." | "音频输出需要原生 cpal 后端；网页构建以只读方式显示此页面。" |
| `audio_error_decode` | "Couldn't decode the file: %{error}" | "无法解码文件：%{error}" |
| `audio_error_output` | "Audio output failed: %{error}" | "音频输出失败：%{error}" |
| `audio_meta_format` | "%{channels} ch · %{rate} Hz · %{duration}" | "%{channels} 声道 · %{rate} Hz · %{duration}" |

Note: the localization scanner only sees literal first arguments (`seams.md` §5) — every key above is called with a literal, so all are guard-covered.

## Design Compliance

- **Task-first layout** (guides "Start from the task"): the overview + transport are the primary surface (design thesis 1); load actions sit above as one row; the live monitor is a titled secondary section. Primary action = Play, a default ghost icon button with tooltip + accessible name — not primary-styled ("Button means application action": a lone frequent action is not automatically primary).
- **Tokens** (Color and themes): played waveform `cx.theme().primary`, unplayed `muted_foreground`, playhead `foreground`, borders `border`, radii `cx.theme().radius`; no raw hex/rgb/hsla anywhere. Opacity layering (0.9/0.45) follows the repo chart precedent (`system_monitor.rs:291` via plan 16).
- **Fixed-pixel exceptions** (audited classes only): overview bars and playhead snap to device pixels (`1 / scale_factor` hairline + raster geometry class); everything else uses rem helpers (`p_6`, `gap_4`, `h_24`, `w_32`, `text_sm`).
- **Density** (Density tiers): medium default; the transport row uses component sizes as shipped (no custom heights). Compact spacing (`gap_2`) inside the transport row — one control group.
- **Interaction states** (Interaction states table): rest/hover/pressed/focus from kit Buttons; `disabled` for every transport control while unloaded/loading; loading state (`Loading…` badge + disabled actions) prevents duplicate loads; error state inline `Alert` near the action that caused it; the overview shows a persistent playhead (selection-equivalent state) and a pressed/open n/a. Status never color-only: status text label beside the playhead.
- **Pointer conventions**: default arrow cursor on the overview; drag = seek (slider-class interaction, kit slider precedent); no hover-revealed actions.
- **a11y checklist**: overview exposes `Role::Slider` + label + min/max/value and a Click action; time readout is a polite live region; icon-only buttons carry `accessibility_label` + tooltip; heading levels 1/2; the live monitor's own content is plan 16's documented limitation (no a11y node, `16-waveform-foundation.md` Design Compliance).
- **Motion**: no decorative animation; playhead movement is data-driven redraw; reduced-motion unaffected.
- **Interface language**: sentence case labels, verbs for commands, ellipsis on `Open file…` (opens a dialog), no terminal punctuation on control labels, full-width punctuation in zh-CN sentences.
- **Feedback surfaces** (Feedback and overlays): inline Alert for errors (smallest fitting surface), Empty states for unloaded/unsupported; no dialogs, no notifications.

## Verification

Gates, in order (AGENTS.md; `justfile` equivalents in parentheses):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features        # CI adds -D warnings
cargo test --workspace --all-features          # (just test)
just wasm-check
```

Feature-targeted automated coverage wired in the phases:

```sh
cargo test -p gpui-starter --lib services::audio
cargo test -p gpui-starter --lib features::pages::audio
cargo test --test e2e_navigation
```

covering transport atomics, peak decimation, WAV roundtrip + normalization + error inputs, symphonia path + routing, demo generators, config pick + fill callback + resample, engine load/tick/seek/volume/meter with deterministic clocks, and page render smoke.

Mechanical checks: `grep -rn "use gpui::" src/services/audio/ src/features/pages/audio.rs` returns nothing; `git diff Cargo.lock` in the implementation commit contains cpal/hound/symphonia (+ symphonia-core, alsa on Linux) additions.

Manual QA (native, scratch tree, never committed):

1. Generate sine sweep → plays, overview peaks appear before play, playhead advances, live oscilloscope shows the sweep brightening toward the treble end over time.
2. Chords demo → spectrum toggle peaks at the chord fundamentals (261/329/392 Hz ±bar band).
3. Open a real WAV, FLAC, MP3, and OGG file; each shows correct duration/meta; scrub by click and drag (playhead follows while paused and while playing); seek during playback is click-free enough for v1 (volume-change clicks noted in Risks).
4. Output-device selector: switch devices mid-playback — playback continues at the same position on the new device; refresh after unplugging a device degrades with a reason.
5. Volume slider live-scales output; 0 = silence.
6. Space bar toggles play/pause when focus is on the page/overview; Tab reaches the overview slider; focused kit buttons keep their own space activation.
7. EOF: playback stops at the end, status Stopped, pressing play restarts from 0.
8. 2× display: overview bars and playhead are one physical pixel, no seams; light + dark themes both legible.
9. On a machine with no audio device (or `pw-cli`/null sink removed): load works, play surfaces "no output device", app does not panic.
10. Wasm build (`wasm/build.sh` dev loop): Audio page shows the unsupported notice; navigation/deep link `gpui-starter://audio` and `#/audio` still resolve.

Acceptance criteria: all four gates pass; targeted tests pass; manual checklist items 1–8 confirmed on a native machine with an output device, 9 on a device-less machine or waived with a note, 10 on the wasm harness.

## Risks & Open Questions

Ranked risks:

1. **Plan 16 is contract-only.** `src/ui/mod.rs:1` has no `waveform` module (verified). Steps 4.2 and 7.5 hard-depend on it; if plan 16 has not landed, implementation must land plan 16 Phases 1–4 (state) and 6 (element) first. Tried: `ls src/ui/`, read the file.
2. **cpal's ALSA link on CI is conditional on landing order.** cpal enters the Linux graph via the `alsa` crate; `Cargo.lock` has no alsa today (`grep -c '^name = "cpal"' Cargo.lock` → 0, run this session) and `.github/workflows/ci.yml` itself installs nothing — the Linux lanes run `scripts/install-linux-deps.sh` (PKGS at `:14`), and plan 18's seam row 2 already adds `libasound2-dev` there (`18-microphone.md:414-416`). This plan carries the same row conditionally (Seam Touches 2b); if BOTH land without anyone applying it, the CI check/test lanes fail to link. Not verifiable from this sandbox (no CI run).
3. **symphonia is MPL-2.0** (LICENSE in the downloaded tarball; repo is MIT). File-level copyleft for the symphonia sources; acceptable for an app (not a library relicensing), but it must be recorded in any license audit. The spec mandates symphonia, so the alternative (rodio-only) is out of scope.
4. **symphonia 0.6.1 API churn.** 0.6 reorganized modules (`formats::probe::Hint`, `make_audio_decoder`, `GenericAudioBufferRef::copy_to_vec_interleaved`, no `SampleBuffer`); most public examples online describe 0.5. All claims here come from the 0.6.1 tarball sources read this session; upstream GETTING_STARTED.md line 95 still shows a stale import path. Implementers must not copy 0.5 snippets.
5. **Memory: decoded source is retained for device re-planning.** `LoadState` keeps `Arc<DecodedAudio>` (source rate) plus the render buffer (device rate) — a 3-minute 48 kHz stereo file ≈ 2 × 69 MB peak during resample. If that matters, drop the source after prepare and re-decode on device switch; deferred (v1 targets demo-length audio).
6. **Volume changes apply per-fill, not ramped** — a step change can click. Fix later inside `fill_buffer` (per-sample linear ramp from the previous volume); no API change.
7. **Space vs focused Button.** A focused kit Button may consume space as its own activation; the page binding then does not fire. Accepted (standard desktop behavior); if double-toggles are observed, the fix is stopping propagation in the button path or binding on a narrower context.
8. **Device selection by name.** Duplicate device names would make `select_device` ambiguous; cpal `DeviceId` exists for stable identity (Display/FromStr string form — plan 18 already keys its input devices that way, `18-microphone.md:283`) but adds a cfg-crossing type to this contract. v1 matches by name; adopting 18's id-string convention is the natural follow-up if duplicates appear.
9. **CI executes `cargo test -p gpui-starter --lib`** (`ci.yml:245,285`) — engine tests are device-guarded, but cpal still initializes its host during enumeration; if any host panics headless (ALSA missing at runtime), the guard must also skip enumeration (wrap in `std::panic::catch_unwind`? no — prefer a lazy enumeration only on user action, keeping `initialize()` device-probe failure non-fatal via `degraded`). Watch on first CI run.
10. **SelectState has no runtime item mutation** — the page recreates the entity when the device list changes; focus during a rebuild blips once. Acceptable; revisit if it annoys.

Open questions (each with what was tried):

- **hound API verification remains docs-only.** hound is in no cargo registry dir on this machine (`ls ~/.cargo/registry/src/*/ | grep -i hound` → no hits), no local tarball exists, and the review's docs.rs fetch failed; `WavWriter::finalize`/`into_inner` and `WavReader::samples`/`spec`/`duration` claims rest on this session's docs.rs fetches. First compile is the verification point; adapt mechanically if a signature differs (both are documented public items).
- **Page-count seam ordering vs plans 13/18.** All three append a page and bump `tests/e2e_navigation.rs`. Decision procedure in Seam Touches row 17: read the current assert, add 1. Tried: no merge-order signal on disk.
- **Whether `Default` buffer size on every cpal host stays interactive.** BufferSize::Default is host-chosen (docs verified for the type; per-host latencies are not documented). If a host picks a huge buffer, switching to `Fixed(1024)` where supported is a local change in `plan_stream`.
- **No gates were run during planning** (doc-only scope; `cargo`/`just` invocations quoted from `AGENTS.md` and `justfile`, not executed here). External-crate claims rest on the docs.rs fetches and the cpal/symphonia tarball reads cited in Research Digest.
