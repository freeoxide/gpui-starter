# 18 Microphone Capture Panel Plan

## Goal

A Microphone page lists the machine's active input devices (cpal), remembers the
chosen device and gain in AppConfig, and on Start feeds live captured audio
through plan 16's waveform widget: a 30 Hz-refreshed level meter (RMS + peak
dBFS), a scrolling oscilloscope/spectrum, and a digital gain slider. macOS shows
a runtime permission state with request/denied guidance; the web build shows the
same UI driven by a synthetic demo signal instead of real capture.

## Dependencies

- **Plan 16 (`docs/plans/16-waveform-foundation.md`)**: consumes from its
  Interface Contract: `SampleProducer` (`push_interleaved`, `is_attached`,
  `dropped_samples`), `WaveformState` (`attach_source`, `detach_source`,
  `set_mode`, `dropped_samples`, readers), `Waveform::new(id, entity)`
  element, `WaveformMode`, `DEFAULT_RETENTION`. The 33 ms tick inside
  `WaveformState` is this feature's bounded UI refresh rate. Plan 16 is NOT yet
  implemented (`ls src/ui/` shows only `widgets/`); its phases must land first.
- Survey inputs: `docs/plans/_survey/seams.md` sections 1 (page seam), 3
  (threading rule), 4 (events/config/capability recipes), 5 (i18n), 6 (wasm),
  7 (tests), 8 (gates), 9 (rules); `docs/plans/_survey/ecosystem.md` section h
  (cpal). Every load-bearing claim re-verified this session (see Research
  Digest).
- **Plan 20 (`docs/plans/20-release-bundling.md`, on disk)**: coordinated,
  not consumed. Spec requires coordinating `NSMicrophoneUsageDescription`
  with it. Split of ownership: plan 18 pins the string copy ("Captures
  microphone audio to display the live input level and waveform.") and lands
  it in the dev bundle (`scripts/macos-dev-app.sh`); plan 20's Step 1.2
  inserts the same byte-identical string into `scripts/create-release-app.sh`
  grep-guarded, so merge order cannot produce a duplicate key
  (`20-release-bundling.md:149-160`, Interface Contract item 2 at `:95`).
  Plan 18 therefore never edits `create-release-app.sh`.
- **Plan 19 (`docs/plans/19-audio-playback.md`, on disk)**: peer, not
  consumed. It owns the output side (Audio page, decode+output engine in
  `src/services/audio/`, `19-audio-playback.md:5`) and explicitly declines
  reusing this plan's input-device surface (`:14`). Two shared seams it names:
  exactly one `cpal = "0.18"` entry in the not-wasm table (whichever plan
  lands second must not duplicate the key, `:14`), and this plan's
  `libasound2-dev` CI dependency (`:14,31`).

## Research Digest

Path key (all read this session):

| Short | Path |
| --- | --- |
| `CPAL` | `/tmp/cpal-0.18.2` (crate downloaded from crates.io API this session; docs.rs pages fetched where cited) |
| `AVF` | `/tmp/objc2-av-foundation-0.3.2` (crate downloaded from crates.io API this session) |
| `GPUI` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-0.3.8/src` |
| `COMP` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-component-0.7.1/src` |
| `GB` | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-base-0.7.1/src` |
| `REPO` | `/home/thragg/fo/gpui-starter` |

### cpal 0.18.2 (crate source + docs.rs, fetched 2026-10-12)

- Crate root re-exports: `default_host`, `available_hosts`, `Device`, `Host`,
  `Stream`, `SampleFormat`, `FromSample`/`Sample` (`CPAL/src/lib.rs:186-194`).
  `default = []` features; per-OS primary host without any feature flag
  (`CPAL/Cargo.toml:85`).
- `HostTrait`: `devices(&self) -> Result<Self::Devices, Error>` where
  `Devices: Iterator<Item = Self::Device>`; `default_input_device(&self) ->
  Option<Self::Device>`; provided `input_devices()` filter
  (`CPAL/src/traits.rs`, docs.rs `traits` source fetch).
- `DeviceTrait`: `PartialEq + Eq + Hash + Debug + Display` supertrait;
  `description() -> Result<DeviceDescription, Error>`; `id() -> Result<DeviceId,
  Error>` ("uniquely identifies the device on the host", stable across runs);
  `default_input_config() -> Result<SupportedStreamConfig, Error>`;
  `build_input_stream<T, D, E>(&self, config: StreamConfig, data_callback: D,
  error_callback: E, timeout: Option<Duration>) -> Result<Self::Stream, Error>`
  where `T: SizedSample`, `D: FnMut(&[T], &InputCallbackInfo) + Send + 'static`,
  `E: FnMut(Error) + Send + 'static` (docs.rs `src/cpal/traits.rs.html` fetch).
- `DeviceId` is opaque with `Display` (`"wasapi:device_identifier"`,
  `"alsa:hw:0,0"` style) and `FromStr`; stable across restarts; no serde: docs
  say "can be serialized using Display/FromStr" (docs.rs
  `struct.DeviceId.html` fetch). Persisted device key = `id().to_string()`.
- `DeviceDescription::name() -> &str` always present (docs.rs
  `device_description/struct.DeviceDescription.html` fetch); fallback display
  name = `device.to_string()` (Display supertrait).
- `SupportedStreamConfig` accessors `channels()/sample_rate()/sample_format()`
  and `config() -> StreamConfig` (`CPAL/src/lib.rs:531-571`);
  `StreamConfig { channels: ChannelCount, sample_rate: SampleRate,
  buffer_size: BufferSize }` pub fields, `SampleRate = u32` type alias
  (`CPAL/src/lib.rs:223,452-456`), `BufferSize::{Default, Fixed}`
  (`CPAL/src/lib.rs:388-392`). `SampleFormat` variants include `I8, I16, I24,
  I32, U16, F32, F64, DsdU16` (`CPAL/src/sample_format.rs:66-110`).
- Stream starts paused; `StreamTrait::play()/pause()` (docs.rs index fetch).
  `Stream` is `Send` + `Sync`: compile-time asserted per host: coreaudio
  (`CPAL/src/host/coreaudio/mod.rs:141-143`), wasapi
  (`CPAL/src/host/wasapi/stream.rs:239-243`), alsa
  (`CPAL/src/host/alsa/mod.rs:773`), jack (`CPAL/src/host/jack/stream.rs:45`);
  assertion macro at `CPAL/src/traits.rs:520-525`.
- **No per-device volume/gain API.** `DeviceTrait` exposes only the methods
  listed above; `StreamTrait` only `play/pause/buffer_size/now` (docs.rs
  `src/cpal/traits.rs.html` fetch; survey section h agrees). Hardware gain via
  cpal is impossible; and cpal's `Device` exposes no handle to the OS device
  object, so mapping a cpal device onto a CoreAudio HAL / WASAPI endpoint for
  platform-side gain would require re-implementing enumeration outside cpal.
  Consequence (spec's fallback branch): digital gain applied in the data
  callback + an explicit "hardware gain not supported" state; platform-API
  hardware gain is a documented follow-up, not v1.
- Linux build cost: cpal's ALSA backend needs `libasound2-dev` headers at build
  time. This Linux box has only the runtime lib (`dpkg -l | grep libasound` →
  `libasound2t64`, no `-dev`). CI's Linux lanes already run
  `./scripts/install-linux-deps.sh`
  (`.github/workflows/ci.yml:120,147,225`), so one package-list edit covers
  CI and local dev.

### macOS permission (crate source, downloaded this session)

- `objc2-av-foundation` 0.3.2 (updated 2025-10-04, Zlib OR Apache-2.0 OR
  MIT, the same license family as the repo's `objc2-app-kit` entry). Deps:
  `objc2 >=0.6.2, <0.8.0` (repo lock: 0.6.4), `block2 >=0.6.1, <0.8.0` (repo
  macOS table: `block2 = "0.6"`), `objc2-foundation 0.3.2` (already in lock), so
  no second copy of any binding family (`AVF/Cargo.toml:854-874,942-946`).
- `AVCaptureDevice::authorizationStatusForMediaType(media_type: &AVMediaType)
  -> AVAuthorizationStatus`: `unsafe`, synchronous, features `AVMediaFormat`
  (`AVF/src/generated/AVCaptureDevice.rs:2363-2367`).
- `AVCaptureDevice::requestAccessForMediaType_completionHandler(media_type:
  &AVMediaType, handler: &block2::DynBlock<dyn Fn(Bool)>)`: `unsafe`,
  features `AVMediaFormat` + `block2` (`AVF/src/generated/AVCaptureDevice.rs:2387-2392`).
- `pub type AVMediaType = NSString` and `pub static AVMediaTypeAudio:
  Option<&'static AVMediaType>` (`AVF/src/generated/AVMediaFormat.rs:9,17-19`).
- `AVAuthorizationStatus(pub NSInteger)` with associated consts
  `NotDetermined`, `Restricted`, `Denied`, `Authorized`; `Copy + PartialEq`
  (docs.rs `struct.AVAuthorizationStatus.html` fetch). Feature
  `AVCaptureDevice` pulls `objc2-foundation` features NSArray/NSData/NSError/
  NSNotification/NSSet/NSString/NSValue (`AVF/Cargo.toml:271-280`): feature
  union onto the existing objc2-foundation 0.3.2 build; Cargo.lock gains the
  one new package.
- In-repo precedent for the whole async-objective-C dance (RcBlock + oneshot +
  SAFETY comment): `REPO/src/services/notifications/backend/user_notify.rs:262-304`.
  Bundled-runtime guard (NSBundle identifier; raw `cargo run` binaries lack
  Info.plist): `user_notify.rs:236-254`. macOS System Settings opener pattern:
  `REPO/src/services/notifications/service/api.rs:290-304`.
- Missing `NSMicrophoneUsageDescription` makes macOS abort the process when TCC
  would prompt (same class of gate as the bundle check above); current plists
  have no such key (`scripts/macos-dev-app.sh:19-44`,
  `scripts/create-release-app.sh:57+`).

### GPUI / kit facts used

- `App` implements `background_spawn` (`GPUI/app.rs` `impl AppContext for App`,
  `fn background_spawn<R>(&self, future: impl Future<Output = R> + Send +
  'static) -> Task<R> where R: Send + 'static`; repo usage
  `src/gallery/sections/tables/data_table.rs:1102`). `cx.spawn` + `cx.update`
  loop pattern: `src/services/notifications/service/api.rs:95-108,176-208`.
- `Context::observe_global_in<G: Global>(window, FnMut(&mut T, &mut Window,
  &mut Context<T>))`: window is available in the callback
  (`GPUI/app/context.rs:700-721`; repo usage
  `src/features/pages/notifications.rs:13-17`).
- Global + snapshot + `update_global` + `refresh_windows` service pattern:
  `src/services/notifications/service/api.rs:379-384`; capability struct
  literal `src/services/notifications/service/api.rs:32-42`.
- Select: `SelectState<D: SearchableListDelegate>` with `new(delegate,
  selected_index: Option<IndexPath>, window, cx)`, `set_items(&mut self, items:
  D, &mut Window, &mut Context<Self>)`, `selected_value()`; emits
  `SelectEvent::Confirm(Option<Value>)` (`COMP/src/select.rs:152-172,216,357,372`;
  event enum `select.rs:71-74`). Two distinct traits with back-compat aliases
  in `COMP/src/select.rs:29-33`: `SearchableListDelegate` (aliased
  `SelectDelegate`, `select.rs:31`) is the Vec-level delegate trait, and
  `SearchableListItem` (aliased `SelectItem`, `select.rs:33`) is the per-item
  trait. `Vec<T: SearchableListItem>` implements the delegate
  (`COMP/src/searchable_list/vec.rs:47-49`); the item trait needs
  `Clone`, `type Value: Clone + PartialEq`, `title() -> SharedString`,
  `value() -> &Self::Value` (`COMP/src/searchable_list/delegate.rs:8-38`).
  So a `SelectState<Vec<DeviceOption>>` requires `DeviceOption:
  SearchableListItem`: impl the item trait, never `SelectDelegate`.
  `IndexPath::new(row)` with pub fields (`GB/src/index_path.rs:29-36`).
  Builder: `Select::new(&entity).placeholder(..).accessibility_label(..).disabled(bool)`
  (`COMP/src/select.rs:649-720`).
- Slider: `SliderState::new().min(f32).max(f32).step(f32).default_value(f32)`;
  events `SliderEvent::Change(SliderValue)` / `Release`; `SliderValue::start()`
  for the single-value case; `set_value(impl Into<SliderValue>, &mut Window,
  &mut Context<Self>)` (`GB/src/slider.rs:15-23,273-285`; repo usage
  `src/gallery/sections/controls/slider.rs:39-46`,
  `src/gallery/sections/text/text_view.rs:84-95`). Component:
  `Slider::new(&Entity<SliderState>)` (`COMP/src/slider.rs:96`).
- Progress: `Progress::new(id).value(f32 /* 0..100, clamped */)
  .accessibility_label(..)` (`COMP/src/progress/progress.rs:26-67`).
- Layout sizing: `Styled` is implemented for `Select` (`COMP/src/select.rs:795`),
  `Slider` (`COMP/src/slider.rs:139`), and `Progress`
  (`COMP/src/progress/progress.rs:68`): `.flex_1()` and width helpers apply
  to all three.
- Alert: `Alert::new(id, message).title(..)` (`COMP/src/alert.rs:74,130`) with
  variant constructors `.info(id, message)` (`:90`), `.warning(id, message)`
  (`:104`), `.error(id, message)` (`:111`).
- Switch: `Switch::new(id).checked(bool).on_click(|&bool, _, cx|)` (repo
  usage `src/features/pages/settings/mod.rs:114`).
- Icons exist as svg files (IconName is generated from filenames):
  `mic.svg`, `mic-off.svg`, `refresh-cw.svg`, `audio-waveform.svg` under
  `gpui-kit-assets-0.7.1/default-icons.txt` (the default 106 the app's
  native asset source actually embeds — full-catalog variants render blank)
  → `IconName::Mic` and `IconName::RefreshCw` are usable; `MicOff` and
  `AudioWaveform` are full-catalog only and are NOT used.
- Page/config/i18n/capability seams (seams.md sections 1, 4, 5), spot-verified:
  `src/shell/sidebar.rs` (10 pages, exhaustive matches), `src/shell/route.rs:10-21`
  (`VALID_HOSTS`), `src/shell/root/app_root/state.rs:22-31,47-56,211-223`,
  `tests/e2e_navigation.rs:14-19` (count pinned at 10),
  `src/state/config_store.rs:66-90` (schema comment; adds need
  `#[serde(default)]`), `src/runtime/capabilities.rs:64-79`,
  `src/app/init.rs:202-263` (runtime_services_init block).
- Snapshot tests pin the config schema: default inline yaml
  `tests/snapshot_tests.rs:21-46` and an exhaustive struct literal + inline yaml
  in the round-trip test (`:57-79,88-119`): both need the new fields. The
  round-trip test already persists `"microphone"` in `granted_permissions`
  (`:73`); this plan does not write that field (TCC persists permission
  system-side; duplicating it invites drift).
- `#[gpui_kit::test]` + `TestAppContext` + `cx.add_window` availability:
  `src/state/config_store.test.rs:280-295`, plan 16 Research Digest;
  `TestAppContext::add_window` (`GPUI/app/test_context.rs:279-296`, cited by
  plan 16, re-checked there this session via plan text).
- qa_migration kit-facade rule (`tests/qa_migration.rs:386-405`): no `use
  gpui::` anywhere in new code; flume is already an unconditional dependency
  (`Cargo.toml:208`, `flume = "0.11"`).

### Crate picks

| Crate | Version | Table | Why |
| --- | --- | --- | --- |
| `cpal` | `0.18.2` | `[target.'cfg(not(target_family = "wasm"))'.dependencies]` | The RustAudio standard device-enumeration + stream crate; verified API above; no wasm build in this feature (synthetic demo instead), so the not-wasm table is correct per seams §6 |
| `objc2-av-foundation` | `0.3.2`, `default-features = false`, features `["std", "AVCaptureDevice", "AVMediaFormat", "block2"]` | `[target.'cfg(target_os = "macos")'.dependencies]` | Only the authorization surface is needed; pairs with the locked objc2 0.6.4 / block2 0.6 / objc2-foundation 0.3.2 line (versions verified above) |

Not run during planning (doc-only scope): every `cargo` gate, any cpal/AVF
compile. The Verification section names the exact commands for the implementer.

## Architecture

Entities and ownership:

- **`MicrophoneState`** (GPUI `Global`, `src/services/microphone/controller.rs`)
  owns the feature's retained state: a `MicrophoneSnapshot` (plain data +
  `Entity<WaveformState>`), an `Arc<LevelMeter>`, an `Arc<AtomicI32>` gain
  cell (dB × 256), and the live session handle. Mutated only through
  controller functions; every mutation ends with `cx.refresh_windows()` (the
  notifications-service pattern, `api.rs:379-384`). Initialized once at boot.
- **`Entity<WaveformState>`** (plan 16): one instance, created in
  `initialize`, stored in the snapshot. `attach_source` is called on the main
  thread inside `start`; the returned `SampleProducer` moves to the capture
  side (cpal callback thread or demo task). Stop calls `detach_source`.
- **Capture session (native)**: `cx.spawn` main-thread loop + a
  `cx.background_spawn` runner. The runner enumerates, matches the persisted
  device id, builds and plays the cpal stream on the background executor, then
  parks on a flume command receiver. The cpal callback owns the producer and
  the meter arc; it never touches GPUI contexts (seams §3 threading rule).
  Events flow back over flume (`Started` / `Failed` / `Stopped`) into the main
  loop, which updates snapshot status. Dropping the session task cancels the
  loop, dropping the runner `Task` drops the stream (cpal streams pause on
  drop, `#[must_use]` on `Stream`, `CPAL/src/platform/mod.rs:94`).
- **Demo session (wasm)**: a `cx.spawn` task ticking every 33 ms,
  synthesizing 1584 stereo frames at 48 kHz into the producer and updating the
  meter. Pure `DemoSignal` math is shared with native tests.
- **Data flow (native)**: cpal audio thread → per-callback convert → first
  min(channels, 2) channels → gain multiply → `SampleProducer::push_interleaved`
  (never blocks; overflow counted by plan 16) + `LevelMeter::update(rms, peak)`.
  UI side: plan 16's 33 ms tick drains the ring and notifies; the page observes
  the waveform entity and re-renders, reading `LevelMeter` atomics and
  `WaveformState::dropped_samples()` during render. UI refresh is therefore
  bounded at ~30 Hz with no extra timer.
- **Gain**: `set_gain_db` clamps to [-24, +24], persists to AppConfig
  (debounced), stores dB×256 into the `AtomicI32`; the callback reads it per
  invocation: no session restart on gain change.
- **Permission (macOS)**: synchronous status query at page-relevant moments
  (`AVCaptureDevice::authorizationStatusForMediaType`); async request via
  `requestAccessForMediaType_completionHandler` + `RcBlock` + tokio oneshot
  (mirroring `user_notify.rs:262-304`). Both paths are gated on the
  bundle-identifier check first: without `NSMicrophoneUsageDescription` the
  process aborts when TCC prompts. Other native platforms: status is
  `Authorized` (no queryable permission system); failures surface as capture
  errors. Wasm: `Unsupported`, section hidden.
- **Persistence**: `AppConfig.mic_device_id` (`DeviceId::to_string()` form) +
  `AppConfig.mic_gain_db`; written via `update_config`, read at initialize and
  on page render. Device resolution on start: persisted id → default device →
  first device; a stale id (unplugged) falls back the same way and re-persists.

Page placement decision (spec: "own Audio-page section or its own page"):
own `Microphone` page. There is no Audio page to join: plan 19 owns that
page and makes it output-only; its Goal is an Audio page for file playback with
an output-device selector and volume slider (`19-audio-playback.md:5`), and
its dependency note explicitly treats 18 as the capture-side peer with a
separate page and declines reusing this plan's input surface
(`19-audio-playback.md:14`). Merging a capture section into 19's playback
page would couple two removal seams and two service modules
(`services/microphone` vs `services/audio`) for no shared state beyond plan
16's widget and the cpal dependency. The variant is appended last in
`Page::all()` so no digit-key rebinds occur (Seam Touches, keys.rs note).

Module tree:

```text
src/services/microphone/
├── mod.rs         seam: header + module decls + re-exports
├── types.rs       pure types: InputDeviceInfo, CaptureStatus,
│                  MicrophonePermissionState (+label), DeviceOption, dbfs()
├── meter.rs       LevelMeter: lock-free rms/peak atomics (callback → UI)
├── demo.rs        DemoSignal synthesis + start_demo task body (all targets)
├── permission.rs  per-OS runtime permission: AVFoundation on macOS
├── capture.rs     native-only cpal session: enumerate, build, callback
├── controller.rs  MicrophoneState global + public API + persistence
└── *.test.rs      sibling test modules

src/features/pages/microphone.rs   MicrophonePage (UI)
```

## Interface Contract

Public surface (re-exported from `src/services/microphone/mod.rs`; consumed by
the page. Plan 19 was the other candidate consumer and declines input-device
reuse in favor of its output-only surface, `19-audio-playback.md:14`).
Behavioral structs keep private fields.

```rust
// types.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputDeviceInfo {
    pub id: String,            // cpal DeviceId Display form, stable across runs
    pub name: String,          // DeviceDescription::name() fallback device.to_string()
    pub is_default: bool,
    pub channels: u16,         // device default input config
    pub sample_rate: u32,
    pub sample_format: String, // Debug form of cpal SampleFormat, display-only
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaptureStatus {
    Idle,
    Starting,
    Capturing { device: String },
    Error(String),             // already localized at the call site
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MicrophonePermissionState {
    Unknown,
    NotDetermined,
    Authorized,
    Denied,
    Restricted,
    Unsupported,               // non-macOS native + wasm
    Unavailable(String),       // e.g. raw binary without a bundle identifier
}
impl MicrophonePermissionState {
    pub fn label(&self) -> SharedString;   // localized, keys below
}

/// Select dropdown item; `Value` is the device id so Confirm events carry ids.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceOption { pub id: String, pub name: String }
impl SearchableListItem for DeviceOption {   // importable as gpui_kit::component::select::SelectItem
    type Value = String;
    fn title(&self) -> SharedString;   // name
    fn value(&self) -> &String;        // id
}

/// "-12.3 dB" / "-∞ dB" for a 0..=1 amplitude. Pure, testable.
pub fn dbfs(amplitude: f32) -> String;

// meter.rs
/// Lock-free level meter shared between the audio side and render. rms is the
/// most recent callback's RMS; peak accumulates via fetch_max until read.
pub struct LevelMeter { /* private atomics */ }
impl LevelMeter {
    pub fn new() -> Self;
    /// Audio-side write; amplitudes clamped to 0..=1.
    pub fn update(&self, rms: f32, peak: f32);
    /// UI-side read: (rms, peak since last read); resets the peak.
    pub fn read(&self) -> (f32, f32);
    pub fn reset(&self);
}

// demo.rs: pure synthesis, compiled on every target
pub struct DemoSignal { /* private phase state */ }
impl DemoSignal {
    pub fn new() -> Self;
    /// Appends `frames` stereo interleaved f32 frames at 48 kHz to `out` and
    /// returns (rms, peak) of the chunk.
    pub fn fill_chunk(&mut self, out: &mut Vec<f32>, frames: usize) -> (f32, f32);
}

// controller.rs: the API consumed by the page
pub struct MicrophoneSnapshot {           // plain Clone snapshot
    pub devices: Vec<InputDeviceInfo>,
    pub selected_device: Option<String>,  // device id
    pub status: CaptureStatus,
    pub permission: MicrophonePermissionState,
    pub gain_db: f32,
    pub demo_only: bool,                  // true on wasm
    pub waveform: Entity<WaveformState>,
}

pub fn initialize(cx: &mut App);          // boot: global, capability, config load
pub fn snapshot(cx: &App) -> MicrophoneSnapshot;
pub fn meter(cx: &App) -> std::sync::Arc<LevelMeter>;
pub fn refresh_devices(cx: &mut App);     // native: background enumerate; wasm: no-op
pub fn select_device(id: Option<String>, cx: &mut App);   // persists
pub fn set_gain_db(db: f32, cx: &mut App);                // clamps [-24, 24], persists
pub fn refresh_permission(cx: &mut App);
pub fn request_permission(cx: &mut App);  // macOS only; no-op elsewhere
pub fn start(cx: &mut App);               // wasm: demo; native: session (permission-gated)
pub fn stop(cx: &mut App);
pub fn open_microphone_settings(cx: &mut App); // per-OS privacy pane opener

// permission.rs (exported for controller only; pub(crate) surface):
//   fn status() -> MicrophonePermissionState       (sync, macOS; consts elsewhere)
//   async fn request() -> MicrophonePermissionState (macOS; others Unsupported)
//   fn bundled_runtime() -> Result<(), String>      (macOS NSBundle check)
// capture.rs pub(crate): enumerate(), run_session(...), SessionCommand, SessionEvent
```

Behavioral notes that are part of the contract:

- `start` is idempotent-ish: returns early while `Starting`/`Capturing`. On
  native it requires `permission == Authorized` (macOS): with `NotDetermined`
  it first runs the request and starts only on grant. With no devices it sets
  `CaptureStatus::Error(localize("microphone_no_devices"))`.
- `stop` always runs to completion: signals the session, drops the task,
  calls `detach_source`, resets the meter, sets `Idle`.
- The meter is polled in render (no event); correctness does not depend on
  poll rate. Only the page renders it.
- Nothing in this module emits `AppEventKind` events; the page is the only
  consumer and observes the global + waveform entity.

## Module Layout

| File | Purpose |
|---|---|
| `src/services/microphone/mod.rs` | Seam header with removal recipe, `mod` decls, `pub use` re-exports |
| `src/services/microphone/types.rs` | `InputDeviceInfo`, `CaptureStatus`, `MicrophonePermissionState`, `DeviceOption`, `dbfs()` |
| `src/services/microphone/meter.rs` | `LevelMeter` fixed-point atomic rms/peak |
| `src/services/microphone/demo.rs` | `DemoSignal` synthesis (two sines + LFO + noise) |
| `src/services/microphone/permission.rs` | Per-OS permission status/request + bundle guard + settings opener |
| `src/services/microphone/capture.rs` | cpal enumeration, stream build, audio callback, session runner (cfg not-wasm) |
| `src/services/microphone/controller.rs` | `MicrophoneState` global, public API, persistence, capability updates |
| `src/services/microphone/{types,meter,demo,permission,capture,controller}.test.rs` | Sibling tests (`#[cfg(test)] #[path]` wiring) |
| `src/features/pages/microphone.rs` | `MicrophonePage` UI |

## Seam Touches

Edits outside the new modules:

1. `Cargo.toml`: `cpal = "0.18"` under
   `[target.'cfg(not(target_family = "wasm"))'.dependencies]`, added only if
   the key is not already present (plan 19 adds the identical line; exactly
   one entry may exist, `19-audio-playback.md:14`);
   `objc2-av-foundation = { version = "0.3.2", default-features = false,
   features = ["std", "AVCaptureDevice", "AVMediaFormat", "block2"] }` under
   `[target.'cfg(target_os = "macos")'.dependencies]`. Regenerate + commit
   `Cargo.lock` (qa_migration lock rule).
2. `scripts/install-linux-deps.sh`: add `libasound2-dev` to `PKGS`, only if
   absent (cpal's ALSA backend needs headers; the script already runs in every
   CI Linux lane, `.github/workflows/ci.yml:120,147,225`).
3. `scripts/macos-dev-app.sh`: add `NSMicrophoneUsageDescription` key/value
   to the Info.plist heredoc. This is the only plist this plan touches;
   `scripts/create-release-app.sh` is plan 20's (its Step 1.2 inserts the
   same string there, `20-release-bundling.md:149-160`).
4. `src/services/mod.rs`: `pub mod microphone;`.
5. `src/lib.rs`: add `microphone` to the `pub use services::{...}` list
   (`src/lib.rs:56-68`).
6. `src/app/init.rs`: `crate::microphone::initialize(cx);` immediately after
   `crate::notifications::initialize(cx);` (`init.rs:274`). That call sits
   AFTER the `runtime_services_init` startup_step block (which spans
   `init.rs:202-263`), so the new line goes outside the block too.
7. `src/state/config_store.rs`: two fields on `AppConfig`
   (`mic_device_id: Option<String>`, `mic_gain_db: f32`, both
   `#[serde(default)]`) + `Default` impl arms.
8. `tests/snapshot_tests.rs`: extend both inline yaml snapshots and the
   round-trip struct literal with the new fields.
9. Page seam (15-touch list from seams §1, files):
   `src/features/pages/mod.rs` (mod + re-export), `src/shell/sidebar.rs`
   (variant, `title`, `title_key`, `host`, `icon`, `all`), `src/shell/route.rs`
   (`VALID_HOSTS`), `src/shell/root/app_root/state.rs` (field, construct,
   `unchecked_active_page_view` arm, import), `tests/e2e_navigation.rs`
   (count 10 → 11 + `expected` entry), `locales/en.yml` + `locales/zh-CN.yml`
   (`nav_microphone` + `microphone_*` keys in both).
10. `src/app/keys.rs`: no change. `Page::all()` position 11 (index 10) gets no
    digit shortcut (`state.rs:144-166` binds only `i < 9`), so appending after
    `Gallery` rebinds nothing (seams §1 gotcha).

Deliberate non-touches: no command-palette entry, no gallery section, no
`KNOWN_PERMISSIONS` change (this feature never writes `granted_permissions`),
no `src/platform/` module (all OS code lives in the feature module with cfg
gates, matching `services/notifications`).

### Removal Recipe

`src/services/microphone/mod.rs` opens with (mirroring
`src/gallery/mod.rs:1-7`):

```rust
//! Removable microphone capture feature. The `microphone` service and
//! `MicrophonePage` are the only public surfaces. Removal = delete this
//! directory and src/features/pages/microphone.rs, then revert the seam
//! touches: the pub mod + re-export lines in services/mod.rs and
//! features/pages/mod.rs, the microphone entry in lib.rs's services re-export,
//! the initialize call in app/init.rs, the Page::Microphone arms and all()
//! entry in shell/sidebar.rs, the VALID_HOSTS entry in shell/route.rs, the
//! AppRoot wiring in shell/root/app_root/state.rs, the page count in
//! tests/e2e_navigation.rs, the mic_* fields in state/config_store.rs and both
//! snapshot_tests.rs snapshots, the microphone_* + nav_microphone keys in
//! locales/en.yml and locales/zh-CN.yml, the cpal/objc2-av-foundation lines
//! and regenerated Cargo.lock in Cargo.toml (keep the cpal line if the audio
//! playback feature still uses it), libasound2-dev in
//! scripts/install-linux-deps.sh, and NSMicrophoneUsageDescription in
//! scripts/macos-dev-app.sh (the release plist's copy belongs to the release
//! bundling plan, not this one).
```

Ordered removal steps:

1. Remove `pub mod microphone;` from `src/services/mod.rs` and the
   `microphone` name from `src/lib.rs`'s `pub use services::{...}`.
2. `rm -r src/services/microphone/`.
3. Remove `crate::microphone::initialize(cx);` from `src/app/init.rs`.
4. Remove `mod microphone;` + `pub use microphone::MicrophonePage;` from
   `src/features/pages/mod.rs`; `rm src/features/pages/microphone.rs
   src/features/pages/microphone.test.rs`.
5. Remove the `Page::Microphone` variant and its four match arms (`title`,
   `title_key`, `host`, `icon`) plus the `all()` entry in
   `src/shell/sidebar.rs`; remove `Page::Microphone.host(),` from
   `VALID_HOSTS` in `src/shell/route.rs`.
6. Remove the `microphone_page` field, its construction, its import, and the
   `Page::Microphone` arm in `src/shell/root/app_root/state.rs`.
7. Restore `assert_eq!(all.len(), 10, ...)` and drop `Page::Microphone` from
   `expected` in `tests/e2e_navigation.rs`.
8. Remove `mic_device_id` / `mic_gain_db` from `AppConfig` + `Default` in
   `src/state/config_store.rs` and the corresponding lines from both inline
   snapshots in `tests/snapshot_tests.rs`.
9. Remove `nav_microphone` and every `microphone_*` key from both locale files.
10. Remove the `objc2-av-foundation` entry from `Cargo.toml`, and the `cpal`
    entry only if the audio playback feature (`crate::audio`,
    `19-audio-playback.md`) has not landed; run `cargo check --workspace`,
    commit the regenerated `Cargo.lock`.
11. Revert `libasound2-dev` in `scripts/install-linux-deps.sh` and the
    `NSMicrophoneUsageDescription` key in `scripts/macos-dev-app.sh` (the
    `scripts/create-release-app.sh` copy belongs to the release bundling
    plan; leave it).
12. Run the gates in Verification; grep `microphone` across `src/ tests/
    locales/ scripts/`; only docs/plans/ hits remain.

## Implementation Phases

## Phase 1: Dependencies, config schema, service skeleton

### Step 1.1: Cargo dependencies

- Files: `Cargo.toml`, `Cargo.lock`, `scripts/install-linux-deps.sh`.
- Build: add `cpal = "0.18"` to
  `[target.'cfg(not(target_family = "wasm"))'.dependencies]` (after the
  `tokio` native re-add block) ONLY if the key is absent: plan 19
  (`19-audio-playback.md`) adds the identical line and a duplicated Cargo.toml
  key is a TOML error; add the `objc2-av-foundation` entry with
  `default-features = false, features = ["std", "AVCaptureDevice",
  "AVMediaFormat", "block2"]` to `[target.'cfg(target_os = "macos")'.dependencies]`
  beside the existing `objc2-*` entries; add `libasound2-dev` to the `PKGS`
  array in `scripts/install-linux-deps.sh` (skip when already listed). Run
  `cargo check --workspace` to regenerate the lock.
- Tests: none.
- Done-check: `cargo check --workspace` passes on Linux with
  `libasound2-dev` installed; `git diff Cargo.lock` contains the new `cpal`
  (or no cpal diff when plan 19 landed it first), `objc2-av-foundation`, and
  `alsa-sys` package entries. `objc2-foundation` stays at 0.3.2: its extra
  features are a build-flag union, invisible to Cargo.lock; a version bump
  there would be a finding, not an expectation.
  `grep -n 'libasound2-dev' scripts/install-linux-deps.sh` hits exactly once.

### Step 1.2: AppConfig fields

- Files: `src/state/config_store.rs`, `tests/snapshot_tests.rs`.
- Build: on `AppConfig` add
  `#[serde(default)] pub mic_device_id: Option<String>` and
  `#[serde(default)] pub mic_gain_db: f32`; add `mic_device_id: None,
  mic_gain_db: 0.0` to `Default`. Extend the default inline yaml snapshot with
  `mic_device_id: ~` / `mic_gain_db: 0.0`; in the round-trip test's struct
  literal add `mic_device_id: Some("coreaudio:Built-in Microphone".to_string()),
  mic_gain_db: 4.5` and the matching yaml lines.
- Tests: the two edited snapshot tests are the tests.
- Done-check: `cargo test -p gpui-starter --test snapshot_tests` passes
  (run `cargo insta accept` only if the inline edits are rejected; prefer
  editing the inline bodies by hand as written).

### Step 1.3: Service skeleton, initialize, capability

- Files: `src/services/mod.rs`, `src/lib.rs`, `src/app/init.rs`,
  `src/services/microphone/mod.rs`, `src/services/microphone/types.rs`,
  `src/services/microphone/types.test.rs`.
- Build: `pub mod microphone;` in services/mod.rs; `microphone` added to
  lib.rs's services re-export; `crate::microphone::initialize(cx);` on the
  line after `crate::notifications::initialize(cx);` at `init.rs:274` (that
  call sits after the `runtime_services_init` startup_step block, which ends
  at `init.rs:263`; the new call goes outside the block, next to the other
  feature initializes). `mod.rs` carries
  the removal-recipe header, `mod types;` and a temporary re-export of its
  items (submodules meter/demo/permission/capture/controller get added by
  later phases; declare them as they land). `types.rs` gets
  `InputDeviceInfo`, `CaptureStatus`, `MicrophonePermissionState` with
  `label()` mapping to the i18n keys of the table below, `DeviceOption` with
  the `SearchableListItem` impl: import the trait as
  `use gpui_kit::component::select::SelectItem;` (the back-compat alias at
  `COMP/src/select.rs:33`; the trait is defined at
  `COMP/src/searchable_list/delegate.rs:8` with the exact shape impl'd here:
  `Clone` supertrait, `type Value: Clone + PartialEq`, `title() ->
  SharedString`, `value() -> &Self::Value`. Do NOT use `SelectDelegate`:
  that alias at `select.rs:31` is the Vec-level `SearchableListDelegate`
  delegate trait, a different type), and `dbfs()`
  (`amplitude <= 1e-5 → "-∞ dB"` else `format!("{:.1} dB",
  20.0 * amplitude.log10())`).
- Tests (`types.test.rs`): `dbfs(0.5) == "-6.0 dB"`, `dbfs(0.0) == "-∞ dB"`,
  `dbfs(1.0) == "0.0 dB"`; `label()` for every
  `MicrophonePermissionState` variant returns its i18n key verbatim at this
  point (`localize` falls back to the key when the catalog lacks it,
  `src/services/i18n.rs:1-2`): e.g. `Denied.label() ==
  "microphone_permission_denied"`, which keeps the test locale-independent
  and start failing only when Step 5.1 adds the real keys (update the
  assertions then to match the catalog values).
- Done-check: `cargo check --workspace` passes; `cargo test -p gpui-starter
  --lib microphone::types` passes; `grep -rn "use gpui::"
  src/services/microphone/` returns nothing.

## Phase 2: Level meter and demo signal

### Step 2.1: LevelMeter

- Files: `src/services/microphone/meter.rs`, `meter.test.rs`, `mod.rs`
  (re-export).
- Build: `pub struct LevelMeter { rms: AtomicU32, peak: AtomicU32 }` with
  fixed-point scale `const SCALE: f32 = 1_048_576.;` (amplitude × 2^20).
  `update` clamps to 0..=1, encodes, `rms.store(v, Relaxed)`,
  `peak.fetch_max(v, Relaxed)`. `read` decodes rms and
  `peak.swap(0, Relaxed)`. `reset` stores 0 into both.
- Tests: encode/decode roundtrip at 0.0/0.5/1.0 is exact; `update(0.2, 0.7)`
  then `read` → (0.2, 0.7) within 1/SCALE; consecutive updates accumulate the
  max peak (`update(0.5,0.3); update(0.1,0.9); read` → peak 0.9, rms 0.1);
  `read` resets peak (second `read` → peak 0.0); `reset` zeroes both;
  `_assert_send_sync` compile check for `Arc<LevelMeter>` crossing threads.
- Done-check: `cargo test -p gpui-starter --lib microphone::meter` passes.

### Step 2.2: DemoSignal

- Files: `src/services/microphone/demo.rs`, `demo.test.rs`, `mod.rs`.
- Build: `DemoSignal { phase_a: f32, phase_b: f32, lfo: f32, seed: u64 }`
  starting at 0; `SAMPLE_RATE: f32 = 48_000.`; per frame t advances phases:
  `phase_a += 2π·220/48000`, `phase_b += 2π·523.25/48000`,
  `lfo += 2π·0.25/48000`, all wrapped mod 2π; amplitude
  `env = 0.55 + 0.45·lfo.sin()`; mono `m = env·(0.35·sin a + 0.20·sin b) +
  0.02·noise` where noise is the xorshift64* of `seed` mapped to [-1,1)
  (`(n >> 11) as f32 / (1 << 53) as f32 · 2. − 1.`); write `[m, m]` per frame
  (identical channels keep the display honest about mono content). Compute
  rms/peak over the chunk, return `(rms, peak)`.
- Tests: `fill_chunk(1584)` yields `out.len() == 3168` and left == right for
  every frame; rms and peak are within (0, 1]; amplitude envelope varies
  (chunk rms at phase 0 differs from a later chunk after simulating ~2 s);
  deterministic given `new()` (two fresh signals produce identical buffers);
  `peak >= rms` always.
- Done-check: `cargo test -p gpui-starter --lib microphone::demo` passes.

## Phase 3: Permission

### Step 3.1: Status query and bundle guard

- Files: `src/services/microphone/permission.rs`, `permission.test.rs`,
  `mod.rs`.
- Build: `pub(crate) fn status() -> MicrophonePermissionState`:
  - `#[cfg(target_os = "macos")]`: if `bundled_runtime()` errs →
    `Unavailable(reason)`; else unsafe-call
    `AVCaptureDevice::authorizationStatusForMediaType(&AVMediaTypeAudio)`:
    read the static as `Option<&AVMediaType>` and map `None` to
    `Unavailable("AVMediaTypeAudio unavailable")`; then match the consts:
    `Authorized → Authorized`, `Denied → Denied`, `Restricted → Restricted`,
    `NotDetermined → NotDetermined`, anything else →
    `Unavailable(format!("{status:?}"))`.
  - `#[cfg(all(not(target_os = "macos"), not(target_family = "wasm")))]`:
    `Authorized` (no queryable permission system; capture failure is the
    observable error path).
  - `#[cfg(target_family = "wasm")]`: `Unsupported`.
  `bundled_runtime()`: `#[cfg(target_os = "macos")]` NSBundle
  `mainBundle().bundleIdentifier()` check returning the
  "launch the bundled app" error string (mirror `user_notify.rs:236-254`);
  `Ok(())` on other targets.
- Tests: on all targets, `status()` returns a value whose `label()` is
  non-empty; on non-macOS native assert `status() == Authorized`; on wasm
  assert `Unsupported`; macOS-specific mapping is manual QA (needs a real
  bundle; noted in Verification).
- Done-check: `cargo test -p gpui-starter --lib microphone::permission`
  passes; `cargo check --workspace` passes on Linux (macOS-only imports all
  behind the cfg).

### Step 3.2: Request and settings opener

- Files: same.
- Build: `pub(crate) async fn request() -> MicrophonePermissionState`:
  non-macOS → `Unsupported`. macOS: `bundled_runtime()` err → `Unavailable`;
  else the `user_notify.rs:262-304` shape: `tokio::sync::oneshot` channel,
  `block2::RcBlock::new(move |granted: Bool| ...)` sending
  `MicrophonePermissionState::Authorized/Denied`, unsafe-call
  `AVCaptureDevice::requestAccessForMediaType_completionHandler(
  &AVMediaTypeAudio, &block)`, `rx.await` with `Unavailable` on channel drop,
  then `status()` to normalize (covers NotDetermined→denied-by-dialog edge).
  SAFETY comment: Apple invokes the handler exactly once; `RefCell<Option<Tx>>`
  + `take()` prevents double-send (same wording discipline as
  `user_notify.rs:273-275`).
  `pub(crate) fn open_microphone_settings() -> Result<(), String>` lives
  here (no cx; the controller's public wrapper logs failures). macOS:
  `std::process::Command::new("open").arg(
  "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")`
  (mirror `api.rs:290-304`); Windows:
  `open::that_detached("ms-settings:privacy-microphone")` (the `open` crate
  is already a native-only dep, `Cargo.toml` not-wasm table); other targets:
  `Err("unsupported on this platform")`.
- Tests: the non-macOS `request()` arm, gated
  `#[cfg(not(target_os = "macos"))]`, driven inside `#[gpui_kit::test]` by
  `cx.spawn(async move |_| permission::request().await)` +
  `cx.run_until_parked()` (async-in-test precedent,
  `src/state/config_store.test.rs:280-295`), asserting the result is
  `Unsupported`. (`#[tokio::test]` is unavailable; tokio's `macros` feature
  is not enabled anywhere in the workspace.)
- Done-check: `cargo test -p gpui-starter --lib microphone::permission`
  passes; on macOS `cargo check` passes with the AVF calls.

## Phase 4: Native capture session and controller

### Step 4.1: Enumeration

- Files: `src/services/microphone/capture.rs` (gate the whole module via
  `#[cfg(not(target_family = "wasm"))] mod capture;` in `mod.rs`; no cfg
  attributes inside the file), `capture.test.rs`, `mod.rs`.
- Build:
  - `pub(crate) fn enumerate() -> Result<Vec<InputDeviceInfo>, String>`:
    `cpal::default_host()`; `default_input_device()` id string captured
    first; `input_devices()` iterator; per device skip when `id()` or
    `default_input_config()` errors or the config reports 0 channels; name =
    `description().map(|d| d.name().to_string()).unwrap_or_else(|_|
    device.to_string())`; `is_default = id_str == default_id_str`;
    `sample_format = format!("{:?}", cfg.sample_format())`.
  - `pub(crate) enum SessionCommand { Stop }`, `pub(crate) enum
    SessionEvent { Started, Failed(String), Stopped }`.
  - `pub(crate) fn pick_device<'a>(devices: impl Iterator<Item = &'a
    InputDeviceInfo>, wanted: Option<&str>) -> Option<InputDeviceInfo>`:
    resolution order wanted-id → `is_default` → first.
- Tests (pure parts only; enumeration needs real audio hardware, not
  portable): `pick_device`: stale id falls back to default, `None` picks
  default, empty list → None, exact id wins over default.
- Done-check: `cargo test -p gpui-starter --lib microphone::capture` passes.

### Step 4.2: Stream build and audio callback

- Files: same.
- Build:
  - `trait ToF32 { fn to_f32_(self) -> f32 }` impls: `f32` identity; `i16` →
    `s as f32 / 32768.`; `u16` → `(s as f32 - 32768.) / 32768.`; `i32` →
    `s as f32 / 2147483648.`.
  - `fn gain_multiplier(q8: i32) -> f32 { 10f32.powf(q8 as f32 / 256. / 20.) }`.
  - `fn make_callback<T: SizedSample + Copy + ToF32 + Send + 'static>(
    producer: SampleProducer, meter: Arc<LevelMeter>, gain_q8:
    Arc<AtomicI32>, dev_channels: usize, channels: usize, ev_tx:
    flume::Sender<SessionEvent>) -> impl FnMut(&[T], &cpal::InputCallbackInfo)
    + Send + 'static`: stack buffer `let mut buf = [0f32; 1024]; let mut len
    = 0;` per callback: gain once; iterate `data.chunks(dev_channels)`; for
    `ch in 0..channels` take `frame.get(ch)`, convert, multiply gain, replace
    non-finite with 0., accumulate `peak`/`sum_sq`/`n`, append to buf, flush
    at 1024 via `producer.push_interleaved(&buf[..len])`; trailing flush;
    `meter.update(rms, peak)`; nothing else. Never allocate, never lock,
    never touch cx.
  - error callback: `move |err| { tracing::warn!(target:
    "gpui_starter::microphone", error = %err, "input stream error"); let _ =
    ev_tx.send(SessionEvent::Failed(err.to_string())); }`. The controller's
    event loop sees `Failed` and stops.
  - `fn build_and_play(device_id: &str, producer, meter, gain_q8, ev_tx) ->
    Result<cpal::Stream, String>`: re-enumerate, `pick_device` by exact id
    (error "device disappeared" when absent), `default_input_config()`,
    `channels = cfg.channels()` device-side, `wave_channels =
    (cfg.channels() as usize).clamp(1, 2)` producer-side, `build_input_stream`
    matched on `cfg.sample_format()` arms `F32/I16/U16/I32` (callback via
    `make_callback::<f32>` etc.), other formats → `Err(format!("{}:
    {other:?}", crate::i18n::localize("microphone_unsupported_format")))`,
    timeout `None`; `stream.play()`; return stream.
  - `pub(crate) async fn run_session(device_id: String, producer:
    crate::ui::waveform::SampleProducer, meter: Arc<LevelMeter>, gain_q8:
    Arc<AtomicI32>, cmd_rx: flume::Receiver<SessionCommand>, ev_tx:
    flume::Sender<SessionEvent>)`: `match build_and_play(...) { Ok(stream) =>
    { ev_tx.send(Started); while let Ok(_cmd) = cmd_rx.recv_async().await {
    break on Stop } drop(stream); ev_tx.send(Stopped) } Err(e) =>
    ev_tx.send(Failed(e)) }`. Cancellation-safe: dropping the outer `Task`
    drops this future at its await, dropping the stream; dropping the command
    sender also ends the loop.
- Tests: `ToF32` conversions (i16 −32768 → −1.0, +32767 → ≈0.99997; u16 0 →
  −1.0, 65535 → ≈0.99997; i32 extremes); `gain_multiplier(0) == 1.0`,
  `gain_multiplier(6*256) ≈ 1.9953`, `gain_multiplier(-6*256) ≈ 0.5012`;
  callback chunking is exercised in Step 4.3 through a fake producer path:
  `make_callback` needs a `SampleProducer`, which only plan 16's
  `attach_source` mints: add a `#[cfg(test)]` helper that creates a
  `WaveformState` entity in `#[gpui_kit::test]`, attaches a source, and drives
  the callback closure with a synthetic 4-channel × 128-frame i16 buffer,
  asserting `state.channels() == 2`, dropped == 0, and (after `refresh`)
  `dropped_samples()` still 0, plus `meter.read()` peak > 0. This test
  compiles only when plan 16 has landed (declared dependency ordering).
- Done-check: `cargo test -p gpui-starter --lib microphone::capture` passes.

### Step 4.3: Controller

- Files: `src/services/microphone/controller.rs`, `controller.test.rs`,
  `mod.rs` (final re-exports + `#[cfg]` gates).
- Build:
  - `struct MicrophoneState { snapshot: MicrophoneSnapshot, meter:
    Arc<LevelMeter>, gain_q8: Arc<AtomicI32>, demo_task: Option<Task<()>>,
    #[cfg(not(target_family = "wasm"))] session: Option<SessionHandle> }` —
    `demo_task` is deliberately UNCONDITIONAL: the demo path is selected by
    the runtime flag `snapshot.demo_only` (true on wasm via `initialize`,
    settable on native by `force_demo_for_tests`), not by `#[cfg]`, so the
    native test that drives the demo arm has a field to hold its task in.
    Only the cpal-backed `session` is cfg-gated. `struct SessionHandle {
    stop: flume::Sender<SessionCommand>, task: Task<()> }`;
    `impl Global for MicrophoneState {}`.
  - `pub fn initialize(cx: &mut App)`: build the waveform entity
    (`cx.new(|_| WaveformState::new())`); read
    `mic_device_id`/`mic_gain_db` from config into the snapshot; clamp + store
    gain into `gain_q8`; wasm sets `demo_only = true`; set capability
    `microphone_input`: native → `CapabilityStatus::supported_enabled()` with
    a follow-up degraded flag once enumeration ran; wasm → `CapabilityStatus {
    supported: false, enabled: false, degraded: true, reason: Some(
    localize("microphone_demo_note").into()), last_error: None }`
    (struct-literal precedent `notifications/service/api.rs:32-42`);
    `refresh_permission` synchronously (macOS status() is cheap);
    `refresh_devices(cx)` fire-and-forget on native.
  - `pub fn refresh_devices(cx: &mut App)`: wasm, no-op; native:
    `cx.spawn(async move |cx| { let devices =
    cx.background_spawn(async move { capture::enumerate() }).await;
    cx.update(|cx| mutate(cx, |s| match devices { Ok(devs) => { s.devices =
    devs; set capability supported_enabled, or degraded("no input devices")
    when empty } Err(e) => { s.devices = vec![]; set capability degraded(e) }
    }) ); }).detach()`: the `data_table.rs:1102` background_spawn shape plus
    the api.rs `mutate_snapshot` + `refresh_windows` closure.
  - `pub fn select_device(id: Option<String>, cx)`: `update_config` writes
    `mic_device_id`; snapshot update; refresh. No start/stop interference
    (selection UI is disabled while capturing: page-side rule).
  - `pub fn set_gain_db(db: f32, cx)`: clamp [-24, 24]; `update_config`
    writes `mic_gain_db`; `gain_q8.store((db * 256.) as i32, Relaxed)`;
    snapshot update; refresh.
  - `pub fn start(cx: &mut App)`: bail when status is Starting/Capturing.
    Demo arm (runtime dispatch on `snapshot.demo_only` — true on wasm, forced
    in native tests): `let producer = waveform.update(cx, |s, cx|
    s.attach_source(48_000, 2, DEFAULT_RETENTION, cx)); let meter =
    meter.clone(); demo_task = Some(cx.spawn(async move |cx| { let mut sig =
    DemoSignal::new(); let mut buf = Vec::new(); loop {
    cx.background_executor().timer(Duration::from_millis(33)).await; let (rms,
    peak) = sig.fill_chunk(&mut buf, 1584); meter.update(rms, peak);
    producer.push_interleaved(&buf); buf.clear(); if
    !producer.is_attached() { break } } }));` status → Capturing { device:
    localize("microphone_demo_device") }. Capture arm (`snapshot.demo_only ==
    false`, native only by construction — `initialize` never sets the flag
    there): macOS permission gate:
    `Authorized` → spawn_session; `NotDetermined` → status Starting +
    `cx.spawn(request-then-start)` (on grant spawn_session, else Idle +
    permission refresh); anything else → return (page disables the button).
    non-macOS native → spawn_session.
    `fn spawn_session(cx)`: `pick_device(devices, selected)` → None →
    `Error(localize("microphone_no_devices"))`; attach_source with
    `sample_rate = info.sample_rate`, `channels =
    (info.channels as usize).clamp(1, 2)`, `DEFAULT_RETENTION`; flume
    bounded(1) command + unbounded event channels; `session = Some(handle)`;
    status Starting; `let task = cx.spawn(async move |cx| { let runner =
    cx.background_spawn(capture::run_session(device_id, producer, meter,
    gain_q8, cmd_rx, ev_tx)); while let Ok(ev) = ev_rx.recv_async().await {
    match ev { Started => cx.update(set Capturing{device}), Failed(e) => {
    cx.update(set Error(e) + detach + meter.reset); break } Stopped => {
    cx.update(set Idle); break } } } drop(runner); });`: keep `runner` alive
    across the loop (cancellation chain documented in Architecture).
  - `pub fn stop(cx)`: demo arm (`demo_only`): drop demo_task,
    detach_source, meter.reset, Idle. capture arm (native): take session
    handle, `let _ = stop.send(Stop);` drop both
    fields (task drop cancels loop → runner drop → stream drop), then
    unconditionally `waveform.update(detach_source)`, `meter.reset()`,
    status Idle, refresh. Idempotent with a late `Stopped` event.
  - `pub fn refresh_permission(cx)`: snapshot.permission =
    permission::status(); refresh.
  - `pub fn request_permission(cx)`: `cx.spawn(async move |cx| { let p =
    permission::request().await; cx.update(|cx| mutate snapshot.permission) })
    .detach()` (api.rs:95-108 shape).
  - `pub fn open_microphone_settings(cx)`: calls the permission.rs opener,
    logs failure via `tracing::warn!` (target
    `gpui_starter::microphone`).
  - `pub fn meter(cx) -> Arc<LevelMeter>` and `pub fn snapshot(cx) ->
    MicrophoneSnapshot` read the global (`cx.global::<MicrophoneState>()`).
  - All snapshot mutations go through one `mutate(cx, f)` helper that ends
    with `cx.refresh_windows()` (api.rs:379-384 pattern).
- Tests (`#[gpui_kit::test]`, native host):
  - `initialize` after pre-seeding the config (setup mirrors
    `src/state/config_store.test.rs:280-295`: `crate::app_state::initialize(cx)`
    first, then `crate::app_state::update_config(cx, |c| c.mic_gain_db =
    6.0)`, then `crate::microphone::initialize(cx)`):
    `snapshot(cx).gain_db == 6.0`; permission is non-`Unknown`;
    `crate::capabilities::snapshot(cx)` contains `microphone_input`.
  - `set_gain_db(99.)` clamps to 24 and persists in
    `crate::app_state::with_config` (read back 24.0); `set_gain_db(-99.)` →
    −24.
  - `select_device(None)` clears; `select_device(Some("x"))` persists.
  - `pick_device` fallback: seed snapshot devices, call the pub(crate) helper
    directly (already covered in 4.1; here only the wiring asserts selected
    id survives snapshot round trip).
  - Demo path on a native host: `controller.rs` exposes
    `#[cfg(test)] pub(crate) fn force_demo_for_tests(cx: &mut App)` which
    sets `snapshot.demo_only = true` on the global. Test: force demo,
    `start(cx)` → status `Capturing`; `cx.executor().advance_clock(
    Duration::from_millis(100))` + `run_until_parked()` → the waveform
    entity reports `has_source() == true` and `dropped_samples() == 0`
    (timer-driven pushes landed); `stop(cx)` → `has_source() == false`,
    meter reads (0, 0), status Idle.
  - `stop` without start: no panic, status Idle.
- Done-check: `cargo test -p gpui-starter --lib microphone` passes.

## Phase 5: MicrophonePage

### Step 5.1: Page file and full page seam

- Files: `src/features/pages/microphone.rs`, `src/features/pages/mod.rs`,
  `src/shell/sidebar.rs`, `src/shell/route.rs`,
  `src/shell/root/app_root/state.rs`, `tests/e2e_navigation.rs`,
  `locales/en.yml`, `locales/zh-CN.yml`.
- Build:
  - Page struct: `pub struct MicrophonePage { waveform:
    Entity<WaveformState>, device_select: Entity<SelectState<Vec<DeviceOption>>>,
    gain_slider: Entity<SliderState>, _subscriptions: Vec<Subscription> }`;
    `new(window, cx)`: entities from `microphone::snapshot(cx)`; slider
    `min(-24.).max(24.).step(1.).default_value(snapshot.gain_db)`;
    select state with current devices (delegate `Vec<DeviceOption>`,
    selected `IndexPath::new(ix)` when a selected device exists); subscribe
    slider (`SliderEvent::Change`/`Release` → `value.start()` →
    `microphone::set_gain_db`), select (`SelectEvent::Confirm(opt)` →
    `microphone::select_device(opt, cx)`); `cx.observe(&waveform, |_, _, cx|
    cx.notify())` (plan 16 mount recipe: this is the 30 Hz re-render driver
    while capturing); `cx.observe_global_in::<crate::microphone::
    MicrophoneState>(window, move |this, window, cx| { cx.notify();
    reconcile_devices(this, window, cx); })`: the concrete global type
    `MicrophoneState` is re-exported from `mod.rs` so the page can name it
    (`observe_global_in` provides `&mut Window` in the callback,
    `GPUI/app/context.rs:700-721`). `reconcile_devices` is a page-local fn:
    it reads `microphone::snapshot(cx)` and, when the snapshot's device ids
    differ from the select's current items, runs
    `this.device_select.update(cx, |s, cx| s.set_items(items, window, cx))`
    (`set_items` requires the window, `COMP/src/select.rs:357`).
  - Render (`v_flex().min_h_full().p_6().gap_4()`, notifications-page
    density):
    1. Header `h_flex().justify_between().items_center()`: heading div
       (`a11y(Role::Heading, title).aria_level(1).text_xl().font_weight(
       FontWeight::BOLD)`) with localized title; trailing `h_flex().gap_2()`:
       Start/Stop outline `Button` (`mic_start`/`mic_stop`; Start carries
       `IconName::Mic`, Stop is label-only — `MicOff` is full-catalog and
       renders blank on native): Start `disabled` unless (native &&
       permission == Authorized && devices non-empty) || demo_only; Stop
       `disabled` unless Starting/Capturing.
    2. Device card (native only; wasm shows the demo note instead):
       `h_flex().gap_3().items_center()`: `Label::new(localize(
       "microphone_device"))`, `Select::new(&self.device_select).flex_1()
       .accessibility_label(localize("microphone_device")).placeholder(
       localize("microphone_no_devices"))`, refresh `Button::new(
       "mic-refresh").outline().small().label(localize(
       "microphone_refresh_devices")).on_click(|_,_,cx|
       microphone::refresh_devices(cx))`.
    3. Gain row: `Label::new(localize("microphone_gain"))`, `Slider::new(
       &self.gain_slider).flex_1()`, value label `div().w_12().text_sm().
       text_right()` showing `format!("{:+.0} dB", snapshot.gain_db)`, and a
       muted note line `div().text_xs().text_color(cx.theme().
       muted_foreground)` with `microphone_gain_note` (the explicit
       "hardware gain not supported" state).
    4. Level row: `Label::new(localize("microphone_level"))`,
       `Progress::new("mic-level").flex_1().value(peak * 100.).
       accessibility_label(localize("microphone_level"))`, readout
       `div().text_sm()`: `RMS {dbfs(rms)} · Peak {dbfs(peak)}` from
       `microphone::meter(cx).read()` (meter read is a render-time poll; peak
       reset per frame is the design, the displayed value is the peak since
       the previous frame at ~30 Hz).
    5. Waveform: `div().h_64().w_full().border_1().rounded_lg().
       overflow_hidden()` wrapping `Waveform::new("mic-waveform",
       self.waveform.clone())`; below it a `Switch::new("mic-spectrum").
       checked(mode == Spectrum).label(localize("microphone_spectrum")).
       on_click(...)` toggling `waveform.update(set_mode(...))`.
    6. Status rows (local `status_row` helper copied in spirit from
       `settings/notifications_section.rs:17-37`): Status
       (`status_label(status)` → idle/starting/capturing/error text),
       Permission (`permission.label()`), Dropped samples
       (`waveform.read(cx).dropped_samples().to_string()`).
    7. Guidance `Alert`s: permission `NotDetermined`/`Unavailable` →
       `Alert::warning("mic-permission-alert", ...)` with Request Permission
       button; `Denied`/`Restricted` → warning alert + Open Microphone
       Settings button + `microphone_permission_denied_guidance` body;
       `Unavailable` adds `microphone_permission_bundle_hint`. wasm →
       `Alert::info` with `microphone_demo_note`. Errors in
       `CaptureStatus::Error(e)` → `Alert::error` carrying `e`.
  - Seam edits exactly per the numbered list in Seam Touches (item 9): pages
    mod, sidebar (variant + the four match arms + `all()` appended after
    `Page::Gallery`), route VALID_HOSTS, app_root
    field/construct/match/import, e2e count 10 to 11 and `expected` gains
    `Page::Microphone`, locale keys (full table below) in both files.
- Tests: `tests/e2e_navigation.rs` updated assertions are the test.
- Done-check: `cargo test -p gpui-starter --test e2e_navigation` passes;
  `cargo test -p gpui-starter --test qa_localization` passes (key lockstep);
  `cargo check --workspace` passes.

### Step 5.2: Page render smoke test

- Files: `src/features/pages/microphone.test.rs`, wired at the bottom of
  `microphone.rs` with the sibling pattern
  `#[cfg(test)] #[path = "microphone.test.rs"] mod microphone_test;`:
  single-file modules take sibling tests exactly like `src/shell/route.rs:171-173`
  does.
- Build (`#[gpui_kit::test]`): initialize the service global
  (`crate::microphone::initialize(cx)`: safe inside a test App), then
  `cx.add_window(|window, cx| MicrophonePage::new(window, cx))`; assert no
  panic and that the page's waveform entity reports `has_source() == false`
  (idle). Second window with a forced demo session started (the Step 4.3
  `#[cfg(test)]` demo setter): assert the page renders and the meter reads
  (0, 0) before any tick.
- Done-check: `cargo test -p gpui-starter --lib microphone` passes (the
  filter already covers the page module; confirm with `cargo test -p
  gpui-starter --lib -- microphone --list` before relying on it).

### Step 5.3: Re-export freeze and module header

- Files: `src/services/microphone/mod.rs`.
- Build: final re-export list exactly matching the Interface Contract; doc
  comments on `LevelMeter`, `MicrophoneSnapshot`, and the controller fns
  stating ownership (global, single instance), the 30 Hz refresh bound, and
  the wasm demo behavior; removal-recipe header per the Removal Recipe.
- Tests: `tests/qa_migration.rs` stays green.
- Done-check: `cargo test --workspace --all-features` passes and
  `grep -rn "use gpui::|use gpui_platform::|use gpui_component::|
  use gpui_kit_assets::" src/services/microphone/
  src/features/pages/microphone.rs` returns nothing (qa_migration regex).

## Phase 6: macOS dev bundle and final verification

### Step 6.1: Info.plist key (dev bundle)

- Files: `scripts/macos-dev-app.sh` only. `scripts/create-release-app.sh` is
  plan 20's surface: its Step 1.2 inserts the identical key+string there
  grep-guarded, so this plan must not touch it and merge order cannot
  duplicate the key (`20-release-bundling.md:149-160`).
- Build: add to the dev-bundle heredoc:
  `<key>NSMicrophoneUsageDescription</key><string>Captures microphone
  audio to display the live input level and waveform.</string>`: the string
  is byte-identical to the one plan 20 quotes; rewording means editing both
  plans (plain English; the system localizes the prompt chrome itself).
- Tests: none (script change).
- Done-check: `grep -c NSMicrophoneUsageDescription
  scripts/macos-dev-app.sh` → 1; the same grep over
  `scripts/create-release-app.sh` reports whatever plan 20's state dictates
  (0 before plan 20 lands, 1 after) and is NOT this plan's gate.

### Step 6.2: Gates and manual QA

- Files: none.
- Build: nothing.
- Tests / checks: run the four gates (Verification section).
- Done-check: all four gates pass; the manual QA checklist below is executed
  or explicitly waived with notes.

## Platform & Wasm Gating

- `capture.rs` is declared `#[cfg(not(target_family = "wasm"))] mod capture;`
  in `mod.rs`; in `controller.rs` only the cpal-backed `session` field and
  `spawn_session` are `#[cfg(not(target_family = "wasm"))]`. `start`/`stop`
  dispatch on the runtime flag `snapshot.demo_only` (set in `initialize` on
  wasm, via `force_demo_for_tests` in native tests) — NOT on `#[cfg]`, so the
  demo arm compiles and runs on every target and the native demo test has a
  real code path. cpal is imported only inside `capture.rs`, so the whole cpal
  tree drops out of the wasm build.
- `permission.rs`: AVFoundation imports behind `#[cfg(target_os = "macos")]`;
  every other target compiles a matching arm. `objc2-av-foundation` sits in
  the macOS target table only.
- Dependencies: `cpal = "0.18"` under
  `[target.'cfg(not(target_family = "wasm"))'.dependencies]`;
  `objc2-av-foundation` under `[target.'cfg(target_os = "macos")'.dependencies]`.
  Nothing new goes in the common or wasm tables (seams §6).
- Capability registry: `microphone_input`: native starts
  `supported_enabled` and flips to `degraded(reason)` when enumeration fails
  or returns zero devices (the diagnostics page renders it automatically, no
  page edit); wasm registers `supported: false` with the demo-note reason
  (the "hidden with capability reason" story: the page stays visible with a
  synthetic signal, the capability row states why).
- Platform behavior matrix:

| Platform | Device list | Permission UI | Gain | Capture |
| --- | --- | --- | --- | --- |
| macOS | cpal CoreAudio input devices | status row + request/open-settings guidance; bundle guard | digital slider + "hardware gain not supported" note | real, TCC-gated |
| Windows | cpal WASAPI devices | status shows Granted (no queryable state); capture failure surfaces as Error alert | same | real |
| Linux | cpal ALSA devices | same as Windows | same | real |
| wasm | none (demo note replaces selector) | hidden (`Unsupported`) | same slider, applied to the demo signal | synthetic `DemoSignal` at 48 kHz stereo |

- `just wasm-check` (`cargo +nightly check --target wasm32-unknown-unknown -p
  gpui-starter --lib`, `justfile:24-25`) must pass: the lib gains the module
  with cpal/AVF compiled out. Demo + meter + types + controller are all
  wasm-safe (atomics, `Duration`, `cx.spawn`, `background_executor().timer`;
  the gallery's wasm-compiled timer loop is the precedent,
  `system_monitor.rs:217-230`).
- The page renders on every platform; on wasm the Select and permission
  guidance are replaced by the demo note (degradation convention: visible
  state over a missing surface, seams §6).

## i18n

Every user-facing string, `crate::i18n::localize` facade (seams §5). Keys
added to BOTH `locales/en.yml` and `locales/zh-CN.yml` (lockstep enforced by
`tests/qa_localization.rs:188-198`). `nav_microphone` is consumed indirectly
via `Page::title_key()`: invisible to the scanner, so the catalogs are the
only guard (seams §5 scanner blind spot).

| key | en | zh-CN |
|---|---|---|
| `nav_microphone` | "Microphone" | "麦克风" |
| `microphone_title` | "Microphone" | "麦克风" |
| `microphone_start` | "Start" | "开始" |
| `microphone_stop` | "Stop" | "停止" |
| `microphone_device` | "Input device" | "输入设备" |
| `microphone_refresh_devices` | "Refresh" | "刷新" |
| `microphone_no_devices` | "No input devices found" | "未找到输入设备" |
| `microphone_gain` | "Gain" | "增益" |
| `microphone_gain_note` | "Digital gain applied to captured samples; the audio host exposes no per-device hardware gain." | "对采集样本施加数字增益；音频主机不提供设备级硬件增益。" |
| `microphone_level` | "Input level" | "输入电平" |
| `microphone_spectrum` | "Spectrum view" | "频谱视图" |
| `microphone_status` | "Status" | "状态" |
| `microphone_status_idle` | "Idle" | "空闲" |
| `microphone_status_starting` | "Starting" | "启动中" |
| `microphone_status_capturing` | "Capturing" | "采集进行中" |
| `microphone_status_error` | "Error" | "错误" |
| `microphone_dropped` | "Dropped samples" | "丢弃样本数" |
| `microphone_demo_note` | "Web build: showing a synthetic demo signal instead of microphone capture." | "网页构建：以合成演示信号代替麦克风采集。" |
| `microphone_demo_device` | "Synthetic demo input" | "合成演示输入" |
| `microphone_permission` | "Permission" | "权限" |
| `microphone_permission_unknown` | "Unknown" | "未知" |
| `microphone_permission_not_determined` | "Not requested" | "尚未请求" |
| `microphone_permission_authorized` | "Granted" | "已授权" |
| `microphone_permission_denied` | "Denied" | "已拒绝" |
| `microphone_permission_restricted` | "Restricted" | "受限制" |
| `microphone_permission_unsupported` | "Not applicable" | "不适用" |
| `microphone_permission_unavailable` | "Unavailable" | "不可用" |
| `microphone_request_permission` | "Request Permission" | "请求权限" |
| `microphone_open_settings` | "Open Microphone Settings" | "打开麦克风设置" |
| `microphone_permission_denied_guidance` | "Microphone access is denied. Allow it in System Settings, then start again." | "麦克风访问被拒绝。请在系统设置中允许后重试。" |
| `microphone_permission_bundle_hint` | "Run the bundled app (scripts/macos-dev-app.sh); a raw binary has no microphone usage description." | "请运行打包后的应用（scripts/macos-dev-app.sh）；裸二进制缺少麦克风用途声明。" |
| `microphone_unsupported_format` | "Device sample format not supported" | "不支持的设备采样格式" |

Sentence-case English, no trailing periods on labels, full-width Chinese
punctuation in sentences only: matches the design-guide interface-language
rules and the existing catalog style.

## Design Compliance

- Layout and density: single-column utility page at medium density
  (`p_6`/`gap_4`, same as the notifications page `notifications.rs:37-40`);
  one section per decision (device, gain, level+waveform); the waveform is
  the page's primary object and gets the largest fixed region (`h_64`)
  (design guides, "Start from the task" + hierarchy).
- Hierarchy: one `Role::Heading` level-1 title; `Label` for control labels;
  muted `text_xs` for the gain note; `Alert` reserved for the permission
  guidance and errors (guides: Alerts are for exceptional, attention-needing
  information; a denied permission and a failed session qualify; the demo
  note uses `info` because the whole capture capability is replaced).
- Tokens: colors only via `cx.theme()` (`muted_foreground`, Alert/Progress
  bring their own themed surfaces); no raw colors anywhere. Sizing via rem
  helpers (`p_6`, `gap_4`, `h_64`, `w_12`, `text_sm`/`text_xs`); no `px()`
  literals in the page.
- Components: standard `Select` (device), `Slider` (gain), `Switch`
  (spectrum), `Progress` (meter), `Button` outline (start/stop/refresh),
  `Alert` (guidance): no custom clickable divs, no new primitives; the
  waveform is plan 16's element. Start is not auto-primary: it is an ordinary
  toolbar command (guides: a lone frequent action is not automatically
  primary), rendered as a default/outline button.
- Interaction states: Start/Stop disabled logic makes availability visible
  (permission, device presence, session state); Select disabled while
  capturing; Slider always enabled (gain applies live); every control keeps
  the kit's focus-visible and hover treatment (no overrides).
- A11y: heading level 1; `accessibility_label` on Select and Progress (their
  visible label is adjacent text, which AT does not infer per
  `select.rs:681-686`); status rows are `Role::Paragraph` with combined
  "label: value" text (the `status_row` pattern); the level readout is text,
  so level is never color/length alone; the waveform remains a non-a11y
  raster (plan 16's documented v1 limitation): the numeric RMS/peak readout
  beside it is the accessible equivalent.
- Motion: none added; the meter/waveform motion is data-driven redraw at a
  bounded 30 Hz, so reduced-motion needs no special handling.
- Zoom: page uses only rem helpers, so base-font zoom scales everything; the
  waveform paints from resolved bounds each frame (no cached geometry).

## Verification

Gates (run in this order; first three are the repo's mandatory set, AGENTS.md):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

Feature-targeted automated runs:

```sh
cargo test -p gpui-starter --lib microphone
cargo test -p gpui-starter --test e2e_navigation
cargo test -p gpui-starter --test snapshot_tests
cargo test -p gpui-starter --test qa_localization
```

Mechanical checks: `grep -rn "use gpui::" src/services/microphone/
src/features/pages/microphone.rs` returns nothing; `git diff Cargo.lock`
in the implementation commit contains cpal/objc2-av-foundation/alsa-sys;
`grep -c NSMicrophoneUsageDescription scripts/macos-dev-app.sh` → 1 (the
`scripts/create-release-app.sh` copy is plan 20's gate, not this plan's).

Manual QA (macOS, bundled via `bash scripts/macos-dev-app.sh`; Linux/Windows
with any input device):

1. Fresh bundle, first run: Microphone page shows status Idle, permission
   "Not requested" (macOS). Start → TCC prompt appears (NSMicrophoneUsageDescription shown) → Allow → status Capturing, meter moves
   with voice, waveform scrolls.
2. Deny path: deny the prompt (or System Settings → Privacy → Microphone
   off) → page shows denied guidance + Open Microphone Settings button opens
   the Privacy pane; Start disabled.
3. Device selector lists all input devices; selecting a different one (after
   Stop) and starting captures from it; restart the app → the selection and
   gain survived (config file).
4. Gain slider: change while capturing → meter/waveform amplitude changes
   live with no session restart; at +24 dB on a hot signal the display
   clamps at ±1 (plan 16) without artifacts, and the "hardware gain not
   supported" note remains visible and accurate.
5. Unplug the capture device mid-session → stream error alert appears
   (error callback path), status returns to Idle, app does not panic.
6. Raw `cargo run` on macOS (no bundle): permission shows Unavailable with
   the bundle hint; Start disabled (no TCC crash).
7. Linux/Windows: permission shows "Granted"; capture works; zero-device
   machine shows the no-devices placeholder and a degraded
   `microphone_input` capability on the diagnostics page.
8. Wasm (`just wasm-check` for compile; run the web harness per its README):
   page shows the demo note, Start runs the synthetic signal, meter and
   waveform animate, Stop freezes; no permission UI.
9. Dropping samples: sustained capture shows `dropped_samples` 0; switching
   to a tiny buffer device or stressing the machine may raise it: the
   readout must stay a number and the app must not stutter.
10. Theme + zoom: light/dark both legible; base font at 16/18 px keeps rows
    aligned and the waveform region scales.

Acceptance criteria: all four gates pass; targeted tests pass; manual items 1,
2, 3, 4, 6 (macOS), 7 (Linux/Windows), 8 (wasm) confirmed or waived with a
note naming what was checked instead.

## Risks & Open Questions

Ranked:

1. **Plan 16 is not implemented yet.** `ls src/ui/` shows only `widgets/`;
   every `WaveformState`/`SampleProducer` reference in Phases 4–5 fails until
   plan 16 lands. Tried: `ls src/ui/`, `grep -rn waveform src/` (no hits).
   Mitigation: land plan 16 first; this plan's Phase 4.2 test that mints a
   producer via `attach_source` is the integration seam.
2. **Cross-plan plist key with plan 20.** Plan 20 exists
   (`20-release-bundling.md`, `ls docs/plans/` re-run this session: 01–20
   present). Ownership is split: 18 pins the string and lands the dev-bundle
   key in `scripts/macos-dev-app.sh`; 20 lands the same key+string in
   `scripts/create-release-app.sh` behind a grep-guard, so merge order cannot
   duplicate the key (`20-release-bundling.md:149-160`, risk 6 at `:473`).
   Residual risk: an implementer rewording the string in one plan only;
   plan 20's `grep -A2` done-check catches the divergence loudly. Tried:
   both plan texts read this session; no shared constant is possible across
   two bash heredocs without a script refactor neither plan budgets.
3. **No cpal/AVF compile check was run** (doc-only planning scope). All API
   claims come from the crate sources downloaded/read this session (cited).
   Residual risk concentrates in signature details the plan paraphrases
   rather than quotes (`SliderValue::start()` on a single-value slider is
   verified from `GB/src/slider.rs` + gallery usage `text_view.rs:91-93`;
   the Select item trait is pinned by re-read this session:
   `SearchableListItem` at `COMP/src/searchable_list/delegate.rs:8`, aliased
   `SelectItem` at `select.rs:33`. An earlier draft confused it with
   `SelectDelegate`, the Vec-level delegate at `select.rs:31`).
4. **CI Linux alsa headers.** `libasound2-dev` added to
   `install-linux-deps.sh`, which every Linux lane runs
   (ci.yml:120,147,225), but whether ubuntu-24.04 runner images already ship
   it was not verifiable from here; the edit is harmless either way. Local
   Linux box currently lacks the -dev package (dpkg check this session);
   the first `cargo check --workspace` needs it installed.
5. **Hardware gain is out of scope, honestly.** cpal 0.18.2 exposes no volume
   API (verified, Research Digest) and no OS-device handle to reach one
   around it; per-OS HAL/WASAPI gain would mean re-implementing enumeration.
   The UI states this explicitly (gain note). Follow-up work, not a gap to
   paper over.
6. **Device hotplug is not detected live.** Enumeration runs at boot/panel
   refresh/manual refresh; mid-session unplug surfaces via the stream error
   callback (reliable on CoreAudio/WASAPI; ALSA behavior varies). A
   device-change notification API is a follow-up.
7. **objc2-foundation feature union.** Adding objc2-av-foundation enables
   NSArray/NSData/NSSet/NSValue/NSNotification on the existing
   objc2-foundation 0.3.2 build (feature union, same version, so no second
   copy and no Cargo.lock entry for it (locks record versions, not
   features); a lock version change for objc2-foundation would be a finding).
   Not run: build.
8. **Meter peak semantics.** `LevelMeter::read` resets peak each poll; at 30
   Hz a sub-frame transient between polls can be missed. Accepted for a
   panel meter (this is peak-per-frame, not true peak-hold); a hold-with-
   decay display is cosmetic follow-up.
9. **`assert!(channels 1..=2)` in plan 16's `attach_source`.** Devices with
   >2 input channels are clamped to their first two; 0-channel devices are
   skipped at enumeration. Documented in the Interface Contract.
10. **Wasm timer throttling.** Background-tab browsers throttle timers; the
    demo signal may stutter there. Accepted (demo only).

Open questions (each with what was tried):

- **`AVMediaTypeAudio` static loading.** It is an `extern "C"` `Option<&>`
  static (`AVF/src/generated/AVMediaFormat.rs:17-19`); `None` handling is
  coded (`Unavailable`) but cannot be exercised off real macOS. Manual QA 6
  covers the nearest path.
- **Localization of the TCC prompt.** `NSMicrophoneUsageDescription` is a
  plain plist string; Apple's InfoPlist.strings localization is a bundling
  concern owned by plan 20. English-only for now.
- **No `cargo` gate was run during planning** (doc-only scope). All
  commands in Verification are for the implementer.
