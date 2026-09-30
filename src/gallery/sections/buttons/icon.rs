//! Icon section, ported from the upstream `IconStory`.

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _,
    button::{Button, ButtonVariant, ButtonVariants as _, DropdownButton},
    h_flex, v_flex,
};
use gpui_kit::*;
use serde::Deserialize;

use crate::gallery::registry::GallerySection;

use super::demo::section;

const SEARCH_SVG: &[u8] = include_bytes!("assets/search.svg");
const ARROW_SVG: &[u8] = include_bytes!("assets/arrow-up.svg");
const LOADER_SVG: &[u8] = include_bytes!("assets/loader-circle.svg");

struct Search;

impl From<Search> for Icon {
    fn from(_: Search) -> Self {
        Icon::default().data(SEARCH_SVG)
    }
}

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_icon_section, no_json)]
enum IconMenu {
    RunSearch,
}

pub struct IconSection {
    arrow: Icon,
    arrow_view: Entity<Icon>,
    message: &'static str,
}

impl IconSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let arrow = Icon::default()
                .data(ARROW_SVG)
                .rotate(radians(std::f32::consts::FRAC_PI_2))
                .large();
            Self {
                arrow_view: arrow.clone().view(cx),
                arrow,
                message: "Choose a button or menu item to try the icon slots.",
            }
        })
    }
}

impl Render for IconSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, _: &IconMenu, _, cx| {
                this.message = "Search selected from the menu.";
                cx.notify();
            }))
            .child(
                section("icon-svg-bytes", "SVG bytes")
                    .description(
                        "Embedded icons share the same sizing, colors, and loading behavior.",
                    )
                    .w(rems(30.))
                    .child(
                        v_flex()
                            .gap_3()
                            .child(
                                h_flex()
                                    .gap_4()
                                    .child(Icon::default().data(SEARCH_SVG).small())
                                    .child(
                                        Icon::default()
                                            .data(SEARCH_SVG)
                                            .large()
                                            .text_color(cx.theme().primary),
                                    )
                                    .child(
                                        Button::new("icon-embedded-search")
                                            .icon(Search)
                                            .label("Search")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.message = "Search selected from the button.";
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Button::new("icon-embedded-loading")
                                            .icon(Search)
                                            .loading_icon(Icon::default().data(LOADER_SVG))
                                            .loading(true)
                                            .label("Searching"),
                                    )
                                    .child(
                                        DropdownButton::new("icon-embedded-menu")
                                            .button(
                                                Button::new("icon-embedded-menu-main")
                                                    .label("Actions"),
                                            )
                                            .dropdown_menu(|menu, _, _| {
                                                menu.menu("Search", Box::new(IconMenu::RunSearch))
                                            }),
                                    ),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(self.message),
                            ),
                    ),
            )
            .child(
                section("icon-clone-views", "Cloning and views")
                    .description("Both arrows retain their SVG bytes and 90° rotation.")
                    .w(rems(30.))
                    .child(h_flex().gap_4().child("Clone").child(self.arrow.clone()))
                    .child(
                        h_flex()
                            .gap_4()
                            .child("Entity")
                            .child(self.arrow_view.clone()),
                    ),
            )
            .child(
                section("icon-catalog", "Icons")
                    .description("Common interface symbols from the bundled icon set.")
                    .w(rems(30.))
                    .text_lg()
                    .child(IconName::Info)
                    .child(IconName::Map)
                    .child(IconName::Bot)
                    .child(IconName::Github)
                    .child(IconName::Calendar)
                    .child(IconName::Globe)
                    .child(IconName::Heart),
            )
            .child(
                section("icon-color", "Color")
                    .description("Icons inherit semantic foreground colors.")
                    .w(rems(30.))
                    .child(
                        Icon::new(IconName::Maximize)
                            .size_6()
                            .text_color(cx.theme().green),
                    )
                    .child(
                        Icon::new(IconName::Minimize)
                            .size_6()
                            .text_color(cx.theme().red),
                    ),
            )
            .child(
                section("icon-buttons", "Icon Buttons")
                    .description("Icons can be used as compact button content.")
                    .w(rems(30.))
                    .child(
                        h_flex()
                            .gap_4()
                            .child(
                                Button::new("icon-like-neutral")
                                    .icon(
                                        Icon::new(IconName::Heart)
                                            .text_color(cx.theme().muted_foreground)
                                            .size_6(),
                                    )
                                    .with_variant(ButtonVariant::Ghost),
                            )
                            .child(
                                Button::new("icon-like-off")
                                    .icon(
                                        Icon::new(IconName::HeartOff)
                                            .text_color(cx.theme().red)
                                            .size_6(),
                                    )
                                    .with_variant(ButtonVariant::Ghost),
                            )
                            .child(
                                Button::new("icon-like-on")
                                    .icon(
                                        Icon::new(IconName::Heart)
                                            .text_color(cx.theme().green)
                                            .size_6(),
                                    )
                                    .with_variant(ButtonVariant::Ghost),
                            ),
                    ),
            )
            .child(
                section("icon-custom-size", "Custom Size")
                    .description("Explicit dimensions support dense controls and counters.")
                    .w(rems(30.))
                    .child(
                        Button::new("icon-counter")
                            .outline()
                            .size_5()
                            .small()
                            .px_0()
                            .label("10"),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "icon",
        "Icon",
        "SVG icons based on Lucide.dev.",
        IconSection::view(window, cx),
    ));
}
