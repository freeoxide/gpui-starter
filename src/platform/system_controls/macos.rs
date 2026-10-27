use std::ptr::NonNull;

use objc2_core_audio::{
    AudioObjectGetPropertyData, AudioObjectHasProperty, AudioObjectID,
    AudioObjectIsPropertySettable, AudioObjectPropertyAddress, AudioObjectSetPropertyData,
    kAudioDevicePropertyMute, kAudioDevicePropertyVolumeScalar,
    kAudioHardwarePropertyDefaultOutputDevice, kAudioObjectPropertyElementMain,
    kAudioObjectPropertyScopeGlobal, kAudioObjectPropertyScopeOutput, kAudioObjectSystemObject,
};
use objc2_foundation::NSProcessInfo;
use objc2_io_bluetooth::{BluetoothHCIPowerState, IOBluetoothDevice, IOBluetoothHostController};

use super::SystemOp;
use crate::system_controls::{
    BatteryStatus, BluetoothStatus, ControlState, PairedDevice, REASON_MACOS_BRIGHTNESS,
    SystemControlsSnapshot, VolumeStatus,
};

pub(super) fn read() -> SystemControlsSnapshot {
    SystemControlsSnapshot {
        battery: read_battery(),
        bluetooth: read_bluetooth(),
        volume: read_volume(),
        brightness: ControlState::Unsupported {
            reason_key: REASON_MACOS_BRIGHTNESS,
        },
    }
}

pub(super) fn apply(op: SystemOp) -> Result<(), String> {
    match op {
        SystemOp::SetVolume { level } => set_volume(level),
        SystemOp::SetVolumeMuted { muted } => set_volume_muted(muted),
        SystemOp::SetBrightness { level } => Err(format!(
            "macOS has no public brightness API (requested level {level})"
        )),
        SystemOp::SetBluetoothPowered { .. } => {
            Err("bluetooth power is read-only on macOS".to_string())
        }
    }
}

fn unavailable<T>(error: &str) -> ControlState<T> {
    ControlState::Unavailable {
        error: error.into(),
    }
}

fn read_battery() -> ControlState<BatteryStatus> {
    super::read_battery(Some(NSProcessInfo::processInfo().isLowPowerModeEnabled()))
}

fn read_bluetooth() -> ControlState<BluetoothStatus> {
    // SAFETY: IOBluetooth queries are thread-safe and none of these selectors
    // throw Objective-C exceptions.
    unsafe {
        let Some(controller) = IOBluetoothHostController::defaultController() else {
            return unavailable("no bluetooth controller");
        };
        let powered = Some(controller.powerState() == BluetoothHCIPowerState::ON);
        let mut devices = IOBluetoothDevice::pairedDevices()
            .map(|paired| {
                (0..paired.count())
                    .filter_map(|index| {
                        let device = paired
                            .objectAtIndex(index)
                            .downcast::<IOBluetoothDevice>()
                            .ok()?;
                        Some(PairedDevice {
                            address: device
                                .addressString()
                                .map(|address| address.to_string())
                                .unwrap_or_default(),
                            alias: device.name().to_string(),
                            connected: device.isConnected(),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        devices.sort_by(|a, b| a.alias.cmp(&b.alias));
        ControlState::Ready(BluetoothStatus { powered, devices })
    }
}

fn element_address(selector: u32, element: u32) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: kAudioObjectPropertyScopeOutput,
        mElement: element,
    }
}

fn default_output_device() -> Option<AudioObjectID> {
    let address = AudioObjectPropertyAddress {
        mSelector: kAudioHardwarePropertyDefaultOutputDevice,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    };
    read_property(kAudioObjectSystemObject as AudioObjectID, &address)
}

fn read_property<T>(object: AudioObjectID, address: &AudioObjectPropertyAddress) -> Option<T> {
    // SAFETY: plain FFI read into a T-sized stack buffer.
    unsafe {
        if !AudioObjectHasProperty(object, NonNull::from(address)) {
            return None;
        }
        let mut value = std::mem::zeroed::<T>();
        let mut size = std::mem::size_of::<T>() as u32;
        let status = AudioObjectGetPropertyData(
            object,
            NonNull::from(address),
            0,
            std::ptr::null(),
            NonNull::from(&mut size),
            NonNull::from(&mut value).cast(),
        );
        (status == 0).then_some(value)
    }
}

fn write_property<T>(
    object: AudioObjectID,
    address: &AudioObjectPropertyAddress,
    value: T,
) -> Option<()> {
    // SAFETY: plain FFI write of a T-sized value after the settable check.
    unsafe {
        if !AudioObjectHasProperty(object, NonNull::from(address)) {
            return None;
        }
        let mut settable = 0u8;
        let status = AudioObjectIsPropertySettable(
            object,
            NonNull::from(address),
            NonNull::from(&mut settable),
        );
        if status != 0 || settable == 0 {
            return None;
        }
        let size = std::mem::size_of::<T>() as u32;
        let status = AudioObjectSetPropertyData(
            object,
            NonNull::from(address),
            0,
            std::ptr::null(),
            size,
            NonNull::from(&value).cast(),
        );
        (status == 0).then_some(())
    }
}

fn read_volume() -> ControlState<VolumeStatus> {
    let read = || -> Option<VolumeStatus> {
        let device = default_output_device()?;
        let main = element_address(
            kAudioDevicePropertyVolumeScalar,
            kAudioObjectPropertyElementMain,
        );
        let channel_one = element_address(kAudioDevicePropertyVolumeScalar, 1);
        let level = match read_property::<f32>(device, &main) {
            Some(level) => level,
            None => read_property::<f32>(device, &channel_one)?,
        };
        let mute = element_address(kAudioDevicePropertyMute, kAudioObjectPropertyElementMain);
        let muted = read_property::<u32>(device, &mute)? != 0;
        Some(VolumeStatus {
            level: level.clamp(0., 1.),
            muted,
        })
    };
    match read() {
        Some(status) => ControlState::Ready(status),
        None => unavailable("no default audio device volume"),
    }
}

fn set_volume(level: f32) -> Result<(), String> {
    let level = level.clamp(0., 1.);
    let device = default_output_device().ok_or_else(|| "no default output device".to_string())?;
    let main = element_address(
        kAudioDevicePropertyVolumeScalar,
        kAudioObjectPropertyElementMain,
    );
    if write_property(device, &main, level).is_some() {
        return Ok(());
    }
    let channel_one = element_address(kAudioDevicePropertyVolumeScalar, 1);
    let channel_two = element_address(kAudioDevicePropertyVolumeScalar, 2);
    write_property(device, &channel_one, level)
        .and(write_property(device, &channel_two, level))
        .ok_or_else(|| "CoreAudio volume write failed".to_string())
}

fn set_volume_muted(muted: bool) -> Result<(), String> {
    let device = default_output_device().ok_or_else(|| "no default output device".to_string())?;
    let address = element_address(kAudioDevicePropertyMute, kAudioObjectPropertyElementMain);
    write_property::<u32>(device, &address, u32::from(muted))
        .ok_or_else(|| "CoreAudio mute write failed".to_string())
}
