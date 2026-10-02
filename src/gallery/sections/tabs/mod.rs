//! Tabs-area sections, ported from the upstream stories: Tabs, Resizable,
//! Scrollbar, Pagination, and Carousel.

mod carousel;
mod demo;
mod pagination;
mod resizable;
mod scrollbar;
mod tab_bar;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    tab_bar::register(sections, window, cx);
    resizable::register(sections, window, cx);
    scrollbar::register(sections, window, cx);
    pagination::register(sections, window, cx);
    carousel::register(sections, window, cx);
}
