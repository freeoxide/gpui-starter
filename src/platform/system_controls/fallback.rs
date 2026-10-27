use crate::system_controls::{BluetoothStatus, ControlState, CoreControlsSnapshot};

const NO_BACKEND: &str = "no system-controls backend on this platform";

fn unavailable<T>() -> ControlState<T> {
    ControlState::Unavailable {
        error: NO_BACKEND.into(),
    }
}

pub(super) fn read_core() -> CoreControlsSnapshot {
    CoreControlsSnapshot {
        battery: unavailable(),
        volume: unavailable(),
        brightness: unavailable(),
    }
}

pub(super) fn read_bluetooth() -> ControlState<BluetoothStatus> {
    unavailable()
}

pub(super) fn apply(_op: super::SystemOp) -> Result<(), String> {
    Err(NO_BACKEND.to_string())
}
