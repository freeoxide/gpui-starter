//! Button-family and indicator sections, ported from the upstream stories:
//! Button, DropdownButton, Badge, Tag, Kbd, Label, Icon, Spinner, Progress,
//! and Toolbar.

mod badge;
mod button;
mod demo;
mod dropdown_button;
mod icon;
mod kbd;
mod label;
mod progress;
mod spinner;
mod tag;
mod toolbar;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    badge::register(sections, window, cx);
    button::register(sections, window, cx);
    dropdown_button::register(sections, window, cx);
    icon::register(sections, window, cx);
    kbd::register(sections, window, cx);
    label::register(sections, window, cx);
    progress::register(sections, window, cx);
    spinner::register(sections, window, cx);
    tag::register(sections, window, cx);
    toolbar::register(sections, window, cx);
}
