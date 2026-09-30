//! Introduction section, ported from the upstream `WelcomeStory`: a markdown
//! [`TextView`] as the gallery's landing entry.

use gpui_kit::component::text::TextView;
use gpui_kit::{prelude::*, *};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "welcome",
        "Welcome",
        "What the component gallery is and how to use it.",
        WelcomeSection::view(window, cx),
    ));
}

pub struct WelcomeSection;

impl WelcomeSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self)
    }
}

const INTRO: &str = r#"
# Component gallery

Every entry in this gallery ports an example from the [gpui-kit](https://github.com/longbridge/gpui-kit) repository so its components can be exercised inside this application.

The gallery is boilerplate, not product code. It lives entirely under `src/gallery/` and is wired into the shell through five removable touches; deleting the module and those touches removes it completely.

## Using the sections

- Click a section in the list to show it.
- Type in the search field to filter sections by title.
- The section title and description sit above the content area.

## Adding a section

A new section owns its files under `src/gallery/sections/<name>/` and registers with two lines: a `pub mod` line in `sections/mod.rs` and a `register(...)` call in `src/gallery/registry.rs`. Sections keep their state in GPUI entities, use `cx.theme()` tokens instead of raw colors, and derive element ids from stable domain data.
"#;

impl Render for WelcomeSection {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        // The gallery page's content pane owns scrolling; no nested scroll
        // container here.
        TextView::markdown("welcome-intro", INTRO)
            .p_4()
            .selectable(true)
    }
}
