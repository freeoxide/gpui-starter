//! Radio section, ported from the upstream `RadioStory`.

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, Size,
    radio::{Radio, RadioGroup},
    v_flex,
};
use gpui_kit::{
    App, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement, Render,
    Styled, Window, div, rems,
};

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, section, size_dropdown};

pub struct RadioSection {
    delivery: Option<usize>,
    billing: Option<usize>,
    size: Size,
}

impl RadioSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            delivery: Some(0),
            billing: Some(1),
            size: Size::default(),
        })
    }
}

impl Render for RadioSection {
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
            .child(demo_toolbar(vec![
                size_dropdown("radio-size", size).into_any_element(),
            ]))
            .child(
                section("radio-delivery", "Delivery")
                    .description("Choose one option from a clearly described set.")
                    .w(rems(20.))
                    .items_center()
                    .child(
                        RadioGroup::vertical("radio-delivery-group")
                            .w(rems(20.))
                            .gap_3()
                            .child(
                                Radio::new("radio-standard")
                                    .with_size(size)
                                    .w_full()
                                    .label("Standard delivery")
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("Arrives in 3–5 business days."),
                                    ),
                            )
                            .child(
                                Radio::new("radio-express")
                                    .with_size(size)
                                    .w_full()
                                    .label("Express delivery")
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("Arrives the next business day."),
                                    ),
                            )
                            .child(
                                Radio::new("radio-pickup")
                                    .with_size(size)
                                    .w_full()
                                    .label("Store pickup")
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("Unavailable for this order."),
                                    )
                                    .disabled(true),
                            )
                            .selected_index(self.delivery)
                            .on_click(cx.listener(|this, selected: &usize, _, cx| {
                                this.delivery = Some(*selected);
                                cx.notify();
                            })),
                    ),
            )
            .child(
                section("radio-billing", "Billing cycle")
                    .description("Horizontal groups work for short, related choices.")
                    .w(rems(20.))
                    .items_center()
                    .child(
                        RadioGroup::horizontal("radio-billing-group")
                            .w(rems(20.))
                            .justify_between()
                            .child(Radio::new("radio-monthly").with_size(size).label("Monthly"))
                            .child(Radio::new("radio-yearly").with_size(size).label("Yearly"))
                            .child(
                                Radio::new("radio-lifetime")
                                    .with_size(size)
                                    .label("Lifetime"),
                            )
                            .selected_index(self.billing)
                            .on_click(cx.listener(|this, selected_ix: &usize, _, cx| {
                                this.billing = Some(*selected_ix);
                                cx.notify();
                            })),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "radio",
        "Radio",
        "Choose one option from a set.",
        RadioSection::view(window, cx),
    ));
}
