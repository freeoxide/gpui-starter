//! Selection-and-focus sections, ported from the upstream sources: the
//! `text_selection` and `touch_selection` examples, the base-layer
//! text-selection showcase, and the `focus_trap` example.

mod focus_trap;
mod selectable_text;
mod text_selection;
mod touch_selection;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    text_selection::register(sections, window, cx);
    selectable_text::register(sections, window, cx);
    touch_selection::register(sections, window, cx);
    focus_trap::register(sections, window, cx);
}
