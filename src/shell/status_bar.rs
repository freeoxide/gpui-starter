use gpui::{
    App, Div, InteractiveElement as _, ParentElement as _, Role, Stateful, Styled as _, div,
};
use gpui_component::ActiveTheme as _;

use crate::accessibility::A11yExt;
use crate::{connectivity, notifications, routes::AppRoute, services::updater, session, tasks};

pub fn render(route: &AppRoute, cx: &App) -> impl gpui::IntoElement {
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
        .unwrap_or("No");
    let active_backend = cx
        .try_global::<notifications::NativeNotificationState>()
        .map(|s| s.snapshot.active_backend)
        .unwrap_or(notifications::NotificationBackendKind::UiOnly);
    let session_state = cx
        .try_global::<session::SessionSnapshot>()
        .map(|s| &s.state);
    let latest_error =
        crate::error_surface::latest_message(cx).unwrap_or_else(|| "None".to_string());

    let updater_status = cx
        .try_global::<updater::UpdateSnapshot>()
        .map(|s| &s.status);
    let updater_label = match updater_status {
        Some(updater::UpdateStatus::Available { version, .. }) => {
            Some(format!("Update: {version} available"))
        }
        Some(updater::UpdateStatus::Downloading { progress }) => {
            Some(format!("Update: downloading {progress}%"))
        }
        Some(updater::UpdateStatus::Downloaded { version, .. }) => {
            Some(format!("Update: {version} ready"))
        }
        Some(updater::UpdateStatus::ReadyToInstall) => {
            Some("Update: restart to install".to_string())
        }
        Some(updater::UpdateStatus::Error(err)) => {
            Some(format!("Update: error ({})", truncate_error(err, 30)))
        }
        Some(updater::UpdateStatus::Checking) => Some("Update: checking...".to_string()),
        _ => None,
    };

    let session_label = match session_state {
        Some(session::SessionState::SignedOut) => "SignedOut".to_string(),
        Some(session::SessionState::SigningIn) => "SigningIn".to_string(),
        Some(session::SessionState::SignedIn { account_label }) => {
            format!("SignedIn({account_label})")
        }
        Some(session::SessionState::Error(error)) => format!("Error({error})"),
        None => "Unknown".to_string(),
    };

    // Dev-only frame-time readout.
    let frame_time_el = render_frame_time(cx);

    div()
        .id("status-bar")
        .a11y(Role::Status, "Status")
        .w_full()
        .px_3()
        .py_2()
        .border_t_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().secondary.opacity(0.35))
        .text_xs()
        .child({
            let mut children: Vec<Stateful<Div>> = vec![
                status_row("status-route", format!("Route: {}", route.title())),
                status_row("status-tasks", format!("Tasks: {tasks_active}")),
                status_row("status-unread", format!("Unread: {unread}")),
                status_row(
                    "status-connectivity",
                    format!(
                        "Connectivity: {:?}",
                        connectivity_state.unwrap_or(&connectivity::ConnectivityState::Unknown)
                    ),
                ),
                status_row("status-session", format!("Session: {session_label}")),
                status_row(
                    "status-notifications",
                    format!("Notifications: {active_backend}"),
                ),
                status_row("status-degraded", format!("Degraded: {degraded}")),
                // New errors should be spoken; the rest of the bar is
                // browse-only, so no other row is live.
                status_row("status-last-error", format!("LastError: {latest_error}"))
                    .a11y_live(gpui::accesskit::Live::Polite),
            ];
            if let Some(label) = updater_label {
                children.push(status_row("status-updater", label));
            }
            div()
                .flex()
                .gap_4()
                .items_center()
                .children(frame_time_el)
                .children(children)
        })
}

/// One read-out row. Paragraph, not Label: Label-role nodes draw their AT
/// name from the value property, so these rows would read unnamed.
fn status_row(id: &'static str, text: String) -> Stateful<Div> {
    div().id(id).a11y(Role::Paragraph, text.clone()).child(text)
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

// ---------------------------------------------------------------------------
// Dev-only frame-time readout
// ---------------------------------------------------------------------------

/// Frame-time label; debug builds only, toggled by `show_frame_time`.
#[cfg(debug_assertions)]
fn render_frame_time(cx: &App) -> Option<gpui::Div> {
    if !crate::app_state::with_config(cx, |c| c.show_frame_time) {
        return None;
    }

    let us = crate::root::last_frame_time_us();
    let threshold = crate::root::slow_frame_threshold_us();

    // Format as milliseconds (e.g. "Frame: 2.13ms").
    let ms = us as f64 / 1000.0;
    let label = format!("Frame: {ms:.2}ms");

    // Colour-code: green < 50% threshold, yellow < threshold, red >= threshold.
    let color = if us < threshold / 2 {
        gpui::rgb(0x22c55e) // green
    } else if us < threshold {
        gpui::rgb(0xeab308) // yellow
    } else {
        gpui::rgb(0xef4444) // red
    };

    Some(div().text_color(color).child(label))
}

#[cfg(not(debug_assertions))]
fn render_frame_time(_cx: &App) -> Option<gpui::Div> {
    None
}
