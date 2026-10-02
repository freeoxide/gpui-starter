//! Messaging sections, ported from the upstream stories: Bubble, Message,
//! and MessageScroller.

mod bubble;
mod demo;
mod message;
mod scroller;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    bubble::register(sections, window, cx);
    message::register(sections, window, cx);
    scroller::register(sections, window, cx);
}
