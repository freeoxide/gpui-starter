//! Editor sections, ported from the upstream sources: the `EditorStory`
//! (code and decorations tabs), the `editor` example (file tree and option
//! row), the `html` example (live HTML preview), and the `large-text`
//! example. The `fixtures/` directory holds the language samples those ports
//! embed; `fixtures/editor_preview.rs` is demo content, never compiled.

mod editor_story;
mod html_render;
mod large_text;
mod workspace;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    editor_story::register(sections, window, cx);
    workspace::register(sections, window, cx);
    html_render::register(sections, window, cx);
    large_text::register(sections, window, cx);
}
