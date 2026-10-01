//! Loading sections, ported from the upstream stories: Skeleton, Shimmer, and
//! Empty.

mod demo;
mod empty;
mod shimmer;
mod skeleton;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    skeleton::register(sections, window, cx);
    shimmer::register(sections, window, cx);
    empty::register(sections, window, cx);
}
