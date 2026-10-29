use std::f32::consts::TAU;
use std::time::Duration;

use gpui_kit::component::{
    ActiveTheme as _,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use gpui_kit::{prelude::*, *};

use crate::ui::floating::{
    FloatAnchor, FloatLevel, FloatingHandle, FloatingSpec, FloatingWindowId, Placement9,
    spawn_screen_floating,
};
use crate::ui::waveform::{DEFAULT_RETENTION, SampleProducer, Waveform, WaveformState};

pub(crate) const DICTATION_ID: &str = "chromeless-dictation";
const SAMPLE_RATE: u32 = 24_000;
const CHUNK_FRAMES: usize = 480;
const PUSH_TICK: Duration = Duration::from_millis(20);

pub(crate) fn speech_sample(t: f32) -> f32 {
    let phrase = (t * 0.7).sin() * 0.5 + 0.5;
    let syllable = ((t * 3.4).sin() * 0.5 + 0.5).powi(3);
    let carrier = (t * 190. * TAU).sin() * 0.6 + (t * 287. * TAU).sin() * 0.3;
    (carrier * syllable * phrase * 0.6).clamp(-1., 1.)
}

/// Synthetic mic signal. Frozen (reduced motion) keeps `t` at zero, so every
/// chunk is identical and the painted trace never moves.
pub(crate) struct SpeechSynth {
    frozen: bool,
    t: f32,
}

impl SpeechSynth {
    pub(crate) fn new(frozen: bool) -> Self {
        Self { frozen, t: 0. }
    }

    pub(crate) fn chunk(&mut self) -> Vec<f32> {
        if !self.frozen {
            self.t += CHUNK_FRAMES as f32 / SAMPLE_RATE as f32;
        }
        (0..CHUNK_FRAMES)
            .map(|i| speech_sample(self.t + i as f32 / SAMPLE_RATE as f32))
            .collect()
    }
}

pub(crate) fn format_elapsed(elapsed: Duration) -> String {
    let secs = elapsed.as_secs();
    format!("{}:{:02}", secs / 60, secs % 60)
}

pub(crate) struct DictationHud {
    waveform: Entity<WaveformState>,
    producer: Option<SampleProducer>,
    push_task: Option<Task<()>>,
    listening: bool,
    listened: Duration,
    _subscriptions: Vec<Subscription>,
}

impl DictationHud {
    pub(crate) fn new_view(_: &mut Window, cx: &mut App) -> AnyView {
        cx.new(Self::new).into()
    }

    fn new(cx: &mut Context<Self>) -> Self {
        let waveform = cx.new(|_| WaveformState::new());
        let subscription = cx.observe(&waveform, |_, _, cx| cx.notify());
        Self {
            waveform,
            producer: None,
            push_task: None,
            listening: false,
            listened: Duration::ZERO,
            _subscriptions: vec![subscription],
        }
    }

    pub(crate) fn is_listening(&self) -> bool {
        self.listening
    }

    pub(crate) fn listened(&self) -> Duration {
        self.listened
    }

    pub(crate) fn set_listening(&mut self, listening: bool, cx: &mut Context<Self>) {
        if self.listening == listening {
            return;
        }
        self.listening = listening;
        if listening {
            self.listened = Duration::ZERO;
            let producer = self.waveform.update(cx, |state, cx| {
                state.attach_source(SAMPLE_RATE, 1, DEFAULT_RETENTION, cx)
            });
            self.producer = Some(producer);
            let frozen = cx.reduce_motion();
            self.push_task = Some(cx.spawn(async move |this, cx| {
                let mut synth = SpeechSynth::new(frozen);
                loop {
                    cx.background_executor().timer(PUSH_TICK).await;
                    let frames = synth.chunk();
                    let keep_going = this.update(cx, |hud, cx| {
                        if let Some(producer) = hud.producer.as_mut() {
                            producer.push_interleaved(&frames);
                            hud.listened += PUSH_TICK;
                            cx.notify();
                        }
                        hud.listening
                    });
                    if !keep_going.unwrap_or(false) {
                        break;
                    }
                }
            }));
        } else {
            self.push_task = None;
            self.producer = None;
            self.waveform
                .update(cx, |state, cx| state.detach_source(cx));
        }
        cx.notify();
    }
}

impl Render for DictationHud {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let listening = self.is_listening();
        let (status, status_color) = if listening {
            ("Listening", cx.theme().foreground)
        } else {
            ("Idle", cx.theme().muted_foreground)
        };
        v_flex()
            .size_full()
            .p_2()
            .gap_2()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(div().text_sm().text_color(status_color).child(status))
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format_elapsed(self.listened())),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .rounded(cx.theme().radius)
                    .child(Waveform::new("dictation-wave", self.waveform.clone())),
            )
            .child(
                Button::new("dictation-toggle")
                    .primary()
                    .w_full()
                    .label(if listening {
                        "Stop listening"
                    } else {
                        "Start listening"
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.set_listening(!this.is_listening(), cx);
                    })),
            )
    }
}

pub(crate) fn dictation_spec() -> FloatingSpec {
    FloatingSpec::new(
        FloatingWindowId(DICTATION_ID.into()),
        size(px(320.), px(140.)),
    )
    .title("Dictation")
    .level(FloatLevel::AlwaysOnTop)
    .anchor(FloatAnchor::Screen {
        display: None,
        placement: Placement9::Bottom,
        offset: point(px(0.), px(-64.)),
    })
    .chromeless(true)
    .focus(true)
    .movable(true)
}

/// Open the dictation HUD as a screen float; idempotent per id.
pub fn spawn_dictation(cx: &mut App) -> anyhow::Result<FloatingHandle> {
    if let Some(handle) = super::entry_handle(DICTATION_ID, cx) {
        return Ok(handle);
    }
    spawn_screen_floating(dictation_spec(), DictationHud::new_view, cx)
}

#[cfg(test)]
#[path = "dictation.test.rs"]
mod dictation_test;
