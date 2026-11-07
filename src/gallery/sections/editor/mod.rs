//! Editor sections, ported from the upstream sources: the `EditorStory`
//! (code and decorations tabs), the `editor` example (file tree and option
//! row), the `html` example (live HTML preview), and the `large-text`
//! example. The `fixtures/` directory holds the language samples those ports
//! embed; `fixtures/editor_preview.rs` is demo content, never compiled.

mod editor_story;
mod html_render;
mod large_text;
mod workspace;

use gpui_kit::component::input;
use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

/// Parses the go-to-line query `line[:column]`, both 1-based like the dialog
/// hints, into the 0-based position the input states take.
fn parse_go_to_line(query: &str) -> Option<input::Position> {
    let mut parts = query
        .split(':')
        .map(|part| part.trim().parse::<usize>().ok());
    let line = parts.next().flatten()?;
    let column = parts.next().flatten().unwrap_or(1);
    Some(input::Position::new(
        line.saturating_sub(1) as u32,
        column.saturating_sub(1) as u32,
    ))
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    editor_story::register(sections, window, cx);
    workspace::register(sections, window, cx);
    html_render::register(sections, window, cx);
    large_text::register(sections, window, cx);
}
