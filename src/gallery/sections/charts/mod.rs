//! Chart-family sections: the `ChartStory` card gallery (with its custom
//! stacked-bar `Plot`), the `system_monitor` example on fixture data, and the
//! `fps_monitor` example's Hilbert-curve scene.

mod chart;
mod fps;
mod stacked_bar_chart;
mod system_monitor;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    chart::register(sections, window, cx);
    system_monitor::register(sections, window, cx);
    fps::register(sections, window, cx);
}
