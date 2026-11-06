#[cfg(not(target_family = "wasm"))]
use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use gpui_kit::*;

use super::decode::{AudioError, DecodedAudio, decode_bytes};
#[cfg(not(target_family = "wasm"))]
use super::output;
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
    /// Stable cpal device key; two devices may share a display name.
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

struct LoadState {
    meta: AudioMeta,
    decoded: Arc<DecodedAudio>,
    render: Arc<[f32]>,
    render_rate: u32,
    render_channels: usize,
    #[cfg(not(target_family = "wasm"))]
    plan: Option<super::output::StreamPlan>,
    peaks: Arc<WaveformPeaks>,
    transport: Arc<SharedTransport>,
    producer: Option<SampleProducer>,
}

struct PreparedRender {
    render: Arc<[f32]>,
    render_rate: u32,
    render_channels: usize,
    #[cfg(not(target_family = "wasm"))]
    plan: Option<super::output::StreamPlan>,
    plan_error: Option<AudioError>,
}

/// Everything a session build needs; the builder runs off-thread.
#[cfg(not(target_family = "wasm"))]
struct SessionRequest {
    wanted_device: Option<String>,
    render: Arc<[f32]>,
    render_channels: usize,
    source_rate: u32,
    plan: Option<super::output::StreamPlan>,
    transport: Arc<SharedTransport>,
    producer: Option<SampleProducer>,
}

#[cfg(not(target_family = "wasm"))]
type SessionBuilder =
    Arc<dyn Fn(SessionRequest) -> Result<Arc<dyn output::SessionSink>, AudioError> + Send + Sync>;

#[cfg(not(target_family = "wasm"))]
fn native_session(request: SessionRequest) -> Result<Arc<dyn output::SessionSink>, AudioError> {
    let device = output::find_output_device(request.wanted_device.as_deref())?;
    let plan = match request.plan {
        Some(plan) => plan,
        None => output::plan_stream(&device, request.source_rate)?,
    };
    output::build_session(
        &device,
        request.render,
        request.render_channels,
        plan,
        request.transport,
        request.producer,
    )
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
    last_error: Option<AudioError>,
    cached_position_ms: u64,
    tick: Option<Task<()>>,
    #[cfg(not(target_family = "wasm"))]
    session: Option<Arc<dyn output::SessionSink>>,
    #[cfg(not(target_family = "wasm"))]
    session_builder: SessionBuilder,
    #[cfg(not(target_family = "wasm"))]
    pending_play: bool,
    #[cfg(not(target_family = "wasm"))]
    session_generation: u64,
}

impl AudioEngine {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            load: None,
            status: None,
            volume: 0.8,
            devices: Vec::new(),
            selected_device: None,
            meter: None,
            load_generation: 0,
            last_error: None,
            cached_position_ms: 0,
            tick: Some(Self::spawn_tick(cx)),
            #[cfg(not(target_family = "wasm"))]
            session: None,
            #[cfg(not(target_family = "wasm"))]
            session_builder: Arc::new(native_session),
            #[cfg(not(target_family = "wasm"))]
            pending_play: false,
            #[cfg(not(target_family = "wasm"))]
            session_generation: 0,
        }
    }

    fn spawn_tick(cx: &mut Context<Self>) -> Task<()> {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(TICK_INTERVAL).await;
                let still_live = this
                    .update(cx, |engine, cx| engine.refresh_from_transport(cx))
                    .unwrap_or(false);
                if !still_live {
                    break;
                }
            }
        })
    }

    /// Restarts the tick after it parked itself; call whenever playback starts.
    fn wake_tick(&mut self, cx: &mut Context<Self>) {
        if self.tick.is_none() {
            self.tick = Some(Self::spawn_tick(cx));
        }
    }

    /// Sets Loading, decodes and builds peaks off-thread, then prepares the render.
    /// Success replaces the current load; a decode failure keeps it and sets `last_error`.
    pub fn load_bytes(&mut self, name: &str, bytes: Vec<u8>, cx: &mut Context<Self>) {
        let owned_name = name.to_string();
        let task =
            cx.background_spawn(async move { decode_bytes(&owned_name, bytes).map(with_peaks) });
        self.start_load(cx, task);
        cx.notify();
    }

    /// Native only: reads the file off-thread, then loads it like [`Self::load_bytes`].
    #[cfg(not(target_family = "wasm"))]
    pub fn load_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let task = cx.background_spawn(async move {
            std::fs::read(&path)
                .map_err(|err| AudioError::Decode(err.to_string()))
                .and_then(|bytes| decode_bytes(&name, bytes))
                .map(with_peaks)
        });
        self.start_load(cx, task);
        cx.notify();
    }

    pub fn unload(&mut self, cx: &mut Context<Self>) {
        self.load_generation += 1;
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
                            self.last_error = Some(err);
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
            self.last_error = Some(AudioError::Unavailable);
        }
        cx.notify();
    }

    pub fn stop(&mut self, cx: &mut Context<Self>) {
        #[cfg(not(target_family = "wasm"))]
        {
            if let Some(session) = &self.session {
                if let Err(err) = session.pause() {
                    self.last_error = Some(err);
                }
            }
            // Cancel any queued build: it must not honor a stale pending_play.
            self.session_generation += 1;
            self.pending_play = false;
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

    pub fn meta(&self) -> Option<&AudioMeta> {
        self.load.as_ref().map(|load| &load.meta)
    }

    pub fn peaks(&self) -> Option<Arc<WaveformPeaks>> {
        self.load.as_ref().map(|load| load.peaks.clone())
    }

    /// Read once per render; cleared on the next successful action.
    pub fn last_error(&self) -> Option<&AudioError> {
        self.last_error.as_ref()
    }

    pub fn devices(&self) -> &[OutputDevice] {
        &self.devices
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

    /// Rebuilds the session on the device whose display name is `name`, keeping
    /// the position; the device query, channel adapt, and resample run off-thread.
    pub fn select_device(&mut self, name: &str, cx: &mut Context<Self>) {
        #[cfg(not(target_family = "wasm"))]
        {
            let Some(device) = self.devices.iter().find(|device| device.name == name) else {
                return;
            };
            let id = device.id.clone();
            if self.selected_device.as_deref() == Some(id.as_str()) {
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
            self.selected_device = Some(id.clone());
            if let Some(decoded) = decoded {
                let generation = self.load_generation;
                let replan_device = id.clone();
                let task = self.spawn_prepare(&decoded, Some(id), cx);
                cx.spawn(async move |this, cx| {
                    let prepared = task.await;
                    this.update(cx, |engine, cx| {
                        engine.finish_replan(
                            generation,
                            replan_device,
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

    /// Mirrors transport state into status. Returns false when nothing is
    /// playing; the tick then parks until `wake_tick`.
    fn refresh_from_transport(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(load) = &self.load else {
            self.tick = None;
            return false;
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
            // The stream keeps running silently at EOF; pause it so a later
            // seek cannot emit audio while the UI reports Stopped.
            #[cfg(not(target_family = "wasm"))]
            if let Some(session) = &self.session {
                if let Err(err) = session.pause() {
                    self.last_error = Some(err);
                }
            }
            self.status = Some(PlaybackStatus::Stopped);
            changed = true;
        } else if self.status == Some(PlaybackStatus::Playing) && !load.transport.is_playing() {
            self.status = Some(PlaybackStatus::Paused);
            changed = true;
        }
        if changed {
            cx.notify();
        }
        let live = load.transport.is_playing();
        if !live {
            self.tick = None;
        }
        live
    }

    fn start_load(
        &mut self,
        cx: &mut Context<Self>,
        task: Task<Result<(Arc<DecodedAudio>, Arc<WaveformPeaks>), AudioError>>,
    ) {
        self.load_generation += 1;
        let generation = self.load_generation;
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
        match result {
            Err(err) => {
                self.last_error = Some(err);
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
        let PreparedRender {
            render,
            render_rate,
            render_channels,
            #[cfg(not(target_family = "wasm"))]
            plan,
            plan_error,
        } = prepared;
        let meta = AudioMeta {
            source_name: decoded.source_name().to_string(),
            sample_rate: decoded.sample_rate(),
            channels: decoded.channels(),
            duration: decoded.duration(),
        };
        self.load = Some(LoadState {
            meta,
            decoded,
            render,
            render_rate,
            render_channels,
            #[cfg(not(target_family = "wasm"))]
            plan,
            peaks,
            transport: Arc::new(SharedTransport::new(self.volume)),
            producer: None,
        });
        if let Some(err) = plan_error {
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
        device: String,
        decoded: Arc<DecodedAudio>,
        prepared: PreparedRender,
        position_secs: f64,
        was_playing: bool,
        cx: &mut Context<Self>,
    ) {
        if generation != self.load_generation {
            return;
        }
        // A newer select_device repointed the engine: this plan is stale.
        if self.selected_device.as_deref() != Some(device.as_str()) {
            return;
        }
        let replanable = self
            .load
            .as_ref()
            .is_some_and(|load| Arc::ptr_eq(&load.decoded, &decoded));
        if !replanable {
            return;
        }
        let PreparedRender {
            render,
            render_rate,
            render_channels,
            plan,
            plan_error,
        } = prepared;
        if let Some(err) = plan_error {
            self.last_error = Some(err);
        }
        if let Some(load) = &mut self.load {
            load.render = render;
            load.render_rate = render_rate;
            load.render_channels = render_channels;
            load.plan = plan;
            load.transport
                .set_cursor((position_secs * render_rate as f64) as u64);
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
        wanted: Option<String>,
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
                let render = if out_rate == decoded.sample_rate() {
                    match adapted {
                        Cow::Borrowed(_) => decoded.share_samples(),
                        Cow::Owned(owned) => Arc::from(owned),
                    }
                } else {
                    Arc::from(
                        output::resample_linear(
                            &adapted,
                            out_channels,
                            decoded.sample_rate(),
                            out_rate,
                        )
                        .into_owned(),
                    )
                };
                let (plan, plan_error) = match plan {
                    Ok(plan) => (Some(plan), None),
                    Err(err) => (None, Some(err)),
                };
                PreparedRender {
                    render,
                    render_rate: out_rate,
                    render_channels: out_channels,
                    plan,
                    plan_error,
                }
            })
        }
        #[cfg(target_family = "wasm")]
        {
            let _ = wanted;
            cx.background_spawn(async move {
                let render = decoded.share_samples();
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
                    self.last_error = Some(err);
                    return false;
                }
                if let Some(load) = &self.load {
                    load.transport.set_playing(true);
                }
                self.status = Some(PlaybackStatus::Playing);
                self.wake_tick(cx);
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
        // A meter is optional: with no producer the stream plays unmetered.
        let producer = self.load.as_mut().and_then(|load| load.producer.take());
        let request = match &self.load {
            Some(load) => SessionRequest {
                wanted_device: self.selected_device.clone(),
                render: load.render.clone(),
                render_channels: load.render_channels,
                source_rate: load.decoded.sample_rate(),
                plan: load.plan.clone(),
                transport: load.transport.clone(),
                producer,
            },
            None => return,
        };
        let transport = request.transport.clone();
        let builder = self.session_builder.clone();
        self.session_generation += 1;
        let token = self.session_generation;
        let task = cx.background_spawn(async move { builder(request) });
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
        result: Result<Arc<dyn output::SessionSink>, AudioError>,
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
                transport.set_playing(false);
                if self.pending_play {
                    self.pending_play = false;
                    match session.play() {
                        Ok(()) => {
                            transport.set_playing(true);
                            self.status = Some(PlaybackStatus::Playing);
                            self.last_error = None;
                            self.wake_tick(cx);
                        }
                        Err(err) => self.last_error = Some(err),
                    }
                }
                self.session = Some(session);
            }
            Err(err) => {
                // The queued play cannot happen on this build; do not leave
                // pending_play armed for some later session to honor.
                self.pending_play = false;
                self.last_error = Some(err);
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
