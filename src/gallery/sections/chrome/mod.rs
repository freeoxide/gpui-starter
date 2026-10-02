//! Chrome-area sections, ported from the upstream stories and examples:
//! Sidebar, StatusBar, TitleBar, and NavStack.

mod demo;
mod nav_stack;
mod sidebar;
mod status_bar;
mod title_bar;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sidebar::register(sections, window, cx);
    status_bar::register(sections, window, cx);
    title_bar::register(sections, window, cx);
    nav_stack::register(sections, window, cx);
}
