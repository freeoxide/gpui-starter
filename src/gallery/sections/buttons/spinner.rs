//! Spinner section, ported from the upstream `SpinnerStory`.

use gpui_kit::component::{
    ActiveTheme as _, IconName, Sizable as _, Size, spinner::Spinner, v_flex,
};
use gpui_kit::{
    App, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement, Render,
    Styled, Window, bounce, ease_in_out, ease_out_quint, linear, px,
};

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, section, size_dropdown};

pub struct SpinnerSection {
    size: Size,
}

impl SpinnerSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self { size: Size::Medium })
    }
}

impl Render for SpinnerSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let size = self.size;

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
                size_dropdown("spinner-size", size).into_any_element(),
            ]))
            .child(
                section("spinner-default", "Default")
                    .description("An indeterminate loading indicator.")
                    .gap_x_2()
                    .child(Spinner::new().with_size(size)),
            )
            .child(
                section("spinner-color", "Color")
                    .description("Use a color that suits the surrounding status.")
                    .gap_x_2()
                    .child(Spinner::new().with_size(size).color(cx.theme().blue))
                    .child(Spinner::new().with_size(size).color(cx.theme().green)),
            )
            .child(
                section("spinner-custom-size", "Custom size")
                    .description("A fixed pixel size is also supported.")
                    .gap_x_2()
                    .child(Spinner::new().with_size(px(64.))),
            )
            .child(
                section("spinner-icon", "Icon")
                    .description("Replace the default spinner glyph.")
                    .gap_x_2()
                    .child(Spinner::new().with_size(size).icon(IconName::LoaderCircle))
                    .child(
                        Spinner::new()
                            .with_size(size)
                            .icon(IconName::LoaderCircle)
                            .color(cx.theme().cyan),
                    ),
            )
            .child(
                section("spinner-easing", "Easing")
                    .description("Customize the rotation timing curve.")
                    .gap_x_2()
                    .child(
                        Spinner::new()
                            .with_size(size)
                            .icon(IconName::Loader)
                            .ease(linear),
                    )
                    .child(
                        Spinner::new()
                            .with_size(size)
                            .icon(IconName::Loader)
                            .ease(bounce(ease_in_out)),
                    )
                    .child(
                        Spinner::new()
                            .with_size(size)
                            .icon(IconName::Loader)
                            .ease(ease_out_quint()),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "spinner",
        "Spinner",
        "Displays a spinner showing the completion progress of a task.",
        SpinnerSection::view(window, cx),
    ));
}
