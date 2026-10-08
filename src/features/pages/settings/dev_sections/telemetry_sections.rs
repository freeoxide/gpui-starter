use gpui_kit::component::{ActiveTheme as _, button::Button};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;
use crate::telemetry::{self, TelemetryMode};

pub fn render_telemetry_section(cx: &mut Context<super::super::SettingsPage>) -> impl IntoElement {
    let consent_note = crate::i18n::localize("settings_telemetry_consent_note");
    super::settings_card_base(cx)
        .child(super::section_heading(
            "settings-telemetry-title",
            crate::i18n::localize("settings_telemetry"),
        ))
        .child(
            div()
                .id("settings-telemetry-desc")
                .a11y(Role::Paragraph, consent_note.clone())
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(consent_note),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(
                    Button::new("telemetry-disable")
                        .outline()
                        .label(crate::i18n::localize("settings_telemetry_disable"))
                        .on_click(|_, _, cx| {
                            telemetry::set_mode(TelemetryMode::Disabled, false, None, cx);
                        }),
                )
                .child(
                    Button::new("telemetry-local")
                        .outline()
                        .label(crate::i18n::localize("settings_telemetry_local_only"))
                        .on_click(|_, _, cx| {
                            telemetry::set_mode(TelemetryMode::LocalOnly, true, None, cx);
                        }),
                )
                .child(
                    Button::new("telemetry-remote")
                        .outline()
                        .label(crate::i18n::localize("settings_telemetry_remote"))
                        .on_click(|_, _, cx| {
                            telemetry::set_mode(
                                TelemetryMode::Remote,
                                true,
                                Some("https://telemetry.example.com/v1/events"),
                                cx,
                            );
                        }),
                ),
        )
}

pub fn render_telemetry_runtime_section(
    cx: &mut Context<super::super::SettingsPage>,
) -> impl IntoElement {
    super::settings_card_base(cx)
        .child(super::section_heading(
            "settings-telemetry-runtime-title",
            crate::i18n::localize("settings_telemetry_runtime"),
        ))
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(
                    Button::new("telemetry-record-event")
                        .outline()
                        .label(crate::i18n::localize("settings_record_test_event"))
                        .on_click(|_, _, cx| {
                            telemetry::record_event("settings_test_event", cx);
                        }),
                )
                .child(
                    Button::new("telemetry-record-error")
                        .outline()
                        .label(crate::i18n::localize("settings_record_test_error"))
                        .on_click(|_, _, cx| {
                            telemetry::record_error("settings_test_error", cx);
                        }),
                )
                .child(
                    Button::new("telemetry-set-user-property")
                        .outline()
                        .label(crate::i18n::localize("settings_set_test_user_property"))
                        .on_click(|_, _, cx| {
                            telemetry::set_user_property("plan_phase", "phase21", cx);
                        }),
                )
                .child(
                    Button::new("telemetry-flush")
                        .outline()
                        .label(crate::i18n::localize("settings_flush_telemetry"))
                        .on_click(|_, _, cx| {
                            telemetry::flush(cx);
                        }),
                ),
        )
}
