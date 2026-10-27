use crate::system_controls::{ControlState, SystemControlsSnapshot};

const NO_BACKEND: &str = "no system-controls backend on this platform";

fn unavailable<T>() -> ControlState<T> {
    ControlState::Unavailable {
        error: NO_BACKEND.into(),
    }
}

pub(super) fn read() -> SystemControlsSnapshot {
    SystemControlsSnapshot {
        battery: unavailable(),
        bluetooth: unavailable(),
        volume: unavailable(),
        brightness: unavailable(),
    }
}

pub(super) fn apply(_op: super::SystemOp) -> Result<(), String> {
    Err(NO_BACKEND.to_string())
}
