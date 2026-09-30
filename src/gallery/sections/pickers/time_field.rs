//! TimeField section, ported from the upstream `TimeFieldStory`.

use chrono::NaiveTime;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Sizable as _, Size,
    time_field::{HourCycle, TimeField, TimeFieldEvent, TimeFieldState, TimePrecision},
    v_flex,
};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, section, size_dropdown};

pub struct TimeFieldSection {
    minute: Entity<TimeFieldState>,
    second: Entity<TimeFieldState>,
    twelve_hour: Entity<TimeFieldState>,
    disabled: Entity<TimeFieldState>,
    invalid: Entity<TimeFieldState>,
    value: NaiveTime,
    size: Size,
    _subscriptions: Vec<Subscription>,
}

impl TimeFieldSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let value = NaiveTime::from_hms_opt(9, 30, 0).unwrap();
        let field = |precision: TimePrecision,
                     hour_cycle: HourCycle,
                     window: &mut Window,
                     cx: &mut Context<Self>| {
            cx.new(|cx| {
                let mut state = TimeFieldState::new(window, cx)
                    .precision(precision)
                    .hour_cycle(hour_cycle);
                state.set_time(value, window, cx);
                state
            })
        };
        let minute = field(TimePrecision::Minute, HourCycle::H23, window, cx);
        let second = field(TimePrecision::Second, HourCycle::H23, window, cx);
        let twelve_hour = field(TimePrecision::Minute, HourCycle::H12, window, cx);
        let disabled = field(TimePrecision::Minute, HourCycle::H23, window, cx);
        let invalid = field(TimePrecision::Minute, HourCycle::H23, window, cx);

        let _subscriptions = vec![cx.subscribe(&minute, |this, _, event, cx| match event {
            TimeFieldEvent::Change(time) => {
                this.value = *time;
                cx.notify();
            }
        })];

        Self {
            minute,
            second,
            twelve_hour,
            disabled,
            invalid,
            value,
            size: Size::Medium,
            _subscriptions,
        }
    }
}

impl Render for TimeFieldSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let size = self.size;

        v_flex()
            .w_full()
            .items_center()
            .gap_3()
            .p_4()
            .on_action(cx.listener(|this, action: &DemoToggle, _, cx| {
                if let Some(size) = action.size() {
                    this.size = size;
                    cx.notify();
                }
            }))
            .child(demo_toolbar(vec![
                size_dropdown("time-field-size", size).into_any_element(),
            ]))
            .child(
                section("time-field-default", "Default")
                    .description(
                        "Up/Down change the selected segment; digits type it and move to the next.",
                    )
                    .child(
                        v_flex()
                            .gap_3()
                            .child(TimeField::new(&self.minute).with_size(size))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("Value: {}", self.value)),
                            ),
                    ),
            )
            .child(
                section("time-field-with-seconds", "With seconds")
                    .description("Set the precision to edit seconds as well.")
                    .child(TimeField::new(&self.second).with_size(size)),
            )
            .child(
                section("time-field-12-hour-clock", "12-hour clock")
                    .description("An AM/PM segment follows the time; type a or p to set it.")
                    .child(TimeField::new(&self.twelve_hour).with_size(size)),
            )
            .child(
                section("time-field-disabled", "Disabled").child(
                    TimeField::new(&self.disabled)
                        .with_size(size)
                        .disabled(true),
                ),
            )
            .child(
                section("time-field-invalid", "Invalid")
                    .description("Show a validation result from the owner.")
                    .child(TimeField::new(&self.invalid).with_size(size).invalid(true)),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "time-field",
        "Time Field",
        "Edit a time of day segment by segment, on a 24-hour or 12-hour clock.",
        TimeFieldSection::view(window, cx),
    ));
}
