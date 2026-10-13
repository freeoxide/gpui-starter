use brightness::blocking::Brightness as _;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};

use super::SystemOp;
use crate::system_controls::{
    BrightnessStatus, ControlState, REASON_WINDOWS_BLUETOOTH, SystemControlsSnapshot, VolumeStatus,
};

pub(super) fn read() -> SystemControlsSnapshot {
    SystemControlsSnapshot {
        battery: super::read_battery(None),
        bluetooth: ControlState::Unsupported {
            reason_key: REASON_WINDOWS_BLUETOOTH,
        },
        volume: read_volume(),
        brightness: read_brightness(),
    }
}

pub(super) fn apply(op: SystemOp) -> Result<(), String> {
    match op {
        SystemOp::SetVolume { level } => set_volume(level),
        SystemOp::SetVolumeMuted { muted } => set_volume_muted(muted),
        SystemOp::SetBrightness { level } => apply_brightness(level),
        SystemOp::SetBluetoothPowered { .. } => {
            Err("no supported bluetooth backend for Windows".to_string())
        }
    }
}

fn unavailable<T>(error: &str) -> ControlState<T> {
    ControlState::Unavailable {
        error: error.into(),
    }
}

fn endpoint_volume() -> Option<IAudioEndpointVolume> {
    // SAFETY: plain COM init and activation; no pointers escape.
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
        device
            .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
            .ok()
    }
}

fn read_volume() -> ControlState<VolumeStatus> {
    let read = || -> Option<VolumeStatus> {
        let endpoint = endpoint_volume()?;
        // SAFETY: plain COM property reads on a freshly activated endpoint.
        unsafe {
            let level = endpoint.GetMasterVolumeLevelScalar().ok()?;
            let muted = endpoint.GetMute().ok()?;
            Some(VolumeStatus {
                level: level.clamp(0., 1.),
                muted: muted.as_bool(),
            })
        }
    };
    match read() {
        Some(status) => ControlState::Ready(status),
        None => unavailable("no default audio device volume"),
    }
}

fn set_volume(level: f32) -> Result<(), String> {
    let endpoint = endpoint_volume().ok_or_else(|| "no default audio device".to_string())?;
    // SAFETY: plain COM property write; the event-context GUID is null.
    unsafe {
        endpoint
            .SetMasterVolumeLevelScalar(level.clamp(0., 1.), std::ptr::null())
            .map_err(|error| error.to_string())
    }
}

fn set_volume_muted(muted: bool) -> Result<(), String> {
    let endpoint = endpoint_volume().ok_or_else(|| "no default audio device".to_string())?;
    // SAFETY: plain COM property write; the event-context GUID is null.
    unsafe {
        endpoint
            .SetMute(muted, std::ptr::null())
            .map_err(|error| error.to_string())
    }
}

fn read_brightness() -> ControlState<BrightnessStatus> {
    match brightness_device() {
        Ok(device) => match device.get() {
            Ok(percent) => ControlState::Ready(BrightnessStatus {
                level: (percent as f32 / 100.).clamp(0., 1.),
            }),
            Err(error) => unavailable(&error.to_string()),
        },
        Err(error) => unavailable(&error),
    }
}

fn apply_brightness(level: f32) -> Result<(), String> {
    let device = brightness_device()?;
    device
        .set((level.clamp(0., 1.) * 100.).round() as u32)
        .map_err(|error| error.to_string())
}

fn brightness_device() -> Result<brightness::blocking::BrightnessDevice, String> {
    brightness::blocking::brightness_devices()
        .find_map(Result::ok)
        .ok_or_else(|| "no backlight devices".to_string())
}
