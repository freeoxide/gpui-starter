//! Menu-family sections, ported from the upstream stories: Menu, Native Menu,
//! and Command.

mod command;
mod demo;
mod menu;
mod native_menu;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    menu::init(cx);
    menu::register(sections, window, cx);
    native_menu::register(sections, window, cx);
    command::register(sections, window, cx);
}
