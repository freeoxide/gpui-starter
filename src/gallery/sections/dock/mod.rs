//! Dock-area sections, ported from the upstream `DockStory` and the `dock`
//! example: Dock and Dock Workspace.

mod demo;
mod dock_story;
mod workspace;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    workspace::init(cx);
    dock_story::register(sections, window, cx);
    workspace::register(sections, window, cx);
}
