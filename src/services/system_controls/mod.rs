//! System controls service (battery, volume, brightness, bluetooth); a
//! self-contained feature removed with its platform twin and page.

#[cfg(not(target_family = "wasm"))]
use crate::platform::system_controls::SystemOp;
#[cfg(not(target_family = "wasm"))]
use gpui_kit::Task;
use gpui_kit::{App, Global, SharedString};

use crate::capabilities::Capability;
#[cfg(not(target_family = "wasm"))]
use gpui_kit::{AppContext as _, BorrowAppContext as _};
#[cfg(not(target_family = "wasm"))]
use std::sync::Arc;

// Gates mirror the consumers: the wasm snapshot builder and the platform
// twins in src/platform/system_controls/, plus the test suites.
#[cfg(any(test, target_family = "wasm"))]
pub(crate) const REASON_WEB: &str = "system_controls_reason_web";
#[cfg(any(test, target_os = "macos"))]
pub(crate) const REASON_MACOS_BRIGHTNESS: &str = "system_controls_reason_macos_brightness";
#[cfg(any(test, target_os = "windows"))]
pub(crate) const REASON_WINDOWS_BLUETOOTH: &str = "system_controls_reason_windows_bluetooth";
pub(crate) const REASON_MACOS_BLUETOOTH_POWER: &str =
    "system_controls_reason_macos_bluetooth_power";

/// One control's availability. `Unsupported` = platform/API level (never
/// works here, stable); `Unavailable` = runtime failure (no battery present,
/// no default audio device, D-Bus error); `Ready` = usable value.
#[derive(Clone, Debug, PartialEq)]
pub enum ControlState<T> {
    Unsupported { reason_key: &'static str },
    Unavailable { error: SharedString },
    Ready(T),
}

impl<T> ControlState<T> {
    pub fn is_ready(&self) -> bool {
        matches!(self, ControlState::Ready(_))
    }

    /// i18n key or raw error text, for one-line status rows.
    pub fn reason(&self) -> Option<SharedString> {
        match self {
            ControlState::Unsupported { reason_key } => Some(SharedString::from(*reason_key)),
            ControlState::Unavailable { error } => Some(error.clone()),
            ControlState::Ready(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatteryChargeState {
    Charging,
    Discharging,
    Full,
    Empty,
    Paused,
    Unknown,
}

/// charge: 0.0..=1.0. low_power_mode: None when the OS exposes no status.
#[derive(Clone, Debug, PartialEq)]
pub struct BatteryStatus {
    pub charge: f32,
    pub state: BatteryChargeState,
    pub low_power_mode: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairedDevice {
    pub address: String,
    pub alias: String,
    pub connected: bool,
}

/// powered: None when the power state is not readable; the page renders the
/// switch disabled with a reason when the state is readable but not settable.
#[derive(Clone, Debug, PartialEq)]
pub struct BluetoothStatus {
    pub powered: Option<bool>,
    pub devices: Vec<PairedDevice>,
}

/// level: 0.0..=1.0 master/default-sink/output-device level.
#[derive(Clone, Debug, PartialEq)]
pub struct VolumeStatus {
    pub level: f32,
    pub muted: bool,
}

/// level: 0.0..=1.0 primary display backlight.
#[derive(Clone, Debug, PartialEq)]
pub struct BrightnessStatus {
    pub level: f32,
}

/// Always-on tracks (battery, volume, brightness); none touch a
/// TCC-gated subsystem, so the poller may read them from launch.
#[derive(Clone, Debug, PartialEq)]
pub struct CoreControlsSnapshot {
    pub battery: ControlState<BatteryStatus>,
    pub volume: ControlState<VolumeStatus>,
    pub brightness: ControlState<BrightnessStatus>,
}

/// Bluetooth read gate (RCA §4). IOBluetooth has no request-permission
/// API: the TCC prompt fires on first access, so nothing may call the
/// platform Bluetooth reader until the user opts in. Runtime-only; every
/// launch restarts at `NotRequested` and redoes the handshake even with a
/// persisted opt-in (denial state is not queryable, permission may be
/// revoked).
#[derive(Clone, Debug, PartialEq)]
pub enum BluetoothAccess {
    /// Default at startup; no Bluetooth read has been requested.
    NotRequested,
    /// The single handshake read is in flight; that read is what fires
    /// the prompt.
    Requesting,
    /// The handshake read succeeded at least once; the poller may now
    /// include the Bluetooth track.
    Active,
    /// The handshake failed. Never re-polled automatically; `reason` is
    /// display text for status surfaces.
    Denied { reason: SharedString },
}

impl BluetoothAccess {
    /// Whether the periodic refresh may include the Bluetooth track.
    pub fn polls_bluetooth(&self) -> bool {
        matches!(self, BluetoothAccess::Active)
    }

    fn from_read(state: &ControlState<BluetoothStatus>) -> Self {
        match state {
            ControlState::Ready(_) => BluetoothAccess::Active,
            ControlState::Unsupported { reason_key } => BluetoothAccess::Denied {
                reason: crate::i18n::localize(*reason_key).into(),
            },
            ControlState::Unavailable { error } => BluetoothAccess::Denied {
                reason: error.clone(),
            },
        }
    }
}

/// GPUI Global; Default = every control `Unavailable { "pending first read" }`
/// with the Bluetooth gate at `NotRequested`.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemControlsSnapshot {
    pub battery: ControlState<BatteryStatus>,
    pub bluetooth: ControlState<BluetoothStatus>,
    pub volume: ControlState<VolumeStatus>,
    pub brightness: ControlState<BrightnessStatus>,
    pub bluetooth_access: BluetoothAccess,
}

impl Global for SystemControlsSnapshot {}

impl Default for SystemControlsSnapshot {
    fn default() -> Self {
        pending_snapshot()
    }
}

#[cfg(any(target_family = "wasm", test))]
pub(crate) fn web_unsupported_snapshot() -> SystemControlsSnapshot {
    SystemControlsSnapshot {
        battery: ControlState::Unsupported {
            reason_key: REASON_WEB,
        },
        bluetooth: ControlState::Unsupported {
            reason_key: REASON_WEB,
        },
        volume: ControlState::Unsupported {
            reason_key: REASON_WEB,
        },
        brightness: ControlState::Unsupported {
            reason_key: REASON_WEB,
        },
        bluetooth_access: BluetoothAccess::Denied {
            reason: crate::i18n::localize(REASON_WEB).into(),
        },
    }
}

fn pending<T>() -> ControlState<T> {
    ControlState::Unavailable {
        error: SharedString::from("pending first read"),
    }
}

fn gated_bluetooth() -> ControlState<BluetoothStatus> {
    ControlState::Unavailable {
        error: SharedString::from("bluetooth monitoring not enabled"),
    }
}

fn requesting_bluetooth() -> ControlState<BluetoothStatus> {
    ControlState::Unavailable {
        error: SharedString::from("awaiting bluetooth access"),
    }
}

fn pending_snapshot() -> SystemControlsSnapshot {
    SystemControlsSnapshot {
        battery: pending(),
        bluetooth: gated_bluetooth(),
        volume: pending(),
        brightness: pending(),
        bluetooth_access: BluetoothAccess::NotRequested,
    }
}

#[cfg(any(not(target_family = "wasm"), test))]
fn clamp01(value: f32) -> f32 {
    value.clamp(0., 1.)
}

/// Installs the Global and starts the always-on tracks (battery, volume,
/// brightness); on wasm every control registers as unsupported. Bluetooth
/// stays gated behind [`enable_bluetooth`]: with a persisted opt-in the
/// handshake reruns anyway, because permission may have been revoked.
pub fn initialize(cx: &mut App) {
    #[cfg(target_family = "wasm")]
    {
        let snapshot = web_unsupported_snapshot();
        cx.set_global(snapshot.clone());
        write_capabilities(&snapshot, cx);
    }
    #[cfg(not(target_family = "wasm"))]
    {
        initialize_with(Readers::platform(), cx);
        start_poller(cx);
    }
}

#[cfg(not(target_family = "wasm"))]
const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

/// The one periodic refresh loop, shared by every page and widget; it reads
/// only while an app window is focused (nobody watches a backgrounded app).
#[cfg(not(target_family = "wasm"))]
fn start_poller(cx: &mut App) {
    cx.spawn(async move |cx| {
        loop {
            cx.background_executor().timer(POLL_INTERVAL).await;
            cx.update(|cx| {
                if cx.active_window().is_some() {
                    refresh(cx);
                }
            });
        }
    })
    .detach();
}

#[cfg(not(target_family = "wasm"))]
fn initialize_with(readers: Readers, cx: &mut App) {
    let snapshot = pending_snapshot();
    cx.set_global(readers.clone());
    cx.set_global(snapshot.clone());
    write_capabilities(&snapshot, cx);
    refresh_with(readers, cx);
    if crate::app_state::with_config(cx, |config| config.bluetooth_monitoring_enabled) {
        enable_with(seam(cx), cx);
    }
}

pub fn snapshot(cx: &App) -> SystemControlsSnapshot {
    cx.try_global::<SystemControlsSnapshot>()
        .cloned()
        .unwrap_or_default()
}

/// Background re-read of the always-on tracks, plus Bluetooth only while
/// the gate is Active; Global swap, capability write; no-op on wasm.
pub fn refresh(cx: &mut App) {
    #[cfg(not(target_family = "wasm"))]
    refresh_with(seam(cx), cx);
    #[cfg(target_family = "wasm")]
    let _ = cx;
}

/// Platform read/apply seam; Arc'd so tests can substitute counting mocks
/// (the real Bluetooth reader fires the TCC prompt on first call, the real
/// applier writes to the OS).
#[cfg(not(target_family = "wasm"))]
#[derive(Clone)]
struct Readers {
    read_core: Arc<dyn Fn() -> CoreControlsSnapshot + Send + Sync>,
    read_bluetooth: Arc<dyn Fn() -> ControlState<BluetoothStatus> + Send + Sync>,
    apply_op: Arc<dyn Fn(SystemOp) -> Result<(), String> + Send + Sync>,
}

#[cfg(not(target_family = "wasm"))]
impl Global for Readers {}

#[cfg(not(target_family = "wasm"))]
impl Readers {
    fn platform() -> Self {
        Self {
            read_core: Arc::new(crate::platform::system_controls::read_core),
            read_bluetooth: Arc::new(crate::platform::system_controls::read_bluetooth),
            apply_op: Arc::new(crate::platform::system_controls::apply),
        }
    }
}

/// The installed seam; production installs the platform readers at
/// initialize, tests substitute mocks so no test touches the OS.
#[cfg(not(target_family = "wasm"))]
fn seam(cx: &App) -> Readers {
    cx.try_global::<Readers>()
        .cloned()
        .unwrap_or_else(Readers::platform)
}

/// Single-flight for refreshes plus read ordering: the task slot keeps
/// refreshes from overlapping, and the generation counters drop reads
/// superseded by a write or a newer handshake while they were in flight.
#[cfg(not(target_family = "wasm"))]
#[derive(Default)]
struct ControlGate {
    task: Option<Task<()>>,
    refresh_generation: u64,
    handshake_generation: u64,
}

#[cfg(not(target_family = "wasm"))]
impl Global for ControlGate {}

#[cfg(not(target_family = "wasm"))]
fn refresh_with(readers: Readers, cx: &mut App) {
    if cx
        .try_global::<ControlGate>()
        .is_some_and(|gate| gate.task.is_some())
    {
        return;
    }
    let include_bluetooth = cx
        .try_global::<SystemControlsSnapshot>()
        .is_some_and(|snapshot| snapshot.bluetooth_access.polls_bluetooth());
    let generation = cx.default_global::<ControlGate>().refresh_generation;
    let task = cx.spawn(async move |cx| {
        let core_readers = readers.clone();
        let core = cx
            .background_spawn(async move { (core_readers.read_core)() })
            .await;
        let bluetooth = if include_bluetooth {
            Some(
                cx.background_spawn(async move { (readers.read_bluetooth)() })
                    .await,
            )
        } else {
            None
        };
        let _ = cx.update(|cx| {
            let current_generation = cx
                .try_global::<ControlGate>()
                .map_or(0, |gate| gate.refresh_generation);
            let snapshot = cx.update_global::<SystemControlsSnapshot, _>(|global, _| {
                // A disable that landed while the read was in flight owns the
                // gate; the closed gate keeps its gated row.
                let bluetooth = bluetooth.filter(|_| global.bluetooth_access.polls_bluetooth());
                *global = merge_refresh(global, generation, current_generation, core, bluetooth);
                global.clone()
            });
            write_capabilities(&snapshot, cx);
            cx.global_mut::<ControlGate>().task = None;
        });
    });
    cx.default_global::<ControlGate>().task = Some(task);
}

/// Fold a platform read into the Global: core tracks always replace, the
/// Bluetooth track only when the read included it (gate Active). A read
/// whose captured generation was superseded by a write while it was in
/// flight is dropped, so it cannot revert the just-written value. The
/// access state belongs to the handshake and is never changed here, so a
/// runtime poll failure degrades the row without stopping the poller.
fn merge_refresh(
    current: &SystemControlsSnapshot,
    captured_generation: u64,
    current_generation: u64,
    core: CoreControlsSnapshot,
    bluetooth: Option<ControlState<BluetoothStatus>>,
) -> SystemControlsSnapshot {
    if captured_generation != current_generation {
        return current.clone();
    }
    let mut next = current.clone();
    next.battery = core.battery;
    next.volume = core.volume;
    next.brightness = core.brightness;
    if let Some(state) = bluetooth {
        next.bluetooth = state;
    }
    next
}

/// User opt-in: runs the single handshake read (the one that fires the
/// TCC prompt), persists the choice, and lets the poller join the
/// Bluetooth track on success. No-op while a handshake is in flight or
/// the gate is already Active.
pub fn enable_bluetooth(cx: &mut App) {
    #[cfg(not(target_family = "wasm"))]
    enable_with(seam(cx), cx);
    #[cfg(target_family = "wasm")]
    let _ = cx;
}

#[cfg(not(target_family = "wasm"))]
fn enable_with(readers: Readers, cx: &mut App) {
    let busy = cx
        .try_global::<SystemControlsSnapshot>()
        .is_some_and(|snapshot| {
            matches!(
                snapshot.bluetooth_access,
                BluetoothAccess::Requesting | BluetoothAccess::Active
            )
        });
    if busy {
        return;
    }
    let generation = {
        let gate = cx.default_global::<ControlGate>();
        gate.handshake_generation += 1;
        gate.handshake_generation
    };
    let snapshot = cx.update_global::<SystemControlsSnapshot, _>(|global, _| {
        global.bluetooth_access = BluetoothAccess::Requesting;
        global.bluetooth = requesting_bluetooth();
        global.clone()
    });
    write_capabilities(&snapshot, cx);
    crate::app_state::update_config(cx, |config| config.bluetooth_monitoring_enabled = true);
    cx.spawn(async move |cx| {
        let state = cx
            .background_spawn(async move { (readers.read_bluetooth)() })
            .await;
        cx.update(|cx| {
            let current_generation = cx
                .try_global::<ControlGate>()
                .map_or(0, |gate| gate.handshake_generation);
            let snapshot = cx.update_global::<SystemControlsSnapshot, _>(|global, _| {
                // A disable or re-enable that landed while the read was in
                // flight owns the gate; only the newest handshake commits.
                if current_generation == generation {
                    global.bluetooth = state.clone();
                    global.bluetooth_access = BluetoothAccess::from_read(&state);
                }
                global.clone()
            });
            write_capabilities(&snapshot, cx);
        })
    })
    .detach();
}

/// User opt-out: stops Bluetooth polling for this launch and persists the
/// choice so the next launch starts gated again. A read that was in flight
/// when this lands is discarded on completion.
pub fn disable_bluetooth(cx: &mut App) {
    #[cfg(not(target_family = "wasm"))]
    {
        // Any handshake read still in flight belongs to a superseded gate.
        cx.default_global::<ControlGate>().handshake_generation += 1;
        let snapshot = cx.update_global::<SystemControlsSnapshot, _>(|global, _| {
            global.bluetooth_access = BluetoothAccess::NotRequested;
            global.bluetooth = gated_bluetooth();
            global.clone()
        });
        write_capabilities(&snapshot, cx);
        crate::app_state::update_config(cx, |config| config.bluetooth_monitoring_enabled = false);
    }
    #[cfg(target_family = "wasm")]
    let _ = cx;
}

/// Clamps 0..1, applies to the OS, then reconciles from a re-read.
pub fn set_volume(level: f32, cx: &mut App) {
    #[cfg(not(target_family = "wasm"))]
    {
        let level = clamp01(level);
        let mut wrote = false;
        cx.update_global::<SystemControlsSnapshot, _>(|global, _| {
            if let ControlState::Ready(volume) = &mut global.volume {
                volume.level = level;
                wrote = true;
            }
        });
        if wrote {
            bump_refresh_generation(cx);
        }
        apply_and_refresh(SystemOp::SetVolume { level }, "system_controls_volume", cx);
    }
    #[cfg(target_family = "wasm")]
    let _ = (level, cx);
}

/// Applies the mute switch, then reconciles from a re-read.
pub fn set_volume_muted(muted: bool, cx: &mut App) {
    #[cfg(not(target_family = "wasm"))]
    {
        let mut wrote = false;
        cx.update_global::<SystemControlsSnapshot, _>(|global, _| {
            if let ControlState::Ready(volume) = &mut global.volume {
                volume.muted = muted;
                wrote = true;
            }
        });
        if wrote {
            bump_refresh_generation(cx);
        }
        apply_and_refresh(
            SystemOp::SetVolumeMuted { muted },
            "system_controls_mute",
            cx,
        );
    }
    #[cfg(target_family = "wasm")]
    let _ = (muted, cx);
}

/// Clamps 0..1, applies to the primary display; no-op when unsupported.
pub fn set_brightness(level: f32, cx: &mut App) {
    #[cfg(not(target_family = "wasm"))]
    {
        let level = clamp01(level);
        let mut ready = false;
        cx.update_global::<SystemControlsSnapshot, _>(|global, _| {
            if let ControlState::Ready(brightness) = &mut global.brightness {
                brightness.level = level;
                ready = true;
            }
        });
        if !ready {
            tracing::debug!(
                target: "gpui_starter::system_controls",
                "brightness write skipped: control not ready"
            );
            return;
        }
        bump_refresh_generation(cx);
        apply_and_refresh(
            SystemOp::SetBrightness { level },
            "system_controls_brightness",
            cx,
        );
    }
    #[cfg(target_family = "wasm")]
    let _ = (level, cx);
}

/// Applies the bluetooth power switch where the OS allows it; no-op when
/// read-only or unsupported. The `Ready` precondition is reachable only
/// while the gate is Active (a gated or denied track is never `Ready`).
pub fn set_bluetooth_powered(powered: bool, cx: &mut App) {
    #[cfg(not(target_family = "wasm"))]
    {
        let mut ready = false;
        cx.update_global::<SystemControlsSnapshot, _>(|global, _| {
            if let ControlState::Ready(bluetooth) = &mut global.bluetooth
                && let Some(current) = &mut bluetooth.powered
            {
                *current = powered;
                ready = true;
            }
        });
        if !ready {
            tracing::debug!(
                target: "gpui_starter::system_controls",
                "bluetooth power write skipped: control not ready"
            );
            return;
        }
        bump_refresh_generation(cx);
        apply_and_refresh(
            SystemOp::SetBluetoothPowered { powered },
            "system_controls_powered",
            cx,
        );
    }
    #[cfg(target_family = "wasm")]
    let _ = (powered, cx);
}

/// Every optimistic mirror write bumps the refresh generation so a read
/// that was already in flight cannot merge the pre-write value over it.
#[cfg(not(target_family = "wasm"))]
fn bump_refresh_generation(cx: &mut App) {
    cx.default_global::<ControlGate>().refresh_generation += 1;
}

#[cfg(not(target_family = "wasm"))]
fn apply_and_refresh(op: SystemOp, label_key: &'static str, cx: &mut App) {
    let applier = seam(cx).apply_op;
    cx.spawn(async move |cx| {
        let result = cx.background_spawn(async move { applier(op) }).await;
        cx.update(|cx| {
            if let Err(error) = result {
                tracing::warn!(
                    target: "gpui_starter::system_controls",
                    error = %error,
                    "control write failed"
                );
                let control = crate::i18n::localize(label_key);
                let message = crate::i18n::localize_with_args(
                    "system_controls_set_failed",
                    &[("control", &control), ("error", &error)],
                );
                crate::error_surface::report(
                    message,
                    crate::errors::AppErrorSeverity::Error,
                    crate::error_surface::ErrorCategory::System,
                    vec![crate::error_surface::ErrorAction::Dismiss],
                    cx,
                );
            }
            refresh(cx);
        })
    })
    .detach();
}

fn write_capabilities(snapshot: &SystemControlsSnapshot, cx: &mut App) {
    write_capability(Capability::SYSTEM_CONTROLS_BATTERY, &snapshot.battery, cx);
    crate::capabilities::set(
        Capability::SYSTEM_CONTROLS_BLUETOOTH,
        bluetooth_capability(&snapshot.bluetooth_access, &snapshot.bluetooth),
        cx,
    );
    write_capability(Capability::SYSTEM_CONTROLS_VOLUME, &snapshot.volume, cx);
    write_capability(
        Capability::SYSTEM_CONTROLS_BRIGHTNESS,
        &snapshot.brightness,
        cx,
    );
}

fn write_capability<T>(capability: Capability, state: &ControlState<T>, cx: &mut App) {
    crate::capabilities::set(capability, control_capability(state), cx);
}

fn control_capability<T>(state: &ControlState<T>) -> crate::capabilities::CapabilityStatus {
    match state {
        ControlState::Ready(_) => crate::capabilities::CapabilityStatus::supported_enabled(),
        ControlState::Unsupported { reason_key } => crate::capabilities::CapabilityStatus {
            supported: false,
            enabled: false,
            degraded: false,
            reason: Some(crate::i18n::localize(*reason_key).into()),
            last_error: None,
        },
        ControlState::Unavailable { error } => {
            crate::capabilities::CapabilityStatus::degraded(error.clone())
        }
    }
}

/// Whether a platform Bluetooth reader exists; false on wasm (the whole
/// platform module is compiled out there). A cfg constant, never a read:
/// probing would itself fire the TCC prompt on macOS.
fn bluetooth_supported() -> bool {
    #[cfg(target_family = "wasm")]
    {
        false
    }
    #[cfg(not(target_family = "wasm"))]
    {
        crate::platform::system_controls::bluetooth_supported()
    }
}

/// The Bluetooth capability mirrors the gate, not just the last read:
/// before opt-in it is off rather than degraded, and a denied handshake
/// is a terminal error until the user retries.
fn bluetooth_capability(
    access: &BluetoothAccess,
    state: &ControlState<BluetoothStatus>,
) -> crate::capabilities::CapabilityStatus {
    let supported = bluetooth_supported();
    match access {
        BluetoothAccess::NotRequested => crate::capabilities::CapabilityStatus {
            supported,
            enabled: false,
            degraded: false,
            reason: Some(SharedString::from("bluetooth monitoring is off")),
            last_error: None,
        },
        BluetoothAccess::Requesting => crate::capabilities::CapabilityStatus {
            supported,
            enabled: true,
            degraded: false,
            reason: Some(SharedString::from("awaiting bluetooth access")),
            last_error: None,
        },
        BluetoothAccess::Active => control_capability(state),
        BluetoothAccess::Denied { reason } => {
            if supported {
                crate::capabilities::CapabilityStatus::error(reason.clone())
            } else {
                crate::capabilities::CapabilityStatus {
                    supported: false,
                    enabled: false,
                    degraded: false,
                    reason: Some(reason.clone()),
                    last_error: None,
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "system_controls.test.rs"]
mod system_controls_test;
