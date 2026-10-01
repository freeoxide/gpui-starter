//! Title bar section, ported from the `window_title` example. The app shell
//! owns this window's real title bar, so the component renders inside the pane.

use gpui_kit::component::{
    ActiveTheme as _, StyledExt as _, TitleBar, WindowExt as _,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::section;

pub struct TitleBarSection;

impl TitleBarSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self)
    }
}

impl Render for TitleBarSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .child(
                section("title-bar-box", "Window title bar")
                    .description(
                        "Title content and platform window controls in one bar. On Linux its close control answers with a notification instead of closing this window.",
                    )
                    .v_flex()
                    .gap_3()
                    .child(
                        // The demo frame stands in for the window the example
                        // owned; the title bar draws at its top edge.
                        v_flex()
                            .w_full()
                            .h(rems(25.))
                            .rounded(cx.theme().radius)
                            .border_1()
                            .border_color(cx.theme().border)
                            .overflow_hidden()
                            .child(
                                TitleBar::new()
                                    .on_close_window(|_, window, cx| {
                                        window.push_notification("Close was requested.", cx);
                                    })
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .pr_2()
                                            .justify_between()
                                            .child("App with custom title bar")
                                            .child("Right Item"),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .flex_1()
                                    .min_h_0()
                                    .items_center()
                                    .justify_center()
                                    .gap_3()
                                    .p_5()
                                    .child("Hello, World!")
                                    .child(
                                        Button::new("title-bar-go")
                                            .primary()
                                            .label("Let's Go!")
                                            .on_click(|_, window, cx| {
                                                window.push_notification("Let's Go! was clicked.", cx);
                                            }),
                                    ),
                            ),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "title-bar",
        "Title Bar",
        "A custom window title bar with window controls and custom content.",
        TitleBarSection::view(window, cx),
    ));
}
