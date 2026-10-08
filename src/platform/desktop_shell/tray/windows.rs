use crossbeam_channel::Select;
use gpui_kit::App;
use tray_icon::{
    MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

use super::{LOG, icon};
use crate::capabilities::{self, CapabilityStatus};

#[cfg(test)]
#[path = "windows.test.rs"]
mod windows_test;

// 16×16 is the tray glyph at 100% DPI; tray-icon exposes no dims query, so a
// DPI-aware 24×24 variant stays future work.
const ICON_SIZE: usize = 16;

const SHOW_MENU_ID: &str = "gpui-starter-show";
const SETTINGS_MENU_ID: &str = "gpui-starter-settings";
const QUIT_MENU_ID: &str = "gpui-starter-quit";

/// One row of the tray menu, kept as data so the item set stays testable
/// without building OS menu handles.
enum MenuItemSpec {
    Action { id: &'static str, label: String },
    Separator,
}

fn menu_items() -> Vec<MenuItemSpec> {
    vec![
        MenuItemSpec::Action {
            id: SHOW_MENU_ID,
            label: crate::i18n::localize("tray_show_window"),
        },
        MenuItemSpec::Action {
            id: SETTINGS_MENU_ID,
            label: crate::i18n::localize("tray_open_settings"),
        },
        MenuItemSpec::Separator,
        MenuItemSpec::Action {
            id: QUIT_MENU_ID,
            label: crate::i18n::localize("tray_quit"),
        },
    ]
}

fn build_menu() -> tray_icon::menu::Result<Menu> {
    let menu = Menu::new();
    for spec in menu_items() {
        match spec {
            MenuItemSpec::Action { id, label } => {
                menu.append(&MenuItem::with_id(id, label, true, None))?;
            }
            MenuItemSpec::Separator => menu.append(&PredefinedMenuItem::separator())?,
        }
    }
    Ok(menu)
}

enum TrayAction {
    OpenLauncher,
    ShowWindow,
    OpenSettings,
    Quit,
}

fn action_for_menu_id(id: &str) -> Option<TrayAction> {
    match id {
        SHOW_MENU_ID => Some(TrayAction::ShowWindow),
        SETTINGS_MENU_ID => Some(TrayAction::OpenSettings),
        QUIT_MENU_ID => Some(TrayAction::Quit),
        _ => None,
    }
}

// Left-button release opens the launcher (macOS parity); right click shows
// the menu, raised by tray-icon itself, so no event handling is needed there.
fn action_from_tray_event(event: &TrayIconEvent) -> Option<TrayAction> {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        tracing::info!(target: LOG, source = "tray_click", "Launcher trigger");
        return Some(TrayAction::OpenLauncher);
    }
    None
}

fn action_from_menu_event(event: &MenuEvent) -> Option<TrayAction> {
    let action = action_for_menu_id(event.id().as_ref());
    if action.is_some() {
        tracing::info!(
            target: LOG,
            source = "tray_menu",
            id = %event.id().as_ref(),
            "Tray menu action"
        );
    }
    action
}

fn show_window(cx: &mut App) {
    let Some(handle) = crate::app::window::root_window(cx) else {
        tracing::warn!(target: LOG, "no root window recorded; tray show ignored");
        return;
    };
    if let Err(err) = handle.update(cx, |_, window, _| window.activate_window()) {
        tracing::warn!(target: LOG, error = %err, "failed to activate root window from tray");
    }
}

fn apply_action(action: TrayAction, cx: &mut App) {
    match action {
        TrayAction::OpenLauncher => crate::launcher::open_launcher(cx),
        TrayAction::ShowWindow => show_window(cx),
        TrayAction::OpenSettings => crate::events::emit(
            crate::events::AppEventKind::Navigate(crate::routes::AppRoute::page(
                crate::sidebar::Page::Settings,
            )),
            cx,
        ),
        // Deferred like app/init.rs's Quit path: dispatching mid-update hits
        // the active-window resolution that fails during teardown on Windows.
        TrayAction::Quit => cx.defer(crate::app::window::dispatch_quit),
    }
}

// The drain thread parks on blocking crossbeam recvs and forwards over flume:
// `AsyncApp` is `!Send`, so it cannot `cx.update` (same shape as
// spawn_hotkey_event_forwarder in input/shortcuts.rs).
fn spawn_action_pump(rx: flume::Receiver<TrayAction>, cx: &mut App) {
    cx.spawn(async move |cx| {
        loop {
            // Parked until a tray click or menu pick fires — no periodic
            // wake-ups.
            let Ok(action) = rx.recv_async().await else {
                break;
            };
            let _ = cx.update(|cx| apply_action(action, cx));
        }
    })
    .detach();
}

/// Tray entry point; call once from the bootstrap launch closure on Windows.
pub fn setup(cx: &mut App) {
    tracing::info!(target: LOG, "Setting up tray icon");

    let menu = match build_menu() {
        Ok(menu) => menu,
        Err(err) => {
            tracing::error!(target: LOG, error = %err, "failed to build tray menu");
            capabilities::set("system_tray", CapabilityStatus::error(err.to_string()), cx);
            return;
        }
    };

    // tray-icon creates its hidden window on this (main) thread and its
    // messages ride the app's own pump — no manager worker thread is needed.
    let tray = match TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon(ICON_SIZE))
        .with_tooltip("GPUI Starter")
        .with_menu_on_left_click(false)
        .build()
    {
        Ok(tray) => tray,
        Err(err) => {
            tracing::error!(target: LOG, error = %err, "failed to create system tray icon");
            capabilities::set("system_tray", CapabilityStatus::error(err.to_string()), cx);
            return;
        }
    };
    Box::leak(Box::new(tray));
    tracing::debug!(target: LOG, "Tray icon created");

    let (tx, rx) = flume::unbounded::<TrayAction>();
    let drain = spawn_event_drain(tx);

    match drain {
        Ok(()) => capabilities::set("system_tray", CapabilityStatus::supported_enabled(), cx),
        Err(err) => {
            tracing::error!(
                target: LOG,
                error = %err,
                "failed to spawn gpui-tray-events thread; tray clicks and menu will not be delivered"
            );
            // The icon is live but no event will ever reach the app.
            capabilities::set("system_tray", CapabilityStatus::degraded(err), cx);
        }
    }

    spawn_action_pump(rx, cx);
}

// One drain thread multiplexes tray-icon's two global crossbeam channels with
// Select and forwards actions over flume; a receiver disconnect ends the loop.
fn spawn_event_drain(tx: flume::Sender<TrayAction>) -> Result<(), String> {
    std::thread::Builder::new()
        .name("gpui-tray-events".into())
        // The join handle is dropped like the macOS setup's: this drain parks
        // on the global receivers for the process lifetime.
        .spawn(move || {
            let tray_rx = TrayIconEvent::receiver();
            let menu_rx = MenuEvent::receiver();
            let mut select = Select::new();
            let tray_op = select.recv(tray_rx);
            let menu_op = select.recv(menu_rx);
            loop {
                let oper = select.select();
                let action = if oper.index() == tray_op {
                    match oper.recv(tray_rx) {
                        Ok(event) => action_from_tray_event(&event),
                        Err(_) => return,
                    }
                } else if oper.index() == menu_op {
                    match oper.recv(menu_rx) {
                        Ok(event) => action_from_menu_event(&event),
                        Err(_) => return,
                    }
                } else {
                    // Select only has the two registered operations.
                    unreachable!("unknown select operation index")
                };
                if let Some(action) = action {
                    let _ = tx.send(action);
                }
            }
        })
        .map(|_| ())
        .map_err(|err| format!("failed to spawn gpui-tray-events thread: {err}"))
}
