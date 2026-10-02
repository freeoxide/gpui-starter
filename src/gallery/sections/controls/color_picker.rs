//! Color picker section, ported from the upstream `ColorPickerStory`.

use gpui_kit::component::{
    ActiveTheme as _, Colorize as _, Sizable as _, Size,
    color_picker::{ColorPicker, ColorPickerEvent, ColorPickerState},
    h_flex, indigo_500, v_flex,
};
use gpui_kit::{
    App, AppContext, Context, Entity, FontWeight, Hsla, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Subscription, Window, div, prelude::FluentBuilder as _, rems,
};

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, section, size_dropdown};

pub struct ColorPickerSection {
    color: Entity<ColorPickerState>,
    selected_color: Option<Hsla>,
    size: Size,
    _subscriptions: Vec<Subscription>,
}

impl ColorPickerSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let default_color = indigo_500();
        let color = cx.new(|cx| ColorPickerState::new(window, cx).default_value(default_color));

        let _subscriptions = vec![cx.subscribe(&color, |this, _, ev, cx| match ev {
            ColorPickerEvent::Change(color) => {
                this.selected_color = *color;
                // The swatch preview reads this field, so the owner must repaint.
                cx.notify();
            }
        })];

        Self {
            color,
            selected_color: Some(default_color),
            size: Size::default(),
            _subscriptions,
        }
    }
}

impl Render for ColorPickerSection {
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
                size_dropdown("color-picker-size", size).into_any_element(),
            ]))
            .child(
                section("color-picker-theme-color", "Theme Color")
                    .description("Select a color and preview the resulting value.")
                    .w(rems(27.5))
                    .child(
                        v_flex()
                            .w_full()
                            .gap_4()
                            .p_4()
                            .rounded(cx.theme().radius_lg)
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_4()
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .child("Accent color"),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(
                                                        "Used for primary actions and highlights.",
                                                    ),
                                            ),
                                    )
                                    .child(ColorPicker::new(&self.color).with_size(size)),
                            )
                            .when_some(self.selected_color, |this, color| {
                                this.child(
                                    v_flex()
                                        .w_full()
                                        .overflow_hidden()
                                        .rounded(cx.theme().radius_lg)
                                        .border_1()
                                        .border_color(cx.theme().border)
                                        .child(
                                            div()
                                                .w_full()
                                                .rounded_t(cx.theme().radius_lg)
                                                .h(rems(6.))
                                                .bg(color),
                                        )
                                        .child(
                                            h_flex()
                                                .w_full()
                                                .items_center()
                                                .justify_between()
                                                .px_3()
                                                .py_2()
                                                .bg(cx.theme().muted)
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .text_color(cx.theme().muted_foreground)
                                                        .child("Selected color"),
                                                )
                                                .child(
                                                    div()
                                                        .font_family("monospace")
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .child(color.to_hex()),
                                                ),
                                        ),
                                )
                            }),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "color-picker",
        "Color Picker",
        "Choose and preview a color value.",
        ColorPickerSection::view(window, cx),
    ));
}
