use gpui_kit::component::ActiveTheme as _;
use gpui_kit::{
    AnyElement, App, Div, InteractiveElement as _, IntoElement, ParentElement as _, Role, Stateful,
    Styled as _, accesskit::Live, div,
};

use crate::accessibility::A11yExt;
use crate::{connectivity, notifications, routes::AppRoute, services::updater, session, tasks};

pub fn render(route: &AppRoute, cx: &App) -> impl IntoElement {
    let tasks_active = tasks::active_count(cx);
    let unread = notifications::inbox::unread_count(cx);

    // Borrow globals directly instead of cloning snapshot structs.
    let connectivity_state = cx
        .try_global::<connectivity::ConnectivitySnapshot>()
        .map(|s| &s.state);
    let degraded = cx
        .try_global::<notifications::NativeNotificationState>()
        .map(|s| s.snapshot.degraded_reason.as_deref())
        .flatten()
        .map(|reason| reason.to_string())
        .unwrap_or_else(|| crate::i18n::localize("status_no"));
    let active_backend = cx
        .try_global::<notifications::NativeNotificationState>()
        .map(|s| s.snapshot.active_backend)
        .unwrap_or(notifications::NotificationBackendKind::UiOnly);
    let session_state = cx
        .try_global::<session::SessionSnapshot>()
        .map(|s| &s.state);
    let latest_error = crate::error_surface::latest_message(cx)
        .unwrap_or_else(|| crate::i18n::localize("status_none"));

    let updater_status = cx
        .try_global::<updater::UpdateSnapshot>()
        .map(|s| &s.status);
    let updater_detail = match updater_status {
        Some(updater::UpdateStatus::Available { version, .. }) => Some(format!(
            "{version} {}",
            crate::i18n::localize("status_update_available")
        )),
        Some(updater::UpdateStatus::Downloading { progress }) => Some(format!(
            "{} {progress}%",
            crate::i18n::localize("status_update_downloading")
        )),
        Some(updater::UpdateStatus::Downloaded { version, .. }) => Some(format!(
            "{version} {}",
            crate::i18n::localize("status_update_ready")
        )),
        Some(updater::UpdateStatus::ReadyToInstall) => {
            Some(crate::i18n::localize("status_update_restart"))
        }
        Some(updater::UpdateStatus::Error(err)) => Some(format!(
            "{} ({})",
            crate::i18n::localize("status_update_error"),
            truncate_error(err, 30)
        )),
        Some(updater::UpdateStatus::Checking) => {
            Some(crate::i18n::localize("status_update_checking"))
        }
        _ => None,
    };

    let session_label = match session_state {
        Some(session::SessionState::SignedOut) => {
            crate::i18n::localize("status_session_signed_out")
        }
        Some(session::SessionState::SigningIn) => {
            crate::i18n::localize("status_session_signing_in")
        }
        Some(session::SessionState::SignedIn { account_label }) => format!(
            "{}({account_label})",
            crate::i18n::localize("status_session_signed_in")
        ),
        Some(session::SessionState::Error(error)) => {
            format!("{}({error})", crate::i18n::localize("status_session_error"))
        }
        None => crate::i18n::localize("status_unknown"),
    };

    let frame_time_el = render_frame_time(cx);

    div()
        .id("status-bar")
        .a11y(Role::Status, crate::i18n::localize("status_a11y"))
        .w_full()
        .px_3()
        .py_2()
        .border_t_1()
        // Same surface roles the kit's StatusBar consumes, so a theme
        // customizing the status bar applies here too.
        .border_color(cx.theme().status_bar_border)
        .bg(cx.theme().tokens.status_bar)
        .text_xs()
        .child({
            let mut children: Vec<Stateful<Div>> = vec![
                status_row(
                    "status-route",
                    format!(
                        "{}: {}",
                        crate::i18n::localize("status_route"),
                        route.localized_title()
                    ),
                ),
                status_row(
                    "status-tasks",
                    format!("{}: {tasks_active}", crate::i18n::localize("status_tasks")),
                ),
                status_row(
                    "status-unread",
                    format!("{}: {unread}", crate::i18n::localize("status_unread")),
                ),
                status_row(
                    "status-connectivity",
                    format!(
                        "{}: {}",
                        crate::i18n::localize("status_connectivity"),
                        connectivity_state
                            .map(connectivity_label)
                            .unwrap_or_else(|| crate::i18n::localize("status_unknown"))
                    ),
                ),
                status_row(
                    "status-session",
                    format!(
                        "{}: {session_label}",
                        crate::i18n::localize("status_session")
                    ),
                ),
                status_row(
                    "status-notifications",
                    format!(
                        "{}: {active_backend}",
                        crate::i18n::localize("status_notifications")
                    ),
                ),
                status_row(
                    "status-degraded",
                    format!("{}: {degraded}", crate::i18n::localize("status_degraded")),
                ),
                // New errors should be spoken; the rest of the bar is
                // browse-only, so no other row is live.
                status_row(
                    "status-last-error",
                    format!(
                        "{}: {latest_error}",
                        crate::i18n::localize("status_last_error")
                    ),
                )
                .a11y_live(Live::Polite),
            ];
            if let Some(detail) = updater_detail {
                children.push(status_row(
                    "status-updater",
                    format!("{}: {detail}", crate::i18n::localize("status_update")),
                ));
            }
            let mut segments: Vec<AnyElement> = Vec::with_capacity(children.len() * 2);
            if let Some(frame_time_el) = frame_time_el {
                segments.push(frame_time_el.into_any_element());
            }
            for child in children {
                if !segments.is_empty() {
                    // Separators keep the read-outs from reading as one
                    // run-on line.
                    segments.push(separator(cx).into_any_element());
                }
                segments.push(child.into_any_element());
            }
            div()
                .flex()
                // Segments wrap as whole units instead of shrinking to
                // fragments; a lone oversized segment still truncates.
                .flex_wrap()
                .min_w_0()
                .overflow_x_hidden()
                .items_center()
                .children(segments)
        })
}

/// One read-out row. Paragraph, not Label: Label-role nodes draw their AT
/// name from the value property, so these rows would read unnamed.
fn status_row(id: &'static str, text: String) -> Stateful<Div> {
    div()
        .id(id)
        .min_w_0()
        .truncate()
        .a11y(Role::Paragraph, text.clone())
        .child(text)
}

/// Decoration only, no a11y node.
fn separator(cx: &App) -> Div {
    div()
        .flex_shrink_0()
        .px_2()
        .text_color(cx.theme().foreground.opacity(0.45))
        .child("·")
}

fn connectivity_label(state: &connectivity::ConnectivityState) -> String {
    match state {
        connectivity::ConnectivityState::Online => {
            crate::i18n::localize("status_connectivity_online")
        }
        connectivity::ConnectivityState::Offline => {
            crate::i18n::localize("status_connectivity_offline")
        }
        connectivity::ConnectivityState::CaptiveOrFiltered => {
            crate::i18n::localize("status_connectivity_captive")
        }
        connectivity::ConnectivityState::Unknown => crate::i18n::localize("status_unknown"),
    }
}

fn truncate_error(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        match s.char_indices().nth(max_len) {
            Some((idx, _)) => &s[..idx],
            None => s,
        }
    }
}

#[cfg(debug_assertions)]
fn render_frame_time(cx: &App) -> Option<Div> {
    if !crate::app_state::with_config(cx, |c| c.show_frame_time) {
        return None;
    }

    let us = crate::root::last_frame_time_us();
    let threshold = crate::root::slow_frame_threshold_us();

    let ms = us as f64 / 1000.0;
    let label = format!("Frame: {ms:.2}ms");

    let color = if us < threshold / 2 {
        cx.theme().success
    } else if us < threshold {
        cx.theme().warning
    } else {
        cx.theme().danger
    };

    Some(div().text_color(color).child(label))
}

#[cfg(not(debug_assertions))]
fn render_frame_time(_cx: &App) -> Option<Div> {
    None
}
