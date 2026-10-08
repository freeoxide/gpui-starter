//! Speech section, ported from the upstream `SpeechStory`.

use std::time::Duration;

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, WindowExt as _, h_flex,
    input::{Input, InputState},
    notification::Notification,
    speech::{
        AudioFormat, AudioInput, AudioSink, RecognitionSession, SpeechButton, SpeechError,
        SpeechEvent, SpeechRecognizer, SpeechSink, SpeechState, SpeechWaveform,
    },
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::gallery::registry::GallerySection;

use super::demo::section;

const SCRIPT: [&str; 2] = [
    "Speech input turns what you say into text.",
    "Any recognizer plugs in through one trait.",
];

const SAMPLES_PER_WORD: usize = 4_800;

struct DemoRecognizer;

impl SpeechRecognizer for DemoRecognizer {
    fn start(
        &self,
        sink: SpeechSink,
        cx: &mut App,
    ) -> Result<Box<dyn RecognitionSession>, SpeechError> {
        sink.ready(cx);
        Ok(Box::new(DemoSession {
            sink,
            phrase_ix: 0,
            words: 0,
            samples: 0,
        }))
    }
}

struct DemoSession {
    sink: SpeechSink,
    phrase_ix: usize,
    words: usize,
    samples: usize,
}

impl DemoSession {
    fn spoken(&self) -> Option<String> {
        let phrase = SCRIPT.get(self.phrase_ix)?;
        let words = phrase.split(' ').take(self.words).collect::<Vec<_>>();
        if words.is_empty() {
            return None;
        }
        let separator = if self.phrase_ix > 0 { " " } else { "" };
        Some(format!("{separator}{}", words.join(" ")))
    }

    fn next_word(&mut self, cx: &mut App) {
        let Some(phrase) = SCRIPT.get(self.phrase_ix) else {
            return;
        };
        self.words += 1;
        if self.words < phrase.split(' ').count() {
            if let Some(spoken) = self.spoken() {
                self.sink.hypothesis(spoken, cx);
            }
        } else {
            self.commit(cx);
        }
    }

    fn commit(&mut self, cx: &mut App) {
        if let Some(spoken) = self.spoken() {
            self.sink.phrase(spoken, cx);
        }
        self.phrase_ix += 1;
        self.words = 0;
    }
}

impl RecognitionSession for DemoSession {
    fn push_audio(&mut self, samples: &[i16], cx: &mut App) {
        self.samples += samples.len();
        while self.samples >= SAMPLES_PER_WORD {
            self.samples -= SAMPLES_PER_WORD;
            self.next_word(cx);
        }
    }

    fn finish(&mut self, cx: &mut App) {
        self.commit(cx);
        self.sink.finish(cx);
    }
}

// The app builds without the kit's `speech` feature, so no target has a
// default microphone; synthetic tone audio drives the demo everywhere.
struct GeneratedInput;

impl AudioInput for GeneratedInput {
    fn start(
        &self,
        format: AudioFormat,
        sink: AudioSink,
        cx: &mut App,
    ) -> Result<Subscription, SpeechError> {
        const CHUNK: Duration = Duration::from_millis(100);
        let len = (format.sample_rate() / 10) as usize * format.channels() as usize;
        let task = cx.spawn(async move |cx| {
            let mut tick = 0u8;
            loop {
                cx.background_executor().timer(CHUNK).await;
                tick = tick.wrapping_add(1);
                let loudness = 0.05 + 0.25 * (tick as f32 * 0.9).sin().abs();
                let samples = (0..len)
                    .map(|ix| ((ix as f32 * 0.07).sin() * loudness * i16::MAX as f32) as i16)
                    .collect();
                cx.update(|cx| sink.push(samples, cx));
            }
        });
        Ok(Subscription::new(move || drop(task)))
    }
}

struct Dictation {
    speech: Entity<SpeechState>,
    input: Entity<InputState>,
}

impl Dictation {
    fn new(
        speech: Entity<SpeechState>,
        window: &mut Window,
        cx: &mut Context<SpeechSection>,
    ) -> (Self, Subscription) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Type or dictate"));
        let subscription = cx.subscribe_in(&speech, window, {
            let input = input.clone();
            move |_, _, event, window, cx| match event {
                SpeechEvent::Final(text) if !text.is_empty() => {
                    input.update(cx, |input, cx| input.insert(text.clone(), window, cx));
                }
                SpeechEvent::Error(error) => {
                    window.push_notification(Notification::error(error.to_string()), cx);
                }
                _ => {}
            }
        });
        (Self { speech, input }, subscription)
    }

    fn render(&self, show_when_unsupported: bool, cx: &App) -> impl IntoElement {
        let speech = self.speech.read(cx);
        let status = speech.status();

        v_flex()
            .w_full()
            .gap_2()
            .child(
                Input::new(&self.input).suffix(
                    h_flex()
                        .gap_2()
                        .when(status.is_capturing(), |this| {
                            this.child(SpeechWaveform::new(&self.speech).w(px(48.)).xsmall())
                        })
                        .child(
                            SpeechButton::new(&self.speech)
                                .xsmall()
                                .show_when_unsupported(show_when_unsupported),
                        ),
                ),
            )
            .when(status.is_active(), |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(speech.transcript()),
                )
            })
    }
}

pub struct SpeechSection {
    custom: Dictation,
    system: Dictation,
    _subscriptions: Vec<Subscription>,
}

impl SpeechSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let custom = cx.new(|cx| {
            SpeechState::new(cx)
                .recognizer(DemoRecognizer)
                .input(GeneratedInput)
        });
        let system = cx.new(SpeechState::new);

        let (custom, custom_subscription) = Dictation::new(custom, window, cx);
        let (system, system_subscription) = Dictation::new(system, window, cx);

        Self {
            custom,
            system,
            _subscriptions: vec![custom_subscription, system_subscription],
        }
    }
}

impl Render for SpeechSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_6()
            .p_4()
            .child(
                section("speech-custom-recognizer", "Custom recognizer")
                    .description(
                        "A recognizer defined in this section types a scripted phrase \
                        while it receives audio.",
                    )
                    .w_128()
                    .child(self.custom.render(false, cx)),
            )
            .child(
                section("speech-system-recognizer", "System recognizer")
                    .description(
                        "Dictates on macOS and Windows once the kit's speech feature is \
                        enabled; this build leaves it off, so the button stays disabled.",
                    )
                    .w_128()
                    .child(self.system.render(true, cx)),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "speech",
        "Speech",
        "Dictate text into an input through a custom recognizer or the platform's own.",
        SpeechSection::view(window, cx),
    ));
}
