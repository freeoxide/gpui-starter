//! Registration seam between the gallery page and content sections.
//!
//! A content area owns `src/gallery/sections/<name>/` and hooks in with two
//! mechanical lines (Rust has no module autodiscovery and no linkme-style
//! registry dep is allowed here):
//!
//! 1. `pub mod <name>;` in `src/gallery/sections/mod.rs`
//! 2. `sections::<name>::register(&mut sections, window, cx);` below.
//!
//! Contract for every section module: expose
//! `pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window,
//! cx: &mut App)` that pushes its sections. Each section is a GPUI entity
//! (`Render`) constructed once at page build; state lives in entities, never
//! in render locals. Section `id`s and titles must be unique gallery-wide:
//! titles are the sidebar labels and the search corpus, ids namespace
//! element ids.

use gpui_kit::{AnyView, App, SharedString, Window};

/// One independently renderable gallery entry shown in the section navigator.
pub(crate) struct GallerySection {
    id: SharedString,
    title: SharedString,
    description: SharedString,
    view: AnyView,
}

impl GallerySection {
    /// `id` is a stable kebab-case slug (e.g. `"welcome"`); `view` is usually
    /// `<Section>::view(window, cx)` following the kit constructor idiom.
    pub(crate) fn new(
        id: &'static str,
        title: &'static str,
        description: &'static str,
        view: impl Into<AnyView>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: description.into(),
            view: view.into(),
        }
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn description(&self) -> &str {
        &self.description
    }

    pub(crate) fn view(&self) -> AnyView {
        self.view.clone()
    }
}

/// All gallery sections, built eagerly when `GalleryPage` is constructed
/// (same lifecycle as the upstream story gallery and the app's own pages).
pub(crate) fn sections(window: &mut Window, cx: &mut App) -> Vec<GallerySection> {
    let mut sections = Vec::new();
    super::sections::buttons::register(&mut sections, window, cx);
    super::sections::welcome::register(&mut sections, window, cx);
    sections
}
