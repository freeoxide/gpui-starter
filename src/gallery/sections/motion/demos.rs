//! Motion section, ported from the upstream `gpui-base-examples` motion demo:
//! one stage that swaps between six keyed-motion demos.

use std::time::Duration;

use gpui_kit::base::{
    Easing, IterationCount, Keyframe, Keyframes, MotionStatus, Presence, Sequence, Spring, Stagger,
    StaggerOrigin, Timing, Transition, animate_keyframes, spring, transition,
};
use gpui_kit::component::{
    ActiveTheme as _,
    button::{Button, ButtonVariants as _, Toggle, ToggleVariants as _},
    h_flex, v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::gallery::registry::GallerySection;

use super::demo::section;

const START_MINUTES: u32 = 8 * 60;
const END_MINUTES: u32 = 20 * 60;
// The roll offset is computed as a fraction of the digit cell height, so the
// cell height is demo math rather than layout chrome.
const DIGIT_HEIGHT: f32 = 38.;

#[derive(Clone, Copy, PartialEq)]
enum Demo {
    SlidingTime,
    Spring,
    Keyframes,
    Presence,
    Stagger,
    Sequence,
}

impl Demo {
    const ALL: [Self; 6] = [
        Self::SlidingTime,
        Self::Spring,
        Self::Keyframes,
        Self::Stagger,
        Self::Presence,
        Self::Sequence,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::SlidingTime => "Sliding time",
            Self::Spring => "Spring",
            Self::Keyframes => "Keyframes",
            Self::Presence => "Presence",
            Self::Stagger => "Stagger",
            Self::Sequence => "Sequence",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::SlidingTime => {
                "Interruptible transitions roll the clock from morning to evening."
            }
            Self::Spring => "A retargeted spring keeps its velocity instead of restarting.",
            Self::Keyframes => "Seven values follow one keyframe track with offset timing.",
            Self::Presence => "A surface stays mounted until its exit transition completes.",
            Self::Stagger => "List rows enter in order from one allocation-free delay policy.",
            Self::Sequence => "Three steps play in turn: slide in, fill, then rest and fade out.",
        }
    }
}

pub struct MotionSection {
    demo: Demo,
    minutes: u32,
    digit_targets: [f32; 4],
    playback: Option<Task<()>>,
    spring_selected: bool,
    present: bool,
    stagger_generation: usize,
    sequence_generation: usize,
}

impl MotionSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            demo: Demo::SlidingTime,
            minutes: START_MINUTES,
            digit_targets: [0., 8., 0., 0.],
            playback: None,
            spring_selected: false,
            present: true,
            stagger_generation: 0,
            sequence_generation: 0,
        })
    }

    fn play_time(&mut self, cx: &mut Context<Self>) {
        if self.playback.take().is_some() {
            cx.notify();
            return;
        }
        if self.minutes == END_MINUTES {
            self.minutes = START_MINUTES;
            self.digit_targets = [0., 8., 0., 0.];
        }
        self.playback = Some(cx.spawn(async move |this, cx: &mut AsyncApp| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(500))
                    .await;
                let finished = this
                    .update(cx, |this, cx| {
                        this.minutes = (this.minutes + 30).min(END_MINUTES);
                        for (target, digit) in
                            this.digit_targets.iter_mut().zip(time_digits(this.minutes))
                        {
                            *target = advance_digit(*target, digit);
                        }
                        cx.notify();
                        this.minutes == END_MINUTES
                    })
                    .unwrap_or(true);
                if finished {
                    _ = this.update(cx, |this, cx| {
                        this.playback = None;
                        cx.notify();
                    });
                    break;
                }
            }
        }));
        cx.notify();
    }

    fn rolling_digit(
        &self,
        ix: usize,
        target: f32,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let value = transition(
            ("motion-clock-digit", ix.to_string()),
            target,
            Transition::new(Duration::from_millis(620)).easing(Easing::EaseInOut),
            window,
            cx,
        );
        let digit = value.floor() as i32;
        let offset = value.fract() * DIGIT_HEIGHT;
        div()
            .relative()
            .w(px(25.))
            .h(px(DIGIT_HEIGHT))
            .overflow_hidden()
            .child(clock_digit(digit, -offset))
            .child(clock_digit(digit + 1, DIGIT_HEIGHT - offset))
    }

    fn sliding_time(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let playing = self.playback.is_some();
        div()
            .flex()
            .items_center()
            .gap_6()
            .child(
                div()
                    .flex()
                    .items_center()
                    .text_size(px(30.))
                    .font_weight(FontWeight::MEDIUM)
                    .children(
                        self.digit_targets[..2]
                            .iter()
                            .enumerate()
                            .map(|(ix, target)| {
                                self.rolling_digit(ix, *target, window, cx)
                                    .into_any_element()
                            }),
                    )
                    .child(div().px_1().child(":"))
                    .children(
                        self.digit_targets[2..]
                            .iter()
                            .enumerate()
                            .map(|(ix, target)| {
                                self.rolling_digit(ix + 2, *target, window, cx)
                                    .into_any_element()
                            }),
                    ),
            )
            .child(
                Button::new("motion-play-time")
                    .outline()
                    .label(if playing {
                        "Stop"
                    } else if self.minutes == END_MINUTES {
                        "Replay"
                    } else {
                        "Play"
                    })
                    .on_click(cx.listener(|this, _, _, cx| this.play_time(cx))),
            )
    }

    fn spring_demo(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let x = spring(
            "motion-selector-indicator",
            if self.spring_selected { 120. } else { 0. },
            Spring::new(Duration::from_millis(420)).with_damping(0.68),
            window,
            cx,
        );
        div()
            .relative()
            .w(px(240.))
            .h_10()
            .border_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .absolute()
                    .top(px(0.))
                    .left(px(x))
                    .w(px(119.))
                    .h_full()
                    .bg(cx.theme().foreground),
            )
            .child(
                div().relative().h_full().flex().children(
                    [("Focus", false), ("Flow", true)]
                        .into_iter()
                        .enumerate()
                        .map(|(ix, (label, selected))| {
                            Button::new(("motion-spring-option", ix))
                                .ghost()
                                .w(px(119.))
                                .h_full()
                                .text_color(if self.spring_selected == selected {
                                    cx.theme().background
                                } else {
                                    cx.theme().muted_foreground
                                })
                                .label(label)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.spring_selected = selected;
                                    cx.notify();
                                }))
                        }),
                ),
            )
    }

    fn keyframes_demo(&self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let frames = Keyframes::try_new([
            Keyframe::new(0., 0.),
            Keyframe::new(0.35, 1.).ease(Easing::EaseOut),
            Keyframe::new(0.7, 0.),
            Keyframe::new(1., 0.),
        ])
        .expect("static keyframes are valid");
        div()
            .w(rems(20.))
            .h(rems(8.25))
            .p_4()
            .border_1()
            .border_color(cx.theme().border)
            .flex()
            .flex_col()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_xs().child("Playback"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Infinite · 1200ms"),
                    ),
            )
            .child(
                div()
                    .h(rems(4.))
                    .flex()
                    .items_end()
                    .justify_center()
                    .gap_2()
                    .children((0..7).map(|ix| {
                        let value = animate_keyframes(
                            (ix, "motion-keyframe-bar"),
                            &frames,
                            Timing::new(Duration::from_millis(1200))
                                .delay(Duration::from_millis(ix as u64 * 80).into())
                                .iterations(IterationCount::Infinite),
                            window,
                            cx,
                        )
                        .value;
                        // The bar height is the animated value itself, in px.
                        div()
                            .w_5()
                            .h(px(18. + 38. * value))
                            .bg(cx.theme().foreground)
                            .opacity(0.35 + 0.65 * value)
                    })),
            )
    }

    fn presence_demo(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sample = Presence::new("motion-presence-notice", self.present)
            .transition(Transition::new(Duration::from_millis(360)).easing(Easing::EaseInOut))
            .sample(window, cx);
        div()
            .h(rems(7.5))
            .flex()
            .items_center()
            .gap_4()
            .child(div().w(rems(20.)).child(if sample.should_render() {
                div()
                    .w_full()
                    .p_3()
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().background)
                    .opacity(sample.progress)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child("Background task"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Complete"),
                            ),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Mounted through the exit phase."),
                    )
                    .into_any_element()
            } else {
                div().into_any_element()
            }))
            .child(
                Button::new("motion-toggle-presence")
                    .outline()
                    .label(if self.present { "Remove" } else { "Insert" })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.present = !this.present;
                        cx.notify();
                    })),
            )
    }

    fn stagger_demo(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let stagger = Stagger::new(Duration::from_millis(90), StaggerOrigin::First);
        let generation = self.stagger_generation;
        let frames = Keyframes::try_new([Keyframe::new(0., 0.), Keyframe::new(1., 1.)])
            .expect("static keyframes are valid");
        div()
            .w(rems(23.75))
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .border_1()
                    .border_color(cx.theme().border)
                    .children((0..3).map(|ix| {
                        let value = animate_keyframes(
                            ("motion-stagger-item", format!("{generation}-{ix}")),
                            &frames,
                            Timing::new(Duration::from_millis(360))
                                .delay(stagger.delay(ix, 3).into())
                                .ease(Easing::EaseOut),
                            window,
                            cx,
                        )
                        .value;
                        let title = ["Transition", "Spring", "Keyframes"][ix];
                        div()
                            .ml(px((1. - value) * 24.))
                            .w_full()
                            .h_10()
                            .px_3()
                            .when(ix < 2, |this| {
                                this.border_b_1().border_color(cx.theme().border)
                            })
                            .bg(cx.theme().background)
                            .opacity(value)
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .w_5()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("0{}", ix + 1)),
                            )
                            .child(div().text_sm().child(title))
                    })),
            )
            .child(
                Button::new("motion-replay-stagger")
                    .outline()
                    .self_end()
                    .label("Replay")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.stagger_generation += 1;
                        cx.notify();
                    })),
            )
    }

    /// One `Sequence<f32>` runs 0 → 1 → 2 → 3 with a different transition per
    /// step, and the demo reads each property off the segment the value is in:
    /// step 0 slides the card in, step 1 fills its bar, step 2 rests and fades.
    fn sequence_demo(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        const STEPS: [&str; 3] = ["Slide in", "Fill", "Rest, then fade out"];
        let sample = Sequence::new(
            format!("motion-sequence-card-{}", self.sequence_generation),
            0.,
        )
        .with_step(
            1.,
            Transition::new(Duration::from_millis(360)).easing(Easing::EaseOut),
        )
        .with_step(
            2.,
            Transition::new(Duration::from_millis(900)).easing(Easing::EaseInOut),
        )
        .with_step(
            3.,
            Transition::new(Duration::from_millis(420))
                .delay(Duration::from_millis(600))
                .easing(Easing::EaseIn),
        )
        .sample(window, cx);
        let value = *sample.value();
        let slide = value.min(1.);
        let fill = (value - 1.).clamp(0., 1.);
        let fade = (value - 2.).clamp(0., 1.);
        let status = match sample.status() {
            MotionStatus::Idle => "idle",
            MotionStatus::Delayed => "delayed",
            MotionStatus::Running => "running",
            MotionStatus::Finished => "finished",
        };
        div()
            .w(rems(23.75))
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div().h(rems(6.)).flex().items_center().child(
                    div()
                        .w_full()
                        .ml(px((1. - slide) * 48.))
                        .opacity(slide * (1. - fade))
                        .p_3()
                        .border_1()
                        .border_color(cx.theme().border)
                        .bg(cx.theme().background)
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child("Uploading report.pdf"),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(format!("{}%", (fill * 100.).round())),
                                ),
                        )
                        .child(
                            div()
                                .h(rems(0.375))
                                .w_full()
                                .bg(cx.theme().muted)
                                .child(div().h_full().w(relative(fill)).bg(cx.theme().foreground)),
                        ),
                ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!(
                                "step {} of {} · {} · {status}",
                                sample.step() + 1,
                                STEPS.len(),
                                STEPS[sample.step()],
                            )),
                    )
                    .child(
                        Button::new("motion-replay-sequence")
                            .outline()
                            .label("Replay")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.sequence_generation += 1;
                                cx.notify();
                            })),
                    ),
            )
    }
}

impl Render for MotionSection {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.demo {
            Demo::SlidingTime => self.sliding_time(window, cx).into_any_element(),
            Demo::Spring => self.spring_demo(window, cx).into_any_element(),
            Demo::Keyframes => self.keyframes_demo(window, cx).into_any_element(),
            Demo::Presence => self.presence_demo(window, cx).into_any_element(),
            Demo::Stagger => self.stagger_demo(window, cx).into_any_element(),
            Demo::Sequence => self.sequence_demo(window, cx).into_any_element(),
        };
        let demo = self.demo;
        v_flex().gap_3().w_full().items_center().p_4().child(
            section("motion-examples", "Motion examples")
                .description(
                    "Keyed motion from the upstream example. Retarget a demo before it \
                         settles to feel interruption, reversal, and replay.",
                )
                .child(
                    v_flex()
                        .w_full()
                        .items_center()
                        .gap_3()
                        .child(h_flex().flex_wrap().justify_center().gap_1().children(
                            Demo::ALL.into_iter().enumerate().map(|(ix, demo)| {
                                Toggle::new(("motion-demo", ix))
                                    .outline()
                                    .checked(self.demo == demo)
                                    .label(demo.label())
                                    .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                                        if *checked {
                                            this.demo = demo;
                                            cx.notify();
                                        }
                                    }))
                            }),
                        ))
                        .child(
                            div()
                                .w_full()
                                .h(rems(16.25))
                                .pt_4()
                                .border_1()
                                .border_color(cx.theme().border)
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(
                                    div()
                                        .px_4()
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(demo.label()),
                                )
                                .child(
                                    div()
                                        .px_4()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(demo.description()),
                                )
                                .child(
                                    div()
                                        .pb_4()
                                        .flex_1()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(content),
                                ),
                        ),
                ),
        )
    }
}

fn clock_digit(value: i32, top: f32) -> Div {
    div()
        .absolute()
        .top(px(top))
        .w_full()
        .h(px(DIGIT_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .child(value.rem_euclid(10).to_string())
}

fn time_digits(minutes: u32) -> [u8; 4] {
    let hour = minutes / 60;
    let minute = minutes % 60;
    [
        (hour / 10) as u8,
        (hour % 10) as u8,
        (minute / 10) as u8,
        (minute % 10) as u8,
    ]
}

fn advance_digit(current: f32, digit: u8) -> f32 {
    let visible = current.floor() as i32 % 10;
    current + (i32::from(digit) - visible).rem_euclid(10) as f32
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "motion",
        "Motion",
        "Keyed transitions, springs, keyframes, presence, stagger, and sequence playback.",
        MotionSection::view(window, cx),
    ));
}
