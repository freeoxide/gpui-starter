# 17 System Controls Plan

## Goal

A "System controls" sidebar page shows battery (charge %, charge state, low-power-mode status), bluetooth (power state, paired devices), output volume (level, mute), and screen brightness. Every control is a capability-gated row that renders one of three honest states: working control, read-only value with reason, or unsupported-with-reason; the page compiles and degrades on wasm with all rows unsupported. Mutations (set volume, mute, brightness, bluetooth power where the OS allows it) apply to the real OS state and reconcile from a re-read on completion.

## Dependencies

None consumed. This plan is a foundation; it adds no dependency on plans 01–16 and is not consumed by them.

Interface Contract items published for any future dependent (e.g. a menubar battery widget in the style of plan 04): `SystemControlsSnapshot`, `ControlState<T>`, `BatteryStatus`, `BluetoothStatus`, `PairedDevice`, `VolumeStatus`, `BrightnessStatus`, `snapshot(cx)`, `refresh(cx)`, `set_*` functions (see Interface Contract).

## Research Digest

Survey trust note: `docs/plans/_survey/seams.md` and `docs/plans/_survey/ecosystem.md` were used as the map; every load-bearing claim below was re-verified this session against files on disk or a live docs.rs/crates.io fetch (marked "web").

### Page / service seams (repo, verified on disk)

- New page = 15-step seam table; canonical removal-seam comment style at `src/gallery/mod.rs:1-7`. Page module pattern: `src/features/pages/notifications.rs:12` (`pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self`); context-free `AboutPage::new()` at `src/shell/root/app_root/state.rs:55`.
- `Page` enum arms: variant (`src/shell/sidebar.rs:5-16`), `title()` (`:19-31`), `title_key()` (`:36-48`), `host()` (`:57-69`), `icon()` (`:77-89`), `all()` (`:92-105`). `page as usize` cast in the sidebar context menu (`src/shell/root/app_root/render.rs:104`) requires `all()` order to equal enum declaration order; digit shortcuts bind only indexes 0..=8 (`src/shell/root/app_root/state.rs:144-166`), so inserting at index 9 (between `About` and `Gallery`) rebinds nothing.
- `VALID_HOSTS` at `src/shell/route.rs:10-21`; AppRoot wiring: field (`state.rs:22-31`), construction (`:47-56`), `unchecked_active_page_view` arm (`:211-223`).
- Page-count assertion `assert_eq!(all.len(), 10, ...)` at `tests/e2e_navigation.rs:14-19` — bump to 11.
- Service global pattern: `ConnectivitySnapshot` + `impl Global` + `initialize` + `snapshot` at `src/services/connectivity.rs:14-43`; polling loop pattern `cx.spawn(async move |this, cx| loop { cx.background_executor().timer(INTERVAL).await; this.update(cx, ...)? })` at `src/gallery/sections/charts/system_monitor.rs:217-229` (break on `Err` = entity dropped).
- Capability registry: `CapabilityStatus::{supported_enabled, degraded, error}` at `src/runtime/capabilities.rs:16-50`; `set` at `:64-79`; per-OS capability writer example `src/platform/input/shortcuts.rs:290-305`. Diagnostics page renders every registered entry automatically (`src/features/pages/diagnostics/rows.rs:387`); services initialize in `src/app/init.rs:203-217`.
- Commands: `CommandId` variants `src/services/commands.rs:12-29`, `availability()` arm `:45-99`, registry entry via `command(id, "command_x", "command_x_subtitle", icon)` `:102-200` (helper `:306-313`), `execute()` arm `:203-285` with `navigate` helper `:287-289`. Palette consumes `registry()` automatically (`src/features/command_palette.rs:82`, survey §1).
- i18n facade: `crate::i18n::localize("key")` and `localize_with_args("key", &[("name", val)])` with `%{name}` placeholders (`src/services/i18n.rs:3-13`); catalog keys must be identical in `locales/en.yml` + `locales/zh-CN.yml` (`tests/qa_localization.rs:188-198`); `nav_*` block at `locales/en.yml:53-62`, `locales/zh-CN.yml:53-62`.
- Error surfacing for failed mutations: `crate::error_surface::report(message, severity, category, actions, cx)` pattern from `src/services/commands.rs` (`report_error` helper, `commands.rs:291-305`).

### Kit components (verified on disk in `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`)

- `SliderState::new().min(0.).max(1.).step(0.01).default_value(f32)`; `Slider::new(&Entity<SliderState>).disabled(bool)`; events `SliderEvent::Change(SliderValue)` / `Release(SliderValue)` with `value.start()`; programmatic `slider.set_value(f32, window, cx)` — all exercised in `src/gallery/sections/controls/slider.rs:41-47,122-131,139-150,260`.
- `Switch::new(id).accessibility_label(s).checked(bool).on_click(|checked: &bool, _, cx| ...)`; `Disableable` impl for Switch (`.disabled(bool)`) — `gpui-component-0.7.1/src/switch.rs:56,79,140`; repo usage `src/features/pages/settings/mod.rs:114-119`.
- Icons exist as svg files (variant name = kebab→Pascal of filename, `gpui-kit-assets-0.7.1/src/icon.rs:3` + survey Open Questions): `battery.svg`, `battery-charging.svg`, `battery-full.svg`, `battery-low.svg`, `battery-medium.svg`, `bluetooth.svg`, `bluetooth-off.svg`, `bluetooth-connected.svg`, `volume-2.svg`, `volume-x.svg`, `volume-1.svg`, `sun.svg`, `sliders-horizontal.svg` — all present under `gpui-kit-assets-0.7.1/assets/icons/`.
- A11y row/list pattern: `Role::List` + `aria_size_of_set` on the ancestor, items `Role::ListItem` + `aria_position_in_set`/`aria_size_of_set` (`src/features/pages/notifications.rs:82-124`); `a11y(Role::Group, label)` group container (`src/features/pages/settings/mod.rs:131`).

### Crate picks (versions from crates.io API / docs.rs, fetched 2026-10-12 — web)

| Crate | Version | Platform | Justification |
| --- | --- | --- | --- |
| `starship-battery` | 0.12.0 (2026-09-27, ISC) | all native | Only maintained cross-platform battery reader (Linux 2.6.39+, macOS 10.10+, Windows 7+). API: `Manager::new()`, `manager.batteries()`, `battery.state_of_charge() -> Ratio`, `battery.state() -> State` with variants `Unknown, Charging, Discharging, Empty, Full, Paused` (docs.rs struct.Battery.html + enum.State.html, fetched this session). No low-power-mode API — macOS status read via `NSProcessInfo` instead. |
| `brightness` | 0.8.0 (2025-09-02, 0BSD) | linux, windows | Blocking module verified: `brightness::blocking::brightness_devices() -> impl Iterator<Item = Result<BrightnessDevice, Error>>`; trait methods `get() -> Result<u32>`, `set(percentage: u32)` (docs.rs, fetched this session). Docs state "Linux and Windows are supported"; macOS absent → macOS row ships unsupported-with-reason. Dep goes only in the linux + windows tables so macOS never compiles it. |
| `objc2-core-audio` | 0.3.2 (in registry cache; lock already carries it) | macos | CoreAudio HAL volume. Verified in `objc2-core-audio-0.3.2/src/generated/AudioHardware.rs`: `kAudioHardwarePropertyDefaultOutputDevice` `:1034`, `kAudioDevicePropertyVolumeScalar` `:1334`, `kAudioDevicePropertyMute` `:1348`, `kAudioObjectPropertyScopeGlobal` `:220`, `kAudioObjectPropertyScopeOutput` `:224`, `kAudioObjectPropertyElementMain` `:228`, `kAudioObjectSystemObject = 1` `:635`, `AudioObjectPropertyAddress` struct `:166-171`, `AudioObjectHasProperty` `:715`, `AudioObjectIsPropertySettable` `:745`, `AudioObjectGetPropertyData` `:815`, `AudioObjectSetPropertyData` `:856`. |
| `objc2-io-bluetooth` | 0.3.2 (2025-10-04; web) | macos | Verified on docs.rs (fetched this session): `IOBluetoothHostController::defaultController() -> Option<Retained<Self>>` and `powerState()` (1 = on, 0 = off); `IOBluetoothDevice` class fn `pairedDevices() -> Option<Retained<NSArray>>`, instance `name()`, `addressString()`, `isConnected()`, `isPaired()`. No `setPowered` exists in the bindings → bluetooth power is read-only on macOS. All needed items are behind default-on features (`IOBluetoothDevice`, `IOBluetoothHostController`, `IOBluetoothObject`, `objc2`, `objc2-foundation`, `std` — docs.rs feature page lists them all as default). |
| `objc2-foundation` (existing dep, add feature) | 0.3.0 line (lock: 0.3.2) | macos | `NSProcessInfo::isLowPowerModeEnabled() -> bool` behind feature `NSProcessInfo` (`objc2-foundation-0.3.2/src/generated/NSProcessInfo.rs:344-346`; feature gate `objc2-foundation-0.3.2/src/lib.rs:158`). Add `"NSProcessInfo"` to the existing macOS dep line (`Cargo.toml:227-232`). |
| `zbus` | 5.16.0 in lock (notify-rust 4.18, accesskit_unix, ashpd, oo7 all consume it; verified in `Cargo.lock`) | linux | BlueZ runs on the system bus. `zbus::blocking::Connection::system()` (`zbus-5.16.0/src/blocking/connection/mod.rs:36`), `zbus::blocking::Proxy::get_property` (`src/blocking/proxy/mod.rs:192`) / `set_property` (`:203`) / `new` (`:80`). Default features already include `async-io` + `blocking-api` (`zbus-5.16.0/Cargo.toml [features]`); repo rule: never enable zbus `tokio` alongside `async-io` (`Cargo.toml:254-258` comment). |
| `libpulse-binding` | 2.30.1 (2025-04-19; web) | linux | Default-sink volume + mute. Verified on docs.rs (fetched this session): `Introspector::get_server_info` → `ServerInfo.default_sink_name: Option<Cow<str>>`; `get_sink_info_by_name` → `SinkInfo { name, description, index, volume: ChannelVolumes, mute: bool }`; `set_sink_volume_by_name(name, &ChannelVolumes, callback)`, `set_sink_mute_by_name(name, bool, callback)`; `ChannelVolumes::{set(channels, Volume), avg() -> Volume, len(), is_muted()}`; volume anchors `Volume::NORMAL` / `Volume::MUTED`; threaded mainloop `mainloop::threaded::Mainloop::{new, start, stop, lock, unlock, signal, wait, get_api}`. |
| `windows` (existing dep, add feature) | 0.61 (lock: 0.61.3) | windows | Add `"Win32_Media_Audio"`. Verified in `windows-0.61.3/src/Windows/Win32/Media/Audio/mod.rs`: `IMMDeviceEnumerator` `:5585`, `GetDefaultAudioEndpoint(EDataFlow, ERole) -> IMMDevice` `:5594`, `MMDeviceEnumerator` CLSID `:8626`, `eRender` `:9300`, `eConsole` `:9298`; `IMMDevice::Activate::<T>(CLSCTX, None)` `:5372-5376`; `IAudioEndpointVolume::{Get,Set}MasterVolumeLevelScalar` and `{Get,Set}Mute` in `Media/Audio/Endpoints/mod.rs:299,323,332,356,359`; `CoCreateInstance::<T>(&GUID, None, CLSCTX_ALL)` in `Win32/System/Com/mod.rs:117`, `CLSCTX_ALL` `:1704`. `Win32_Foundation`, `Win32_System_Com` already enabled (`Cargo.toml:286-300`). |

### Decisions with evidence

- **Linux volume = libpulse-binding, not zbus→PulseAudio D-Bus.** PipeWire's pulse server implements the native protocol only; there is no equivalent of `module-dbus-protocol`, and the `org.pulseaudio.Server` name it claims is registration-only (docs.pipewire.org `page_module_protocol_pulse.html` + freedesktop.org PulseAudio D-Bus wiki, via web search this session). A zbus client would silently fail on every PipeWire-default distro (Fedora 34+, Ubuntu 22.10+). libpulse's native protocol works against both PulseAudio and pipewire-pulse. Cost: C library link → add `libpulse-dev` to `scripts/install-linux-deps.sh` (CI Linux lanes call it: `.github/workflows/ci.yml:121,148,226`); deb `depends = "$auto"` and rpm auto-req pick up `libpulse0` from the linked binary (Cargo.toml `[package.metadata.deb]`, `:94-104`).
- **Linux bluetooth = zbus → BlueZ** (`org.bluez.Adapter1` `Powered` property, `org.bluez.Device1` `Paired`/`Connected`/`Alias`/`Address`, devices enumerated via the ObjectManager `GetManagedObjects` on `/`). BlueZ D-Bus is the documented public BlueZ API.
- **macOS brightness = unsupported row.** brightness crate documents Linux+Windows only (docs.rs, fetched this session); macOS display brightness is private SPI (DisplayServices), not in any objc2 crate in the graph. Honest row: read-only label with reason, no fake slider.
- **Low-power-mode: status where exposed, no toggle anywhere.** macOS exposes status (`isLowPowerModeEnabled`, cited above). No public setter API exists on any of the three OSes (macOS private `IOPMSetPMPreferences`; Windows energy saver via powrprof is scheme-level and out of scope; Linux has no single kernel LPM concept). Per spec "where the OS exposes it" — none does, so the row is read-only with reason. Extension point documented in Architecture.
- **Windows bluetooth = unsupported row.** Survey verified no maintained Rust crate for classic Bluetooth device control on Windows (ecosystem.md h-table); nothing new found to change that.
- **All backends are blocking, run on GPUI's background executor.** starship-battery sysfs/IOKit reads, CoreAudio HAL calls, libpulse threaded mainloop, zbus blocking wrappers, brightness blocking module are all synchronous. `cx.spawn` + `cx.background_spawn` avoids tokio entirely (gallery precedent `system_monitor.rs:217`). libpulse `Context`/`Mainloop` are `!Send` (Rc inner) → a fresh connection per operation; the page poll interval (2 s) makes that cheap.
- **Brightness = first device.** `brightness_devices()` may yield several (laptop panel + ddcci externals); contract exposes the primary (first `Ok`) device only.
- **Battery = first battery.** `manager.batteries()` iterator's first `Ok` entry; multi-battery aggregation is out of boilerplate scope.

## Architecture

Entities and ownership:

- `SystemControlsSnapshot` — a GPUI `Global` owned by the app (pattern: `ConnectivitySnapshot`, `src/services/connectivity.rs:33`). Single source of truth for the four control states. Observers: `SystemControlsPage` (re-renders), any future dependent.
- `src/platform/system_controls/*` — pure, `!cx`-taking blocking functions, `pub(crate)`: `read() -> SystemControlsSnapshot` and `apply(op: SystemOp) -> Result<(), String>`. One file per OS; wasm compiles neither (module is cfg'd out).
- `src/services/system_controls` — owns the domain types, the Global, capability writes, and the async orchestration: `refresh` re-reads in the background and swaps the Global; `set_*` apply optimistically then always refresh (the refresh reconciles truth, so there is no separate revert path).
- `SystemControlsPage` — owns the two `SliderState` entities (volume, brightness), the 2 s polling loop (`cx.spawn` + weak-entity `update`, break on drop), and render. No business logic.

Events: none new. `AppEventKind` is untouched (`src/runtime/events.rs:10-24`); mutation failures go through `crate::error_surface::report` exactly as commands do.

Data flow:

```
OS API (blocking)          background thread            main thread
----------------           ------------------           ------------------
macos.rs / linux.rs   <--  services::refresh  <---- SystemControlsPage poll loop (2 s)
windows.rs / stub           (background_spawn)           + initialize() first read
        |                          |
        v                          v
   apply(SystemOp)  <----  services::set_volume / set_volume_muted /
        |                  set_brightness / set_bluetooth_powered
        v                          |
   Result<(), String>  ---->  on completion: update Global, write capabilities,
                              report failures via error_surface
```

Polling: the page starts the loop in `new` (native only, `#[cfg]`); the loop calls `crate::system_controls::refresh(cx)` each tick. `refresh` no-ops on wasm. Capability entries (`system_controls_battery`, `system_controls_bluetooth`, `system_controls_volume`, `system_controls_brightness`) are rewritten by `refresh` from the resulting `ControlState`s.

Module tree (new parts):

```
src/
├── services/
│   ├── mod.rs                        (edit: + pub mod system_controls;)
│   ├── system_controls/
│   │   ├── mod.rs                    types, Global, initialize/snapshot/refresh/set_*
│   │   └── system_controls.test.rs   unit tests (pure + localization keys)
├── platform/
│   ├── mod.rs                        (edit: + #[cfg(not(wasm))] pub mod system_controls;)
│   └── system_controls/
│       ├── mod.rs                    SystemOp, cfg dispatch read()/apply(), stub fallback
│       ├── macos.rs                  battery (starship), LPM, bluetooth (objc2-io-bluetooth), volume (CoreAudio)
│       ├── linux.rs                  battery, bluetooth (zbus/BlueZ), volume (libpulse), brightness
│       └── windows.rs                battery, volume (MMDevice COM), brightness; bluetooth unsupported
└── features/pages/
    ├── mod.rs                        (edit: + mod system_controls; + pub use)
    └── system_controls.rs            SystemControlsPage (render only + polling loop)
```

Low-power-mode toggle extension point: if an OS ever grows a public setter, add `low_power_mode_toggle: Option<bool>` to `BatteryStatus` and a `SystemOp::SetLowPowerMode` variant; the page then renders the Switch from that field. Nothing else changes.

## Interface Contract

All items live in `src/services/system_controls/mod.rs`, re-exported as `gpui_starter::system_controls::*` (lib.rs services re-export). Consumers depend only on this section.

```rust
/// One control's availability. `Unsupported` = platform/API level (never
/// works here, stable); `Unavailable` = runtime failure (no battery present,
/// no default audio device, D-Bus error...); `Ready` = usable value.
#[derive(Clone, Debug, PartialEq)]
pub enum ControlState<T> {
    Unsupported { reason_key: &'static str },
    Unavailable { error: SharedString },
    Ready(T),
}

impl<T> ControlState<T> {
    pub fn is_ready(&self) -> bool;
    /// i18n key or raw error text, for one-line status rows.
    pub fn reason(&self) -> Option<SharedString>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatteryChargeState { Charging, Discharging, Full, Empty, Paused, Unknown }

/// charge: 0.0..=1.0. low_power_mode: None when the OS exposes no status.
#[derive(Clone, Debug, PartialEq)]
pub struct BatteryStatus {
    pub charge: f32,
    pub state: BatteryChargeState,
    pub low_power_mode: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairedDevice { pub address: String, pub alias: String, pub connected: bool }

/// powered: None when the power state is not readable; the page renders the
/// switch disabled with a reason when the state is readable but not settable
/// (macOS).
#[derive(Clone, Debug, PartialEq)]
pub struct BluetoothStatus {
    pub powered: Option<bool>,
    pub devices: Vec<PairedDevice>,
}

/// level: 0.0..=1.0 master/default-sink/output-device level.
#[derive(Clone, Debug, PartialEq)]
pub struct VolumeStatus { pub level: f32, pub muted: bool }

/// level: 0.0..=1.0 primary display backlight.
#[derive(Clone, Debug, PartialEq)]
pub struct BrightnessStatus { pub level: f32 }

/// GPUI Global; Default = every control `Unavailable { "pending first read" }`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SystemControlsSnapshot {
    pub battery: ControlState<BatteryStatus>,
    pub bluetooth: ControlState<BluetoothStatus>,
    pub volume: ControlState<VolumeStatus>,
    pub brightness: ControlState<BrightnessStatus>,
}

pub fn initialize(cx: &mut App);          // set Global, write capabilities, first refresh (native)
pub fn snapshot(cx: &App) -> SystemControlsSnapshot;
pub fn refresh(cx: &mut App);             // background re-read + Global swap + capability write; no-op on wasm
pub fn set_volume(level: f32, cx: &mut App);          // clamps 0..1, optimistic, refresh after
pub fn set_volume_muted(muted: bool, cx: &mut App);
pub fn set_brightness(level: f32, cx: &mut App);      // clamps 0..1; no-op when Unsupported
pub fn set_bluetooth_powered(powered: bool, cx: &mut App); // no-op when read-only/Unsupported
```

Stable `reason_key` values (i18n keys, en + zh-CN): `system_controls_reason_web`, `system_controls_reason_macos_brightness`, `system_controls_reason_windows_bluetooth`, `system_controls_reason_macos_bluetooth_power`.

Behavioral guarantees: all `set_*` are async fire-and-forget (no return value); failures surface via `error_surface` and the next `refresh` re-reads OS truth; `set_brightness`/`set_bluetooth_powered` on a non-`Ready` control log at debug and do nothing.

## Module Layout

| File | Purpose |
| --- | --- |
| `src/services/system_controls/mod.rs` | Domain types, Global, `initialize`/`snapshot`/`refresh`/`set_*`, capability writes, wasm paths. |
| `src/services/system_controls/system_controls.test.rs` | Unit tests: state constructors, clamping, wasm snapshot shape, i18n key presence. |
| `src/platform/system_controls/mod.rs` | `SystemOp` enum + cfg dispatch to per-OS `read()`/`apply()`; non-implemented platform stubs return `Unsupported`/`Err`. |
| `src/platform/system_controls/macos.rs` | macOS backend: starship-battery, NSProcessInfo LPM, IOBluetooth, CoreAudio volume. |
| `src/platform/system_controls/linux.rs` | Linux backend: starship-battery, BlueZ over zbus, libpulse volume, brightness. |
| `src/platform/system_controls/windows.rs` | Windows backend: starship-battery, MMDevice volume, brightness. |
| `src/features/pages/system_controls.rs` | `SystemControlsPage`: polling loop + capability-gated rows. |

## Seam Touches

| # | File | Change | Why minimal |
| --- | --- | --- | --- |
| 1 | `src/services/mod.rs` | `pub mod system_controls;` (alphabetical) | one line, follows existing list |
| 2 | `src/lib.rs` | add `system_controls` to the `pub use services::{...}` list (`src/lib.rs:60-64`) | one identifier |
| 3 | `src/platform/mod.rs` | `#[cfg(not(target_family = "wasm"))] pub mod system_controls;` + 1-line comment | mirrors `deep_link_registration` gate (`platform/mod.rs:2-3`) |
| 4 | `src/app/init.rs` | in `runtime_services_init` block: `crate::system_controls::initialize(cx);` after `connectivity::initialize` (`init.rs:210`) | one call |
| 5 | `Cargo.toml` | macOS table: `objc2-core-audio`, `objc2-io-bluetooth`, `"NSProcessInfo"` on existing `objc2-foundation`; Linux table: `zbus`, `libpulse-binding`, `brightness`; Windows table: `brightness` + `"Win32_Media_Audio"` on existing `windows`; not-wasm table: `starship-battery` | per-OS tables only (`Cargo.toml:223-247, 250-272, 274-300, 310-353`) |
| 6 | `Cargo.lock` | regenerate (`cargo update -p gpui-starter` or first build); commit with manifest | `tests/qa_migration.rs:167-186` requires lock+manifest together |
| 7 | `scripts/install-linux-deps.sh` | add `libpulse-dev` to `PKGS` | CI Linux lanes source this script |
| 8 | `src/features/pages/mod.rs` | `mod system_controls;` + `pub use system_controls::SystemControlsPage;` | two lines |
| 9 | `src/shell/sidebar.rs` | `SystemControls` variant after `About` + arms in `title`, `title_key`, `host`, `icon` + `all()` entry at index 9 | exhaustive matches force exactly these five |
| 10 | `src/shell/route.rs` | `Page::SystemControls.host(),` in `VALID_HOSTS` | one line |
| 11 | `src/shell/root/app_root/state.rs` | field `system_controls_page: Entity<SystemControlsPage>`, construct in `new`, arm in `unchecked_active_page_view` | three touches, matches `state.rs:22-56,211-223` |
| 12 | `src/services/commands.rs` | `CommandId::OpenSystemControls` + `availability` arm (always enabled) + `registry` entry + `execute` arm `navigate(Page::SystemControls, cx)` | one variant, three arms |
| 13 | `tests/e2e_navigation.rs` | count `10` → `11` in `test_sidebar_pages_exist`; add `Page::SystemControls` to `expected`; add `("gpui-starter://system-controls", AppRoute::Page(Page::SystemControls))` case | three line edits |
| 14 | `locales/en.yml` + `locales/zh-CN.yml` | identical key set per the i18n table below | lockstep or `qa_localization` fails |

### Removal Recipe

Mirror of the `src/gallery/mod.rs:1-7` header comment. Delete in order:

1. Delete `src/features/pages/system_controls.rs`, `src/platform/system_controls/`, `src/services/system_controls/`.
2. `src/features/pages/mod.rs`: remove `mod system_controls;` and `pub use system_controls::SystemControlsPage;`.
3. `src/shell/sidebar.rs`: remove the `Page::SystemControls` variant and its five match arms plus the `all()` entry.
4. `src/shell/route.rs`: remove the `VALID_HOSTS` entry.
5. `src/shell/root/app_root/state.rs`: remove the field, constructor line, import, and view arm.
6. `src/services/mod.rs` + `src/lib.rs`: remove the module line and re-export.
7. `src/platform/mod.rs`: remove the gated `pub mod system_controls;`.
8. `src/app/init.rs`: remove the `initialize` call.
9. `src/services/commands.rs`: remove `CommandId::OpenSystemControls` and its three arms.
10. `tests/e2e_navigation.rs`: restore the count to the remaining page total and drop the added case.
11. `Cargo.toml`: remove the five dep lines and one feature flag; regenerate and commit `Cargo.lock`.
12. `scripts/install-linux-deps.sh`: remove `libpulse-dev`.
13. Both locale files: remove every `system_controls*` / `nav_system_controls` key.

## Implementation Phases

Each phase leaves the workspace compiling and `cargo test -p gpui-starter --lib` green.

## Phase 1: Domain types and service core (wasm-safe)

### Step 1.1: Create the service module

Files: `src/services/system_controls/mod.rs`, `src/services/mod.rs`, `src/lib.rs`.
Build: all types from Interface Contract plus private helpers `fn pending_snapshot() -> SystemControlsSnapshot` (all four `Unavailable { error: "pending first read".into() }`), `pub(crate) const REASON_*: &str` = the four stable keys, `fn clamp01(v: f32) -> f32`, `impl Global for SystemControlsSnapshot`, `Default` = `pending_snapshot()`.
Functions: `initialize` (sets global from `pending_snapshot()`, wasm: sets the four capability entries `supported: false` with English literal reason "not available on the web build"; native: also spawns one `refresh`), `snapshot` (`cx.try_global...cloned().unwrap_or_else(pending_snapshot)`, connectivity.rs:39-43 pattern), `refresh` with `#[cfg(target_family = "wasm")]` no-op body and native body deferred to Phase 2 (`#[cfg(not(target_family = "wasm"))]` early-return placeholder calling nothing yet), `set_*` stubs that only clamp + write the global field (apply wiring lands in Phase 2), `write_capabilities(&SystemControlsSnapshot, cx)` mapping `Ready→CapabilityStatus::supported_enabled()`, `Unsupported{..}→CapabilityStatus { supported: false, enabled: false, degraded: false, reason: Some(reason_key.into()) }`, `Unavailable{error}→CapabilityStatus::degraded(error)` (capabilities.rs:16-50).
Doc comments on pub items per Interface Contract wording.
Done-check: `cargo check -p gpui-starter` passes with the module registered and `pub use` in lib.rs.

### Step 1.2: Unit tests

File: `src/services/system_controls/system_controls.test.rs`, wired `#[cfg(test)] #[path = "system_controls.test.rs"] mod system_controls_test;` in mod.rs (CONTRIBUTING.md:110 pattern).
Tests: `clamp01` boundaries (`-0.5→0.0`, `1.7→1.0`, `0.42→0.42`); `pending_snapshot` = all `Unavailable`; `web_unsupported_snapshot()` helper (add it, pure fn returning all four `Unsupported { REASON_WEB }`) equals expected keys; `ControlState::reason()` returns the key for `Unsupported` and the error for `Unavailable`, `None` for `Ready`; `is_ready` matches variant; for each of the four reason keys plus the detail/state/label keys from the i18n table, `crate::i18n::localize(key)` does not equal the key itself (catches missing catalog entries; scanner blind spot, survey §5).
Done-check: `cargo test -p gpui-starter --lib system_controls` passes.

Same commit adds every `system_controls_*` key from the i18n table (both locale files), so the localization assertions pass from Phase 1 on and no phase ships a missing key. Phase 7 adds only `nav_system_controls` and the two `command_system_controls*` keys.

## Phase 2: Dependencies and platform dispatch

### Step 2.1: Cargo manifest edits

File: `Cargo.toml`.
macOS table: `objc2-core-audio = { version = "0.3.0", default-features = false, features = ["std", "AudioHardware"] }` (every symbol cited in the Research Digest lives under the `AudioHardware` feature; plain FFI, no objc2 bridge needed); `objc2-io-bluetooth = "0.3"` with default features (all required items are default features, Research Digest); extend the existing `objc2-foundation` features with `"NSProcessInfo"`.
Linux table: `zbus = "5"` (defaults stay on: `async-io` + `blocking-api`); `libpulse-binding = "2.30"` defaults.
Windows table: `brightness = "0.8"`; add `"Win32_Media_Audio"` to the existing `windows` features list.
not-wasm table: `starship-battery = "0.12"`.
Run `cargo update -p gpui-starter` (or first build) and commit `Cargo.lock` together with the manifest.
Done-check: `cargo check -p gpui-starter` on the host passes; `Cargo.lock` diff shows the new crates and no second `gpui-pre*`/`gpui-component`/`gpui-kit-assets` copy (`tests/qa_migration.rs:131-146` will enforce).

### Step 2.2: Platform dispatch skeleton

Files: `src/platform/mod.rs`, `src/platform/system_controls/mod.rs`.
Build in `platform/system_controls/mod.rs`:
```rust
#[derive(Clone, Copy, Debug)]
pub(crate) enum SystemOp {
    SetVolume { level: f32 },
    SetVolumeMuted { muted: bool },
    SetBrightness { level: f32 },
    SetBluetoothPowered { powered: bool },
}
#[cfg(target_os = "macos")] mod macos;
#[cfg(target_os = "linux")] mod linux;
#[cfg(target_os = "windows")] mod windows;
// per-OS files each export: pub(crate) fn read() -> SystemControlsSnapshot;
//                          pub(crate) fn apply(op: SystemOp) -> Result<(), String>;
```
Each per-OS file starts as a stub: `read()` returns `pending_snapshot()`-shaped `Unavailable { "backend not implemented yet" }` rows, `apply` returns `Err("backend not implemented yet")`.
Wire the service: replace Step 1.1's placeholder `refresh` native body with `cx.spawn(async move |cx| { let snap = cx.background_spawn(async move { crate::platform::system_controls::read() }).await; cx.update(|cx| { cx.update_global::<SystemControlsSnapshot,_>(|g,_| *g = snap.clone()); write_capabilities(&snap, cx); }); }).detach();` — `background_spawn` is the `AsyncApp` trait method taking a `Send` future (`gpui-pre-0.3.8/src/app/async_context.rs:131`); `SystemControlsSnapshot` is `Send` (SharedString/f32/Vec/String only). `set_*` native bodies: optimistic global write, then the same spawn shape with `apply(op)` and, on completion (either way), `refresh(cx)`; `Err(e)` additionally reports through `crate::error_surface::report` (System category, Dismiss; `commands.rs:291-305` pattern) with `localize_with_args("system_controls_set_failed", &[("control", label), ("error", e)])`. Wasm bodies stay no-op.
Done-check: `cargo test -p gpui-starter --lib` green on host; `just wasm-check` still green (platform module cfg'd out; service wasm path compiles).

### Step 2.3: CI Linux dep

File: `scripts/install-linux-deps.sh`. Add `libpulse-dev` to `PKGS` (alphabetical). Done-check: `grep libpulse-dev scripts/install-linux-deps.sh` prints one line; comment stays accurate.

## Phase 3: macOS backend

### Step 3.1: Battery + low power mode

File: `src/platform/system_controls/macos.rs`.
Build `read_battery() -> ControlState<BatteryStatus>`: `Manager::new()` → `manager.batteries()` → first `Ok(battery)`; `charge = battery.state_of_charge().get::<starship_battery::units::ratio::ratio>()` (ratio unit gives 0..1); map `State::{Charging,Discharging,Full,Empty,Paused,Unknown}` 1:1; `low_power_mode = Some(unsafe { NSProcessInfo::processInfo() }.isLowPowerModeEnabled())` — exact call form: `objc2_foundation::NSProcessInfo::processInfo()` returns `Retained<NSProcessInfo>`; method is safe on the retained value (unsafe only where the bindings mark it). No batteries → `Unavailable { "no battery" }`; manager error → `Unavailable { error }`.
Done-check: `cargo check -p gpui-starter` on the development host passes (the file is `#[cfg(target_os = "macos")]`, so Linux hosts skip compiling it); macOS compile + behavior is proven by the Phase 8 manual QA on a macOS host, with every API call in it backed by the cited crate lines.

### Step 3.2: Bluetooth

Same file. `read_bluetooth()`: `IOBluetoothHostController::defaultController()`; `None` → `Unavailable { "no bluetooth controller" }`; `Some` → `powered = Some(ctrl.powerState() == 1)`; devices from `IOBluetoothDevice::pairedDevices()` (class method), iterate `NSArray`, downcast each element to `IOBluetoothDevice` (`objc2::cast::ObjCastable`/downcast per objc2-foundation 0.3 API — use `NSArray::iter()` of `NSObject` then `downcast_into::<IOBluetoothDevice>()`; if the downcast helper name differs, follow the objc2-foundation 0.3.2 sources in the registry), collect `PairedDevice { address: addressString, alias: name, connected: isConnected }`. Sort by alias. All bindings calls are `unsafe` — wrap the block once with a 2-line `// Safety:` comment (framework calls are thread-safe here; no exception-throwing selectors per objc2-io-bluetooth docs).
`apply(SetBluetoothPowered)` → `Err("bluetooth power is read-only on macOS")` (page renders the reason key `system_controls_reason_macos_bluetooth_power` from the snapshot's `powered: Some(_)` + platform const, not from this error).
Done-check: same as 3.1.

### Step 3.3: Volume (CoreAudio HAL)

Same file. Private helpers:
`fn default_output_device() -> Option<AudioDeviceID>` — property `kAudioHardwarePropertyDefaultOutputDevice`, scope `kAudioObjectPropertyScopeGlobal`, element `kAudioObjectPropertyElementMain` on object `kAudioObjectSystemObject as AudioObjectID`, `AudioObjectGetPropertyData` into `u32` (`AudioHardware.rs:815`, selectors `:1034`,`:220`,`:228`, sysobj `:635`).
`fn device_f32(dev, sel, scope, elem) -> Option<f32>` and `fn set_device_f32(...)` — generic get/set of `f32` (VolumeScalar) / `u32` (Mute as 0/1) using `AudioObjectPropertyAddress { mSelector, mScope, mElement }` (`:166-171`); check `AudioObjectHasProperty` (`:715`) first, and for writes `AudioObjectIsPropertySettable` (`:745`).
Read: `read_volume()` → default device; `level = read(VolumeScalar, Output, ElementMain)`, on failure retry element `1` (some devices expose per-channel only); `muted = read(Mute, Output, ElementMain) == 1`. All steps `Option`-chained → `Unavailable { error }` on `None`.
Apply: `SetVolume` writes VolumeScalar on ElementMain, else elements `1` and `2` (PreferredChannelsForStereo is `:406` if channel enumeration is needed — keep to the two-element fallback, simpler). `SetVolumeMuted` writes Mute on ElementMain. `OSStatus != 0` → `Err(format!("CoreAudio error {status:#x}"))`.
`read_brightness()` → `Unsupported { REASON_MACOS_BRIGHTNESS }`; `apply(SetBrightness)` → `Err` with the same reason text.
Assemble `read()` = battery + bluetooth + volume + brightness; `apply(op)` dispatches.
Done-check: `cargo check -p gpui-starter` on Linux (module excluded) stays green; macOS host check passes with zero `unsafe` outside the two documented blocks (code-comments skill discipline).

## Phase 4: Linux backend

### Step 4.1: Battery and brightness

File: `src/platform/system_controls/linux.rs`.
`read_battery()`: identical starship-battery logic to Step 3.1 minus the LPM read (`low_power_mode: None`).
`read_brightness()` / `apply(SetBrightness)`: `brightness::blocking::brightness_devices()` → first `Ok(dev)`; `level = dev.get()? as f32 / 100.0`; set: `dev.set((level.clamp(0.,1.) * 100.0).round() as u32)?`. Empty iterator → `Unavailable { "no backlight devices" }`.
Done-check: `cargo check -p gpui-starter` passes on Linux host with the module compiled.

### Step 4.2: Bluetooth over BlueZ

Same file. Use `zbus::blocking::Connection::system()` (`zbus-5.16.0/src/blocking/connection/mod.rs:36`) — store nothing; reconnect per `read()`/`apply()` call (system bus connect is a cheap unix socket).
Adapter: `zbus::blocking::Proxy::new(&conn, "org.bluez", "/org/bluez/hci0", "org.bluez.Adapter1")` (`Proxy::new` at `src/blocking/proxy/mod.rs:80`), `powered: proxy.get_property::<bool>("Powered")`. Missing `hci0`: iterate the ObjectManager instead — `Proxy::new(&conn, "org.bluez", "/", "org.freedesktop.DBus.ObjectManager")` and `call_method("GetManagedObjects", &())`, parse the returned `HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>>` for the first path with an `org.bluez.Adapter1` interface; devices = every path with `org.bluez.Device1` where `Paired == true`, fields `Alias`, `Address`, `Connected`. Implementation order: implement the ObjectManager path directly (it also yields devices; `hci0` hardcode is not portable across adapters).
`apply(SetBluetoothPowered)`: `proxy.set_property("Powered", powered)` on the first adapter path (`:203`).
Errors (no system bus, no bluez daemon) → `Unavailable { error: e.to_string() }`.
Done-check: `cargo check -p gpui-starter`; unit tests from Phase 1 still green.

### Step 4.3: Volume over libpulse

Same file. Private `with_pulse<T>(f: impl FnOnce(&mut Introspector) -> Result<T, String>) -> Result<T, String>` implementing the canonical threaded-mainloop session per operation:
1. `Mainloop::new()?` → `ml.start()`;
2. `ml.lock()`; `Context::new(&mut ml, ml.get_api(), "gpui-starter")`; `set_state_callback(Box::new(...))` that calls `ml.signal(false)` on `Ready`/`Failed`; `ctx.connect(None, FlagSet::NOFLAGS, None)?`;
3. `ml.wait()` for Ready (loop with timeout via a deadline counter, 10 iterations max, give up with error);
4. run `f(&mut ctx.introspect())` — introspection callbacks copy results into `Arc<Mutex<Option<...>>>` captured state, call `ml.signal(false)`, and the outer side `ml.wait()`s on them (callbacks for `get_server_info` then `get_sink_info_by_name` chain: default sink name first, then sink info);
5. `ml.unlock(); ml.stop();` in a guard so early returns still stop the loop.
Read: `level = info.volume.avg() / Volume::NORMAL` as f32 (guard divide by zero: `Volume::NORMAL` is a nonzero const), `muted = info.mute`.
Apply: `SetVolume` → `let mut cv = info.volume.clone(); let v = Volume(((level.clamp(0.,1.)) * Volume::NORMAL as f32).round() as u32); cv.set(cv.len(), v); introspector.set_sink_volume_by_name(name, &cv, None);` wait for the success callback; `SetVolumeMuted` → `set_sink_mute_by_name(name, muted, None)`.
No default sink / no server → `Unavailable { error }`.
Done-check: `cargo check -p gpui-starter` on Linux links `libpulse` (`ldd target/debug/gpui-starter | grep pulse` once built) and tests pass.

## Phase 5: Windows backend

### Step 5.1: Battery and bluetooth

File: `src/platform/system_controls/windows.rs`.
`read_battery()`: starship-battery logic, `low_power_mode: None`.
`read_bluetooth()` → `Unsupported { REASON_WINDOWS_BLUETOOTH }`; `apply(SetBluetoothPowered)` → `Err` same text.
Done-check: `cargo check -p gpui-starter` on Linux (module excluded) green.

### Step 5.2: Volume (MMDevice COM) and brightness

Same file. `fn endpoint_volume() -> Option<IAudioEndpointVolume>`:
```rust
// Safety: plain COM init + activation, no pointers escape.
unsafe {
    CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;   // ignore S_FALSE
    let enum_: IMMDeviceEnumerator =
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
    let dev = enum_.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
    dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).ok()
}
}
```
(`CoInitializeEx` + `COINIT_MULTITHREADED` live in `Win32_System_Com`/`Win32_System_Ole` — verify the exact module during implementation: `Win32_System_Com` already enabled; if `COINIT_MULTITHREADED` sits behind `Win32_System_Ole`, add that feature in the same edit.) Read: `GetMasterVolumeLevelScalar()` → level; `GetMute()` → muted. Apply: `SetMasterVolumeLevelScalar(level, std::ptr::null())` / `SetMute(muted, std::ptr::null())` (signatures `Endpoints/mod.rs:323,332,356,359`).
Brightness: same as Step 4.1 (`brightness` crate).
Done-check: `cargo check --target x86_64-pc-windows-msvc` on Windows CI lane compiles (CI check job covers it; locally verify no non-Windows regressions with host check).

## Phase 6: The page

### Step 6.1: Page entity and polling loop

File: `src/features/pages/system_controls.rs`.
Struct `SystemControlsPage { volume: Entity<SliderState>, brightness: Entity<SliderState>, _subscriptions: Vec<Subscription> }`.
`new(window, cx)`: create both `SliderState::new().min(0.).max(1.).step(0.01).default_value(0.)`; subscribe `SliderEvent` — `Change` only updates the readout via `cx.notify()` (the readout reads `self.volume.read(cx).value().start()`), `Release` calls `crate::system_controls::set_volume/set_brightness(value.start(), cx)`; polling loop (native only, `#[cfg(not(target_family = "wasm"))]` on the spawn block) copied from `system_monitor.rs:217-229` with `INTERVAL = Duration::from_secs(2)` calling `crate::system_controls::refresh(cx)`; initial `refresh` also fires once in `new`.
Done-check: `cargo check -p gpui-starter` passes; no `std::time::Instant` anywhere (survey §6 clocks rule; `Duration` is fine).

### Step 6.2: Rendering

Same file. Layout (all tokens/sizes from the kit; no `px()` except nothing needed):
`v_flex().min_h_full().p_6().gap_4()` containing:
1. Heading: `div().id("system-controls-title").a11y(Role::Heading, title).aria_level(1).text_xl().font_weight(FontWeight::BOLD)` with `localize("system_controls_title")` (notifications.rs:47-59 minus `a11y_live`, static title).
2. Four section groups, each `v_flex().gap_2().p_3().border_1().border_color(cx.theme().border).rounded(cx.theme().radius_lg)` headed by `h_flex().items_center().gap_2()` = `Icon::new(<icon>)` in a fixed `size_4()` slot + `Label::new(...)` + trailing muted status text (`text_sm().text_color(cx.theme().muted_foreground)`).
   - Battery: icon by state (`BatteryCharging`/`BatteryFull`/`BatteryLow` when charge < .2/`Battery` otherwise); value row `"{charge:.0}% · {state text}"` (state text from `system_controls_state_*` keys); LPM sub-row: `Label::new(localize("system_controls_low_power_mode"))` + `localize("system_controls_on"/"system_controls_off")` when `Some`, else muted `system_controls_not_exposed`.
   - Bluetooth: powered row — `Some(true/false)` + Linux → `Switch::new("bluetooth-powered").checked(powered).on_click(cx.listener(...))`; macOS → same Switch `.disabled(true)` plus muted reason text `system_controls_reason_macos_bluetooth_power` (never a fake control); `None`/Unsupported → reason row. Device list: `div().a11y(Role::List, devices_label).aria_size_of_set(n)` with `Role::ListItem` rows `alias` + `connected ? system_controls_connected : system_controls_not_connected` (notifications.rs:82-124 pattern); empty → muted `system_controls_no_devices`.
   - Volume: header `Label` + right-aligned `"{level:.0}%"`; `Slider::new(&self.volume)`; mute row `Switch::new("volume-mute").checked(muted)` + `on_click` → `set_volume_muted`.
   - Brightness: header + `"{level:.0}%"` + `Slider::new(&self.brightness)`.
3. Degraded rendering shared helper `fn control_row<T>(...)` — for `Unsupported { reason_key }`: icon + label + `localize_with_args("system_controls_unsupported_detail", &[("reason", localize(reason_key))])`; for `Unavailable { error }`: `localize_with_args("system_controls_unavailable_detail", &[("error", error)])`. Text carries the reason (not color alone). The row for an Unsupported control shows no control widget at all.
Group IDs are static strings; device rows use `ElementId::Name(format!("bt-device-{address}"))` (stable domain identity, notifications.rs:116-119).
Done-check: `cargo clippy -p gpui-starter --all-features` clean; page compiles on wasm (`just wasm-check`) with all rows taking the reason path.

## Phase 7: Shell seams

### Step 7.1: Sidebar, route, AppRoot

Files: `src/shell/sidebar.rs`, `src/shell/route.rs`, `src/shell/root/app_root/state.rs`.
`Page::SystemControls` after `About` in the enum and in `all()` (index 9). `title()` → `"System Controls"`; `title_key()` → `"nav_system_controls"`; `host()` → `"system-controls"`; `icon()` → `IconName::Settings2` (verified default; `SlidersHorizontal` exists only in the full catalog whose bytes the app does not register natively). `VALID_HOSTS` entry. AppRoot: field, `cx.new(|cx| SystemControlsPage::new(window, cx))`, view arm.
Done-check: `cargo test -p gpui-starter --lib` green; sidebar renders the new item (manual QA, Phase 8).

### Step 7.2: Command palette entry

File: `src/services/commands.rs`. `CommandId::OpenSystemControls`; `availability` arm → always enabled (join the `OpenHome | ...` group, `commands.rs:53-63`); registry `command(CommandId::OpenSystemControls, "command_system_controls", "command_system_controls_subtitle", IconName::Settings2)`; `execute` arm → `navigate(Page::SystemControls, cx)`.
Done-check: `cargo test -p gpui-starter --lib` green; palette lists the command (manual QA).

### Step 7.3: Locales + navigation tests

Files: `locales/en.yml`, `locales/zh-CN.yml`, `tests/e2e_navigation.rs`.
Add `nav_system_controls`, `command_system_controls`, `command_system_controls_subtitle` to both catalogs (the `system_controls_*` set already landed with Step 1.2); key sets stay identical in both files (flat, quoted, snake_case, parser rules `tests/qa_localization.rs:20-71`). Test edits per Seam Touches #13.
Done-check: `cargo test --test qa_localization` and `cargo test --test e2e_navigation` pass.

## Phase 8: Verification

### Step 8.1: Repo gates

Run the four commands named in the Verification section (`cargo fmt --all -- --check`, `cargo clippy --workspace --all-features`, `cargo test --workspace --all-features`, `just wasm-check`) and fix anything they surface. No new code beyond fixes.
Done-check: all four commands exit 0 in one run, invoked verbatim.

### Step 8.2: Manual QA matrix

Execute the per-platform manual QA list from the Verification section on each available host (macOS, Linux, Windows) plus the wasm build.
Done-check: every listed observation holds; the acceptance criteria paragraph in the Verification section is satisfied, with any platform not physically available recorded as not run rather than assumed.

## Platform & Wasm Gating

cfg map (single source: `src/platform/system_controls/mod.rs` + the wasm paths in the service):

| Target | Battery | Bluetooth | Volume | Brightness |
| --- | --- | --- | --- | --- |
| macOS | read (starship-battery) + LPM status (`NSProcessInfo`); no toggle | read power + paired list; power read-only | get/set/mute (CoreAudio) | unsupported row (`system_controls_reason_macos_brightness`) |
| Linux | read (starship-battery); no LPM | read + set power, paired list (BlueZ/zbus) | get/set/mute (libpulse) | get/set (`brightness`) |
| Windows | read (starship-battery); no LPM | unsupported row (`system_controls_reason_windows_bluetooth`) | get/set/mute (MMDevice COM) | get/set (`brightness`) |
| wasm | unsupported row (`system_controls_reason_web`) | same | same | same |

- Dep gates: `starship-battery` in `[target.'cfg(not(target_family = "wasm"))'.dependencies]` (`Cargo.toml:310` table); `objc2-core-audio`, `objc2-io-bluetooth` + `NSProcessInfo` feature in the macOS table; `zbus`, `libpulse-binding`, `brightness` in the Linux table; `brightness` + `Win32_Media_Audio` in the Windows table. Nothing new in the wasm table.
- Service wasm path: `refresh`/`set_*` no-op; `initialize` writes the four capability entries with `supported: false` and a reason, and sets the Global to the all-`Unsupported` web snapshot. Page polling loop is cfg'd out; the reason rows render.
- Capability registry: four entries, names `system_controls_battery`, `system_controls_bluetooth`, `system_controls_volume`, `system_controls_brightness`; written on every `refresh` (native) and once at `initialize` (wasm). Reasons in the registry are English literals (matching the existing `shortcuts.rs:290-305` convention); the page uses i18n keys.
- zbus flavor rule: never add the zbus `tokio` feature (repo comment `Cargo.toml:254-258`); `zbus::blocking` works under the default `async-io` + `blocking-api` features.
- libpulse link: `scripts/install-linux-deps.sh` gains `libpulse-dev` (CI); deb/rpm runtime deps auto-derive (`depends = "$auto"`). AppImage bundling of `libpulse.so.0` follows whatever `scripts/build-appimage.sh` already bundles for other linked system libs — if it does not bundle system libs by default, the AppImage depends on the host's libpulse0 like any other distro lib already linked (check at packaging QA, see Risks).

## i18n

Facade: `crate::i18n::localize` / `localize_with_args` (`src/services/i18n.rs:3-13`). Every key exists in both `locales/en.yml` and `locales/zh-CN.yml`.

| Key | en | zh-CN |
| --- | --- | --- |
| `nav_system_controls` | "System controls" | "系统控制" |
| `command_system_controls` | "Open system controls" | "打开系统控制" |
| `command_system_controls_subtitle` | "Battery, bluetooth, volume, and brightness" | "电池、蓝牙、音量与亮度" |
| `system_controls_title` | "System controls" | "系统控制" |
| `system_controls_battery` | "Battery" | "电池" |
| `system_controls_charge` | "Charge" | "电量" |
| `system_controls_state_charging` | "Charging" | "充电中" |
| `system_controls_state_discharging` | "On battery" | "使用电池" |
| `system_controls_state_full` | "Fully charged" | "已充满" |
| `system_controls_state_empty` | "Empty" | "已耗尽" |
| `system_controls_state_paused` | "Paused" | "已暂停" |
| `system_controls_state_unknown` | "Unknown" | "未知" |
| `system_controls_low_power_mode` | "Low power mode" | "低电量模式" |
| `system_controls_on` | "On" | "开" |
| `system_controls_off` | "Off" | "关" |
| `system_controls_not_exposed` | "Not exposed by this system" | "此系统未提供该状态" |
| `system_controls_bluetooth` | "Bluetooth" | "蓝牙" |
| `system_controls_powered` | "Bluetooth power" | "蓝牙电源" |
| `system_controls_devices` | "Paired devices" | "已配对设备" |
| `system_controls_connected` | "Connected" | "已连接" |
| `system_controls_not_connected` | "Not connected" | "未连接" |
| `system_controls_no_devices` | "No paired devices" | "没有已配对的设备" |
| `system_controls_volume` | "Output volume" | "输出音量" |
| `system_controls_mute` | "Mute" | "静音" |
| `system_controls_brightness` | "Screen brightness" | "屏幕亮度" |
| `system_controls_unsupported_detail` | "Unsupported: %{reason}" | "不支持：%{reason}" |
| `system_controls_unavailable_detail` | "Unavailable: %{error}" | "不可用：%{error}" |
| `system_controls_set_failed` | "%{control}: %{error}" | "%{control}：%{error}" |
| `system_controls_reason_web` | "System APIs are not reachable from the web build" | "网页构建无法访问系统 API" |
| `system_controls_reason_macos_brightness` | "macOS has no public brightness API" | "macOS 没有公开的亮度 API" |
| `system_controls_reason_windows_bluetooth` | "No supported bluetooth backend for Windows" | "Windows 没有受支持的蓝牙后端" |
| `system_controls_reason_macos_bluetooth_power` | "Bluetooth power can only be changed in system settings" | "蓝牙电源只能在系统设置中更改" |

## Design Compliance

- **Shell/layout** (Design Guides "Choose a stable shell", "Forms and settings"): the page rides the existing sidebar shell; sections are settings-style groups (`p_3`, single border, `radius_lg`) not nested cards ("avoid nesting cards inside cards"); page padding `p_6` + `gap_4` matches Notifications/Settings pages.
- **Hierarchy**: one `Role::Heading` level-1 title; group `Label`s; muted `text_sm` status text; values right-aligned in trailing slots ("Right-align comparable numbers"; alignment spines via shared `h_flex().justify_between()` header row per group — "Repeat column geometry through the whole region").
- **Tokens**: every color from `cx.theme()` (`border`, `muted_foreground`); sizes via `text_sm/text_xl`, `p_3/p_6`, `gap_2/gap_4`, `size_4()` icon slot, `rounded(cx.theme().radius_lg)`; zero raw colors, zero `px()` (icon fixed slot uses `size_4()`, a rem helper).
- **Interaction states**: Switches use the kit's checked/disabled states (`Disableable`); macOS bluetooth power renders `disabled(true)` + reason text rather than a control that lies ("State must be visible": disabled = "lower emphasis and no misleading hover/pressed response"); sliders use the kit `disabled` styling only in `Ready` contexts, and Unsupported rows render no widget at all (spec: "not a fake control").
- **Destructive/errors**: no destructive actions; failed writes surface through the app's existing error surface with a recovery-free Dismiss (reversible — the next refresh re-reads OS truth), consistent with "prefer undo or a temporary notification for reversible changes".
- **A11y** (checklist + repo pattern `render.rs:210-213`): device list `Role::List`/`ListItem` + set size/position; groups `Role::Group` with labels; Switch `accessibility_label`; reason rows carry the reason as text, never color alone; every control reachable by keyboard (Switch/Slider kit components own their focus model).
- **Icons** ("one icon family", "icons supplement labels"): all lucide, inline with labels; battery icon varies by state to communicate state, not decoration.
- **Density**: medium (default sizes) — settings-page tier, not toolbar compact.
- **Motion**: none added; sliders use kit-native behavior.

## Verification

Gates (run exactly as named):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

`cargo test --workspace --all-features` includes the `tests/` integration targets (`e2e_navigation` count 11, `qa_localization` key lockstep, `qa_migration` lock/dep rules) even though CI executes only `--lib` (`.github/workflows/ci.yml:245,285`).

Feature-specific manual QA (native host per platform):

- macOS: battery % matches the menu-bar battery; unplug/charge flips the state text within ~2 s; Low power mode row reflects System Settings toggle changes; volume slider moves the system volume (verify with the volume key OSD); mute switch silences output; bluetooth section lists paired devices and matches powered state; brightness row shows the unsupported reason; toggling bluetooth power is disabled with the reason.
- Linux: same battery/volume checks against `pactl set-sink-volume` / `wpctl set-volume`; bluetooth power toggle actually powers BlueZ off/on (`bluetoothctl show`); brightness slider moves the backlight; run under a session without a pulse server once — volume row shows `Unavailable: …` and the rest still works; headless/CI-style environment (no system bus): bluetooth row `Unavailable`, app does not crash.
- Windows: volume + mute against the taskbar flyout; battery on a laptop; brightness on a laptop panel; bluetooth row shows the unsupported reason.
- wasm (`wasm/build.sh` dev or `just wasm-check` + manual web run): page renders four unsupported rows with the web reason; no console errors; polling loop absent.

Acceptance criteria: every control row on every platform shows one of working / read-only-with-reason / unsupported-with-reason; no fake controls; `Page::all().len() == 11`; deep link `gpui-starter://system-controls` navigates; command palette entry opens the page; diagnostics page lists the four capability entries with honest statuses.

## Risks & Open Questions

Ranked risks:

1. **objc2-io-bluetooth feature gates (medium).** Items verified default-enabled via the docs.rs feature page, but method-level gates beyond the four named features were not exhaustively enumerated. Mitigation: default features stay on; first macOS `cargo check` catches any miss. Tried: docs.rs index + struct pages + features page (fetched 2026-10-12).
2. **CoreAudio volume on devices without a master volume element (medium).** Algorithm falls back to channels 1/2 and reports `Unavailable` when neither reads; only real-hardware QA proves coverage. Tried: constants/fns verified in crate source; hardware behavior unverifiable without a macOS host this session.
3. **libpulse per-operation connect cost + `!Send` types (low).** Fresh threaded mainloop per op is the Send-safe pattern; 2 s cadence keeps it cheap. Alternative (dedicated pulse thread like the tray flume loop, `tray/macos.rs:92-153`) rejected for boilerplate scope.
4. **zbus `HashMap` result typing for `GetManagedObjects` (low).** Exact zvariant shape must be spelled at compile time; if the signature fight stalls, fall back to `proxy.get_property::<bool>` on `/org/bluez/hci0` + per-path `Device1` proxies discovered from a `ManagedObjects` raw `serde_json::Value::Value`? — no: keep `HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>>` and adjust per compiler messages; zbus docs treat it as the canonical shape. Residual risk noted for the implementer.
5. **AppImage host-lib dependency (low).** AppImage may depend on host `libpulse0` unless the build script bundles it; packaging QA step in Verification covers it. Tried: read `Cargo.toml` deb/rpm metadata (auto-derived); `scripts/build-appimage.sh` bundling policy not audited this session.

Open questions (each with what was tried):

1. **Does `windows` 0.61 gate `COINIT_MULTITHREADED` behind `Win32_System_Ole` or `Win32_System_Com`?** Not verified — `CoCreateInstance`/`CLSCTX_ALL` verified (`Com/mod.rs:117,1704`) but the `CoInitializeEx` constant's module was not grepped. Implementer adds the feature flag the compiler names in Step 5.2. Tried: grep of `Win32/Media/Audio`; registry source tree too large to eyeball without the exact path.
2. **macOS brightness "no public API" is double-source but not crate-source.** docs.rs platform statement (fetched this session) + survey `ecosystem.md` (docs.rs, single-source caveat). The plan does not depend on it being changeable — the row ships unsupported either way; if a future public path appears, `read_brightness` gains an implementation behind the same contract.
3. **Brightness crate set-permission on multi-user seats / Wayland compositors without logind.** Degrades to `Unavailable` via the crate's own error; not testable in this environment (no Linux desktop session). Tried: docs.rs platform notes (D-Bus/logind then sysfs fallback).
