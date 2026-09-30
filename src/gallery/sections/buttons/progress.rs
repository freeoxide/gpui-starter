//! Progress section, ported from the upstream `ProgressStory`.

use gpui_kit::component::{
    ActiveTheme as _, IconName, Sizable as _, Size,
    button::{Button, DropdownButton},
    h_flex,
    progress::{Progress, ProgressCircle},
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};
use serde::Deserialize;
use std::time::Duration;

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, section, size_dropdown};

#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = gallery_progress_section, no_json)]
enum ProgressSectionAction {
    SetValue(f32),
    ToggleLoading,
}

pub struct ProgressSection {
    value: f32,
    loading: bool,
    size: Size,
    _task: Option<Task<()>>,
}

impl ProgressSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            value: 25.,
            loading: false,
            size: Size::Medium,
            _task: None,
        })
    }

    fn start_animation(&mut self, cx: &mut Context<Self>) {
        self.value = 0.;

        self._task = Some(cx.spawn({
            let entity = cx.entity();
            async move |_, cx| {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(15))
                        .await;

                    let mut need_break = false;
                    _ = entity.update(cx, |this, cx| {
                        this.value = (this.value + 2.).min(100.);
                        cx.notify();

                        if this.value >= 100. {
                            this._task = None;
                            need_break = true;
                        }
                    });

                    if need_break {
                        break;
                    }
                }
            }
        }));
    }
}

impl Render for ProgressSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let value = self.value;
        let loading = self.loading;
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
            .on_action(cx.listener(|this, action: &ProgressSectionAction, _, cx| {
                match action {
                    ProgressSectionAction::SetValue(value) => this.value = *value,
                    ProgressSectionAction::ToggleLoading => this.loading = !this.loading,
                }
                cx.notify();
            }))
            .child(demo_toolbar(vec![
                size_dropdown("progress-size", size).into_any_element(),
                DropdownButton::new("progress-value")
                    .button(Button::new("progress-value-trigger").label(format!("Value: {value}%")))
                    .dropdown_menu(move |menu, _, _| {
                        [0., 25., 75., 100.].into_iter().fold(menu, |menu, preset| {
                            menu.menu_with_check(
                                format!("{preset}%"),
                                value == preset,
                                Box::new(ProgressSectionAction::SetValue(preset)),
                            )
                        })
                    })
                    .into_any_element(),
                DropdownButton::new("progress-options")
                    .button(Button::new("progress-options-trigger").label("Options"))
                    .dropdown_menu(move |menu, _, _| {
                        menu.menu_with_check(
                            "Loading",
                            loading,
                            Box::new(ProgressSectionAction::ToggleLoading),
                        )
                    })
                    .into_any_element(),
                Button::new("progress-play")
                    .icon(IconName::Play)
                    .on_click(cx.listener(|this, _, _, cx| this.start_animation(cx)))
                    .into_any_element(),
            ]))
            .child(
                section("progress-upload", "Upload")
                    .description("Pair progress with a clear label, value, and status.")
                    .w(rems(35.))
                    .items_center()
                    .child(
                        v_flex()
                            .w(rems(25.))
                            .gap_3()
                            .p_4()
                            .rounded(cx.theme().radius_lg)
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .child("Uploading design-assets.zip"),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(format!("{}%", value)),
                                    ),
                            )
                            .child(
                                Progress::new("progress-upload-bar")
                                    .value(value)
                                    .loading(loading),
                            )
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_between()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("24.8 MB of 96 MB")
                                    .child(if loading {
                                        "Calculating…"
                                    } else {
                                        "About 1 min left"
                                    }),
                            ),
                    ),
            )
            .child(
                section("progress-circular", "Circular")
                    .description("Use a compact radial indicator for focused tasks.")
                    .w(rems(35.))
                    .items_center()
                    .child(
                        h_flex()
                            .w(rems(25.))
                            .items_center()
                            .gap_5()
                            .p_4()
                            .rounded(cx.theme().radius_lg)
                            .bg(cx.theme().muted.opacity(0.4))
                            .child(
                                ProgressCircle::new("progress-analysis-circle")
                                    .with_size(size)
                                    .value(value)
                                    .loading(loading)
                                    .size_20()
                                    .when(!loading, |this| {
                                        this.child(
                                            div()
                                                .size_full()
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .child(format!("{}%", value)),
                                        )
                                    }),
                            )
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .child("Analyzing project"),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(if loading {
                                                "Preparing analysis…"
                                            } else {
                                                "Scanning components and dependencies."
                                            }),
                                    ),
                            ),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "progress",
        "Progress",
        "Show task completion with determinate or loading indicators.",
        ProgressSection::view(window, cx),
    ));
}
