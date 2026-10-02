//! Selectable Text section, ported from the upstream base-layer showcase
//! (`showcase/components/text_selection.rs`): four `SelectableText` runs
//! sharing one selection document, with a live selection readout.

use gpui_kit::base::{SelectableText, TextSelection, TextSelectionHandle};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, button::Button, v_flex};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

const PRODUCT_PARAGRAPH: &str = "Selection should feel like a natural part of reading a product brief. Start in this paragraph, continue into the next renderer, and GPUI preserves the document order while every frame supplies fresh geometry for the same stable selection handle.";
const IMPLEMENTATION_PARAGRAPH: &str = "This second paragraph is deliberately long enough to wrap in this section. Drag across the boundary to see one continuous highlight, then use the platform copy shortcut to confirm that the copied result follows the visible reading order rather than renderer ownership.";
const INTERNATIONAL_PARAGRAPH: &str = "International text should remain predictable when a line mixes café, déjà vu, Kraków, naïve, and résumé. Resize the window or drag across several wrapped lines; UTF-8 byte ranges still map back to the correct glyphs without splitting a character.";

pub struct SelectableTextSection {
    handles: [TextSelectionHandle; 4],
    _subscriptions: Vec<Subscription>,
}

impl SelectableTextSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let handles = [
            TextSelectionHandle::new("", cx),
            TextSelectionHandle::new("", cx),
            TextSelectionHandle::new("", cx),
            TextSelectionHandle::new("", cx),
        ];
        // A selection change must repaint the window so the readout and the
        // highlights stay in step; each handle owns its own subscription.
        let _subscriptions = handles
            .iter()
            .map(|handle| handle.refresh_window_on_change(window, cx))
            .collect();

        Self {
            handles,
            _subscriptions,
        }
    }

    fn paragraph(
        &self,
        ix: usize,
        id: &'static str,
        text: &'static str,
        heading: bool,
        cx: &App,
    ) -> impl IntoElement {
        // document_order is what makes a drag that spans runs read back in
        // page order instead of paint order.
        let run = SelectableText::with_handle(id, self.handles[ix].clone(), text)
            .document_order(ix as u64);
        let base = if heading {
            div().text_lg().font_weight(FontWeight::SEMIBOLD)
        } else {
            div()
                .line_height(rems(1.375))
                .text_color(cx.theme().muted_foreground)
        };
        base.child(run)
    }
}

impl Render for SelectableTextSection {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Read per frame rather than stored: the subscriptions above repaint
        // the window whenever a drag changes the selection.
        let active = TextSelection::has_selection(window, cx);
        let selected_text = if active {
            TextSelection::selected_text(window, cx)
        } else {
            "Drag across any paragraphs to select text.".to_owned()
        };

        v_flex()
            .w(rems(38.75))
            .max_w_full()
            .gap_3()
            .p_4()
            .child(self.paragraph(
                0,
                "selectable-text-heading",
                "Text selection across renderers",
                true,
                cx,
            ))
            .child(self.paragraph(1, "selectable-text-product", PRODUCT_PARAGRAPH, false, cx))
            .child(self.paragraph(
                2,
                "selectable-text-implementation",
                IMPLEMENTATION_PARAGRAPH,
                false,
                cx,
            ))
            .child(self.paragraph(
                3,
                "selectable-text-international",
                INTERNATIONAL_PARAGRAPH,
                false,
                cx,
            ))
            .child(
                v_flex()
                    .flex_none()
                    .gap_2()
                    .p_3()
                    .bg(cx.theme().muted)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(if active {
                        "Selection active"
                    } else {
                        "No selection"
                    }))
                    .child(
                        div()
                            .text_color(cx.theme().muted_foreground)
                            .child(selected_text),
                    )
                    .child(
                        Button::new("selectable-text-clear")
                            .small()
                            .self_start()
                            .label("Clear selection")
                            .on_click(cx.listener(|_, _, window, cx| {
                                TextSelection::clear(window, cx);
                                cx.notify();
                            })),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "selectable-text",
        "Selectable Text",
        "Plain-text runs that share one window selection document. Drag across paragraphs; the readout shows the text a copy would produce, in reading order rather than renderer ownership.",
        SelectableTextSection::view(window, cx),
    ));
}
