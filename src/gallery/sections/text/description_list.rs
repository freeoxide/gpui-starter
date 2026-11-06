//! Description List section, ported from the upstream `DescriptionListStory`.

use gpui_kit::component::{
    AxisExt as _, Sizable as _, Size,
    button::{Button, DropdownButton},
    description_list::{DescriptionItem, DescriptionList},
    text::TextView,
    v_flex,
};
use gpui_kit::*;
use serde::Deserialize;

use crate::gallery::demo::{self, SizeToggle, demo_toolbar};
use crate::gallery::registry::GallerySection;

/// Toolbar checks for the section: the two list options plus the shared size
/// variants the story's `ChangeStorySize` action carried.
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_text, no_json)]
enum DemoToggle {
    Vertical,
    Bordered,
    SizeXSmall,
    SizeSmall,
    SizeMedium,
    SizeLarge,
}

impl DemoToggle {
    /// The size this toggle selects; `None` for the list options.
    fn size(&self) -> Option<Size> {
        match self {
            Self::SizeXSmall => Some(Size::XSmall),
            Self::SizeSmall => Some(Size::Small),
            Self::SizeMedium => Some(Size::Medium),
            Self::SizeLarge => Some(Size::Large),
            _ => None,
        }
    }
}

impl SizeToggle for DemoToggle {
    fn xsmall() -> Self {
        Self::SizeXSmall
    }
    fn small() -> Self {
        Self::SizeSmall
    }
    fn medium() -> Self {
        Self::SizeMedium
    }
    fn large() -> Self {
        Self::SizeLarge
    }
}

pub struct DescriptionListSection {
    layout: Axis,
    bordered: bool,
    size: Size,
    items: Vec<(&'static str, &'static str, usize)>,
}

impl DescriptionListSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self::new())
    }

    fn new() -> Self {
        let items = vec![
            ("Name", "GPUI Kit", 1),
            (
                "Description",
                "UI components for building fantastic desktop application by using GPUI.\n\nContains a lot of useful UI components, such as **Button**, **Input**, **Table**, **List**, **Select**, **DatePicker** …\n\nYou can easily create your native desktop application by using GPUI Kit.",
                3,
            ),
            ("Version", "0.1.0", 1),
            ("License", "Apache-2.0", 1),
            ("Author", "Longbridge", 1),
            ("--", "--", 1),
            ("Repository", "https://github.com/longbridge/gpui-kit", 2),
            ("Category", "UI, Desktop, Framework", 1),
            (
                "This is a long label for Platform",
                "macOS, Windows, Linux",
                1,
            ),
        ];

        Self {
            items,
            bordered: true,
            size: Size::default(),
            layout: Axis::Horizontal,
        }
    }
}

impl Render for DescriptionListSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let vertical = self.layout.is_vertical();
        let bordered = self.bordered;
        let size = self.size;

        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, action: &DemoToggle, _, cx| {
                match action {
                    DemoToggle::Vertical => {
                        this.layout = if this.layout.is_vertical() {
                            Axis::Horizontal
                        } else {
                            Axis::Vertical
                        };
                    }
                    DemoToggle::Bordered => this.bordered = !this.bordered,
                    _ => {
                        if let Some(size) = action.size() {
                            this.size = size;
                        }
                    }
                }
                cx.notify();
            }))
            .child(demo_toolbar(vec![
                demo::size_dropdown::<DemoToggle>("description-list-size", size).into_any_element(),
                DropdownButton::new("description-list-options")
                    .button(Button::new("description-list-options-trigger").label("Options"))
                    .dropdown_menu(move |menu, _, _| {
                        menu.menu_with_check("Vertical", vertical, Box::new(DemoToggle::Vertical))
                            .menu_with_check("Bordered", bordered, Box::new(DemoToggle::Bordered))
                    })
                    .into_any_element(),
            ]))
            .child(
                div().w(rems(45.)).child(
                    DescriptionList::new()
                        .columns(3)
                        .layout(self.layout)
                        .bordered(self.bordered)
                        .with_size(self.size)
                        .children(self.items.iter().map(|&(label, value, span)| {
                            if label == "--" {
                                return DescriptionItem::Separator;
                            }

                            DescriptionItem::new(label)
                                .value(
                                    TextView::markdown(
                                        SharedString::from(format!(
                                            "description-list-value-{label}"
                                        )),
                                        value,
                                    )
                                    .into_any_element(),
                                )
                                .span(span)
                        })),
                ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "description-list",
        "Description List",
        "Present labels and values in a structured summary.",
        DescriptionListSection::view(window, cx),
    ));
}
