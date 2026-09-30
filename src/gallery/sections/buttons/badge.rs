//! Badge section, ported from the upstream `BadgeStory`.

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, Size, avatar::Avatar, badge::Badge, v_flex,
};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, section, size_dropdown};

pub struct BadgeSection {
    size: Size,
}

impl BadgeSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self { size: Size::Medium })
    }
}

impl Render for BadgeSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let size = self.size;
        let avatar = |src: &'static str| Avatar::new().with_size(size).src(src);

        v_flex()
            .w_full()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, action: &DemoToggle, _, cx| {
                if let Some(size) = action.size() {
                    this.size = size;
                    cx.notify();
                }
            }))
            .child(demo_toolbar(vec![
                size_dropdown("badge-size", size).into_any_element(),
            ]))
            .child(
                section("badge-icon", "Icon")
                    .w_128()
                    .child(
                        Badge::new()
                            .with_size(size)
                            .count(3)
                            .child(Icon::new(IconName::Bell).with_size(size)),
                    )
                    .child(
                        Badge::new()
                            .with_size(size)
                            .count(103)
                            .child(Icon::new(IconName::Inbox).with_size(size)),
                    ),
            )
            .child(
                section("badge-count", "Count")
                    .w_128()
                    .child(
                        Badge::new()
                            .with_size(size)
                            .count(3)
                            .child(avatar("https://avatars.githubusercontent.com/u/5518?v=4")),
                    )
                    .child(Badge::new().with_size(size).count(103).child(avatar(
                        "https://avatars.githubusercontent.com/u/28998859?v=4",
                    ))),
            )
            .child(
                section("badge-status-icon", "Badge icon")
                    .w_128()
                    .child(
                        Badge::new()
                            .with_size(size)
                            .icon(IconName::Check)
                            .color(cx.theme().cyan)
                            .child(avatar("https://avatars.githubusercontent.com/u/5518?v=4")),
                    )
                    .child(
                        Badge::new()
                            .with_size(size)
                            .icon(IconName::Star)
                            .color(cx.theme().yellow)
                            .child(avatar(
                                "https://avatars.githubusercontent.com/u/20092316?v=4",
                            )),
                    ),
            )
            .child(
                section("badge-dot", "Dot").w(rems(30.)).child(
                    Badge::new()
                        .with_size(size)
                        .dot()
                        .count(1)
                        .child(avatar("https://avatars.githubusercontent.com/u/5518?v=4")),
                ),
            )
            .child(
                section("badge-color", "Color")
                    .w_128()
                    .child(
                        Badge::new()
                            .with_size(size)
                            .count(3)
                            .color(cx.theme().blue)
                            .child(avatar("https://avatars.githubusercontent.com/u/5518?v=4")),
                    )
                    .child(
                        Badge::new()
                            .with_size(size)
                            .dot()
                            .color(cx.theme().green)
                            .count(1)
                            .child(avatar("https://avatars.githubusercontent.com/u/5518?v=4")),
                    ),
            )
            .child(
                section("badge-nested", "Nested")
                    .w_128()
                    .child(
                        Badge::new().with_size(size).count(212).large().child(
                            Badge::new()
                                .with_size(size)
                                .icon(IconName::Check)
                                .color(cx.theme().cyan)
                                .child(avatar("https://avatars.githubusercontent.com/u/5518?v=4")),
                        ),
                    )
                    .child(
                        Badge::new()
                            .with_size(size)
                            .count(2)
                            .color(cx.theme().green)
                            .large()
                            .child(
                                Badge::new()
                                    .with_size(size)
                                    .icon(IconName::Star)
                                    .color(cx.theme().yellow)
                                    .child(Avatar::new().with_size(size).large().src(
                                        "https://avatars.githubusercontent.com/u/20092316?v=4",
                                    )),
                            ),
                    )
                    .child(
                        Badge::new()
                            .with_size(size)
                            .count(3)
                            .color(cx.theme().green)
                            .child(
                                Badge::new()
                                    .with_size(size)
                                    .icon(IconName::Asterisk)
                                    .color(cx.theme().green)
                                    .child(avatar(
                                        "https://avatars.githubusercontent.com/u/22312482?v=4",
                                    )),
                            ),
                    )
                    .child(
                        Badge::new().with_size(size).dot().child(
                            Badge::new()
                                .with_size(size)
                                .icon(IconName::Sun)
                                .color(cx.theme().red)
                                .child(
                                    Avatar::new().with_size(size).small().src(
                                        "https://avatars.githubusercontent.com/u/150917089?v=4",
                                    ),
                                ),
                        ),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "badge",
        "Badge",
        "A compact indicator for counts, dots, and status overlays.",
        BadgeSection::view(window, cx),
    ));
}
