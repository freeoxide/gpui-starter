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

/// GPUI Global; Default = every control `Unavailable { "pending first read" }`.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemControlsSnapshot {
    pub battery: ControlState<BatteryStatus>,
    pub bluetooth: ControlState<BluetoothStatus>,
    pub volume: ControlState<VolumeStatus>,
    pub brightness: ControlState<BrightnessStatus>,
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
    }
}

fn pending<T>() -> ControlState<T> {
    ControlState::Unavailable {
        error: SharedString::from("pending first read"),
    }
}

fn pending_snapshot() -> SystemControlsSnapshot {
    SystemControlsSnapshot {
        battery: pending(),
        bluetooth: pending(),
        volume: pending(),
        brightness: pending(),
    }
}

#[cfg(any(not(target_family = "wasm"), test))]
fn clamp01(value: f32) -> f32 {
    value.clamp(0., 1.)
}

/// Installs the Global and starts the first read; on wasm every control
/// registers as unsupported.
pub fn initialize(cx: &mut App) {
    #[cfg(target_family = "wasm")]
    {
        let snapshot = web_unsupported_snapshot();
        cx.set_global(snapshot.clone());
        write_capabilities(&snapshot, cx);
    }
    #[cfg(not(target_family = "wasm"))]
    {
        cx.set_global(pending_snapshot());
        refresh(cx);
    }
}

pub fn snapshot(cx: &App) -> SystemControlsSnapshot {
    cx.try_global::<SystemControlsSnapshot>()
        .cloned()
        .unwrap_or_default()
}

/// Background re-read, Global swap, capability write; no-op on wasm.
pub fn refresh(cx: &mut App) {
    #[cfg(not(target_family = "wasm"))]
    {
        cx.spawn(async move |cx| {
            let snap = cx
                .background_spawn(async move { crate::platform::system_controls::read() })
                .await;
            cx.update(|cx| {
                cx.update_global::<SystemControlsSnapshot, _>(|global, _| *global = snap.clone());
                write_capabilities(&snap, cx);
            })
        })
        .detach();
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
/// read-only or unsupported.
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
    write_capability("system_controls_bluetooth", &snapshot.bluetooth, cx);
    write_capability("system_controls_volume", &snapshot.volume, cx);
    write_capability("system_controls_brightness", &snapshot.brightness, cx);
}

fn write_capability<T>(name: &str, state: &ControlState<T>, cx: &mut App) {
    let status = match state {
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
    };
    crate::capabilities::set(name, status, cx);
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
