//! JS story punt section: examples/js_story is a JavaScript catalog over the
//! unpublished component-shell script API, whose Rust equivalents the other
//! gallery areas already port.

use gpui_kit::component::IconName;
use gpui_kit::*;

use crate::gallery::demo::punt_page;
use crate::gallery::registry::GallerySection;

// Quoted verbatim from examples/js_story/catalog.js: the manifest's own point
// is that nothing is elided, so the quote keeps all eight family imports.
const CATALOG_SHAPE: &str = "```js
// Keep every import explicit. A missing family module is a load error instead
// of a silently incomplete gallery, which makes this the reviewable inventory.
import { stories as foundations } from \"./stories/foundations.js\";
import { stories as actions } from \"./stories/actions.js\";
import { stories as inputs } from \"./stories/inputs.js\";
import { stories as navigation } from \"./stories/navigation.js\";
import { stories as content } from \"./stories/content.js\";
import { stories as overlays } from \"./stories/overlays.js\";
import { stories as collections } from \"./stories/collections.js\";
import { stories as layouts } from \"./stories/layouts.js\";
import { coveredBy } from \"./stories/coverage.js\";

export { coveredBy } from \"./stories/coverage.js\";

/** The complete JavaScript Story route manifest, in Rust Story display order. */
const byRustStory = [
  ...foundations,
  ...actions,
  ...inputs,
  ...navigation,
  ...content,
  ...overlays,
  ...collections,
  ...layouts,
];
```";

pub struct JsStorySection;

impl JsStorySection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self)
    }
}

impl Render for JsStorySection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        punt_page(
            "js-story-why-none",
            "Why the story catalog is Rust-only here",
            "The upstream example mirrors the story gallery in JavaScript over \
             the unpublished script API.",
            IconName::BookOpen,
            "examples/js_story is a JavaScript twin of this gallery: a \
             route catalog of story sections with a search field, written \
             against the component-shell script API.",
            "A coverage audit derives the catalog from the component inventory and \
             fails when either side drifts, so the manifest stays reviewable.",
            "js-story-catalog",
            CATALOG_SHAPE,
            &[
                "The script API it targets is unpublished and expected to change, so the \
                 harness is not ported.",
                "Every component the catalog routes to is already here as a Rust section; \
                 this gallery's own sidebar and search field are the native equivalent of \
                 the harness.",
            ],
            cx,
        )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "js-story",
        "JS Story Catalog",
        "Why the catalog is Rust-only: the route manifest a script harness would \
         load, quoted in full with the coverage audit that keeps it reviewable. \
         The script API it targets is unpublished, so the harness does not run.",
        JsStorySection::view(window, cx),
    ));
}
