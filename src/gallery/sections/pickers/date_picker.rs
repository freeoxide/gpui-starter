//! DatePicker section, ported from the upstream `DatePickerStory`.

use chrono::{Datelike, Days, Duration, Utc};
use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, Size, calendar,
    date_picker::{DatePicker, DatePickerEvent, DatePickerState, DateRangePreset},
    time_field::{HourCycle, TimePrecision},
    v_flex,
};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, section, size_dropdown};

pub struct DatePickerSection {
    date_picker: Entity<DatePickerState>,
    date_picker_small: Entity<DatePickerState>,
    date_picker_large: Entity<DatePickerState>,
    date_picker_custom: Entity<DatePickerState>,
    date_picker_value: Option<String>,
    date_range_picker: Entity<DatePickerState>,
    default_range_mode_picker: Entity<DatePickerState>,
    birthday_picker: Entity<DatePickerState>,
    without_appearance_picker: Entity<DatePickerState>,
    date_time_picker: Entity<DatePickerState>,
    date_time_second_picker: Entity<DatePickerState>,
    date_time_12h_picker: Entity<DatePickerState>,
    date_time_value: Option<String>,
    size: Size,
    _subscriptions: Vec<Subscription>,
}

impl DatePickerSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let now = chrono::Local::now().naive_local().date();
        let date_picker = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx).disabled_matcher(vec![0, 6]);
            picker.set_date(now, window, cx);
            picker
        });
        let date_picker_large = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx)
                .date_format("%Y-%m-%d")
                .disabled_matcher(calendar::Matcher::range(
                    Some(now),
                    now.checked_add_days(Days::new(7)),
                ));
            picker.set_date(
                now.checked_sub_days(Days::new(1)).unwrap_or_default(),
                window,
                cx,
            );
            picker
        });
        let date_picker_small = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx).disabled_matcher(
                calendar::Matcher::interval(Some(now), now.checked_add_days(Days::new(5))),
            );
            picker.set_date(now, window, cx);
            picker
        });
        let date_picker_custom = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx)
                .disabled_matcher(calendar::Matcher::custom(|date| date.day0() < 5));
            picker.set_date(now, window, cx);
            picker
        });
        let date_range_picker = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx);
            picker.set_date(
                (now, now.checked_add_days(Days::new(4)).unwrap()),
                window,
                cx,
            );
            picker
        });

        let default_range_mode_picker = cx.new(|cx| DatePickerState::range(window, cx));

        let birthday_picker = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx);
            picker.set_year_range((1927, now.year() + 1), cx);
            picker
        });

        let without_appearance_picker = cx.new(|cx| DatePickerState::new(window, cx));

        let date_time_picker = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx).time_precision(TimePrecision::Minute);
            picker.set_date_time(chrono::Local::now().naive_local(), window, cx);
            picker
        });
        let date_time_second_picker = cx.new(|cx| {
            DatePickerState::new(window, cx)
                .time_precision(TimePrecision::Second)
                .default_time(chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap())
        });
        let date_time_12h_picker = cx.new(|cx| {
            DatePickerState::new(window, cx)
                .time_precision(TimePrecision::Minute)
                .hour_cycle(HourCycle::H12)
                .default_time(chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap())
        });

        let _subscriptions = vec![
            cx.subscribe(&date_picker, |this, _, ev, cx| match ev {
                DatePickerEvent::Change(date) => {
                    this.date_picker_value = date.format("%Y-%m-%d").map(|s| s.to_string());
                    cx.notify();
                }
            }),
            cx.subscribe(&date_range_picker, |this, _, ev, cx| match ev {
                DatePickerEvent::Change(date) => {
                    this.date_picker_value = date.format("%Y-%m-%d").map(|s| s.to_string());
                    cx.notify();
                }
            }),
            cx.subscribe(&default_range_mode_picker, |this, _, ev, cx| match ev {
                DatePickerEvent::Change(date) => {
                    this.date_picker_value = date.format("%Y-%m-%d").map(|s| s.to_string());
                    cx.notify();
                }
            }),
            cx.subscribe(&date_time_picker, |this, _, ev, cx| match ev {
                DatePickerEvent::Change(value) => {
                    this.date_time_value = value.format("%Y-%m-%d %H:%M:%S").map(|s| s.to_string());
                    cx.notify();
                }
            }),
            cx.subscribe(&date_time_second_picker, |this, _, ev, cx| match ev {
                DatePickerEvent::Change(value) => {
                    this.date_time_value = value.format("%Y-%m-%d %H:%M:%S").map(|s| s.to_string());
                    cx.notify();
                }
            }),
            cx.subscribe(&date_time_12h_picker, |this, _, ev, cx| match ev {
                DatePickerEvent::Change(value) => {
                    this.date_time_value = value.format("%Y-%m-%d %H:%M:%S").map(|s| s.to_string());
                    cx.notify();
                }
            }),
        ];

        Self {
            date_picker,
            date_picker_large,
            date_picker_small,
            date_picker_custom,
            date_range_picker,
            default_range_mode_picker,
            birthday_picker,
            without_appearance_picker,
            date_time_picker,
            date_time_second_picker,
            date_time_12h_picker,
            date_time_value: None,
            size: Size::Medium,
            date_picker_value: None,
            _subscriptions,
        }
    }
}

impl Render for DatePickerSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let size = self.size;
        let presets = vec![
            DateRangePreset::single(
                "Yesterday",
                (Utc::now() - Duration::days(1)).naive_local().date(),
            ),
            DateRangePreset::single(
                "Last Week",
                (Utc::now() - Duration::weeks(1)).naive_local().date(),
            ),
            DateRangePreset::single(
                "Last Month",
                (Utc::now() - Duration::days(30)).naive_local().date(),
            ),
        ];
        let range_presets = vec![
            DateRangePreset::range(
                "Last 7 Days",
                (Utc::now() - Duration::days(7)).naive_local().date(),
                Utc::now().naive_local().date(),
            ),
            DateRangePreset::range(
                "Last 14 Days",
                (Utc::now() - Duration::days(14)).naive_local().date(),
                Utc::now().naive_local().date(),
            ),
            DateRangePreset::range(
                "Last 30 Days",
                (Utc::now() - Duration::days(30)).naive_local().date(),
                Utc::now().naive_local().date(),
            ),
            DateRangePreset::range(
                "Last 90 Days",
                (Utc::now() - Duration::days(90)).naive_local().date(),
                Utc::now().naive_local().date(),
            ),
        ];

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
                size_dropdown("date-picker-size", size).into_any_element(),
            ]))
            .child(
                section("date-picker-default", "Default")
                    .description("Single-date selection with presets and clear action.")
                    .w_128()
                    .child(
                        v_flex()
                            .gap_3()
                            .w_full()
                            .child(
                                DatePicker::new(&self.date_picker)
                                    .with_size(size)
                                    .w(rems(17.5))
                                    .cleanable(true)
                                    .presets(presets),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("Value: {:?}", self.date_picker_value)),
                            ),
                    ),
            )
            .child(
                section("date-picker-date-time", "Date and time")
                    .description(
                        "Edit the time of day below the calendar; changes apply as you make them.",
                    )
                    .w_128()
                    .child(
                        v_flex()
                            .gap_3()
                            .w_full()
                            .child(
                                DatePicker::new(&self.date_time_picker)
                                    .with_size(size)
                                    .w(rems(17.5)),
                            )
                            .child(
                                DatePicker::new(&self.date_time_second_picker)
                                    .with_size(size)
                                    .w(rems(17.5))
                                    .placeholder("With seconds")
                                    .cleanable(true),
                            )
                            .child(
                                DatePicker::new(&self.date_time_12h_picker)
                                    .with_size(size)
                                    .w(rems(17.5))
                                    .placeholder("12-hour clock")
                                    .cleanable(true),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("Value: {:?}", self.date_time_value)),
                            ),
                    ),
            )
            .child(
                section("date-picker-disabled-dates", "Disabled dates")
                    .description("Matchers can block intervals, ranges, or custom dates.")
                    .w_128()
                    .child(
                        v_flex()
                            .gap_3()
                            .w_full()
                            .child(
                                DatePicker::new(&self.date_picker_small)
                                    .with_size(size)
                                    .w(rems(17.5)),
                            )
                            .child(
                                DatePicker::new(&self.date_picker_large)
                                    .with_size(size)
                                    .w(rems(17.5)),
                            )
                            .child(
                                DatePicker::new(&self.date_picker_custom)
                                    .with_size(size)
                                    .w(rems(17.5)),
                            ),
                    ),
            )
            .child(
                section("date-picker-date-range", "Date range")
                    .description("Two months with range presets.")
                    .w_128()
                    .child(
                        DatePicker::new(&self.date_range_picker)
                            .with_size(size)
                            .w(rems(17.5))
                            .number_of_months(2)
                            .cleanable(true)
                            .presets(range_presets.clone()),
                    ),
            )
            .child(
                section("date-picker-empty-range", "Empty range")
                    .description("Empty range with presets.")
                    .w_128()
                    .child(
                        DatePicker::new(&self.default_range_mode_picker)
                            .with_size(size)
                            .w(rems(17.5))
                            .placeholder("Range mode picker")
                            .cleanable(true)
                            .presets(range_presets.clone()),
                    ),
            )
            .child(
                section("date-picker-year-range", "Year range")
                    .description("Custom year range.")
                    .w_128()
                    .child(
                        DatePicker::new(&self.birthday_picker)
                            .with_size(size)
                            .w(rems(17.5))
                            .number_of_months(1)
                            .cleanable(true)
                            .placeholder("Select birthday"),
                    ),
            )
            .child(
                section("date-picker-custom-style", "Custom style")
                    .description("Appearance-free input.")
                    .w_128()
                    .child(
                        div().w(rems(17.5)).bg(cx.theme().secondary).child(
                            DatePicker::new(&self.without_appearance_picker)
                                .with_size(size)
                                .w(rems(17.5))
                                .appearance(false)
                                .placeholder("Without appearance"),
                        ),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "date-picker",
        "Date Picker",
        "A date picker to select a date or date range.",
        DatePickerSection::view(window, cx),
    ));
}
