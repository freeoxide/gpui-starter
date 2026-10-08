use gpui_kit::component::{button::Button, label::Label, switch::Switch};
use gpui_kit::{prelude::*, *};

pub fn render_shortcuts_section(
    app_config: &crate::app_state::AppConfig,
    cx: &mut Context<super::super::SettingsPage>,
) -> impl IntoElement {
    let app_config = app_config.clone();
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
                        .checked(app_config.global_shortcut_enabled)
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
    super::settings_card_base(cx)
        .child(super::section_heading(
            "settings-storage-title",
            crate::i18n::localize("settings_storage"),
        ))
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

pub fn render_developer_section(
    app_config: &crate::app_state::AppConfig,
    cx: &mut Context<super::super::SettingsPage>,
) -> impl IntoElement {
    let app_config = app_config.clone();
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
                        .checked(app_config.show_frame_time)
                        .on_click(|checked, _, cx| {
                            crate::app_state::update_config(cx, |config| {
                                config.show_frame_time = *checked;
                            });
                        }),
                ),
        )
}
