mod backend;
pub mod inbox;
mod service;

pub use service::{
    ACTION_OPEN, ACTION_REPLY, ACTION_SNOOZE, APP_DISPLAY_NAME, CATEGORY_ACTIONS, CATEGORY_REPLY,
    DESKTOP_ENTRY_ID, NativeNotificationState, NotificationPermissionState, NotificationRequest,
    NotificationRuntimeSnapshot, WM_CLASS, apply_send_result_static, initialize,
    open_system_settings, request_permission_from_window, send_from_window,
    set_native_notifications_enabled, snapshot,
};

pub(crate) use service::{NotificationBackendKind, NotificationCapabilities};
// NotificationImportance is only consumed inside Linux-gated notify-rust code;
// re-exporting it unconditionally leaves it unused on macOS/Windows.
#[cfg(target_os = "linux")]
pub(crate) use service::NotificationImportance;
