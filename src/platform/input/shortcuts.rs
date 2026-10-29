use gpui_kit::{App, BorrowAppContext as _, Global};
use std::rc::Rc;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::sync::{Mutex, OnceLock};

gpui_kit::actions!(shortcuts, [ToggleQuickFloat]);

const LOG: &str = "gpui_starter::shortcuts";

/// Capability/stub reason on every platform without the OS-global backends.
pub const UNSUPPORTED_REASON: &str = "OS-global hotkeys need the macOS or Windows backends; this platform has none (Wayland defines no protocol and the X11 backend is not built)";

/// Registry ids for the accelerators the service routes. OS-global
/// registration is macOS/Windows-only; ids also fire via the fallback chord.
pub const LAUNCHER: &str = "launcher";
pub const QUICK_FLOAT: &str = "quick_float";

// Own modifier/key repr, not global_hotkey's: the registry core must stay
// constructible on Linux and wasm, where that crate is not a dependency.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct HotKeySpec {
    pub alt: bool,
    pub shift: bool,
    pub key: AcceleratorKey,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AcceleratorKey {
    Space,
    D,
}

pub struct Accelerator {
    pub id: &'static str,
    pub label: &'static str,
    pub spec: HotKeySpec,
}

/// Every accelerator the service knows; registration and dispatch both
/// resolve ids (and, per platform, crate hotkey ids) against this table.
pub const ACCELERATORS: &[Accelerator] = &[
    Accelerator {
        id: LAUNCHER,
        label: "Alt+Space",
        spec: HotKeySpec {
            alt: true,
            shift: false,
            key: AcceleratorKey::Space,
        },
    },
    Accelerator {
        id: QUICK_FLOAT,
        label: "Alt+Shift+D",
        spec: HotKeySpec {
            alt: true,
            shift: true,
            key: AcceleratorKey::D,
        },
    },
];

pub fn accelerator(id: &str) -> Option<&'static Accelerator> {
    ACCELERATORS.iter().find(|acc| acc.id == id)
}

fn accelerator_labels() -> String {
    ACCELERATORS
        .iter()
        .map(|acc| acc.label)
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RegistryError {
    UnknownAccelerator,
    DuplicateId,
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::UnknownAccelerator => {
                write!(f, "id is not in the accelerator table")
            }
            RegistryError::DuplicateId => write!(f, "accelerator already has a handler"),
        }
    }
}

pub type Handler = Rc<dyn Fn(&mut App)>;

// All mutations and dispatch run on the app thread: OS events hop flume
// (Windows) or the tray channel (macOS) first, so the table needs no lock.
#[derive(Default)]
pub struct HandlerRegistry {
    entries: Vec<(&'static str, Handler)>,
}

impl HandlerRegistry {
    pub fn register(&mut self, id: &'static str, handler: Handler) -> Result<(), RegistryError> {
        if accelerator(id).is_none() {
            return Err(RegistryError::UnknownAccelerator);
        }
        if self.entries.iter().any(|(known, _)| *known == id) {
            return Err(RegistryError::DuplicateId);
        }
        self.entries.push((id, handler));
        Ok(())
    }

    pub fn unregister(&mut self, id: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|(known, _)| *known != id);
        before != self.entries.len()
    }

    pub fn handler(&self, id: &str) -> Option<&Handler> {
        self.entries
            .iter()
            .find(|(known, _)| *known == id)
            .map(|(_, handler)| handler)
    }
}

#[derive(Default)]
struct DispatchTable(HandlerRegistry);

impl Global for DispatchTable {}

/// Wire the handler an accelerator fires (OS-global event or fallback
/// KeyBinding — both converge here). Fails on unknown or already-routed ids.
pub fn set_handler(
    id: &'static str,
    handler: impl Fn(&mut App) + 'static,
    cx: &mut App,
) -> Result<(), RegistryError> {
    cx.update_default_global::<DispatchTable, _>(|table, _| table.0.register(id, Rc::new(handler)))
}

/// Drop an accelerator's handler; true when one was registered.
pub fn remove_handler(id: &str, cx: &mut App) -> bool {
    cx.update_default_global::<DispatchTable, _>(|table, _| table.0.unregister(id))
}

/// Fire an accelerator's handler; false (with a warn) when none is wired.
/// Runs outside the dispatch-table lease, so a handler may (un)register.
pub fn trigger(id: &str, cx: &mut App) -> bool {
    tracing::info!(target: LOG, accelerator = id, "Accelerator fired");
    let handler =
        cx.update_default_global::<DispatchTable, _>(|table, _| table.0.handler(id).cloned());
    match handler {
        Some(handler) => {
            handler(cx);
            true
        }
        None => {
            tracing::warn!(target: LOG, accelerator = id, "no handler registered for accelerator");
            false
        }
    }
}

/// Map a global-hotkey crate event id to its accelerator and fire it.
/// Called on the app thread from the tray (macOS) and pump (Windows) seams.
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn dispatch_hotkey(hotkey_id: u32, cx: &mut App) {
    let Some(acc) = ACCELERATORS
        .iter()
        .find(|acc| crate_hotkey(acc.spec).id() == hotkey_id)
    else {
        tracing::warn!(target: LOG, hotkey_id, "global hotkey event matched no known accelerator");
        return;
    };
    trigger(acc.id, cx);
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn crate_hotkey(spec: HotKeySpec) -> global_hotkey::hotkey::HotKey {
    use global_hotkey::hotkey::{Code, HotKey, Modifiers};

    let mut mods = Modifiers::empty();
    if spec.alt {
        mods.insert(Modifiers::ALT);
    }
    if spec.shift {
        mods.insert(Modifiers::SHIFT);
    }
    let key = match spec.key {
        AcceleratorKey::Space => Code::Space,
        AcceleratorKey::D => Code::KeyD,
    };
    HotKey::new(Some(mods), key)
}

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
        accelerator: accelerator_labels(),
        last_error: None,
    };
    cx.set_global(state);
    if let Err(err) = set_handler(LAUNCHER, crate::launcher::open_launcher, cx) {
        tracing::warn!(target: LOG, error = %err, "launcher shortcut handler registration failed");
    }
    // macOS hotkey events are forwarded by tray.rs, which is macOS-only.
    #[cfg(target_os = "windows")]
    spawn_hotkey_event_forwarder(cx);
    apply_enabled(enabled, cx);
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn initialize(cx: &mut App) {
    let state = ShortcutState {
        enabled: false,
        registered: false,
        accelerator: accelerator_labels(),
        last_error: Some(UNSUPPORTED_REASON.to_string()),
    };
    set_capability(&state, cx);
    cx.set_global(state);
}

#[cfg(target_os = "macos")]
pub fn apply_enabled(enabled: bool, cx: &mut App) {
    use global_hotkey::GlobalHotKeyManager;

    let mut state = snapshot(cx);
    state.enabled = enabled;
    state.registered = false;
    state.last_error = None;

    // All-or-nothing: a partial failure drops the manager, which unregisters
    // the accelerators that did register.
    let slot = HOTKEY_MANAGER.get_or_init(|| Mutex::new(None));
    if let Ok(mut manager_slot) = slot.lock() {
        *manager_slot = None;
        if enabled {
            match GlobalHotKeyManager::new() {
                Ok(manager) => {
                    let mut failures = None;
                    for acc in ACCELERATORS {
                        if let Err(err) = manager.register(crate_hotkey(acc.spec)) {
                            failures = failures.or_else(|| Some(err.to_string()));
                        }
                    }
                    match failures {
                        None => {
                            *manager_slot = Some(manager);
                            state.registered = true;
                        }
                        Some(err) => state.last_error = Some(err),
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
        Some(UNSUPPORTED_REASON.to_string())
    } else {
        None
    };
    commit(state, cx);
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

    let (tx, rx) = flume::unbounded::<u32>();
    let spawn_result = std::thread::Builder::new()
        .name("gpui-hotkey-events".into())
        .spawn(move || {
            let hotkey_rx = GlobalHotKeyEvent::receiver();
            loop {
                let Ok(event) = hotkey_rx.recv() else {
                    return;
                };
                // The crate emits Pressed and Released per press; forwarding
                // both would fire the accelerator twice per tap.
                if event.state() == HotKeyState::Pressed {
                    let _ = tx.send(event.id());
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
            let Ok(hotkey_id) = rx.recv_async().await else {
                break;
            };
            let _ = cx.update(|cx| dispatch_hotkey(hotkey_id, cx));
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
    use global_hotkey::GlobalHotKeyManager;
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, MSG, TranslateMessage,
    };

    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<u32, String>>();
    let join = std::thread::Builder::new()
        .name("gpui-global-hotkey".into())
        .spawn(move || {
            // Register every accelerator before reporting readiness; a failure
            // drops the manager, unregistering the earlier ones.
            let registered = GlobalHotKeyManager::new().and_then(|manager| {
                let mut failure = None;
                for acc in ACCELERATORS {
                    if let Err(err) = manager.register(crate_hotkey(acc.spec)) {
                        failure = failure.or_else(|| Some(err));
                    }
                }
                match failure {
                    None => Ok(manager),
                    Some(err) => Err(err),
                }
            });
            match registered {
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
                    for acc in ACCELERATORS {
                        let _ = manager.unregister(crate_hotkey(acc.spec));
                    }
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
                        Some(UNSUPPORTED_REASON.into())
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

#[cfg(all(test, not(target_family = "wasm")))]
#[path = "shortcuts.test.rs"]
mod shortcuts_test;
