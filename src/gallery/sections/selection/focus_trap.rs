//! Focus Trap section, ported from the upstream `focus_trap` example: Tab
//! cycles inside a `.focus_trap()` container and cannot escape it.

use gpui_kit::component::{
    ActiveTheme as _, FocusTrapElement as _, button::Button, h_flex, v_flex,
};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

pub struct FocusTrapSection {
    trap1_handle: FocusHandle,
    trap2_handle: FocusHandle,
}

impl FocusTrapSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            trap1_handle: cx.focus_handle(),
            trap2_handle: cx.focus_handle(),
        })
    }

    fn area_title(title: &'static str) -> Div {
        div()
            .text_base()
            .font_weight(FontWeight::SEMIBOLD)
            .child(title)
    }

    fn hint(text: &'static str, cx: &App) -> Div {
        div()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(text)
    }
}

impl Render for FocusTrapSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_6()
            .p_4()
            .child(
                v_flex()
                    .gap_3()
                    .child(Self::area_title("Outside Area (No Focus Trap)"))
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Button::new("focus-trap-outside-1").label("Outside Button 1"))
                            .child(Button::new("focus-trap-outside-2").label("Outside Button 2"))
                            .child(Button::new("focus-trap-outside-3").label("Outside Button 3")),
                    ),
            )
            .child(
                v_flex()
                    .gap_3()
                    .child(Self::area_title("Focus Trap Area 1"))
                    .child(
                        h_flex()
                            .gap_2()
                            .p_4()
                            .bg(cx.theme().secondary)
                            .rounded(cx.theme().radius)
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(Button::new("focus-trap-1-1").label("Trap 1 - Button 1"))
                            .child(Button::new("focus-trap-1-2").label("Trap 1 - Button 2"))
                            .child(Button::new("focus-trap-1-3").label("Trap 1 - Button 3"))
                            .focus_trap("focus-trap-area-1", &self.trap1_handle),
                    )
                    .child(Self::hint(
                        "→ Press Tab in this area, focus cycles through 3 buttons without escaping",
                        cx,
                    )),
            )
            .child(
                v_flex()
                    .gap_3()
                    .child(Self::area_title("Outside Area (No Focus Trap)"))
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Button::new("focus-trap-outside-4").label("Outside Button 4"))
                            .child(Button::new("focus-trap-outside-5").label("Outside Button 5")),
                    ),
            )
            .child(
                v_flex()
                    .gap_3()
                    .child(Self::area_title("Focus Trap Area 2"))
                    .child(
                        v_flex()
                            .focus_trap("focus-trap-area-2", &self.trap2_handle)
                            .gap_2()
                            .p_4()
                            .grid()
                            .grid_cols(4)
                            .bg(cx.theme().accent.opacity(0.1))
                            .rounded(cx.theme().radius)
                            .border_1()
                            .border_color(cx.theme().accent)
                            .child(Button::new("focus-trap-2-1").label("Trap 2 - Button 1"))
                            .child(Button::new("focus-trap-2-2").label("Trap 2 - Button 2"))
                            .child(Button::new("focus-trap-2-3").label("Trap 2 - Button 3"))
                            .child(Button::new("focus-trap-2-4").label("Trap 2 - Button 4")),
                    )
                    .child(Self::hint(
                        "→ Press Tab in this area, focus cycles through 4 buttons without escaping",
                        cx,
                    )),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "focus-trap",
        "Focus Trap",
        "Tab cycles focus inside a focus_trap container without escaping it. The outside rows are ordinary tab stops placed between the two trapped areas.",
        FocusTrapSection::view(window, cx),
    ));
}
