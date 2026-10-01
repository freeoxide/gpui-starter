//! Carousel section, ported from the upstream `CarouselStory`.

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, Size, StyledExt as _,
    button::Button,
    carousel::{
        Carousel, CarouselContent, CarouselEvent, CarouselItem, CarouselNext, CarouselPagination,
        CarouselPaginationItem, CarouselPrevious, CarouselState,
    },
    h_flex, v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, section, size_dropdown};

#[derive(Clone, Copy)]
enum SlideTypography {
    Large,
    Medium,
    Small,
}

pub struct CarouselSection {
    horizontal: Entity<CarouselState>,
    custom_controls: Entity<CarouselState>,
    multiple: Entity<CarouselState>,
    vertical: Entity<CarouselState>,
    looped: Entity<CarouselState>,
    controlled: Entity<CarouselState>,
    controlled_index: usize,
    size: Size,
    _subscriptions: Vec<Subscription>,
}

impl CarouselSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let horizontal = cx.new(|_| CarouselState::new(3));
            let custom_controls = cx.new(|_| CarouselState::new(3));
            let multiple = cx.new(|_| CarouselState::new(5));
            let vertical = cx.new(|_| CarouselState::new(3).with_axis(Axis::Vertical));
            let looped = cx.new(|_| CarouselState::new(4).with_looping(true));
            let controlled = cx.new(|_| CarouselState::new(3).with_selected_index(1));

            let subscription = cx.subscribe(
                &controlled,
                |this: &mut Self, _, event: &CarouselEvent, cx| {
                    let CarouselEvent::Change(index) = event;
                    this.controlled_index = *index;
                    cx.notify();
                },
            );

            Self {
                horizontal,
                custom_controls,
                multiple,
                vertical,
                looped,
                controlled,
                controlled_index: 1,
                size: Size::default(),
                _subscriptions: vec![subscription],
            }
        })
    }

    fn slide(
        label: impl Into<SharedString>,
        typography: SlideTypography,
        square: bool,
        cx: &App,
    ) -> impl IntoElement {
        div()
            .w_full()
            .when(square, |this| this.aspect_square())
            .when(!square, |this| this.h_full())
            .flex()
            .items_center()
            .justify_center()
            .p_6()
            .rounded(cx.theme().radius_tokens().xl)
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .font_semibold()
            .when(matches!(typography, SlideTypography::Large), |this| {
                this.text_size(rems(2.25))
            })
            .when(matches!(typography, SlideTypography::Medium), |this| {
                this.text_3xl()
            })
            .when(matches!(typography, SlideTypography::Small), |this| {
                this.text_2xl()
            })
            .child(label.into())
    }

    fn items(
        state: &Entity<CarouselState>,
        prefix: &'static str,
        count: usize,
        typography: SlideTypography,
        square: bool,
        cx: &App,
    ) -> CarouselContent {
        Self::items_with(state, prefix, count, typography, square, |item| item, cx)
    }

    fn items_with(
        state: &Entity<CarouselState>,
        prefix: &'static str,
        count: usize,
        typography: SlideTypography,
        square: bool,
        configure: impl Fn(CarouselItem) -> CarouselItem,
        cx: &App,
    ) -> CarouselContent {
        (0..count).fold(CarouselContent::new(state), |content, index| {
            let label = (index + 1).to_string();
            content.child(
                configure(CarouselItem::new((prefix, index), index, state))
                    .child(Self::slide(label, typography, square, cx)),
            )
        })
    }
}

impl Render for CarouselSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let size = self.size;

        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, action: &DemoToggle, _, cx| {
                if let Some(size) = action.size() {
                    this.size = size;
                    cx.notify();
                }
            }))
            .child(demo_toolbar(vec![size_dropdown("carousel-size", size).into_any_element()]))
            .child(
                section("carousel-basic-box", "Basic")
                    .description("Browse one full-width item at a time with Left and Right.")
                    .v_flex()
                    .gap_3()
                    .child(
                        Carousel::new("carousel-basic", &self.horizontal)
                            .w_full()
                            .max_w_96()
                            .mx_auto()
                            .child(Self::items(
                                &self.horizontal,
                                "carousel-basic-item",
                                3,
                                SlideTypography::Large,
                                true,
                                cx,
                            ))
                            .child(CarouselPrevious::new(&self.horizontal).with_size(size))
                            .child(CarouselNext::new(&self.horizontal).with_size(size))
                            .child(CarouselPagination::new().children((0..3).map(|index| {
                                CarouselPaginationItem::new(
                                    ("carousel-basic-page", index),
                                    index,
                                    &self.horizontal,
                                )
                                .with_size(size)
                                .child((index + 1).to_string())
                            }))),
                    ),
            )
            .child(
                section("carousel-custom-box", "Custom controls")
                    .description(
                        "Replace control content and accessibility labels while retaining navigation behavior.",
                    )
                    .v_flex()
                    .gap_3()
                    .child(
                        Carousel::new("carousel-custom", &self.custom_controls)
                            .w_full()
                            .max_w_96()
                            .mx_auto()
                            .child(Self::items(
                                &self.custom_controls,
                                "carousel-custom-item",
                                3,
                                SlideTypography::Large,
                                true,
                                cx,
                            ))
                            .child(
                                CarouselPrevious::new(&self.custom_controls)
                                    .with_size(size)
                                    .accessibility_label("Previous project")
                                    .child("Back"),
                            )
                            .child(
                                CarouselNext::new(&self.custom_controls)
                                    .with_size(size)
                                    .accessibility_label("Next project")
                                    .child("Forward"),
                            ),
                    ),
            )
            .child(
                section("carousel-multiple-box", "Multiple items")
                    .description(
                        "A fractional flex basis shows several items at once. Pair the content's negative margin with matching item padding to tune the gap between them.",
                    )
                    .v_flex()
                    .gap_3()
                    .child(
                        Carousel::new("carousel-multiple", &self.multiple)
                            .w_full()
                            .max_w_96()
                            .mx_auto()
                            .child(
                                Self::items_with(
                                    &self.multiple,
                                    "carousel-multiple-item",
                                    5,
                                    SlideTypography::Small,
                                    true,
                                    |item| item.flex_basis(relative(1. / 3.)).pl_1(),
                                    cx,
                                )
                                .track_style(StyleRefinement::default().ml_neg_1()),
                            )
                            .child(CarouselPrevious::new(&self.multiple).with_size(size))
                            .child(CarouselNext::new(&self.multiple).with_size(size)),
                    ),
            )
            .child(
                section("carousel-vertical-box", "Vertical")
                    .description("Use Up and Down to navigate a vertical carousel.")
                    .v_flex()
                    .gap_3()
                    .child(
                        Carousel::new("carousel-vertical", &self.vertical)
                            .w_full()
                            .max_w_64()
                            .mx_auto()
                            .child(
                                Self::items_with(
                                    &self.vertical,
                                    "carousel-vertical-item",
                                    3,
                                    SlideTypography::Medium,
                                    false,
                                    |item| item.flex_basis(relative(0.5)),
                                    cx,
                                )
                                .h_48(),
                            )
                            .child(CarouselPrevious::new(&self.vertical).with_size(size))
                            .child(CarouselNext::new(&self.vertical).with_size(size)),
                    ),
            )
            .child(
                section("carousel-looped-box", "Looping")
                    .description("Looping navigation wraps from the last slide to the first.")
                    .v_flex()
                    .gap_3()
                    .child(
                        Carousel::new("carousel-looped", &self.looped)
                            .w_full()
                            .max_w_96()
                            .mx_auto()
                            .child(Self::items(
                                &self.looped,
                                "carousel-looped-item",
                                4,
                                SlideTypography::Large,
                                true,
                                cx,
                            ))
                            .child(CarouselPrevious::new(&self.looped).with_size(size))
                            .child(CarouselNext::new(&self.looped).with_size(size)),
                    ),
            )
            .child(
                section("carousel-controlled-box", "Controlled")
                    .description("The selected index is owned by application state and can be changed programmatically.")
                    .v_flex()
                    .gap_3()
                    .child(
                        Carousel::new("carousel-controlled", &self.controlled)
                            .w_full()
                            .max_w_96()
                            .mx_auto()
                            .child(Self::items(
                                &self.controlled,
                                "carousel-controlled-item",
                                3,
                                SlideTypography::Large,
                                true,
                                cx,
                            ))
                            .child(CarouselPrevious::new(&self.controlled).with_size(size))
                            .child(CarouselNext::new(&self.controlled).with_size(size)),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                div().child(format!(
                                    "Selected slide: {}",
                                    self.controlled_index + 1
                                )),
                            )
                            .child(
                                Button::new("carousel-first").label("Go to first").on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.controlled_index = 0;
                                        let controlled = this.controlled.clone();
                                        controlled.update(cx, |state, cx| {
                                            state.set_selected_index(0, cx);
                                        });
                                        cx.notify();
                                    }),
                                ),
                            )
                            .child(
                                Button::new("carousel-last").label("Go to last").on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.controlled_index = 2;
                                        let controlled = this.controlled.clone();
                                        controlled.update(cx, |state, cx| {
                                            state.set_selected_index(2, cx);
                                        });
                                        cx.notify();
                                    }),
                                ),
                            ),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "carousel",
        "Carousel",
        "A carousel for browsing a set of related items with keyboard and pointer navigation.",
        CarouselSection::view(window, cx),
    ));
}
