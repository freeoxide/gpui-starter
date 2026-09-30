//! Calendar section, ported from the upstream `CalendarStory`.

use gpui_kit::component::{calendar::Calendar, calendar::CalendarState, v_flex};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::section;

pub struct CalendarSection {
    calendar: Entity<CalendarState>,
    calendar_wide: Entity<CalendarState>,
    calendar_with_disabled_matcher: Entity<CalendarState>,
}

impl CalendarSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let calendar = cx.new(|cx| CalendarState::new(window, cx));
        let calendar_wide = cx.new(|cx| CalendarState::new(window, cx));
        let calendar_with_disabled_matcher =
            cx.new(|cx| CalendarState::new(window, cx).disabled_matcher(vec![0, 3, 6]));

        Self {
            calendar,
            calendar_wide,
            calendar_with_disabled_matcher,
        }
    }
}

impl Render for CalendarSection {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .items_center()
            .gap_3()
            .p_4()
            .child(
                section("calendar-single-month", "Single month")
                    .description("Single-date selection.")
                    .w_128()
                    .child(Calendar::new(&self.calendar)),
            )
            .child(
                section("calendar-multiple-months", "Multiple months")
                    .description("Three months shown together.")
                    .w_128()
                    .child(Calendar::new(&self.calendar_wide).number_of_months(3)),
            )
            .child(
                section("calendar-disabled-dates", "Disabled dates")
                    .description("Recurring unavailable weekdays.")
                    .w_128()
                    .child(Calendar::new(&self.calendar_with_disabled_matcher)),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "calendar",
        "Calendar",
        "A calendar to select a date or date range.",
        CalendarSection::view(window, cx),
    ));
}
