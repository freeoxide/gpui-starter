//! Picker sections, ported from the upstream stories: Combobox, Select,
//! DatePicker, Calendar, and TimeField.

mod calendar;
mod combobox;
mod date_picker;
mod demo;
mod select;
mod time_field;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    calendar::register(sections, window, cx);
    combobox::register(sections, window, cx);
    date_picker::register(sections, window, cx);
    select::register(sections, window, cx);
    time_field::register(sections, window, cx);
}
