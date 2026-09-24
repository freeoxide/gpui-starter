use gpui::{App, BorrowAppContext as _, Global};
#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug, Default)]
pub struct ShortcutState {
    pub enabled: bool,
    pub registered: bool,
    pub accelerator: String,
    pub last_error: Option<String>,
}

impl Global for ShortcutState {}

#[cfg(target_os = "macos")]
static HOTKEY_MANAGER: OnceLock<Mutex<Option<global_hotkey::GlobalHotKeyManager>>> =
    OnceLock::new();

#[cfg(target_os = "windows")]
const LOG: &str = "gpui_starter::shortcuts";

// On Windows the manager owns a hidden window created on the calling thread:
// WM_HOTKEY dispatches only while that thread pumps messages and Drop
// (DestroyWindow) must run on that same thread, so the manager's whole
// lifecycle is confined to one worker thread.
#[cfg(target_os = "windows")]
static HOTKEY_WORKER: OnceLock<Mutex<Option<HotkeyWorker>>> = OnceLock::new();

#[cfg(target_os = "windows")]
struct HotkeyWorker {
    thread_id: u32,
    join: std::thread::JoinHandle<()>,
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn initialize(cx: &mut App) {
    let enabled = crate::app_state::with_config(cx, |c| c.global_shortcut_enabled);
    let state = ShortcutState {
        enabled,
        registered: false,
        accelerator: "Alt+Space".to_string(),
        last_error: None,
    };
    cx.set_global(state);
    // macOS hotkey events are forwarded by tray.rs, which is macOS-only.
    #[cfg(target_os = "windows")]
    spawn_hotkey_event_forwarder(cx);
    apply_enabled(enabled, cx);
}

#[cfg(target_os = "macos")]
pub fn apply_enabled(enabled: bool, cx: &mut App) {
    use global_hotkey::{
        GlobalHotKeyManager,
        hotkey::{Code, HotKey, Modifiers},
    };

    let mut state = snapshot(cx);
    state.enabled = enabled;
    state.registered = false;
    state.last_error = None;

    let slot = HOTKEY_MANAGER.get_or_init(|| Mutex::new(None));
    if let Ok(mut manager_slot) = slot.lock() {
        *manager_slot = None;
        if enabled {
            match GlobalHotKeyManager::new() {
                Ok(manager) => {
                    let hotkey = HotKey::new(Some(Modifiers::ALT), Code::Space);
                    match manager.register(hotkey) {
                        Ok(()) => {
                            *manager_slot = Some(manager);
                            state.registered = true;
                        }
                        Err(err) => state.last_error = Some(err.to_string()),
                    }
                }
                Err(err) => state.last_error = Some(err.to_string()),
            }
        }
    } else {
        state.last_error = Some("failed to lock hotkey manager".to_string());
    }

    commit(state, cx);
}

#[cfg(target_os = "windows")]
pub fn apply_enabled(enabled: bool, cx: &mut App) {
    let mut state = snapshot(cx);
    state.enabled = enabled;
    state.registered = false;
    state.last_error = None;

    stop_hotkey_worker();
    if enabled {
        match spawn_hotkey_worker() {
            Ok(()) => state.registered = true,
            Err(err) => state.last_error = Some(err),
        }
    }

    commit(state, cx);
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn apply_enabled(enabled: bool, cx: &mut App) {
    let mut state = snapshot(cx);
    state.enabled = enabled;
    state.registered = false;
    state.last_error = if enabled {
        Some("global shortcut service currently configured for macOS".to_string())
    } else {
        None
    };
    commit(state, cx);
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn initialize(cx: &mut App) {
    let state = ShortcutState {
        enabled: false,
        registered: false,
        accelerator: "Alt+Space".to_string(),
        last_error: Some("global shortcut service currently configured for macOS".to_string()),
    };
    set_capability(&state, cx);
    cx.set_global(state);
}

pub fn snapshot(cx: &App) -> ShortcutState {
    cx.try_global::<ShortcutState>()
        .cloned()
        .unwrap_or_default()
}

#[cfg(target_os = "macos")]
pub fn shutdown(cx: &mut App) {
    let slot = HOTKEY_MANAGER.get_or_init(|| Mutex::new(None));
    if let Ok(mut manager_slot) = slot.lock() {
        *manager_slot = None;
    }
    let mut state = snapshot(cx);
    state.registered = false;
    if state.enabled {
        state.last_error = Some("global shortcuts unregistered during shutdown".to_string());
    }
    commit(state, cx);
}

#[cfg(target_os = "windows")]
pub fn shutdown(cx: &mut App) {
    stop_hotkey_worker();
    let mut state = snapshot(cx);
    state.registered = false;
    if state.enabled {
        state.last_error = Some("global shortcuts unregistered during shutdown".to_string());
    }
    commit(state, cx);
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn shutdown(_cx: &mut App) {}

// The drain thread parks on a blocking crossbeam recv and forwards over flume:
// `AsyncApp` is `!Send`, so it cannot `cx.update` (same shape as tray.rs).
#[cfg(target_os = "windows")]
fn spawn_hotkey_event_forwarder(cx: &mut App) {
    use global_hotkey::{GlobalHotKeyEvent, HotKeyState};

    let (tx, rx) = flume::unbounded::<()>();
    let spawn_result = std::thread::Builder::new()
        .name("gpui-hotkey-events".into())
        .spawn(move || {
            let hotkey_rx = GlobalHotKeyEvent::receiver();
            loop {
                let Ok(event) = hotkey_rx.recv() else {
                    return;
                };
                // The crate emits Pressed and Released per press; forwarding
                // both would trigger the launcher twice per tap.
                if event.state() == HotKeyState::Pressed {
                    tracing::info!(target: LOG, source = "hotkey_alt_space", "Launcher trigger");
                    let _ = tx.send(());
                }
            }
        });
    if let Err(e) = spawn_result {
        tracing::error!(
            target: LOG,
            error = %e,
            "failed to spawn gpui-hotkey-events thread; global hotkey will not be delivered"
        );
        return;
    }

    cx.spawn(async move |cx| {
        loop {
            // Parked until a hotkey fires — no periodic wake-ups.
            if rx.recv_async().await.is_err() {
                break;
            }
            let _ = cx.update(crate::launcher::open_launcher);
        }
    })
    .detach();
}

// WM_QUIT posted as a thread message makes the worker's GetMessageW return 0,
// so it unregisters and drops the manager on its own thread.
#[cfg(target_os = "windows")]
fn stop_hotkey_worker() {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};

    let slot = HOTKEY_WORKER.get_or_init(|| Mutex::new(None));
    let Ok(mut guard) = slot.lock() else {
        return;
    };
    if let Some(worker) = guard.take() {
        if let Err(err) =
            unsafe { PostThreadMessageW(worker.thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) }
        {
            tracing::warn!(target: LOG, error = %err, "failed to post WM_QUIT to hotkey worker");
        }
        let _ = worker.join.join();
    }
}

#[cfg(target_os = "windows")]
fn spawn_hotkey_worker() -> Result<(), String> {
    use global_hotkey::{
        GlobalHotKeyManager,
        hotkey::{Code, HotKey, Modifiers},
    };
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, MSG, TranslateMessage,
    };

    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<u32, String>>();
    let join = std::thread::Builder::new()
        .name("gpui-global-hotkey".into())
        .spawn(move || {
            let hotkey = HotKey::new(Some(Modifiers::ALT), Code::Space);
            let manager = GlobalHotKeyManager::new()
                .and_then(|manager| manager.register(hotkey).map(|_| manager));
            match manager {
                Ok(manager) => {
                    let thread_id = unsafe { GetCurrentThreadId() };
                    let _ = ready_tx.send(Ok(thread_id));
                    let mut msg = MSG::default();
                    // GetMessageW returns 0 on WM_QUIT and -1 on error; both end the pump.
                    while unsafe { GetMessageW(&mut msg, None, 0, 0) }.0 > 0 {
                        unsafe {
                            let _ = TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        }
                    }
                    let _ = manager.unregister(hotkey);
                }
                Err(err) => {
                    let _ = ready_tx.send(Err(err.to_string()));
                }
            }
        })
        .map_err(|err| format!("failed to spawn hotkey worker thread: {err}"))?;

    let worker = match ready_rx.recv() {
        Ok(Ok(thread_id)) => HotkeyWorker { thread_id, join },
        Ok(Err(err)) => {
            let _ = join.join();
            return Err(err);
        }
        Err(_) => {
            let _ = join.join();
            return Err("hotkey worker exited before reporting readiness".to_string());
        }
    };

    let slot = HOTKEY_WORKER.get_or_init(|| Mutex::new(None));
    match slot.lock() {
        Ok(mut guard) => {
            *guard = Some(worker);
            Ok(())
        }
        Err(_) => Err("failed to lock hotkey worker slot".to_string()),
    }
}

fn set_capability(state: &ShortcutState, cx: &mut App) {
    crate::capabilities::set(
        "global_shortcuts",
        crate::capabilities::CapabilityStatus {
            supported: cfg!(any(target_os = "macos", target_os = "windows")),
            enabled: state.registered,
            degraded: state.enabled && !state.registered,
            reason: state
                .last_error
                .as_ref()
                .map(|error| format!("shortcut unavailable: {error}").into())
                .or_else(|| {
                    if cfg!(any(target_os = "macos", target_os = "windows")) {
                        if !state.enabled {
                            Some("disabled by user setting".into())
                        } else {
                            None
                        }
                    } else {
                        Some("global shortcut service currently configured for macOS".into())
                    }
                }),
            last_error: state.last_error.clone().map(Into::into),
        },
        cx,
    );
}

fn commit(state: ShortcutState, cx: &mut App) {
    set_capability(&state, cx);
    cx.update_global::<ShortcutState, _>(|s, _cx| {
        *s = state;
    });
}
