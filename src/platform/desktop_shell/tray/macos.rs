use global_hotkey::GlobalHotKeyEvent;
use gpui::App;
use tray_icon::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

use super::{LOG, icon};

/// Tray + global-hotkey entry point; call once from `main()` on macOS.
pub fn setup(cx: &mut App) {
    tracing::info!(target: LOG, "Setting up tray icon");

    let Ok(tray) = TrayIconBuilder::new()
        .with_icon(icon(36))
        .with_icon_as_template(true)
        .with_tooltip("Open Launcher  (⌥Space)")
        .with_menu_on_left_click(false)
        .build()
    else {
        tracing::error!("failed to create system tray icon");
        return;
    };
    Box::leak(Box::new(tray));
    tracing::debug!(target: LOG, "Tray icon created");

    // Tray/hotkey threads park on blocking crossbeam recvs and forward
    // ticks over flume: `AsyncApp` is `!Send`, so they cannot `cx.update`.
    let (tx, rx) = flume::unbounded::<()>();
    let tx_hotkey = tx.clone();

    if let Err(e) = std::thread::Builder::new()
        .name("gpui-tray-events".into())
        .spawn(move || {
            let tray_rx = TrayIconEvent::receiver();
            loop {
                let Ok(event) = tray_rx.recv() else {
                    return;
                };
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    tracing::info!(target: LOG, source = "tray_click", "Launcher trigger");
                    let _ = tx.send(());
                }
            }
        })
    {
        tracing::error!(
            target: LOG,
            error = %e,
            "failed to spawn gpui-tray-events thread; tray clicks will not be delivered"
        );
    }

    if let Err(e) = std::thread::Builder::new()
        .name("gpui-hotkey-events".into())
        .spawn(move || {
            let hotkey_rx = GlobalHotKeyEvent::receiver();
            loop {
                if hotkey_rx.recv().is_err() {
                    return;
                }
                tracing::info!(target: LOG, source = "hotkey_alt_space", "Launcher trigger");
                let _ = tx_hotkey.send(());
            }
        })
    {
        tracing::error!(
            target: LOG,
            error = %e,
            "failed to spawn gpui-hotkey-events thread; global hotkey will not be delivered"
        );
    }

    cx.spawn(async move |cx| {
        loop {
            // Parked until a tray click or hotkey fires — no periodic wake-ups.
            if rx.recv_async().await.is_err() {
                break;
            }
            let _ = cx.update(crate::launcher::open_launcher);
        }
    })
    .detach();
}
