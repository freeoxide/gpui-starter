use async_trait::async_trait;

use super::NotificationBackend;
use crate::notifications::{
    APP_DISPLAY_NAME, NotificationBackendKind, NotificationCapabilities,
    NotificationPermissionState, NotificationRequest,
};
// DESKTOP_ENTRY_ID and APP_ICON_NAME feed the Linux/D-Bus-only icon wiring
// (see `send` below) — importing them unconditionally makes them unused on
// macOS/Windows.
#[cfg(target_os = "linux")]
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
use crate::notifications::{APP_ICON_NAME, DESKTOP_ENTRY_ID};

#[cfg(all(test, target_os = "linux"))]
#[path = "notify_rust.test.rs"]
mod notify_rust_test;

const LOG: &str = "gpui_starter::notifications::notify_rust";

pub struct NotifyRustBackend;

impl NotifyRustBackend {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_os = "linux")]
fn resolve_app_icon() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(probe_app_icon))
        .unwrap_or_else(|| APP_ICON_NAME.to_string())
}

#[cfg(target_os = "linux")]
fn probe_app_icon(exe_dir: &Path) -> String {
    // A name the daemon cannot resolve is a no-op, so the theme name doubles
    // as the fallback for hosts whose hicolor tree carries the icon.
    icon_candidates(exe_dir)
        .into_iter()
        .find(|candidate| candidate.is_file())
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| APP_ICON_NAME.to_string())
}

#[cfg(target_os = "linux")]
fn icon_candidates(exe_dir: &Path) -> Vec<PathBuf> {
    // 48 is the typical banner size; daemons scale the rest.
    const SIZES: [u32; 5] = [48, 64, 128, 256, 512];
    let hicolor_roots = [
        exe_dir
            .parent()
            .map(|parent| parent.join("share/icons/hicolor")),
        Some(exe_dir.join("hicolor")),
    ];
    let mut candidates = Vec::with_capacity(SIZES.len() * 2 + 1);
    for root in hicolor_roots.into_iter().flatten() {
        for size in SIZES {
            candidates.push(root.join(format!("{size}x{size}/apps/{APP_ICON_NAME}.png")));
        }
    }
    candidates.push(exe_dir.join(format!("resources/{APP_ICON_NAME}.png")));
    candidates
}

#[async_trait]
impl NotificationBackend for NotifyRustBackend {
    fn kind(&self) -> NotificationBackendKind {
        NotificationBackendKind::NotifyRust
    }

    fn capabilities(&self) -> NotificationCapabilities {
        NotificationCapabilities {
            can_request_permission: false,
            can_read_permission_state: false,
            can_send_immediate_native: true,
            can_send_interactive: false,
            requires_packaged_runtime: cfg!(target_os = "windows"),
        }
    }

    async fn refresh_permission_state(&self) -> NotificationPermissionState {
        NotificationPermissionState::Unsupported
    }

    async fn request_permission(&self) -> NotificationPermissionState {
        NotificationPermissionState::Unsupported
    }

    async fn send(&self, request: &NotificationRequest) -> anyhow::Result<()> {
        tracing::info!(
            target: LOG,
            title = %request.title,
            importance = %request.importance,
            "sending notification through notify-rust"
        );

        let mut notification = notify_rust::Notification::new();
        notification
            .appname(APP_DISPLAY_NAME)
            .summary(&request.title)
            .body(&request.body);

        // Tells the daemon which .desktop entry we are so it can resolve icon
        // and app name (`.appname` alone is freeform, never looked up).
        #[cfg(target_os = "linux")]
        notification.hint(notify_rust::Hint::DesktopEntry(
            DESKTOP_ENTRY_ID.to_string(),
        ));

        // The hint brands installs only; portable runs (tarball, un-integrated
        // AppImage) need an explicit path or the theme name.
        #[cfg(target_os = "linux")]
        notification.icon(&resolve_app_icon());

        // GNOME suppresses Normal urgency under DND/mute/busy/fullscreen; only
        // Critical pierces those gates (GNOME Shell js/ui/messageTray.js).
        #[cfg(target_os = "linux")]
        notification.urgency(match request.importance {
            crate::notifications::NotificationImportance::BackgroundWorthy => {
                notify_rust::Urgency::Critical
            }
            crate::notifications::NotificationImportance::ForegroundOnly => {
                notify_rust::Urgency::Normal
            }
        });

        if request.play_sound {
            notification.sound_name("default");
        }

        #[cfg(target_os = "linux")]
        {
            // show_async() is driven by zbus's own executor; the sync show()
            // would block_on (async-io) and park the executor polling this trait.
            match notification.show_async().await {
                Ok(_) => {
                    tracing::info!(target: LOG, "notify-rust send succeeded");
                    Ok(())
                }
                Err(err) => {
                    tracing::warn!(target: LOG, error = %err, "notify-rust send failed");
                    Err(err.into())
                }
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            // macOS/Windows: only the sync show() exists (native backend — no
            // zbus block_on, so it doesn't park the executor like on Linux).
            match notification.show() {
                Ok(_) => {
                    tracing::info!(target: LOG, "notify-rust send succeeded");
                    Ok(())
                }
                Err(err) => {
                    tracing::warn!(target: LOG, error = %err, "notify-rust send failed");
                    Err(err.into())
                }
            }
        }
    }
}
