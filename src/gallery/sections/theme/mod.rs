//! Theme and color sections, ported from the upstream `ThemeColorsStory` and
//! the brush and Oklab color-mix examples.

mod brush;
mod checkerboard;
mod color_mix;
mod colors;
mod demo;
mod mapper;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    colors::register(sections, window, cx);
    brush::register(sections, window, cx);
    color_mix::register(sections, window, cx);
}
