//! Clipboard section, ported from the upstream `ClipboardStory`.

use gpui_kit::component::{
    WindowExt, clipboard::Clipboard, h_flex, input::Input, input::InputState, label::Label, v_flex,
};
use gpui_kit::*;
use gpui_kit::{Entity, SharedString, rems};

use crate::gallery::registry::GallerySection;

use super::demo::section;

pub struct ClipboardSection {
    url_state: Entity<InputState>,
    masked: bool,
}

impl ClipboardSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let url_state =
                cx.new(|cx| InputState::new(window, cx).default_value("https://github.com"));

            Self {
                url_state,
                masked: false,
            }
        })
    }
}

impl Render for ClipboardSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .items_center()
            .justify_start()
            .gap_3()
            .p_4()
            .child(
                section("clipboard-default", "Default")
                    .description("Copies a value supplied by the application.")
                    .w(rems(30.))
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Label::new("A clipboard button"))
                            .child(
                                Clipboard::new("clipboard-value")
                                    .value_fn({
                                        let view = cx.entity().clone();
                                        move |_, cx| {
                                            SharedString::from(format!(
                                                "masked: {}",
                                                view.read(cx).masked
                                            ))
                                        }
                                    })
                                    .on_copied(|value, window, cx| {
                                        window.push_notification(
                                            format!("Copied value: {}", value),
                                            cx,
                                        )
                                    }),
                            ),
                    ),
            )
            .child(
                section("clipboard-input", "With Input")
                    .description("Copies the field's current value.")
                    .w(rems(30.))
                    .child(
                        Input::new(&self.url_state).suffix(
                            Clipboard::new("clipboard-field")
                                .value_fn({
                                    let state = self.url_state.clone();
                                    move |_, cx| state.read(cx).value()
                                })
                                .on_copied(|value, window, cx| {
                                    window.push_notification(format!("Copied value: {}", value), cx)
                                }),
                        ),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "clipboard",
        "Clipboard",
        "Copy text or generated values to the clipboard.",
        ClipboardSection::view(window, cx),
    ));
}
