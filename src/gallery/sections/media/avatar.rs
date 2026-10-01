//! Avatar section, ported from the upstream `AvatarStory`.

use gpui_kit::component::{
    ActiveTheme as _, IconName, Sizable as _, Size, StyledExt as _,
    avatar::{Avatar, AvatarGroup},
    button::{Button, DropdownButton},
    h_flex, v_flex,
};
use gpui_kit::*;
use serde::Deserialize;

use crate::gallery::registry::GallerySection;

use super::demo::section;

const AVATARS: [&str; 11] = [
    "https://avatars.githubusercontent.com/u/5518?v=4",
    "https://avatars.githubusercontent.com/u/28998859?v=4",
    "https://avatars.githubusercontent.com/u/20092316?v=4",
    "https://avatars.githubusercontent.com/u/22312482?v=4",
    "https://avatars.githubusercontent.com/u/150917089?v=4",
    "https://avatars.githubusercontent.com/u/20337280?v=4",
    "https://avatars.githubusercontent.com/u/629429?v=4",
    "https://avatars.githubusercontent.com/u/583231?v=4",
    "https://avatars.githubusercontent.com/u/1264109?v=4",
    "https://avatars.githubusercontent.com/u/2936367?v=4",
    "https://avatars.githubusercontent.com/u/1253486?v=4",
];

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_media_avatar, no_json)]
enum AvatarToggle {
    SizeXSmall,
    SizeSmall,
    SizeMedium,
    SizeLarge,
}

fn size_label(size: Size) -> &'static str {
    match size {
        Size::XSmall => "XSmall",
        Size::Small => "Small",
        Size::Medium => "Medium",
        Size::Large => "Large",
        Size::Size(_) => "Custom",
    }
}

/// The story toolbar's size switcher, on the house split button.
fn size_dropdown(size: Size) -> DropdownButton {
    DropdownButton::new("avatar-size")
        .button(Button::new("avatar-size-trigger").label(format!("Size: {}", size_label(size))))
        .dropdown_menu(move |menu, _, _| {
            menu.menu_with_check(
                "XSmall",
                size == Size::XSmall,
                Box::new(AvatarToggle::SizeXSmall),
            )
            .menu_with_check(
                "Small",
                size == Size::Small,
                Box::new(AvatarToggle::SizeSmall),
            )
            .menu_with_check(
                "Medium",
                size == Size::Medium,
                Box::new(AvatarToggle::SizeMedium),
            )
            .menu_with_check(
                "Large",
                size == Size::Large,
                Box::new(AvatarToggle::SizeLarge),
            )
        })
}

pub struct AvatarSection {
    size: Size,
}

impl AvatarSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self { size: Size::Medium })
    }
}

impl Render for AvatarSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .w_full()
            .items_center()
            .p_4()
            .on_action(cx.listener(|this, action: &AvatarToggle, _, cx| {
                this.size = match action {
                    AvatarToggle::SizeXSmall => Size::XSmall,
                    AvatarToggle::SizeSmall => Size::Small,
                    AvatarToggle::SizeMedium => Size::Medium,
                    AvatarToggle::SizeLarge => Size::Large,
                };
                cx.notify();
            }))
            .child(
                h_flex()
                    .w_full()
                    .justify_end()
                    .gap_1()
                    .child(size_dropdown(self.size)),
            )
            .child(
                section("avatar-image", "Image")
                    .description("Use an image when one is available.")
                    .w_128()
                    .child(
                        Avatar::new()
                            .name("Jason Lee")
                            .src(AVATARS[0])
                            .with_size(self.size),
                    )
                    .child(Avatar::new().src(AVATARS[1]).with_size(self.size)),
            )
            .child(
                section("avatar-fallback", "Fallback")
                    .description("Show initials or an icon when no image is available.")
                    .w_128()
                    .child(Avatar::new().name("Jason Lee").with_size(self.size))
                    .child(Avatar::new().with_size(self.size))
                    .child(
                        Avatar::new()
                            .placeholder(IconName::Building2)
                            .with_size(self.size),
                    ),
            )
            .child(
                section("avatar-group", "Group")
                    .description("Groups can limit visible avatars and show overflow.")
                    .v_flex()
                    .w_128()
                    .items_center()
                    .gap_5()
                    .child(
                        AvatarGroup::new()
                            .with_size(self.size)
                            .children(AVATARS[..6].iter().map(|src| Avatar::new().src(*src))),
                    )
                    .child(
                        AvatarGroup::new()
                            .with_size(self.size)
                            .limit(5)
                            .ellipsis()
                            .children(AVATARS.iter().map(|src| Avatar::new().src(*src))),
                    ),
            )
            .child(
                section("avatar-custom-shape", "Custom shape")
                    .description("Set an explicit size and corner radius.")
                    .child(
                        Avatar::new()
                            .src(AVATARS[0])
                            .with_size(px(100.))
                            .rounded(px(20.)),
                    ),
            )
            .child(
                section("avatar-custom-style", "Custom style")
                    .description("Add borders and shadows to the image.")
                    .child(
                        Avatar::new()
                            .src(AVATARS[2])
                            .with_size(px(100.))
                            .border_3()
                            .border_color(cx.theme().foreground)
                            .shadow_sm(),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "avatar",
        "Avatar",
        "Represent a person or organization with an image or fallback.",
        AvatarSection::view(window, cx),
    ));
}
