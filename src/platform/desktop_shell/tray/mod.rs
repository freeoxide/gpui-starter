use gpui_kit::App;

use crate::capabilities::{self, Capability, CapabilityStatus};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

// Shared skeleton: capability reporting, the leak, and the flume pump live
// here; each OS module is only the tray/event/action seam.
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(target_os = "windows")]
use windows as platform;

const LOG: &str = "gpui_starter::tray";

/// Tray entry point; call once from the bootstrap launch closure.
pub fn setup(cx: &mut App) {
    tracing::info!(target: LOG, "Setting up tray icon");

    let tray = match platform::build_tray() {
        Ok(tray) => tray,
        Err(err) => {
            tracing::error!(target: LOG, error = %err, "failed to create system tray icon");
            capabilities::set(Capability::SYSTEM_TRAY, CapabilityStatus::error(err), cx);
            return;
        }
    };
    Box::leak(Box::new(tray));
    tracing::debug!(target: LOG, "Tray icon created");

    let (tx, rx) = flume::unbounded::<platform::TrayAction>();
    match platform::spawn_event_drain(tx) {
        Ok(()) => capabilities::set(
            Capability::SYSTEM_TRAY,
            CapabilityStatus::supported_enabled(),
            cx,
        ),
        Err(err) => {
            tracing::error!(
                target: LOG,
                error = %err,
                "failed to spawn gpui-tray-events thread; tray events will not be delivered"
            );
            // The icon is live but no event will ever reach the app.
            capabilities::set(Capability::SYSTEM_TRAY, CapabilityStatus::degraded(err), cx);
        }
    }

    spawn_action_pump(rx, cx);
}

// Parked until a tray action fires — no periodic wake-ups. The drain thread
// cannot cx.update (AsyncApp is !Send), so actions hop flume to this pump.
fn spawn_action_pump(rx: flume::Receiver<platform::TrayAction>, cx: &mut App) {
    cx.spawn(async move |cx| {
        loop {
            let Ok(action) = rx.recv_async().await else {
                break;
            };
            let _ = cx.update(|cx| platform::apply_action(action, cx));
        }
    })
    .detach();
}
