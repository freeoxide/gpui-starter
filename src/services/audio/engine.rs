use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use gpui_kit::*;

use super::decode::{AudioError, DecodedAudio, decode_bytes};
#[cfg(not(target_family = "wasm"))]
use super::output::{self, OutputSession};
use super::peaks::WaveformPeaks;
use super::transport::SharedTransport;
use crate::ui::waveform::{DEFAULT_RETENTION, SampleProducer, WaveformState};

const TICK_INTERVAL: Duration = Duration::from_millis(33);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaybackStatus {
    Loading,
    Stopped,
    Playing,
    Paused,
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct AudioMeta {
    pub source_name: String,
    pub sample_rate: u32,
    pub channels: usize,
    pub duration: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputDevice {
    pub name: String,
    pub is_default: bool,
}

struct LoadState {
    meta: AudioMeta,
    decoded: Arc<DecodedAudio>,
    render: Arc<Vec<f32>>,
    render_rate: u32,
    render_channels: usize,
    #[cfg(not(target_family = "wasm"))]
    plan: Option<super::output::StreamPlan>,
    peaks: Arc<WaveformPeaks>,
    transport: Arc<SharedTransport>,
    producer: Option<SampleProducer>,
}

struct PreparedRender {
    render: Arc<Vec<f32>>,
    render_rate: u32,
    render_channels: usize,
    #[cfg(not(target_family = "wasm"))]
    plan: Option<super::output::StreamPlan>,
    plan_error: Option<String>,
}

enum PlayAction {
    Pause,
    Resume,
    RestartAndResume,
    None,
}

/// Playback engine. All mutators end with exactly one `cx.notify()`;
/// consumers re-render via `cx.observe(&engine, |_, _, cx| cx.notify())`.
pub struct AudioEngine {
    load: Option<LoadState>,
    status: Option<PlaybackStatus>,
    volume: f32,
    devices: Vec<OutputDevice>,
    selected_device: Option<String>,
    meter: Option<Entity<WaveformState>>,
    load_generation: u64,
    loading: Option<String>,
    last_error: Option<String>,
    cached_position_ms: u64,
    _tick: Task<()>,
    #[cfg(not(target_family = "wasm"))]
    session: Option<OutputSession>,
    #[cfg(not(target_family = "wasm"))]
    pending_play: bool,
    #[cfg(not(target_family = "wasm"))]
    session_generation: u64,
}

impl AudioEngine {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let tick = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(TICK_INTERVAL).await;
                if this
                    .update(cx, |engine, cx| engine.refresh_from_transport(cx))
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            load: None,
            status: None,
            volume: 0.8,
            devices: Vec::new(),
            selected_device: None,
            meter: None,
            load_generation: 0,
            loading: None,
            last_error: None,
            cached_position_ms: 0,
            _tick: tick,
            #[cfg(not(target_family = "wasm"))]
            session: None,
            #[cfg(not(target_family = "wasm"))]
            pending_play: false,
            #[cfg(not(target_family = "wasm"))]
            session_generation: 0,
        }
    }

    /// Async load: sets Loading, decodes and builds peaks off-thread, then
    /// prepares the render buffer. Replaces any previous load; decode
    /// failures land in `last_error` and leave the previous load intact.
    pub fn load_bytes(&mut self, name: &str, bytes: Vec<u8>, cx: &mut Context<Self>) {
        let owned_name = name.to_string();
        let task =
            cx.background_spawn(async move { decode_bytes(&owned_name, bytes).map(with_peaks) });
        self.start_load(cx, name.to_string(), task);
        cx.notify();
    }

    /// Native only; wasm callers get `last_error()` = Unavailable.
    #[cfg(not(target_family = "wasm"))]
    pub fn load_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let owned_name = name.clone();
        let task = cx.background_spawn(async move {
            std::fs::read(&path)
                .map_err(|err| AudioError::Decode(err.to_string()))
                .and_then(|bytes| decode_bytes(&owned_name, bytes))
                .map(with_peaks)
        });
        self.start_load(cx, name, task);
        cx.notify();
    }

    pub fn unload(&mut self, cx: &mut Context<Self>) {
        self.load_generation += 1;
        self.loading = None;
        self.last_error = None;
        #[cfg(not(target_family = "wasm"))]
        {
            self.session = None;
            self.session_generation += 1;
            self.pending_play = false;
        }
        self.load = None;
        self.status = None;
        self.cached_position_ms = 0;
        self.detach_meter_source(cx);
        cx.notify();
    }

    /// Stopped at EOF restarts from 0.
    pub fn toggle_play(&mut self, cx: &mut Context<Self>) {
        #[cfg(not(target_family = "wasm"))]
        {
            let action = self.play_action();
            match action {
                PlayAction::Pause => {
                    let mut paused = true;
                    if let Some(session) = &self.session {
                        if let Err(err) = session.pause() {
                            self.last_error = Some(err.to_string());
                            paused = false;
                        }
                    }
                    if paused {
                        if let Some(load) = &self.load {
                            load.transport.set_playing(false);
                        }
                        self.status = Some(PlaybackStatus::Paused);
                        self.last_error = None;
                    }
                }
                PlayAction::Resume | PlayAction::RestartAndResume => {
                    if matches!(action, PlayAction::RestartAndResume) {
                        if let Some(load) = &self.load {
                            load.transport.set_cursor(0);
                        }
                    }
                    if self.resume_session(cx) {
                        self.last_error = None;
                    }
                }
                PlayAction::None => {}
            }
        }
        #[cfg(target_family = "wasm")]
        {
            self.last_error = Some(AudioError::Unavailable.to_string());
        }
        cx.notify();
    }

    pub fn stop(&mut self, cx: &mut Context<Self>) {
        #[cfg(not(target_family = "wasm"))]
        if let Some(session) = &self.session {
            if let Err(err) = session.pause() {
                self.last_error = Some(err.to_string());
            }
        }
        if let Some(load) = &self.load {
            load.transport.set_playing(false);
            load.transport.set_cursor(0);
        }
        if self.load.is_some() {
            self.status = Some(PlaybackStatus::Stopped);
        }
        self.cached_position_ms = 0;
        cx.notify();
    }

    /// Seek to `fraction` of the render, clamped 0..=1. Works while paused.
    pub fn seek_fraction(&mut self, fraction: f32, cx: &mut Context<Self>) {
        let Some(load) = &self.load else {
            return;
        };
        let frames = (load.render.len() / load.render_channels.max(1)) as u64;
        let frame = (fraction.clamp(0., 1.) * frames as f32) as u64;
        load.transport.set_cursor(frame);
        self.last_error = None;
        cx.notify();
    }

    /// Clamps to 0..=1; applied per fill by the output callback.
    pub fn set_volume(&mut self, volume: f32, cx: &mut Context<Self>) {
        self.volume = volume.clamp(0., 1.);
        if let Some(load) = &self.load {
            load.transport.set_volume(self.volume);
        }
        cx.notify();
    }

    pub fn status(&self) -> Option<PlaybackStatus> {
        self.status
    }

    pub fn position(&self) -> Duration {
        match &self.load {
            Some(load) => Duration::from_secs_f32(
                load.transport.cursor_frames() as f32 / load.render_rate as f32,
            ),
            None => Duration::ZERO,
        }
    }

    pub fn duration(&self) -> Option<Duration> {
        self.load.as_ref().map(|load| {
            let frames = load.render.len() / load.render_channels.max(1);
            Duration::from_secs_f32(frames as f32 / load.render_rate as f32)
        })
    }

    pub fn progress(&self) -> f32 {
        let Some(load) = &self.load else {
            return 0.;
        };
        let frames = (load.render.len() / load.render_channels.max(1)) as u64;
        if frames == 0 {
            0.
        } else {
            (load.transport.cursor_frames() as f32 / frames as f32).clamp(0., 1.)
        }
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn meta(&self) -> Option<AudioMeta> {
        self.load.as_ref().map(|load| load.meta.clone())
    }

    pub fn peaks(&self) -> Option<Arc<WaveformPeaks>> {
        self.load.as_ref().map(|load| load.peaks.clone())
    }

    /// Read once per render; cleared on the next successful action.
    pub fn last_error(&self) -> Option<String> {
        self.last_error.clone()
    }

    pub fn devices(&self) -> Vec<OutputDevice> {
        self.devices.clone()
    }

    pub fn selected_device(&self) -> Option<String> {
        self.selected_device.clone()
    }

    /// Native: re-enumerates output devices on the background executor
    /// (the host device scan is synchronous and slow). Wasm: no-op.
    pub fn refresh_devices(&mut self, cx: &mut Context<Self>) {
        #[cfg(not(target_family = "wasm"))]
        cx.spawn(async move |this, cx| {
            let devices = cx
                .background_spawn(async move { output::enumerate() })
                .await;
            this.update(cx, |engine, cx| {
                engine.devices = devices;
                cx.notify();
            })
            .ok();
        })
        .detach();
        #[cfg(target_family = "wasm")]
        {
            let _ = cx;
        }
    }

    /// Rebuilds the session on `name` at the saved position. The device
    /// query, channel adapt, and resample all run off-thread.
    pub fn select_device(&mut self, name: &str, cx: &mut Context<Self>) {
        #[cfg(not(target_family = "wasm"))]
        {
            if self.selected_device.as_deref() == Some(name)
                || !self.devices.iter().any(|device| device.name == name)
            {
                return;
            }
            let decoded = self.load.as_ref().map(|load| load.decoded.clone());
            let was_playing = self
                .load
                .as_ref()
                .is_some_and(|load| load.transport.is_playing());
            let position_secs = self.position_secs();
            self.session = None;
            self.session_generation += 1;
            self.selected_device = Some(name.to_string());
            if let Some(decoded) = decoded {
                let generation = self.load_generation;
                let task = self.spawn_prepare(&decoded, Some(name.to_string()), cx);
                cx.spawn(async move |this, cx| {
                    let prepared = task.await;
                    this.update(cx, |engine, cx| {
                        engine.finish_replan(
                            generation,
                            decoded,
                            prepared,
                            position_secs,
                            was_playing,
                            cx,
                        );
                    })
                    .ok();
                })
                .detach();
            }
        }
        #[cfg(target_family = "wasm")]
        {
            let _ = name;
        }
        cx.notify();
    }

    /// Live metering: attaches plan 16's widget on the current load (or the
    /// next one to land) and feeds it from the output callback.
    pub fn attach_meter(&mut self, meter: &Entity<WaveformState>, cx: &mut Context<Self>) {
        self.meter = Some(meter.clone());
        if self.load.is_some() {
            self.reconnect_meter(cx);
        }
    }

    fn refresh_from_transport(&mut self, cx: &mut Context<Self>) {
        let Some(load) = &self.load else {
            return;
        };
        let position_ms = (load.transport.cursor_frames() as f64 / load.render_rate as f64 * 1000.)
            .round() as u64;
        let mut changed = position_ms != self.cached_position_ms;
        self.cached_position_ms = position_ms;
        if load.transport.take_eof()
            && self
                .status
                .is_some_and(|status| status != PlaybackStatus::Stopped)
        {
            self.status = Some(PlaybackStatus::Stopped);
            changed = true;
        } else if self.status == Some(PlaybackStatus::Playing) && !load.transport.is_playing() {
            self.status = Some(PlaybackStatus::Paused);
            changed = true;
        }
        if changed {
            cx.notify();
        }
    }

    fn start_load(
        &mut self,
        cx: &mut Context<Self>,
        name: String,
        task: Task<Result<(Arc<DecodedAudio>, Arc<WaveformPeaks>), AudioError>>,
    ) {
        self.load_generation += 1;
        let generation = self.load_generation;
        self.loading = Some(name);
        self.last_error = None;
        self.status = Some(PlaybackStatus::Loading);
        #[cfg(not(target_family = "wasm"))]
        {
            self.session = None;
            self.session_generation += 1;
            self.pending_play = false;
        }
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |engine, cx| engine.finish_load(generation, result, cx))
                .ok();
        })
        .detach();
    }

    fn finish_load(
        &mut self,
        generation: u64,
        result: Result<(Arc<DecodedAudio>, Arc<WaveformPeaks>), AudioError>,
        cx: &mut Context<Self>,
    ) {
        if generation != self.load_generation {
            return;
        }
        self.loading = None;
        match result {
            Err(err) => {
                self.last_error = Some(err.to_string());
                if self.load.is_some() {
                    self.status = Some(PlaybackStatus::Stopped);
                    #[cfg(not(target_family = "wasm"))]
                    {
                        self.reconnect_meter(cx);
                        self.connect_session(cx);
                    }
                } else {
                    self.status = None;
                }
            }
            Ok((decoded, peaks)) => {
                let task = self.spawn_prepare(&decoded, self.selected_device.clone(), cx);
                cx.spawn(async move |this, cx| {
                    let prepared = task.await;
                    this.update(cx, |engine, cx| {
                        engine.finish_prepare(generation, decoded, peaks, prepared, cx)
                    })
                    .ok();
                })
                .detach();
            }
        }
        cx.notify();
    }

    fn finish_prepare(
        &mut self,
        generation: u64,
        decoded: Arc<DecodedAudio>,
        peaks: Arc<WaveformPeaks>,
        prepared: PreparedRender,
        cx: &mut Context<Self>,
    ) {
        if generation != self.load_generation {
            return;
        }
        self.detach_meter_source(cx);
        let meta = AudioMeta {
            source_name: decoded.source_name().to_string(),
            sample_rate: decoded.sample_rate(),
            channels: decoded.channels(),
            duration: decoded.duration(),
        };
        self.load = Some(LoadState {
            meta,
            decoded,
            render: prepared.render,
            render_rate: prepared.render_rate,
            render_channels: prepared.render_channels,
            #[cfg(not(target_family = "wasm"))]
            plan: prepared.plan,
            peaks,
            transport: Arc::new(SharedTransport::new(self.volume)),
            producer: None,
        });
        if let Some(err) = prepared.plan_error {
            self.last_error = Some(err);
        }
        self.status = Some(PlaybackStatus::Stopped);
        self.cached_position_ms = 0;
        self.reconnect_meter(cx);
        #[cfg(not(target_family = "wasm"))]
        self.connect_session(cx);
        cx.notify();
    }

    #[cfg(not(target_family = "wasm"))]
    fn finish_replan(
        &mut self,
        generation: u64,
        decoded: Arc<DecodedAudio>,
        prepared: PreparedRender,
        position_secs: f64,
        was_playing: bool,
        cx: &mut Context<Self>,
    ) {
        if generation != self.load_generation {
            return;
        }
        let replanable = self
            .load
            .as_ref()
            .is_some_and(|load| Arc::ptr_eq(&load.decoded, &decoded));
        if !replanable {
            return;
        }
        if let Some(err) = &prepared.plan_error {
            self.last_error = Some(err.clone());
        }
        if let Some(load) = &mut self.load {
            load.render = prepared.render;
            load.render_rate = prepared.render_rate;
            load.render_channels = prepared.render_channels;
            load.plan = prepared.plan;
            load.transport
                .set_cursor((position_secs * prepared.render_rate as f64) as u64);
            load.transport.set_playing(false);
        }
        self.reconnect_meter(cx);
        if was_playing {
            self.pending_play = true;
        }
        self.connect_session(cx);
        cx.notify();
    }

    fn spawn_prepare(
        &mut self,
        decoded: &Arc<DecodedAudio>,
        #[cfg(not(target_family = "wasm"))] wanted: Option<String>,
        #[cfg(target_family = "wasm")] wanted: Option<String>,
        cx: &mut Context<Self>,
    ) -> Task<PreparedRender> {
        let decoded = decoded.clone();
        #[cfg(not(target_family = "wasm"))]
        {
            cx.background_spawn(async move {
                let plan = output::find_output_device(wanted.as_deref())
                    .and_then(|device| output::plan_stream(&device, decoded.sample_rate()));
                let (out_rate, out_channels) = plan
                    .as_ref()
                    .map_or((decoded.sample_rate(), decoded.channels()), |plan| {
                        (plan.sample_rate, plan.channels as usize)
                    });
                let adapted =
                    output::adapt_to_channels(decoded.samples(), decoded.channels(), out_channels);
                let render = Arc::new(output::resample_linear(
                    &adapted,
                    out_channels,
                    decoded.sample_rate(),
                    out_rate,
                ));
                let plan_error = plan.as_ref().err().map(|err| err.to_string());
                PreparedRender {
                    render,
                    render_rate: out_rate,
                    render_channels: out_channels,
                    plan: plan.ok(),
                    plan_error,
                }
            })
        }
        #[cfg(target_family = "wasm")]
        {
            let _ = wanted;
            cx.background_spawn(async move {
                let render = Arc::new(decoded.samples().to_vec());
                PreparedRender {
                    render,
                    render_rate: decoded.sample_rate(),
                    render_channels: decoded.channels(),
                    plan_error: None,
                }
            })
        }
    }

    fn reconnect_meter(&mut self, cx: &mut Context<Self>) {
        let (rate, channels) = match &self.load {
            Some(load) => (load.render_rate, load.render_channels),
            None => return,
        };
        let Some(meter) = &self.meter else {
            return;
        };
        let producer = meter.update(cx, |state, cx| {
            state.attach_source(rate, channels, DEFAULT_RETENTION, cx)
        });
        if let Some(load) = &mut self.load {
            load.producer = Some(producer);
        }
    }

    fn detach_meter_source(&mut self, cx: &mut Context<Self>) {
        if let Some(meter) = &self.meter {
            meter.update(cx, |state, cx| state.detach_source(cx));
        }
    }

    #[cfg(not(target_family = "wasm"))]
    fn play_action(&self) -> PlayAction {
        let Some(load) = &self.load else {
            return PlayAction::None;
        };
        match self.status {
            Some(PlaybackStatus::Playing) => PlayAction::Pause,
            Some(PlaybackStatus::Paused) => PlayAction::Resume,
            Some(PlaybackStatus::Stopped) => {
                let frames = (load.render.len() / load.render_channels.max(1)) as u64;
                if load.transport.cursor_frames() >= frames && frames > 0 {
                    PlayAction::RestartAndResume
                } else {
                    PlayAction::Resume
                }
            }
            _ => PlayAction::None,
        }
    }

    /// Returns true only when playback actually resumed; a queued build
    /// (session still absent) is not yet a success.
    #[cfg(not(target_family = "wasm"))]
    fn resume_session(&mut self, cx: &mut Context<Self>) -> bool {
        match &self.session {
            Some(session) => {
                if let Err(err) = session.play() {
                    self.last_error = Some(err.to_string());
                    return false;
                }
                if let Some(load) = &self.load {
                    load.transport.set_playing(true);
                }
                self.status = Some(PlaybackStatus::Playing);
                true
            }
            None => {
                self.pending_play = true;
                self.connect_session(cx);
                self.status == Some(PlaybackStatus::Playing)
            }
        }
    }

    #[cfg(not(target_family = "wasm"))]
    fn connect_session(&mut self, cx: &mut Context<Self>) {
        let (render, render_channels, transport, source_rate, plan) = match &self.load {
            Some(load) => (
                load.render.clone(),
                load.render_channels,
                load.transport.clone(),
                load.decoded.sample_rate(),
                load.plan.clone(),
            ),
            None => return,
        };
        let producer = match self.load.as_mut().and_then(|load| load.producer.take()) {
            Some(producer) => producer,
            None => return,
        };
        let wanted = self.selected_device.clone();
        self.session_generation += 1;
        let token = self.session_generation;
        let build_transport = transport.clone();
        let task = cx.background_spawn(async move {
            let device = output::find_output_device(wanted.as_deref())?;
            let plan = match plan {
                Some(plan) => plan,
                None => output::plan_stream(&device, source_rate)?,
            };
            output::build_session(
                &device,
                render,
                render_channels,
                plan,
                build_transport,
                producer,
            )
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |engine, cx| {
                engine.receive_session(token, transport, result, cx)
            })
            .ok();
        })
        .detach();
    }

    #[cfg(not(target_family = "wasm"))]
    fn receive_session(
        &mut self,
        token: u64,
        transport: Arc<SharedTransport>,
        result: Result<OutputSession, AudioError>,
        cx: &mut Context<Self>,
    ) {
        if token != self.session_generation {
            return;
        }
        let stale = !self
            .load
            .as_ref()
            .is_some_and(|load| Arc::ptr_eq(&load.transport, &transport));
        if stale {
            return;
        }
        match result {
            Ok(session) => {
                self.session = Some(session);
                transport.set_playing(false);
                if self.pending_play {
                    self.pending_play = false;
                    let played = self
                        .session
                        .as_ref()
                        .map(|session| session.play().map_err(|err| err.to_string()));
                    match played {
                        Some(Ok(())) => {
                            transport.set_playing(true);
                            self.status = Some(PlaybackStatus::Playing);
                            self.last_error = None;
                        }
                        Some(Err(err)) => self.last_error = Some(err),
                        None => {}
                    }
                }
            }
            Err(err) => {
                self.last_error = Some(err.to_string());
                self.reconnect_meter(cx);
            }
        }
        cx.notify();
    }

    fn position_secs(&self) -> f64 {
        self.load
            .as_ref()
            .map(|load| load.transport.cursor_frames() as f64 / load.render_rate as f64)
            .unwrap_or(0.)
    }
}

fn with_peaks(decoded: DecodedAudio) -> (Arc<DecodedAudio>, Arc<WaveformPeaks>) {
    let peaks = Arc::new(WaveformPeaks::from_interleaved(
        decoded.samples(),
        decoded.channels(),
    ));
    (Arc::new(decoded), peaks)
}

#[cfg(test)]
#[path = "engine.test.rs"]
mod engine_test;
