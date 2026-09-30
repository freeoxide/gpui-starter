//! Text-display sections, ported from the upstream sources: TextView (the
//! `text_max_lines` example), Marker, and DescriptionList stories.

mod demo;
mod description_list;
mod marker;
mod text_view;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    text_view::register(sections, window, cx);
    marker::register(sections, window, cx);
    description_list::register(sections, window, cx);
}
