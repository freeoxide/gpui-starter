use gpui_kit::component::{ActiveTheme as _, button::Button, label::Label, switch::Switch};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;

pub fn render_shortcuts_section(cx: &mut Context<super::super::SettingsPage>) -> impl IntoElement {
    let shortcut_enabled =
        crate::app_state::with_config(cx, |config| config.global_shortcut_enabled);
    let shortcut_label = crate::i18n::localize("settings_global_shortcut");
    super::settings_card_base(cx)
        .child(super::section_heading(
            "settings-shortcuts-title",
            crate::i18n::localize("settings_shortcuts"),
        ))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(Label::new(shortcut_label.clone()))
                .child(
                    Switch::new("global-shortcut-enabled")
                        .accessibility_label(shortcut_label)
                        .checked(shortcut_enabled)
                        .on_click(|checked, _, cx| {
                            crate::app_state::update_config(cx, |config| {
                                config.global_shortcut_enabled = *checked;
                            });
                            crate::shortcuts::apply_enabled(*checked, cx);
                        }),
                ),
        )
}

pub fn render_storage_section(cx: &mut Context<super::super::SettingsPage>) -> impl IntoElement {
    let load_error = crate::app_state::load_error(cx);
    let save_error = crate::app_state::save_error(cx);
    super::settings_card_base(cx)
        .child(super::section_heading(
            "settings-storage-title",
            crate::i18n::localize("settings_storage"),
        ))
        .when_some(load_error, |this, message| {
            this.child(state_error_banner(
                "settings-state-load-error",
                &message,
                cx,
            ))
        })
        .when_some(save_error, |this, message| {
            this.child(state_error_banner(
                "settings-state-save-error",
                &message,
                cx,
            ))
        })
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(
                    Button::new("storage-health-check")
                        .outline()
                        .label(crate::i18n::localize("settings_run_health_check"))
                        .on_click(|_, _, cx| {
                            crate::storage::run_health_check(cx);
                        }),
                )
                .child(
                    Button::new("storage-maintenance")
                        .outline()
                        .label(crate::i18n::localize("settings_run_maintenance"))
                        .on_click(|_, _, cx| {
                            crate::storage::run_maintenance(cx);
                        }),
                ),
        )
}

/// Warning row for a config load or save failure. The message names the
/// state-file path; after a parse failure that file was quarantined as
/// its `.json.bad` sibling.
fn state_error_banner(
    id: &'static str,
    message: &str,
    cx: &Context<super::super::SettingsPage>,
) -> impl IntoElement {
    let theme = cx.theme();
    let title = crate::i18n::localize("settings_state_error");
    div()
        .id(id)
        .a11y(Role::Alert, format!("{title}. {message}"))
        .a11y_live(accesskit::Live::Polite)
        .flex()
        .flex_col()
        .gap_1()
        .p_3()
        .rounded(theme.radius)
        .border_1()
        .border_color(theme.danger)
        .text_color(theme.danger)
        .child(div().text_sm().font_weight(FontWeight::BOLD).child(title))
        .child(div().text_sm().child(message.to_string()))
}

pub fn render_developer_section(cx: &mut Context<super::super::SettingsPage>) -> impl IntoElement {
    let show_frame_time = crate::app_state::with_config(cx, |config| config.show_frame_time);
    let frame_time_label = crate::i18n::localize("settings_show_frame_time");
    super::settings_card_base(cx)
        .child(super::section_heading(
            "settings-developer-title",
            crate::i18n::localize("settings_developer"),
        ))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(Label::new(frame_time_label.clone()))
                .child(
                    Switch::new("show-frame-time")
                        .accessibility_label(frame_time_label)
                        .checked(show_frame_time)
                        .on_click(|checked, _, cx| {
                            crate::app_state::update_config(cx, |config| {
                                config.show_frame_time = *checked;
                            });
                        }),
                ),
        )
}
