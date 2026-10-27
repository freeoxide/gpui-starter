//! Removable system controls feature. The `system_controls` service and
//! `SystemControlsPage` are the only public surfaces. Removal = delete this
//! directory, src/platform/system_controls/, and src/features/pages/
//! system_controls.rs, then revert the seam touches: the pub mod lines in
//! services/mod.rs, platform/mod.rs, and features/pages/mod.rs, the
//! system_controls entry in lib.rs's services re-export, the initialize call
//! in app/init.rs, the Page::SystemControls arms and all() entry in
//! shell/sidebar.rs, the VALID_HOSTS entry in shell/route.rs, the AppRoot
//! wiring in shell/root/app_root/state.rs, the OpenSystemControls command and
//! its three arms in services/commands.rs, the page count in
//! tests/e2e_navigation.rs, the system_controls_* + nav_system_controls +
//! command_system_controls* keys in locales/en.yml and locales/zh-CN.yml, the
//! starship-battery/zbus/libpulse-binding/brightness/objc2 lines and
//! regenerated Cargo.lock in Cargo.toml, and libpulse-dev in
//! scripts/install-linux-deps.sh.

use gpui_kit::{App, Global, SharedString};
#[cfg(not(target_family = "wasm"))]
use gpui_kit::{AppContext as _, BorrowAppContext as _};
#[cfg(not(target_family = "wasm"))]
use std::sync::Arc;

pub(crate) const REASON_WEB: &str = "system_controls_reason_web";
pub(crate) const REASON_MACOS_BRIGHTNESS: &str = "system_controls_reason_macos_brightness";
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
                reason: reason_literal(*reason_key).into(),
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
            reason: reason_literal(REASON_WEB).into(),
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
    initialize_with(Readers::platform(), cx);
}

#[cfg(not(target_family = "wasm"))]
fn initialize_with(readers: Readers, cx: &mut App) {
    let snapshot = pending_snapshot();
    cx.set_global(snapshot.clone());
    write_capabilities(&snapshot, cx);
    refresh_with(readers.clone(), cx);
    if crate::app_state::with_config(cx, |config| config.bluetooth_monitoring_enabled) {
        enable_with(readers, cx);
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
    refresh_with(Readers::platform(), cx);
    #[cfg(target_family = "wasm")]
    let _ = cx;
}

/// Platform read seam; Arc'd so tests can substitute counting mocks (the
/// real Bluetooth reader fires the TCC prompt on first call).
#[cfg(not(target_family = "wasm"))]
#[derive(Clone)]
struct Readers {
    read_core: Arc<dyn Fn() -> CoreControlsSnapshot + Send + Sync>,
    read_bluetooth: Arc<dyn Fn() -> ControlState<BluetoothStatus> + Send + Sync>,
}

#[cfg(not(target_family = "wasm"))]
impl Readers {
    fn platform() -> Self {
        Self {
            read_core: Arc::new(crate::platform::system_controls::read_core),
            read_bluetooth: Arc::new(crate::platform::system_controls::read_bluetooth),
        }
    }
}

#[cfg(not(target_family = "wasm"))]
fn refresh_with(readers: Readers, cx: &mut App) {
    let include_bluetooth = cx
        .try_global::<SystemControlsSnapshot>()
        .is_some_and(|snapshot| snapshot.bluetooth_access.polls_bluetooth());
    cx.spawn(async move |cx| {
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
        cx.update(|cx| {
            let snapshot = cx.update_global::<SystemControlsSnapshot, _>(|global, _| {
                // A disable that landed while the read was in flight owns the
                // gate; the closed gate keeps its gated row.
                let bluetooth = bluetooth.filter(|_| global.bluetooth_access.polls_bluetooth());
                *global = merge_refresh(global, core, bluetooth);
                global.clone()
            });
            write_capabilities(&snapshot, cx);
        })
    })
    .detach();
}

/// Fold a platform read into the Global: core tracks always replace, the
/// Bluetooth track only when the read included it (gate Active). The
/// access state belongs to the handshake and is never changed here, so a
/// runtime poll failure degrades the row without stopping the poller.
fn merge_refresh(
    current: &SystemControlsSnapshot,
    core: CoreControlsSnapshot,
    bluetooth: Option<ControlState<BluetoothStatus>>,
) -> SystemControlsSnapshot {
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
    enable_with(Readers::platform(), cx);
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
            let snapshot = cx.update_global::<SystemControlsSnapshot, _>(|global, _| {
                // A disable that landed while the read was in flight owns the
                // gate; commit only while this handshake is still pending.
                if matches!(global.bluetooth_access, BluetoothAccess::Requesting) {
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
        cx.update_global::<SystemControlsSnapshot, _>(|global, _| {
            if let ControlState::Ready(volume) = &mut global.volume {
                volume.level = level;
            }
        });
        apply_and_refresh(
            crate::platform::system_controls::SystemOp::SetVolume { level },
            "system_controls_volume",
            cx,
        );
    }
    #[cfg(target_family = "wasm")]
    let _ = (level, cx);
}

/// Applies the mute switch, then reconciles from a re-read.
pub fn set_volume_muted(muted: bool, cx: &mut App) {
    #[cfg(not(target_family = "wasm"))]
    {
        cx.update_global::<SystemControlsSnapshot, _>(|global, _| {
            if let ControlState::Ready(volume) = &mut global.volume {
                volume.muted = muted;
            }
        });
        apply_and_refresh(
            crate::platform::system_controls::SystemOp::SetVolumeMuted { muted },
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
        apply_and_refresh(
            crate::platform::system_controls::SystemOp::SetBrightness { level },
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
        apply_and_refresh(
            crate::platform::system_controls::SystemOp::SetBluetoothPowered { powered },
            "system_controls_powered",
            cx,
        );
    }
    #[cfg(target_family = "wasm")]
    let _ = (powered, cx);
}

#[cfg(not(target_family = "wasm"))]
fn apply_and_refresh(
    op: crate::platform::system_controls::SystemOp,
    label_key: &'static str,
    cx: &mut App,
) {
    cx.spawn(async move |cx| {
        let result = cx
            .background_spawn(async move { crate::platform::system_controls::apply(op) })
            .await;
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
    write_capability("system_controls_battery", &snapshot.battery, cx);
    crate::capabilities::set(
        "system_controls_bluetooth",
        bluetooth_capability(&snapshot.bluetooth_access, &snapshot.bluetooth),
        cx,
    );
    write_capability("system_controls_volume", &snapshot.volume, cx);
    write_capability("system_controls_brightness", &snapshot.brightness, cx);
}

fn write_capability<T>(name: &str, state: &ControlState<T>, cx: &mut App) {
    crate::capabilities::set(name, control_capability(state), cx);
}

fn control_capability<T>(state: &ControlState<T>) -> crate::capabilities::CapabilityStatus {
    match state {
        ControlState::Ready(_) => crate::capabilities::CapabilityStatus::supported_enabled(),
        ControlState::Unsupported { reason_key } => crate::capabilities::CapabilityStatus {
            supported: false,
            enabled: false,
            degraded: false,
            reason: Some(reason_literal(*reason_key).into()),
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

fn reason_literal(reason_key: &'static str) -> &'static str {
    if reason_key == REASON_WEB {
        "not available on the web build"
    } else if reason_key == REASON_MACOS_BRIGHTNESS {
        "macOS has no public brightness API"
    } else if reason_key == REASON_WINDOWS_BLUETOOTH {
        "no supported bluetooth backend for Windows"
    } else if reason_key == REASON_MACOS_BLUETOOTH_POWER {
        "bluetooth power can only be changed in system settings"
    } else {
        reason_key
    }
}

#[cfg(test)]
#[path = "system_controls.test.rs"]
mod system_controls_test;
