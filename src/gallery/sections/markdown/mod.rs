//! Markdown sections, ported from the upstream sources: the `markdown`
//! example (source editor beside a live preview driven by custom Markdown
//! plugins), the `markdown_table` example (table layout modes), and the
//! `stream-markdown` example (a document arriving in streamed chunks). The
//! `fixtures/` directory holds the documents they embed.

mod editor;
mod mention;
mod stream;
mod table;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    editor::register(sections, window, cx);
    table::register(sections, window, cx);
    stream::register(sections, window, cx);
}
