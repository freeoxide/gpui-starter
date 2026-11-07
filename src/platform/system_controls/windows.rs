use brightness::blocking::Brightness as _;
use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};

use super::SystemOp;
use crate::system_controls::{
    BluetoothStatus, BrightnessStatus, ControlState, CoreControlsSnapshot,
    REASON_WINDOWS_BLUETOOTH, VolumeStatus,
};

pub(super) fn read_core() -> CoreControlsSnapshot {
    CoreControlsSnapshot {
        battery: super::read_battery(None),
        volume: read_volume(),
        brightness: read_brightness(),
    }
}

pub(super) fn read_bluetooth() -> ControlState<BluetoothStatus> {
    ControlState::Unsupported {
        reason_key: REASON_WINDOWS_BLUETOOTH,
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

// RAII balance for CoInitializeEx: Owned (S_OK or S_FALSE) releases its
// reference on drop; Foreign (RPC_E_CHANGED_MODE) never acquired one.
enum ComApartment {
    Owned,
    Foreign,
}

impl ComApartment {
    fn new() -> Option<Self> {
        // SAFETY: plain COM init; no pointers escape.
        unsafe {
            // windows-rs 0.61 returns a raw HRESULT here; ok() yields a
            // Result, so test the HRESULT directly.
            let result = CoInitializeEx(None, COINIT_MULTITHREADED);
            if result == RPC_E_CHANGED_MODE {
                Some(ComApartment::Foreign)
            } else if result.is_ok() {
                Some(ComApartment::Owned)
            } else {
                None
            }
        }
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if matches!(self, ComApartment::Owned) {
            // SAFETY: balances the CoInitializeEx from new.
            unsafe { CoUninitialize() };
        }
    }
}

// Runs f on the default endpoint while the COM apartment is held: the
// interface must not outlive the CoUninitialize that balances the init.
fn endpoint_volume<T>(
    f: impl FnOnce(&IAudioEndpointVolume) -> windows::core::Result<T>,
) -> Option<T> {
    let _apartment = ComApartment::new()?;
    // SAFETY: plain COM activation; no pointers escape.
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
        let endpoint = device
            .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
            .ok()?;
        f(&endpoint).ok()
    }
}

fn read_volume() -> ControlState<VolumeStatus> {
    let read = || -> Option<VolumeStatus> {
        endpoint_volume(|endpoint| {
            // SAFETY: plain COM property reads on a freshly activated endpoint.
            unsafe {
                let level = endpoint.GetMasterVolumeLevelScalar()?;
                let muted = endpoint.GetMute()?;
                Ok(VolumeStatus {
                    level: level.clamp(0., 1.),
                    muted: muted.as_bool(),
                })
            }
        })
    };
    match read() {
        Some(status) => ControlState::Ready(status),
        None => unavailable("no default audio device volume"),
    }
}

fn set_volume(level: f32) -> Result<(), String> {
    endpoint_volume(|endpoint| {
        // SAFETY: plain COM property write; the event-context GUID is null.
        unsafe { endpoint.SetMasterVolumeLevelScalar(level.clamp(0., 1.), std::ptr::null()) }
    })
    .ok_or_else(|| "no default audio device".to_string())
}

fn set_volume_muted(muted: bool) -> Result<(), String> {
    endpoint_volume(|endpoint| {
        // SAFETY: plain COM property write; the event-context GUID is null.
        unsafe { endpoint.SetMute(muted, std::ptr::null()) }
    })
    .ok_or_else(|| "no default audio device".to_string())
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
