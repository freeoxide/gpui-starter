use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Duration;

use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::{Context, FlagSet, State as PulseState};
use libpulse_binding::mainloop::standard::{IterateResult, Mainloop as StandardMainloop};
use libpulse_binding::volume::{ChannelVolumes, Volume};
use zbus::fdo::ManagedObjects;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

use brightness::blocking::Brightness as _;

use super::SystemOp;
use crate::system_controls::{
    BluetoothStatus, BrightnessStatus, ControlState, PairedDevice, SystemControlsSnapshot,
    VolumeStatus,
};

const BLUEZ: &str = "org.bluez";
const ADAPTER_INTERFACE: &str = "org.bluez.Adapter1";
const DEVICE_INTERFACE: &str = "org.bluez.Device1";

const PULSE_TICKS: u32 = 300;
const PULSE_TICK: Duration = Duration::from_millis(10);
const PULSE_APP_NAME: &str = "gpui-starter";

pub(super) fn read() -> SystemControlsSnapshot {
    SystemControlsSnapshot {
        battery: super::read_battery(None),
        bluetooth: read_bluetooth(),
        volume: read_volume(),
        brightness: read_brightness(),
    }
}

pub(super) fn apply(op: SystemOp) -> Result<(), String> {
    match op {
        SystemOp::SetVolume { level } => with_pulse(|session| {
            let sink = session.default_sink_name()?;
            let (mut volumes, _) = session.sink_volumes(&sink)?;
            let target = Volume(((level.clamp(0., 1.)) * Volume::NORMAL.0 as f32).round() as u32);
            volumes.set(volumes.len(), target);
            session.set_sink_volume(&sink, &volumes)
        }),
        SystemOp::SetVolumeMuted { muted } => with_pulse(|session| {
            let sink = session.default_sink_name()?;
            session.set_sink_muted(&sink, muted)
        }),
        SystemOp::SetBrightness { level } => apply_brightness(level),
        SystemOp::SetBluetoothPowered { powered } => apply_bluetooth_powered(powered),
    }
}

fn read_brightness() -> ControlState<BrightnessStatus> {
    match brightness_device() {
        Ok(device) => match device.get() {
            Ok(percent) => ControlState::Ready(BrightnessStatus {
                level: (percent as f32 / 100.).clamp(0., 1.),
            }),
            Err(error) => unavailable(error.to_string()),
        },
        Err(error) => unavailable(error),
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

fn unavailable<T>(error: String) -> ControlState<T> {
    ControlState::Unavailable {
        error: error.into(),
    }
}

fn read_bluetooth() -> ControlState<BluetoothStatus> {
    match bluetooth_objects() {
        Ok((adapter, objects)) => {
            let devices = paired_devices(&objects);
            let powered = match adapter {
                Some(path) => match adapter_powered(&path) {
                    Ok(powered) => Some(powered),
                    Err(error) => return unavailable(error),
                },
                None => None,
            };
            ControlState::Ready(BluetoothStatus { powered, devices })
        }
        Err(error) => unavailable(error),
    }
}

fn apply_bluetooth_powered(powered: bool) -> Result<(), String> {
    let (adapter, _) = bluetooth_objects()?;
    let path = adapter.ok_or_else(|| "no bluetooth adapter".to_string())?;
    let connection = zbus::blocking::Connection::system().map_err(|error| error.to_string())?;
    let proxy = zbus::blocking::Proxy::new(&connection, BLUEZ, path.as_str(), ADAPTER_INTERFACE)
        .map_err(|error| error.to_string())?;
    proxy
        .set_property("Powered", powered)
        .map_err(|error| error.to_string())
}

fn bluetooth_objects() -> Result<(Option<OwnedObjectPath>, ManagedObjects), String> {
    let connection = zbus::blocking::Connection::system().map_err(|error| error.to_string())?;
    let manager = zbus::blocking::fdo::ObjectManagerProxy::new(&connection, BLUEZ, "/")
        .map_err(|error| error.to_string())?;
    let objects = manager
        .get_managed_objects()
        .map_err(|error| error.to_string())?;
    let adapter = objects
        .iter()
        .find(|(_, interfaces)| {
            interfaces
                .keys()
                .any(|interface| interface.as_str() == ADAPTER_INTERFACE)
        })
        .map(|(path, _)| path.clone());
    Ok((adapter, objects))
}

fn adapter_powered(path: &OwnedObjectPath) -> Result<bool, String> {
    let connection = zbus::blocking::Connection::system().map_err(|error| error.to_string())?;
    let proxy = zbus::blocking::Proxy::new(&connection, BLUEZ, path.as_str(), ADAPTER_INTERFACE)
        .map_err(|error| error.to_string())?;
    proxy
        .get_property::<bool>("Powered")
        .map_err(|error| error.to_string())
}

fn paired_devices(objects: &ManagedObjects) -> Vec<PairedDevice> {
    let mut devices: Vec<PairedDevice> = objects
        .values()
        .filter_map(|interfaces| {
            let properties = interfaces
                .iter()
                .find(|(interface, _)| interface.as_str() == DEVICE_INTERFACE)
                .map(|(_, properties)| properties)?;
            device_property_bool(properties, "Paired")?;
            Some(PairedDevice {
                address: device_property_string(properties, "Address").unwrap_or_default(),
                alias: device_property_string(properties, "Alias").unwrap_or_default(),
                connected: device_property_bool(properties, "Connected").unwrap_or(false),
            })
        })
        .collect();
    devices.sort_by(|a, b| a.alias.cmp(&b.alias));
    devices
}

fn device_property_bool(properties: &HashMap<String, OwnedValue>, name: &str) -> Option<bool> {
    properties
        .get(name)
        .and_then(|value| bool::try_from(value).ok())
}

fn device_property_string(properties: &HashMap<String, OwnedValue>, name: &str) -> Option<String> {
    properties
        .get(name)
        .and_then(|value| <&str as TryFrom<&OwnedValue>>::try_from(value).ok())
        .map(str::to_string)
}

struct PulseSession {
    mainloop: StandardMainloop,
    context: Context,
}

impl PulseSession {
    fn pump<T>(&mut self, receiver: &mpsc::Receiver<T>) -> Result<T, String> {
        for _ in 0..PULSE_TICKS {
            if let Ok(value) = receiver.try_recv() {
                return Ok(value);
            }
            match self.mainloop.iterate(false) {
                IterateResult::Success(_) => {}
                IterateResult::Quit(_) => return Err("pulse mainloop quit".to_string()),
                IterateResult::Err(error) => {
                    return Err(format!("pulse mainloop error: {error:?}"));
                }
            }
            std::thread::sleep(PULSE_TICK);
        }
        Err("pulse server timed out".to_string())
    }

    fn wait_ready(&mut self) -> Result<(), String> {
        for _ in 0..PULSE_TICKS {
            match self.context.get_state() {
                PulseState::Ready => return Ok(()),
                PulseState::Failed | PulseState::Terminated => {
                    return Err("pulse connection failed".to_string());
                }
                _ => {}
            }
            match self.mainloop.iterate(false) {
                IterateResult::Success(_) => {}
                IterateResult::Quit(_) => return Err("pulse mainloop quit".to_string()),
                IterateResult::Err(error) => {
                    return Err(format!("pulse mainloop error: {error:?}"));
                }
            }
            std::thread::sleep(PULSE_TICK);
        }
        Err("pulse server timed out".to_string())
    }

    fn default_sink_name(&mut self) -> Result<String, String> {
        let (sender, receiver) = mpsc::channel();
        self.context.introspect().get_server_info(
            move |info: &libpulse_binding::context::introspect::ServerInfo| {
                let name = info.default_sink_name.as_ref().map(|name| name.to_string());
                let _ = sender.send(name);
            },
        );
        match self.pump(&receiver)? {
            Some(name) => Ok(name),
            None => Err("no default sink".to_string()),
        }
    }

    fn sink_volumes(&mut self, sink: &str) -> Result<(ChannelVolumes, bool), String> {
        let (sender, receiver) = mpsc::channel();
        self.context.introspect().get_sink_info_by_name(
            sink,
            move |result: ListResult<&libpulse_binding::context::introspect::SinkInfo>| match result
            {
                ListResult::Item(info) => {
                    let _ = sender.send(Some((info.volume, info.mute)));
                }
                ListResult::End => {
                    let _ = sender.send(None);
                }
                ListResult::Error => {
                    let _ = sender.send(None);
                }
            },
        );
        match self.pump(&receiver)? {
            Some(volumes) => Ok(volumes),
            None => Err(format!("sink {sink} not found")),
        }
    }

    fn set_sink_volume(&mut self, sink: &str, volumes: &ChannelVolumes) -> Result<(), String> {
        let (sender, receiver) = mpsc::channel();
        self.context.introspect().set_sink_volume_by_name(
            sink,
            volumes,
            Some(Box::new(move |success| {
                let _ = sender.send(success);
            })),
        );
        if self.pump(&receiver)? {
            Ok(())
        } else {
            Err("pulse refused the volume change".to_string())
        }
    }

    fn set_sink_muted(&mut self, sink: &str, muted: bool) -> Result<(), String> {
        let (sender, receiver) = mpsc::channel();
        self.context.introspect().set_sink_mute_by_name(
            sink,
            muted,
            Some(Box::new(move |success| {
                let _ = sender.send(success);
            })),
        );
        if self.pump(&receiver)? {
            Ok(())
        } else {
            Err("pulse refused the mute change".to_string())
        }
    }
}

fn with_pulse<T>(f: impl FnOnce(&mut PulseSession) -> Result<T, String>) -> Result<T, String> {
    let Some(mainloop) = StandardMainloop::new() else {
        return Err("failed to create pulse mainloop".to_string());
    };
    let Some(mut context) = Context::new(&mainloop, PULSE_APP_NAME) else {
        return Err("failed to create pulse context".to_string());
    };
    context
        .connect(None, FlagSet::NOFLAGS, None)
        .map_err(|error| format!("pulse connect failed: {error:?}"))?;
    let mut session = PulseSession { mainloop, context };
    let result = session.wait_ready().and_then(|()| f(&mut session));
    let _ = session.context.disconnect();
    result
}

fn read_volume() -> ControlState<VolumeStatus> {
    let read = || {
        with_pulse(|session| {
            let sink = session.default_sink_name()?;
            let (volumes, muted) = session.sink_volumes(&sink)?;
            Ok(VolumeStatus {
                level: (volumes.avg().0 as f32 / Volume::NORMAL.0 as f32).clamp(0., 1.),
                muted,
            })
        })
    };
    match read() {
        Ok(status) => ControlState::Ready(status),
        Err(error) => unavailable(error),
    }
}
