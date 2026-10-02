//! Selection-control sections, ported from the upstream stories: Checkbox,
//! Switch, Toggle, Radio, Slider, Color Picker, and Rating.

mod checkbox;
mod color_picker;
mod demo;
mod radio;
mod rating;
mod slider;
mod switch;
mod toggle;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    checkbox::register(sections, window, cx);
    switch::register(sections, window, cx);
    toggle::register(sections, window, cx);
    radio::register(sections, window, cx);
    slider::register(sections, window, cx);
    color_picker::register(sections, window, cx);
    rating::register(sections, window, cx);
}
