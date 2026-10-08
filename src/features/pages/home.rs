use gpui_kit::component::{
    ActiveTheme as _, Selectable as _,
    button::{Button, ButtonVariants as _},
    label::Label,
    switch::Switch,
    v_flex,
};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;

pub struct HomePage;

impl HomePage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HomePage {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for HomePage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let first_run_pending = crate::first_run::is_pending(cx);
        let locale = crate::app::current_locale(cx);
        let notifications_enabled = crate::notifications::snapshot(cx).enabled_by_user;
        let title = crate::i18n::localize("home_title");
        let subtitle = crate::i18n::localize("home_subtitle");

        v_flex()
            .min_h_full()
            .items_center()
            .justify_center()
            .gap_6()
            .child(
                div()
                    .id("home-title")
                    .a11y(Role::Heading, title.clone())
                    .aria_level(1)
                    .text_3xl()
                    .font_weight(FontWeight::BOLD)
                    .child(title),
            )
            .child(
                div()
                    .id("home-subtitle")
                    .a11y(Role::Paragraph, subtitle.clone())
                    .text_color(cx.theme().muted_foreground)
                    .child(subtitle),
            )
            .child(
                Button::new("get-started")
                    .primary()
                    .label(crate::i18n::localize("home_get_started"))
                    .on_click(|_, _, _| {
                        tracing::info!("Get Started clicked");
                    }),
            )
            .child(
                Button::new("start-demo-task")
                    .outline()
                    .label(crate::i18n::localize("command_start_demo_task"))
                    .on_click(cx.listener(|_, _, window, cx| {
                        tracing::info!(
                            target: "gpui_starter::features::pages::home",
                            active_tasks_before = crate::tasks::active_count(cx),
                            "Start Demo Task clicked"
                        );
                        crate::tasks::start_demo_task_in_window(window, cx);
                    })),
            )
            .when(first_run_pending, |this| {
                let first_run_title = crate::i18n::localize("first_run_title");
                let locale_label = crate::i18n::localize("first_run_locale");
                let native_notifications = crate::i18n::localize("first_run_native_notifications");
                this.child(
                    v_flex()
                        .id("first-run-setup")
                        .a11y(Role::Group, first_run_title.clone())
                        .w_full()
                        .max_w(px(520.))
                        .gap_3()
                        .p_4()
                        .rounded_lg()
                        .border_1()
                        .child(
                            div()
                                .id("first-run-title")
                                .a11y(Role::Heading, first_run_title.clone())
                                .aria_level(2)
                                .text_lg()
                                .font_weight(FontWeight::BOLD)
                                .child(first_run_title),
                        )
                        .child(Label::new(crate::i18n::localize("first_run_description")))
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .justify_between()
                                .child(Label::new(locale_label.clone()))
                                .child(
                                    div()
                                        .id("first-run-locale")
                                        .a11y(Role::Group, locale_label)
                                        .flex()
                                        .gap_2()
                                        .child(
                                            Button::new("first-run-locale-en")
                                                .outline()
                                                .selected(locale.as_ref() == crate::app::LOCALE_EN)
                                                .toggled(locale.as_ref() == crate::app::LOCALE_EN)
                                                .label("English")
                                                .on_click(|_, _, cx| {
                                                    crate::app::set_locale(
                                                        crate::app::LOCALE_EN,
                                                        cx,
                                                    );
                                                }),
                                        )
                                        .child(
                                            Button::new("first-run-locale-zh-cn")
                                                .outline()
                                                .selected(
                                                    locale.as_ref() == crate::app::LOCALE_ZH_CN,
                                                )
                                                .toggled(
                                                    locale.as_ref() == crate::app::LOCALE_ZH_CN,
                                                )
                                                .label("简体中文")
                                                .on_click(|_, _, cx| {
                                                    crate::app::set_locale(
                                                        crate::app::LOCALE_ZH_CN,
                                                        cx,
                                                    );
                                                }),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .justify_between()
                                .child(Label::new(native_notifications.clone()))
                                .child(
                                    Switch::new("first-run-notifications")
                                        .accessibility_label(native_notifications)
                                        .checked(notifications_enabled)
                                        .on_click(|checked, _, cx| {
                                            crate::notifications::set_native_notifications_enabled(
                                                *checked, cx,
                                            );
                                        }),
                                ),
                        )
                        .child(
                            div().flex().items_center().gap_2().child(
                                Button::new("first-run-complete")
                                    .primary()
                                    .label(crate::i18n::localize("first_run_finish"))
                                    .on_click(|_, _, cx| {
                                        crate::first_run::complete(cx);
                                    }),
                            ),
                        ),
                )
            })
    }
}
