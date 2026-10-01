//! Status bar section, ported from the upstream `StatusBarStory`.

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    progress::ProgressCircle,
    separator::Separator,
    status_bar::StatusBar,
    v_flex,
};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::section;

pub struct StatusBarSection;

impl StatusBarSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self)
    }
}

impl Render for StatusBarSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .child(
                section("status-bar-editor-box", "Editor")
                    .description(
                        "Places repository state on the left and document state on the right.",
                    )
                    .max_w(rems(47.5))
                    .v_flex()
                    .w_full()
                    .child(
                        StatusBar::new()
                            .left(
                                Button::new("status-bar-branch")
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Github)
                                    .label("main")
                                    .tooltip("Git branch")
                                    .on_click(|_, window, cx| {
                                        window.push_notification("Switch branch", cx);
                                    }),
                            )
                            .left(Separator::vertical().h_3())
                            .left(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_1()
                                            .child(
                                                Icon::new(IconName::CircleCheck)
                                                    .xsmall()
                                                    .text_color(cx.theme().green),
                                            )
                                            .child("0"),
                                    )
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_1()
                                            .child(
                                                Icon::new(IconName::Info)
                                                    .xsmall()
                                                    .text_color(cx.theme().blue),
                                            )
                                            .child("2"),
                                    ),
                            )
                            .right(
                                Button::new("status-bar-position")
                                    .ghost()
                                    .xsmall()
                                    .label("Ln 12, Col 34")
                                    .tooltip("Go to Line/Column")
                                    .on_click(|_, window, cx| {
                                        window.push_notification("Go to Line/Column", cx);
                                    }),
                            )
                            .right(Separator::vertical().h_3())
                            .right(
                                Button::new("status-bar-encoding")
                                    .ghost()
                                    .xsmall()
                                    .label("UTF-8")
                                    .on_click(|_, window, cx| {
                                        window.push_notification("Select encoding", cx);
                                    }),
                            )
                            .right(
                                Button::new("status-bar-language")
                                    .ghost()
                                    .xsmall()
                                    .label("Rust")
                                    .on_click(|_, window, cx| {
                                        window.push_notification("Select language", cx);
                                    }),
                            ),
                    ),
            )
            .child(
                section("status-bar-application-box", "Application")
                    .description("Combines connectivity, progress, save state, and notifications.")
                    .max_w(rems(47.5))
                    .v_flex()
                    .w_full()
                    .child(
                        StatusBar::new()
                            .left(
                                h_flex()
                                    .items_center()
                                    .gap_1()
                                    .child(Icon::new(IconName::Check).xsmall())
                                    .child("Connected"),
                            )
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(ProgressCircle::new("status-bar-syncing").value(45.))
                                    .child("Syncing…"),
                            )
                            .right("All changes saved")
                            .right(
                                Button::new("status-bar-notifications")
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Bell)
                                    .label("3")
                                    .tooltip("3 notifications")
                                    .on_click(|_, window, cx| {
                                        window.push_notification("3 notifications", cx);
                                    }),
                            ),
                    ),
            )
            .child(
                section("status-bar-alignment-box", "Alignment")
                    .description("Center content adapts when either side is empty or populated.")
                    .max_w(rems(47.5))
                    .v_flex()
                    .w_full()
                    .gap_6()
                    .child(StatusBar::new().child("Center only → start-aligned"))
                    .child(
                        StatusBar::new()
                            .left("Left")
                            .child("Center → end (only left)"),
                    )
                    .child(
                        StatusBar::new()
                            .child("Center → start (only right)")
                            .right("Right"),
                    )
                    .child(
                        StatusBar::new()
                            .left("Left")
                            .child("Center → centered (left + right)")
                            .right("Right"),
                    )
                    .child(StatusBar::new().left("Left").right("Right")),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "status-bar",
        "Status Bar",
        "A horizontal bar with left/center/right regions, usually placed at the bottom.",
        StatusBarSection::view(window, cx),
    ));
}
