//! Breadcrumb section, ported from the upstream `BreadcrumbStory`.

use gpui_kit::component::{
    breadcrumb::{Breadcrumb, BreadcrumbItem},
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::gallery::registry::GallerySection;

use super::demo::section;

pub struct BreadcrumbSection {
    clicked_item: Option<String>,
}

impl BreadcrumbSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self { clicked_item: None })
    }
}

impl Render for BreadcrumbSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_6()
            .p_4()
            .child(
                section("breadcrumb-default", "Default")
                    .description("Shows the current location in a hierarchy.")
                    .w(rems(30.))
                    .child(
                        Breadcrumb::new()
                            .child("Home")
                            .child("Documents")
                            .child("Projects"),
                    ),
            )
            .child(
                section("breadcrumb-interactive", "Interactive")
                    .description("Earlier levels can respond to navigation clicks.")
                    .w(rems(30.))
                    .child(
                        v_flex()
                            .gap_4()
                            .items_center()
                            .child(
                                Breadcrumb::new()
                                    .child("Home")
                                    .child(BreadcrumbItem::new("Documents").on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.clicked_item = Some("Documents".to_string());
                                            cx.notify();
                                        },
                                    )))
                                    .child(BreadcrumbItem::new("Projects").on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.clicked_item = Some("Projects".to_string());
                                            cx.notify();
                                        },
                                    )))
                                    .child(BreadcrumbItem::new("Current").on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.clicked_item = Some("Current".to_string());
                                            cx.notify();
                                        },
                                    ))),
                            )
                            .when_some(self.clicked_item.as_ref(), |this, item| {
                                this.child(format!("Selected: {}", item))
                            }),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "breadcrumb",
        "Breadcrumb",
        "A breadcrumb navigation element that shows the current location in a hierarchy.",
        BreadcrumbSection::view(window, cx),
    ));
}
