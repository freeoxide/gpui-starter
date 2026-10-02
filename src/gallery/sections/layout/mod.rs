//! Layout-container sections, ported from the upstream stories: Accordion,
//! Collapsible, GroupBox, Breadcrumb, and Separator.

mod accordion;
mod breadcrumb;
mod collapsible;
mod demo;
mod group_box;
mod separator;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    accordion::register(sections, window, cx);
    collapsible::register(sections, window, cx);
    group_box::register(sections, window, cx);
    breadcrumb::register(sections, window, cx);
    separator::register(sections, window, cx);
}
