#[derive(Clone, Copy, Debug)]
pub(crate) enum SystemOp {
    SetVolume { level: f32 },
    SetVolumeMuted { muted: bool },
    SetBrightness { level: f32 },
    SetBluetoothPowered { powered: bool },
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
use linux as backend;
#[cfg(target_os = "macos")]
use macos as backend;
#[cfg(target_os = "windows")]
use windows as backend;
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
mod fallback;
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
use fallback as backend;

pub(crate) fn read() -> crate::system_controls::SystemControlsSnapshot {
    backend::read()
}

pub(crate) fn apply(op: SystemOp) -> Result<(), String> {
    backend::apply(op)
}

pub(super) fn read_battery(
    low_power_mode: Option<bool>,
) -> crate::system_controls::ControlState<crate::system_controls::BatteryStatus> {
    use crate::system_controls::{BatteryChargeState, BatteryStatus, ControlState};

    let read = || -> Result<Option<BatteryStatus>, String> {
        let manager = starship_battery::Manager::new().map_err(|error| error.to_string())?;
        let batteries = manager.batteries().map_err(|error| error.to_string())?;
        let Some(battery) = batteries.filter_map(Result::ok).next() else {
            return Ok(None);
        };
        let charge = battery
            .state_of_charge()
            .get::<starship_battery::units::ratio::ratio>();
        Ok(Some(BatteryStatus {
            charge: charge.clamp(0., 1.),
            state: match battery.state() {
                starship_battery::State::Charging => BatteryChargeState::Charging,
                starship_battery::State::Discharging => BatteryChargeState::Discharging,
                starship_battery::State::Full => BatteryChargeState::Full,
                starship_battery::State::Empty => BatteryChargeState::Empty,
                starship_battery::State::Paused => BatteryChargeState::Paused,
                starship_battery::State::Unknown => BatteryChargeState::Unknown,
            },
            low_power_mode,
        }))
    };
    match read() {
        Ok(Some(status)) => ControlState::Ready(status),
        Ok(None) => ControlState::Unavailable {
            error: "no battery".into(),
        },
        Err(error) => ControlState::Unavailable {
            error: error.into(),
        },
    }
}
