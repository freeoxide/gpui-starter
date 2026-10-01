//! Motion sections, ported from the upstream base examples: the motion demo
//! stage and scroll bounce.

mod demo;
mod demos;
mod scroll_bounce;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    demos::register(sections, window, cx);
    scroll_bounce::register(sections, window, cx);
}
