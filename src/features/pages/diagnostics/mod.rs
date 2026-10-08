mod rows;

use gpui_kit::component::{button::Button, v_flex};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;
use crate::{
    accessibility, app_state, capabilities, crash_report, desktop_actions, error_surface,
    lifecycle::LifecycleState, notifications, shortcuts, storage, telemetry, undo_stack,
};

pub struct DiagnosticsPage {
    _subscriptions: Vec<Subscription>,
}

impl DiagnosticsPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut subscriptions = Vec::new();
        subscriptions.push(
            cx.observe_global_in::<app_state::AppState>(window, |_, _, cx| {
                cx.notify();
            }),
        );
        subscriptions.push(cx.observe_global_in::<LifecycleState>(window, |_, _, cx| {
            cx.notify();
        }));
        subscriptions.push(
            cx.observe_global_in::<notifications::NativeNotificationState>(window, |_, _, cx| {
                cx.notify();
            }),
        );
        subscriptions.push(cx.observe_global_in::<capabilities::CapabilityRegistry>(
            window,
            |_, _, cx| {
                cx.notify();
            },
        ));
        subscriptions.push(
            cx.observe_global_in::<storage::StorageSnapshot>(window, |_, _, cx| {
                cx.notify();
            }),
        );
        subscriptions.push(cx.observe_global_in::<telemetry::TelemetrySnapshot>(
            window,
            |_, _, cx| {
                cx.notify();
            },
        ));
        subscriptions.push(
            cx.observe_global_in::<desktop_actions::DesktopActionsState>(window, |_, _, cx| {
                cx.notify();
            }),
        );
        subscriptions.push(
            cx.observe_global_in::<shortcuts::ShortcutState>(window, |_, _, cx| {
                cx.notify();
            }),
        );
        subscriptions.push(
            cx.observe_global_in::<undo_stack::UndoState>(window, |_, _, cx| {
                cx.notify();
            }),
        );
        subscriptions.push(
            cx.observe_global_in::<accessibility::AccessibilitySnapshot>(window, |_, _, cx| {
                cx.notify();
            }),
        );
        subscriptions.push(cx.observe_global_in::<error_surface::ErrorSurfaceState>(
            window,
            |_, _, cx| {
                cx.notify();
            },
        ));
        subscriptions.push(cx.observe_global_in::<crash_report::CrashReportSnapshot>(
            window,
            |_, _, cx| {
                cx.notify();
            },
        ));
        Self {
            _subscriptions: subscriptions,
        }
    }
}

impl Render for DiagnosticsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = rows::build_diagnostic_rows(cx);
        let title = crate::i18n::localize("diagnostics_title");

        v_flex()
            .min_h_full()
            .p_6()
            .gap_3()
            .child(
                div()
                    .id("diagnostics-title")
                    .a11y(Role::Heading, title.clone())
                    .aria_level(1)
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .child(title),
            )
            .child(
                Button::new("diagnostics-refresh")
                    .outline()
                    .label(crate::i18n::localize("diagnostics_refresh"))
                    .on_click(|_, _, cx| {
                        crate::events::emit(crate::events::AppEventKind::DiagnosticsChanged, cx);
                    }),
            )
            .child(
                Button::new("diagnostics-reset-first-run")
                    .outline()
                    .label(crate::i18n::localize("diagnostics_reset_first_run"))
                    .on_click(|_, _, cx| {
                        crate::first_run::reset(cx);
                    }),
            )
            .child(
                Button::new("diagnostics-copy")
                    .outline()
                    .label(crate::i18n::localize("diagnostics_copy"))
                    .on_click(|_, _, cx| {
                        let _ = crate::desktop_actions::copy_diagnostics(cx);
                    }),
            )
            .child(
                Button::new("diagnostics-open-logs")
                    .outline()
                    .label(crate::i18n::localize("diagnostics_open_logs"))
                    .on_click(|_, _, cx| {
                        let _ = crate::desktop_actions::open_logs_folder(cx);
                    }),
            )
            .child(
                Button::new("diagnostics-dismiss-latest-error")
                    .outline()
                    .label(crate::i18n::localize("diagnostics_dismiss_latest_error"))
                    .on_click(|_, _, cx| {
                        if let Some(error) = crate::error_surface::latest(cx) {
                            crate::error_surface::dismiss(error.id, cx);
                        }
                    }),
            )
            .child(
                Button::new("diagnostics-retry-crash-upload")
                    .outline()
                    .label(crate::i18n::localize("diagnostics_retry_crash_upload"))
                    .on_click(|_, _, cx| {
                        crate::crash_report::upload_pending_reports(cx);
                    }),
            )
            .when(cfg!(debug_assertions), |this| {
                this.child(
                    Button::new("diagnostics-trigger-test-panic")
                        .outline()
                        .label(crate::i18n::localize("diagnostics_trigger_test_panic"))
                        .on_click(|_, window, cx| {
                            // Window-scoped: App::dispatch_action is broken on
                            // Windows (thread-local active-window lookup).
                            window.dispatch_action(Box::new(crate::app::TriggerTestPanic), cx);
                        }),
                )
            })
            .child(
                div()
                    .id("diagnostics-rows")
                    .a11y(Role::List, crate::i18n::localize("diagnostics_rows"))
                    .aria_orientation(Orientation::Vertical)
                    .children(rows),
            )
    }
}

fn row(label_key: &str, value: &str) -> Stateful<Div> {
    row_labeled(label_key, &crate::i18n::localize(label_key), value)
}

fn capability_row(name: &str, value: &str) -> Stateful<Div> {
    let label = format!("{}:{name}", crate::i18n::localize("diagnostics_capability"));
    row_labeled(&format!("Capability:{name}"), &label, value)
}

fn row_labeled(identity: &str, label: &str, value: &str) -> Stateful<Div> {
    let text = format!("{label}: {value}");
    div()
        // The identity (catalog key or `Capability:{name}`) stays English: it
        // is the row's stable domain id, unlike the localized label.
        .id(ElementId::Name(SharedString::from(format!(
            "diag-{identity}"
        ))))
        .a11y(Role::ListItem, text)
        .child(
            div()
                .flex()
                .gap_2()
                .child(
                    div()
                        .font_weight(FontWeight::BOLD)
                        .child(format!("{label}:")),
                )
                .child(div().child(value.to_string())),
        )
}
