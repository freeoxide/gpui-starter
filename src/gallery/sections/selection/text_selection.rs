//! Text Selection section, ported from the upstream `text_selection`
//! example: one drag started anywhere in the window spans multiple
//! `TextView`s and keeps the top-to-bottom order when copied.

use gpui_kit::component::{
    ActiveTheme as _,
    button::Button,
    h_flex,
    input::{Input, InputState},
    text::TextView,
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::gallery::registry::GallerySection;

pub struct TextSelectionSection {
    input: Entity<InputState>,
}

impl TextSelectionSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            input: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Type here (selection must NOT start from here)")
            }),
        }
    }

    fn bubble(&self, ix: usize, text: &str, mine: bool, cx: &App) -> impl IntoElement {
        div().flex().when(mine, |this| this.justify_end()).child(
            div()
                .max_w(rems(26.25))
                .p_3()
                .rounded(cx.theme().radius_lg)
                .bg(if mine {
                    cx.theme().primary.opacity(0.1)
                } else {
                    cx.theme().muted
                })
                // `selectable(true)` opts this TextView into window-level
                // selection, so a drag started anywhere can extend into it.
                .child(TextView::markdown(("text-selection-msg", ix), text).selectable(true)),
        )
    }
}

impl Render for TextSelectionSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_3()
            .p_4()
            .child(self.bubble(0, "**Hello!** How can I help you today?", false, cx))
            .child(self.bubble(
                1,
                "I want to select text *across* multiple bubbles.",
                true,
                cx,
            ))
            .child(self.bubble(
                2,
                "Sure, drag from anywhere, even from the blank space between \
                 bubbles, then press `cmd-c` to copy everything.",
                false,
                cx,
            ))
            .child(self.bubble(3, "Nice, it also keeps the top-to-bottom order.", true, cx))
            .child(h_flex().child(
                Button::new("text-selection-noop").label("Clicking me must not start selection"),
            ))
            .child(Input::new(&self.input))
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "text-selection",
        "Text Selection",
        "Drag from anywhere in the pane, even the blank space between bubbles, to select across several TextViews. The copy keeps the top-to-bottom order; the button and the input keep their own selection behavior.",
        TextSelectionSection::view(window, cx),
    ));
}
