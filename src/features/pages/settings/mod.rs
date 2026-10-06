mod dev_sections;
mod notifications_section;

use gpui_kit::component::{
    ActiveTheme as _, Selectable as _, Theme, ThemeMode, button::Button, label::Label,
    switch::Switch, v_flex,
};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;
use crate::app::{self, LOCALE_EN, LOCALE_ZH_CN, LocaleState};
use crate::app_state;
use crate::ids::EventId;
use crate::notifications::{self, NativeNotificationState, NotificationRuntimeSnapshot};

pub struct SettingsPage {
    dark_mode: bool,
    locale: SharedString,
    notifications: NotificationRuntimeSnapshot,
    /// (event id, rendered description); the id is the row's stable identity,
    /// which the description string alone cannot provide.
    event_log: Vec<(EventId, String)>,
    _subscriptions: Vec<Subscription>,
}

impl SettingsPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let _subscriptions = vec![
            cx.observe_global_in::<Theme>(window, |this, _, cx| {
                let dark_mode = cx.theme().mode.is_dark();
                if this.dark_mode != dark_mode {
                    this.dark_mode = dark_mode;
                    cx.notify();
                }
            }),
            cx.observe_global_in::<LocaleState>(window, |this, _, cx| {
                let locale = app::current_locale(cx);
                if this.locale != locale {
                    this.locale = locale;
                    cx.notify();
                }
            }),
            cx.observe_global_in::<NativeNotificationState>(window, |this, _, cx| {
                this.notifications = notifications::snapshot(cx);
                cx.notify();
            }),
            cx.observe_global_in::<crate::events::AppEventQueue>(window, |this, _, cx| {
                // Peek, never drain — AppRoot owns the drain. The observer
                // fires per emit, so the id dedupe drops already-logged rows.
                let mut appended = false;
                if let Some(queue) = cx.try_global::<crate::events::AppEventQueue>() {
                    for event in &queue.0 {
                        if this.event_log.iter().any(|(id, _)| *id == event.id) {
                            continue;
                        }
                        let desc = format!("{:?} ({})", event.kind, event.id);
                        this.event_log.push((event.id, desc));
                        appended = true;
                    }
                }
                let drained = this.event_log.len() > 20;
                if drained {
                    let excess = this.event_log.len() - 20;
                    this.event_log.drain(0..excess);
                }
                // The observer fires per emit; skip the redraw when every
                // offered event was already logged and nothing drained.
                if appended || drained {
                    cx.notify();
                }
            }),
        ];

        Self {
            dark_mode: cx.theme().mode.is_dark(),
            locale: app::current_locale(cx),
            notifications: notifications::snapshot(cx),
            event_log: Vec::new(),
            _subscriptions,
        }
    }
}

impl Render for SettingsPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let locale = self.locale.clone();
        let is_dark = self.dark_mode;
        let notifications_snapshot = self.notifications.clone();
        let app_config = app_state::config(cx);
        let title = crate::i18n::localize("settings_title");
        let dark_mode_label = crate::i18n::localize("settings_dark_mode");
        let language_label = crate::i18n::localize("settings_language");

        v_flex()
            .min_h_full()
            .p_6()
            .gap_6()
            .child(
                div()
                    .id("settings-title")
                    .a11y(Role::Heading, title.clone())
                    .aria_level(1)
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .child(title),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(Label::new(dark_mode_label.clone()))
                    .child(
                        Switch::new("dark-mode")
                            .accessibility_label(dark_mode_label)
                            .checked(is_dark)
                            .on_click(move |checked, _, cx| {
                                let mode = if *checked {
                                    ThemeMode::Dark
                                } else {
                                    ThemeMode::Light
                                };
                                app::set_theme_mode(mode, cx);
                            }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(Label::new(language_label.clone()))
                    .child(
                        div()
                            .id("settings-language-group")
                            .a11y(Role::Group, language_label)
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Button::new("settings-language-en")
                                    .outline()
                                    .selected(locale.as_ref() == LOCALE_EN)
                                    .toggled(locale.as_ref() == LOCALE_EN)
                                    .label(crate::i18n::localize("settings_language_english"))
                                    .on_click(|_, _, cx| {
                                        app::set_locale(LOCALE_EN, cx);
                                    }),
                            )
                            .child(
                                Button::new("settings-language-zh-cn")
                                    .outline()
                                    .selected(locale.as_ref() == LOCALE_ZH_CN)
                                    .toggled(locale.as_ref() == LOCALE_ZH_CN)
                                    .label(crate::i18n::localize(
                                        "settings_language_simplified_chinese",
                                    ))
                                    .on_click(|_, _, cx| {
                                        app::set_locale(LOCALE_ZH_CN, cx);
                                    }),
                            ),
                    ),
            )
            .child(notifications_section::render_notifications_section(
                &notifications_snapshot,
                cx,
            ))
            .child(dev_sections::render_shortcuts_section(&app_config, cx))
            .child(dev_sections::render_storage_section(cx))
            .child(dev_sections::render_developer_section(&app_config, cx))
            .child(dev_sections::render_desktop_actions_section(cx))
            .child(dev_sections::render_telemetry_section(cx))
            .child(dev_sections::render_telemetry_runtime_section(cx))
            .child(dev_sections::render_runtime_boundaries_section(cx))
            .child(dev_sections::render_event_emitter_section(
                &self.event_log,
                cx,
            ))
    }
}
