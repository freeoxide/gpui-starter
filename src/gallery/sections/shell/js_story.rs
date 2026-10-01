//! JS story punt section: examples/js_story is a JavaScript catalog over the
//! unpublished component-shell script API, whose Rust equivalents the other
//! gallery areas already port.

use gpui_kit::component::text::TextView;
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, StyledExt as _, h_flex, v_flex};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::section;

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
        let muted = cx.theme().muted_foreground;
        let paragraph = |line: &str| {
            div()
                .text_sm()
                .text_color(muted)
                .child(SharedString::from(line))
        };

        v_flex().w_full().items_center().gap_6().p_4().child(
            section(
                "js-story-why-none",
                "Why the story catalog is Rust-only here",
            )
            .description(
                "The upstream example mirrors the story gallery in JavaScript over \
                     the unpublished script API.",
            )
            .w(rems(30.))
            .v_flex()
            .items_start()
            .gap_3()
            .child(
                h_flex()
                    .gap_3()
                    .child(Icon::new(IconName::BookOpen).size_6())
                    .child(div().text_sm().child(
                        "examples/js_story is a JavaScript twin of this gallery: a \
                                 route catalog of story sections with a search field, written \
                                 against the component-shell script API.",
                    )),
            )
            .child(paragraph(
                "A coverage audit derives the catalog from the component inventory and \
                     fails when either side drifts, so the manifest stays reviewable.",
            ))
            .child(
                v_flex()
                    .w_full()
                    .gap_2()
                    .child(TextView::markdown("js-story-catalog", CATALOG_SHAPE)),
            )
            .child(paragraph(
                "The script API it targets is unpublished and expected to change, so the \
                     harness is not ported.",
            ))
            .child(paragraph(
                "Every component the catalog routes to is already here as a Rust section; \
                     this gallery's own sidebar and search field are the native equivalent of \
                     the harness.",
            )),
        )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "js-story",
        "JS Story Catalog",
        "A JavaScript mirror of the story catalog over the unpublished script API; \
         not portable here.",
        JsStorySection::view(window, cx),
    ));
}
