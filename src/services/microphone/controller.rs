use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;

use gpui_kit::{App, AppContext as _, BorrowAppContext as _, Entity, Global, Task};

use super::demo::DemoSignal;
use super::meter::LevelMeter;
use super::permission;
use super::types::{CaptureStatus, InputDeviceInfo, MicrophonePermissionState};
use crate::runtime::capabilities::CapabilityStatus;
use crate::ui::waveform::{DEFAULT_RETENTION, WaveformState};

const LOG: &str = "gpui_starter::microphone";
const GAIN_MIN_DB: f32 = -24.;
const GAIN_MAX_DB: f32 = 24.;

/// Linear multiplier for the q8 fixed-point gain cell (dB x 256).
pub(crate) fn gain_multiplier(q8: i32) -> f32 {
    10f32.powf(q8 as f32 / 256. / 20.)
}

/// Plain-value view of the microphone feature; the `waveform` entity is the
/// 30 Hz redraw driver while capturing. One instance lives in the
/// [`MicrophoneState`] global.
#[derive(Clone)]
pub struct MicrophoneSnapshot {
    pub devices: Vec<InputDeviceInfo>,
    pub selected_device: Option<String>,
    pub status: CaptureStatus,
    pub permission: MicrophonePermissionState,
    pub gain_db: f32,
    pub demo_only: bool,
    pub waveform: Entity<WaveformState>,
}

#[cfg(not(target_family = "wasm"))]
struct SessionHandle {
    stop: flume::Sender<super::capture::SessionCommand>,
    // Held for ownership: dropping the task cancels the event loop, which
    // drops the runner and the cpal stream with it.
    _task: Task<()>,
}

pub struct MicrophoneState {
    snapshot: MicrophoneSnapshot,
    meter: Arc<LevelMeter>,
    gain_q8: Arc<AtomicI32>,
    demo_task: Option<Task<()>>,
    #[cfg(not(target_family = "wasm"))]
    session: Option<SessionHandle>,
}

impl Global for MicrophoneState {}

/// Installs the global, loads device and gain from AppConfig, registers the
/// `microphone_input` capability, and kicks off enumeration (native). On wasm
/// the snapshot runs demo-only with a synthetic signal.
pub fn initialize(cx: &mut App) {
    let waveform = cx.new(|_| WaveformState::new());
    let (selected_device, gain_db) = crate::app_state::with_config(cx, |config| {
        (
            config.mic_device_id.clone(),
            config.mic_gain_db.clamp(GAIN_MIN_DB, GAIN_MAX_DB),
        )
    });
    #[cfg(target_family = "wasm")]
    let demo_only = true;
    #[cfg(not(target_family = "wasm"))]
    let demo_only = false;
    cx.set_global(MicrophoneState {
        snapshot: MicrophoneSnapshot {
            devices: Vec::new(),
            selected_device,
            status: CaptureStatus::Idle,
            permission: permission::status(),
            gain_db,
            demo_only,
            waveform,
        },
        meter: Arc::new(LevelMeter::new()),
        gain_q8: Arc::new(AtomicI32::new((gain_db * 256.) as i32)),
        demo_task: None,
        #[cfg(not(target_family = "wasm"))]
        session: None,
    });
    #[cfg(target_family = "wasm")]
    crate::capabilities::set(
        "microphone_input",
        CapabilityStatus {
            supported: false,
            enabled: false,
            degraded: true,
            reason: Some(crate::i18n::localize("microphone_demo_note").into()),
            last_error: None,
        },
        cx,
    );
    #[cfg(not(target_family = "wasm"))]
    {
        crate::capabilities::set(
            "microphone_input",
            CapabilityStatus::supported_enabled(),
            cx,
        );
        refresh_devices(cx);
    }
}

pub fn snapshot(cx: &App) -> MicrophoneSnapshot {
    cx.global::<MicrophoneState>().snapshot.clone()
}

pub fn meter(cx: &App) -> Arc<LevelMeter> {
    cx.global::<MicrophoneState>().meter.clone()
}

/// Native: re-enumerates input devices on the background executor. Wasm: no-op.
pub fn refresh_devices(cx: &mut App) {
    #[cfg(not(target_family = "wasm"))]
    {
        cx.spawn(async move |cx| {
            let devices = cx
                .background_spawn(async move { super::capture::enumerate() })
                .await;
            cx.update(|cx| match devices {
                Ok(devices) => {
                    let empty = devices.is_empty();
                    mutate(cx, |state| state.snapshot.devices = devices);
                    if empty {
                        crate::capabilities::set(
                            "microphone_input",
                            CapabilityStatus::degraded("no input devices"),
                            cx,
                        );
                    } else {
                        crate::capabilities::set(
                            "microphone_input",
                            CapabilityStatus::supported_enabled(),
                            cx,
                        );
                    }
                }
                Err(err) => {
                    mutate(cx, |state| state.snapshot.devices = Vec::new());
                    crate::capabilities::set(
                        "microphone_input",
                        CapabilityStatus::degraded(err),
                        cx,
                    );
                }
            });
        })
        .detach();
    }
    #[cfg(target_family = "wasm")]
    {
        let _ = cx;
    }
}

/// Persists the choice; a running session is unaffected (selection UI is
/// disabled while capturing).
pub fn select_device(id: Option<String>, cx: &mut App) {
    crate::app_state::update_config(cx, |config| config.mic_device_id = id.clone());
    mutate(cx, |state| state.snapshot.selected_device = id);
}

/// Live gain for slider drags: clamps, stores the audio-side cell, updates
/// the label state. No persistence; the drag runs at pointer-move frequency.
pub fn set_gain_db_live(db: f32, cx: &mut App) {
    let db = db.clamp(GAIN_MIN_DB, GAIN_MAX_DB);
    if cx.global::<MicrophoneState>().snapshot.gain_db == db {
        return;
    }
    let gain_q8 = cx.global::<MicrophoneState>().gain_q8.clone();
    gain_q8.store((db * 256.) as i32, Ordering::Relaxed);
    mutate(cx, |state| state.snapshot.gain_db = db);
}

/// Clamps to [-24, +24] dB, persists, and stores the live gain the audio
/// callback reads per invocation: no session restart.
pub fn set_gain_db(db: f32, cx: &mut App) {
    let db = db.clamp(GAIN_MIN_DB, GAIN_MAX_DB);
    crate::app_state::update_config(cx, |config| config.mic_gain_db = db);
    let gain_q8 = cx.global::<MicrophoneState>().gain_q8.clone();
    gain_q8.store((db * 256.) as i32, Ordering::Relaxed);
    mutate(cx, |state| state.snapshot.gain_db = db);
}

/// Borrowed device list for hot observers; avoids cloning the snapshot.
pub fn devices(cx: &App) -> &[InputDeviceInfo] {
    &cx.global::<MicrophoneState>().snapshot.devices
}

/// Borrowed status for hot observers; avoids cloning the snapshot.
pub fn status(cx: &App) -> &CaptureStatus {
    &cx.global::<MicrophoneState>().snapshot.status
}

/// Borrowed permission for hot observers; avoids cloning the snapshot.
pub fn permission_state(cx: &App) -> &MicrophonePermissionState {
    &cx.global::<MicrophoneState>().snapshot.permission
}

/// Current gain in dB (label state; the audio side reads the q8 cell).
pub fn gain_db(cx: &App) -> f32 {
    cx.global::<MicrophoneState>().snapshot.gain_db
}

/// True when only the synthetic demo signal is available (wasm).
pub fn demo_only(cx: &App) -> bool {
    cx.global::<MicrophoneState>().snapshot.demo_only
}

pub fn refresh_permission(cx: &mut App) {
    let status = permission::status();
    mutate(cx, |state| state.snapshot.permission = status);
}

/// macOS only (TCC); no-op elsewhere.
pub fn request_permission(cx: &mut App) {
    cx.spawn(async move |cx| {
        let status = permission::request().await;
        cx.update(|cx| mutate(cx, |state| state.snapshot.permission = status));
    })
    .detach();
}

/// Wasm: runs the synthetic demo signal. Native: starts the cpal session,
/// gated on TCC permission for macOS (NotDetermined requests first).
pub fn start(cx: &mut App) {
    if matches!(
        cx.global::<MicrophoneState>().snapshot.status,
        CaptureStatus::Starting | CaptureStatus::Capturing { .. }
    ) {
        return;
    }
    if cx.global::<MicrophoneState>().snapshot.demo_only {
        start_demo(cx);
        return;
    }
    #[cfg(not(target_family = "wasm"))]
    {
        #[cfg(target_os = "macos")]
        match cx.global::<MicrophoneState>().snapshot.permission {
            MicrophonePermissionState::Authorized => spawn_session(cx),
            MicrophonePermissionState::NotDetermined => {
                mutate(cx, |state| state.snapshot.status = CaptureStatus::Starting);
                cx.spawn(async move |cx| {
                    let granted = matches!(
                        permission::request().await,
                        MicrophonePermissionState::Authorized
                    );
                    cx.update(|cx| {
                        refresh_permission(cx);
                        if granted {
                            spawn_session(cx);
                        } else {
                            mutate(cx, |state| state.snapshot.status = CaptureStatus::Idle);
                        }
                    });
                })
                .detach();
            }
            _ => {}
        }
        #[cfg(not(target_os = "macos"))]
        spawn_session(cx);
    }
}

/// Always runs to completion: signals the session, drops the task (dropping
/// the stream), detaches the source, resets the meter, sets Idle.
pub fn stop(cx: &mut App) {
    let state = cx.global::<MicrophoneState>();
    let demo_only = state.snapshot.demo_only;
    let waveform = state.snapshot.waveform.clone();
    let meter = state.meter.clone();
    if demo_only {
        mutate(cx, |state| state.demo_task = None);
    } else {
        #[cfg(not(target_family = "wasm"))]
        {
            let mut handle = None;
            mutate(cx, |state| handle = state.session.take());
            if let Some(handle) = handle {
                let _ = handle.stop.send(super::capture::SessionCommand::Stop);
            }
        }
    }
    waveform.update(cx, |state, cx| state.detach_source(cx));
    meter.reset();
    mutate(cx, |state| state.snapshot.status = CaptureStatus::Idle);
}

/// Opens the OS privacy pane for microphone access; failures are logged.
pub fn open_microphone_settings(_cx: &mut App) {
    if let Err(err) = permission::open_microphone_settings() {
        tracing::warn!(target: LOG, error = %err, "failed to open microphone settings");
    }
}

fn start_demo(cx: &mut App) {
    let state = cx.global::<MicrophoneState>();
    let waveform = state.snapshot.waveform.clone();
    let meter = state.meter.clone();
    let gain_q8 = state.gain_q8.clone();
    let mut producer = waveform.update(cx, |state, cx| {
        state.attach_source(48_000, 2, DEFAULT_RETENTION, cx)
    });
    let task = cx.spawn(async move |cx| {
        let mut signal = DemoSignal::new();
        let mut buf = Vec::new();
        loop {
            cx.background_executor()
                .timer(Duration::from_millis(33))
                .await;
            let (rms, peak) = signal.fill_chunk(&mut buf, 1584);
            let gain = gain_multiplier(gain_q8.load(Ordering::Relaxed));
            if gain != 1. {
                for sample in &mut buf {
                    *sample *= gain;
                }
            }
            meter.update(rms * gain, peak * gain);
            producer.push_interleaved(&buf);
            buf.clear();
            if !producer.is_attached() {
                break;
            }
        }
    });
    mutate(cx, |state| {
        state.demo_task = Some(task);
        state.snapshot.status = CaptureStatus::Capturing {
            device: crate::i18n::localize("microphone_demo_device"),
        };
    });
}

#[cfg(not(target_family = "wasm"))]
fn spawn_session(cx: &mut App) {
    use super::capture::{self, SessionEvent};

    let state = cx.global::<MicrophoneState>();
    let Some(info) = capture::pick_device(
        state.snapshot.devices.iter(),
        state.snapshot.selected_device.as_deref(),
    ) else {
        let message = crate::i18n::localize("microphone_no_devices");
        mutate(cx, |state| {
            state.snapshot.status = CaptureStatus::Error(message)
        });
        return;
    };
    let device_id = info.id.clone();
    let device_name = info.name.clone();
    let channels = (info.channels as usize).clamp(1, 2);
    let sample_rate = info.sample_rate;
    let meter = state.meter.clone();
    let gain_q8 = state.gain_q8.clone();
    let waveform = state.snapshot.waveform.clone();
    let resolved = device_id.clone();
    let producer = waveform.update(cx, |state, cx| {
        state.attach_source(sample_rate, channels, DEFAULT_RETENTION, cx)
    });
    if cx
        .global::<MicrophoneState>()
        .snapshot
        .selected_device
        .as_deref()
        != Some(resolved.as_str())
    {
        // A stale or absent persisted id resolved to a live device: adopt it
        // so the next launch selects what actually captured.
        crate::app_state::update_config(cx, |config| config.mic_device_id = Some(resolved.clone()));
        mutate(cx, |state| {
            state.snapshot.selected_device = Some(resolved);
        });
    }
    let (cmd_tx, cmd_rx) = flume::bounded(1);
    let (ev_tx, ev_rx) = flume::unbounded();
    mutate(cx, |state| state.snapshot.status = CaptureStatus::Starting);
    let task = cx.spawn(async move |cx| {
        let runner = cx.background_spawn(capture::run_session(
            device_id,
            producer,
            meter.clone(),
            gain_q8,
            cmd_rx,
            ev_tx,
        ));
        while let Ok(event) = ev_rx.recv_async().await {
            match event {
                SessionEvent::Started => {
                    let device = device_name.clone();
                    cx.update(|cx| {
                        mutate(cx, |state| {
                            state.snapshot.status = CaptureStatus::Capturing { device }
                        });
                    });
                }
                SessionEvent::Failed(err) => {
                    cx.update(|cx| {
                        let waveform = cx.global::<MicrophoneState>().snapshot.waveform.clone();
                        waveform.update(cx, |state, cx| state.detach_source(cx));
                        cx.global::<MicrophoneState>().meter.reset();
                        mutate(cx, |state| {
                            state.snapshot.status = CaptureStatus::Error(err)
                        });
                    });
                    break;
                }
                SessionEvent::Stopped => {
                    cx.update(|cx| {
                        mutate(cx, |state| state.snapshot.status = CaptureStatus::Idle);
                    });
                    break;
                }
            }
        }
        drop(runner);
    });
    mutate(cx, |state| {
        state.session = Some(SessionHandle {
            stop: cmd_tx,
            _task: task,
        });
    });
}

fn mutate(cx: &mut App, f: impl FnOnce(&mut MicrophoneState)) {
    cx.update_global::<MicrophoneState, _>(|state, cx| {
        f(state);
        cx.refresh_windows();
    });
}

#[cfg(test)]
pub(crate) fn force_demo_for_tests(cx: &mut App) {
    mutate(cx, |state| state.snapshot.demo_only = true);
}

#[cfg(test)]
pub(crate) fn seed_devices_for_tests(cx: &mut App, devices: Vec<InputDeviceInfo>) {
    mutate(cx, |state| state.snapshot.devices = devices);
}

#[cfg(test)]
#[path = "controller.test.rs"]
mod controller_test;
