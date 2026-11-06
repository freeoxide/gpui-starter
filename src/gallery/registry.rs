//! Gallery section registry; section ids and titles are unique
//! gallery-wide, and state lives in entities, never render locals.

use gpui_kit::{AnyView, App, SharedString, Window};

/// One independently renderable gallery entry shown in the section navigator.
pub(crate) struct GallerySection {
    id: SharedString,
    title: SharedString,
    description: SharedString,
    // Search corpus, lowercased once here so keystroke filtering never reallocates.
    title_lower: String,
    description_lower: String,
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
            title_lower: title.to_lowercase(),
            description_lower: description.to_lowercase(),
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

    pub(crate) fn matches_query(&self, query: &str) -> bool {
        self.title_lower.contains(query) || self.description_lower.contains(query)
    }

    pub(crate) fn view(&self) -> AnyView {
        self.view.clone()
    }
}

/// All gallery sections, built eagerly when `GalleryPage` is constructed
/// (same lifecycle as the upstream story gallery and the app's own pages).
pub(crate) fn sections(window: &mut Window, cx: &mut App) -> Vec<GallerySection> {
    let mut sections = Vec::new();
    super::sections::welcome::register(&mut sections, window, cx);
    super::sections::buttons::register(&mut sections, window, cx);
    super::sections::overlays::register(&mut sections, window, cx);
    super::sections::inputs::register(&mut sections, window, cx);
    super::sections::controls::register(&mut sections, window, cx);
    super::sections::pickers::register(&mut sections, window, cx);
    super::sections::text::register(&mut sections, window, cx);
    super::sections::selection::register(&mut sections, window, cx);
    super::sections::editor::register(&mut sections, window, cx);
    super::sections::markdown::register(&mut sections, window, cx);
    super::sections::tables::register(&mut sections, window, cx);
    super::sections::layout::register(&mut sections, window, cx);
    super::sections::tabs::register(&mut sections, window, cx);
    super::sections::chrome::register(&mut sections, window, cx);
    super::sections::dock::register(&mut sections, window, cx);
    super::sections::menus::register(&mut sections, window, cx);
    super::sections::forms::register(&mut sections, window, cx);
    super::sections::charts::register(&mut sections, window, cx);
    super::sections::media::register(&mut sections, window, cx);
    super::sections::messaging::register(&mut sections, window, cx);
    super::sections::theme::register(&mut sections, window, cx);
    super::sections::loading::register(&mut sections, window, cx);
    super::sections::motion::register(&mut sections, window, cx);
    super::sections::shell::register(&mut sections, window, cx);
    sections
}
