use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use futures_util::FutureExt;
use gpui_kit::{Global, SharedString};

#[cfg(target_family = "wasm")]
use super::NotificationBackend;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use super::UserNotifyBackend;
#[cfg(target_family = "wasm")]
use super::WasmStubBackend;
// `service::mod` re-exports only the pre-existing backends, so the web
// backend is imported through the backend module's own path.
use super::types::{
    NotificationBackendKind, NotificationCapabilities, NotificationImportance,
    NotificationPermissionState, NotificationRequest, NotificationSendResult,
};
#[cfg(not(target_family = "wasm"))]
use super::{NotificationBackend, NotifyRustBackend};
#[cfg(target_family = "wasm")]
use crate::services::notifications::backend::WebNotificationBackend;

pub const LOG: &str = "gpui_starter::notifications";

const NO_DAEMON_REASON: &str = "No notification daemon owns org.freedesktop.Notifications — \
                                install notify-osd / dunst / mako";

#[derive(Clone, Debug)]
pub struct NotificationRuntimeSnapshot {
    pub enabled_by_user: bool,
    pub permission: NotificationPermissionState,
    pub active_backend: NotificationBackendKind,
    pub capabilities: NotificationCapabilities,
    pub last_backend_error: Option<SharedString>,
    pub degraded_reason: Option<SharedString>,
    /// Advisory: freedesktop capabilities the daemon advertises, probed at
    /// startup; servers may ignore hints they nominally advertise.
    pub daemon_capabilities: Option<SharedString>,
    /// Probe-owned fact feeding `recompute_degraded_reason`; `true` until the
    /// Linux startup probe reports otherwise (macOS/Windows never probe).
    pub daemon_present: bool,
}

impl NotificationRuntimeSnapshot {
    pub fn new(service: &NotificationService, enabled_by_user: bool) -> Self {
        let mut snapshot = Self {
            enabled_by_user,
            permission: NotificationPermissionState::Unknown,
            active_backend: service.active_backend(),
            capabilities: service.active_capabilities(),
            last_backend_error: service.initial_error.clone().map(Into::into),
            degraded_reason: None,
            daemon_capabilities: None,
            daemon_present: true,
        };
        snapshot.recompute_degraded_reason();
        snapshot
    }

    /// Single derivation point so the user toggle, the daemon probe and send
    /// results cannot clobber each other: user-disable outranks a missing
    /// daemon, which outranks the last backend error.
    pub(super) fn recompute_degraded_reason(&mut self) {
        self.degraded_reason = if !self.enabled_by_user {
            Some(crate::i18n::localize("notifications_degraded_disabled_by_user").into())
        } else if !self.daemon_present {
            Some(NO_DAEMON_REASON.into())
        } else {
            self.last_backend_error.clone()
        };
    }
}

#[derive(Clone)]
pub struct NativeNotificationState {
    pub service: Arc<NotificationService>,
    pub snapshot: NotificationRuntimeSnapshot,
}

impl Global for NativeNotificationState {}

pub struct NotificationService {
    pub(crate) primary: Option<Arc<dyn NotificationBackend>>,
    pub(crate) secondary: Arc<dyn NotificationBackend>,
    pub(crate) initial_error: Option<String>,
    /// Shared tokio runtime hosting the send deadline (and any sync backend
    /// work off the main thread); `None` on wasm, whose shim is undriven.
    pub(crate) runtime: Option<Arc<tokio::runtime::Runtime>>,
}

impl NotificationService {
    pub fn new(runtime: Option<Arc<tokio::runtime::Runtime>>) -> Self {
        tracing::info!(target: LOG, "initializing native notification service");
        #[cfg(not(target_family = "wasm"))]
        let secondary = Arc::new(NotifyRustBackend::new()) as Arc<dyn NotificationBackend>;
        // Wasm: no OS notification daemons exist — a failing stub backend
        // keeps the primary/secondary shape and degrades to in-app policy.
        #[cfg(target_family = "wasm")]
        let secondary = Arc::new(WasmStubBackend::new()) as Arc<dyn NotificationBackend>;
        let (primary, primary_error) = select_primary_backend();
        Self::with_backends(primary, secondary, primary_error, runtime)
    }

    /// Construct with explicit backends (primarily a test hook): `primary` is
    /// tried first, `secondary` is the fallback on any primary `Err`, panic or
    /// deadline miss.
    pub(crate) fn with_backends(
        primary: Option<Arc<dyn NotificationBackend>>,
        secondary: Arc<dyn NotificationBackend>,
        initial_error: Option<String>,
        runtime: Option<Arc<tokio::runtime::Runtime>>,
    ) -> Self {
        let service = Self {
            primary,
            secondary,
            initial_error,
            runtime,
        };

        tracing::info!(
            target: LOG,
            active_backend = %service.active_backend(),
            capabilities = ?service.active_capabilities(),
            degraded_reason = ?service.initial_error,
            "native notification service initialized"
        );

        service
    }

    pub(super) fn active_backend(&self) -> NotificationBackendKind {
        self.primary
            .as_ref()
            .map(|backend| backend.kind())
            // On wasm the secondary is the UiOnly stub, not notify-rust.
            .unwrap_or(if cfg!(target_family = "wasm") {
                NotificationBackendKind::UiOnly
            } else {
                NotificationBackendKind::NotifyRust
            })
    }

    pub(super) fn active_capabilities(&self) -> NotificationCapabilities {
        self.primary
            .as_ref()
            .map(|backend| backend.capabilities())
            .unwrap_or_else(|| self.secondary.capabilities())
    }

    pub(super) async fn refresh_permission_state(&self) -> NotificationPermissionState {
        tracing::debug!(
            target: LOG,
            active_backend = %self.active_backend(),
            "refreshing notification permission state"
        );

        if let Some(primary) = &self.primary {
            primary.refresh_permission_state().await
        } else if cfg!(target_os = "macos") {
            let state = NotificationPermissionState::Unavailable(
                self.initial_error
                    .clone()
                    .unwrap_or_else(|| "primary backend unavailable".to_string()),
            );
            tracing::info!(target: LOG, ?state, "permission state unavailable without primary backend");
            state
        } else {
            tracing::info!(target: LOG, "permission state unsupported on this platform");
            NotificationPermissionState::Unsupported
        }
    }

    pub(super) async fn request_permission(&self) -> NotificationPermissionState {
        tracing::info!(
            target: LOG,
            active_backend = %self.active_backend(),
            "requesting notification permission"
        );

        if let Some(primary) = &self.primary {
            primary.request_permission().await
        } else if cfg!(target_os = "macos") {
            let state = NotificationPermissionState::Unavailable(
                self.initial_error
                    .clone()
                    .unwrap_or_else(|| "primary backend unavailable".to_string()),
            );
            tracing::warn!(target: LOG, ?state, "cannot request permission without primary backend");
            state
        } else {
            tracing::info!(target: LOG, "permission request unsupported on this platform");
            NotificationPermissionState::Unsupported
        }
    }

    pub async fn send(
        &self,
        request: NotificationRequest,
        enabled_by_user: bool,
    ) -> NotificationSendResult {
        tracing::info!(
            target: LOG,
            title = %request.title,
            importance = %request.importance,
            prefer_native = request.prefer_native,
            enabled_by_user,
            active_backend = %self.active_backend(),
            "native notification send requested"
        );

        if !enabled_by_user || !request.prefer_native {
            tracing::warn!(
                target: LOG,
                enabled_by_user,
                prefer_native = request.prefer_native,
                importance = %request.importance,
                "native send skipped; using in-app policy"
            );
            return NotificationSendResult {
                backend_used: NotificationBackendKind::UiOnly,
                degraded: request.importance == NotificationImportance::BackgroundWorthy,
                delivered_natively: false,
                error_summary: Some("native notifications are disabled".into()),
                importance: request.importance,
            };
        }

        let mut errors: Vec<String> = Vec::new();

        if let Some(primary) = &self.primary {
            tracing::debug!(target: LOG, backend = %primary.kind(), "attempting primary notification send");
            match try_backend(self.runtime.as_ref(), primary, &request).await {
                Ok(()) => {
                    tracing::info!(target: LOG, backend = %primary.kind(), "primary notification send succeeded");
                    return NotificationSendResult {
                        backend_used: primary.kind(),
                        degraded: false,
                        delivered_natively: true,
                        error_summary: None,
                        importance: request.importance,
                    };
                }
                Err(err) => {
                    tracing::warn!(
                        target: LOG,
                        backend = %primary.kind(),
                        error = %err,
                        "primary notification send failed"
                    );
                    errors.push(err);
                }
            }
        }

        tracing::debug!(
            target: LOG,
            backend = %self.secondary.kind(),
            "attempting fallback notification send"
        );
        match try_backend(self.runtime.as_ref(), &self.secondary, &request).await {
            Ok(()) => {
                tracing::info!(
                    target: LOG,
                    backend = %self.secondary.kind(),
                    degraded = self.primary.is_some(),
                    "fallback notification send succeeded"
                );
                NotificationSendResult {
                    backend_used: self.secondary.kind(),
                    degraded: self.primary.is_some(),
                    delivered_natively: true,
                    error_summary: if errors.is_empty() {
                        None
                    } else {
                        Some(errors.join("; ").into())
                    },
                    importance: request.importance,
                }
            }
            Err(err) => {
                tracing::warn!(
                    target: LOG,
                    backend = %self.secondary.kind(),
                    error = %err,
                    "fallback notification send failed; using in-app policy"
                );
                errors.push(err);
                NotificationSendResult {
                    backend_used: NotificationBackendKind::UiOnly,
                    degraded: true,
                    delivered_natively: false,
                    error_summary: Some(errors.join("; ").into()),
                    importance: request.importance,
                }
            }
        }
    }
}

/// Upper bound for one backend attempt: a daemon that owns the D-Bus name but
/// never replies must not park the send task (or the fallback chain).
const SEND_DEADLINE: std::time::Duration = std::time::Duration::from_secs(15);

/// Run `fut` on the shared runtime under [`SEND_DEADLINE`]; the deadline also
/// moves sync backend work (win_toast, notify-rust `show()`) off the main
/// thread. `None` (no runtime) runs the future inline, unbounded.
pub(super) async fn bounded<T: Send + 'static>(
    runtime: Option<&Arc<tokio::runtime::Runtime>>,
    fut: impl std::future::Future<Output = T> + Send + 'static,
) -> Result<T, String> {
    let Some(runtime) = runtime else {
        return Ok(fut.await);
    };
    match runtime
        .spawn(async move { tokio::time::timeout(SEND_DEADLINE, fut).await })
        .await
    {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(_deadline)) => Err(format!("no reply within {SEND_DEADLINE:?}")),
        Err(join_err) => Err(format!("deadline task failed: {join_err}")),
    }
}

/// One guarded backend attempt: a panic or a daemon that never replies becomes
/// a regular Err so the fallback chain and in-app feedback still run.
async fn try_backend(
    runtime: Option<&Arc<tokio::runtime::Runtime>>,
    backend: &Arc<dyn NotificationBackend>,
    request: &NotificationRequest,
) -> Result<(), String> {
    let owned_backend = Arc::clone(backend);
    let owned_request = request.clone();
    let send = AssertUnwindSafe(async move { owned_backend.send(&owned_request).await });
    match bounded(runtime, send.catch_unwind()).await {
        Ok(Ok(Ok(()))) => Ok(()),
        Ok(Ok(Err(err))) => Err(format!("{}: {err:#}", backend.kind())),
        Ok(Err(panic_payload)) => Err(format!(
            "{}: panic: {}",
            backend.kind(),
            panic_payload_to_string(panic_payload)
        )),
        Err(deadline_err) => Err(format!("{}: {deadline_err}", backend.kind())),
    }
}

/// Select the preferred (primary) notification backend, or `None` to let the
/// synchronous NotifyRustBackend be the active path (Linux native).
fn select_primary_backend() -> (Option<Arc<dyn NotificationBackend>>, Option<String>) {
    // Wasm: the Web Notifications API is the closest daemon equivalent; very
    // old browsers get `None` and every send degrades to the in-app policy.
    #[cfg(target_family = "wasm")]
    {
        match WebNotificationBackend::new() {
            Ok(backend) => {
                tracing::info!(
                    target: LOG,
                    backend = %NotificationBackendKind::Web,
                    "primary notification backend selected"
                );
                (
                    Some(Arc::new(backend) as Arc<dyn NotificationBackend>),
                    None,
                )
            }
            Err(reason) => {
                tracing::warn!(
                    target: LOG,
                    backend = %NotificationBackendKind::Web,
                    error = %reason,
                    "primary notification backend unavailable; falling back"
                );
                (None, Some(reason))
            }
        }
    }

    // macOS / Windows: user-notify gives the richest native experience.
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        match UserNotifyBackend::new() {
            Ok(backend) => {
                tracing::info!(
                    target: LOG,
                    backend = %NotificationBackendKind::UserNotify,
                    "primary notification backend selected"
                );
                (
                    Some(Arc::new(backend) as Arc<dyn NotificationBackend>),
                    None,
                )
            }
            Err(err) => {
                tracing::warn!(
                    target: LOG,
                    backend = %NotificationBackendKind::UserNotify,
                    error = %err,
                    "primary notification backend unavailable; falling back"
                );
                (None, Some(err.to_string()))
            }
        }
    }

    // Sandboxed (Flatpak/Snap) Linux + the opt-in portal feature: route through
    // the XDG Desktop Portal instead of org.freedesktop.Notifications directly.
    #[cfg(all(feature = "notifications-portal", target_os = "linux"))]
    if crate::platform::environment::is_sandboxed() {
        tracing::info!(
            target: LOG,
            backend = %NotificationBackendKind::Portal,
            "sandbox detected; selecting XDG portal backend"
        );
        return (
            Some(Arc::new(super::PortalBackend::new()) as Arc<dyn NotificationBackend>),
            None,
        );
    } else {
        tracing::info!(
            target: LOG,
            "native (non-sandboxed) Linux; notify-rust (FreeDesktop/D-Bus) is the active backend"
        );
    }

    // Linux native (or portal feature off): NotifyRust is the active backend.
    // The cfg keeps the tail from being flagged unreachable on other targets.
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_family = "wasm")))]
    {
        (None, None)
    }
}

fn panic_payload_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{NO_DAEMON_REASON, NotificationRuntimeSnapshot, NotificationService};
    use crate::services::notifications::backend::NotificationBackend;
    #[cfg(target_os = "linux")]
    use crate::services::notifications::backend::NotifyRustBackend;
    use crate::services::notifications::service::types::{
        NotificationBackendKind, NotificationCapabilities, NotificationPermissionState,
        NotificationRequest,
    };

    use std::sync::Arc;

    use async_trait::async_trait;

    enum SendOutcome {
        Ok,
        Err(&'static str),
        Panic(&'static str),
    }

    struct FakeBackend {
        kind: NotificationBackendKind,
        outcome: SendOutcome,
    }

    #[async_trait]
    impl NotificationBackend for FakeBackend {
        fn kind(&self) -> NotificationBackendKind {
            self.kind
        }
        fn capabilities(&self) -> NotificationCapabilities {
            NotificationCapabilities::default()
        }
        async fn refresh_permission_state(&self) -> NotificationPermissionState {
            NotificationPermissionState::Unsupported
        }
        async fn request_permission(&self) -> NotificationPermissionState {
            NotificationPermissionState::Unsupported
        }
        async fn send(&self, _request: &NotificationRequest) -> anyhow::Result<()> {
            match self.outcome {
                SendOutcome::Ok => Ok(()),
                SendOutcome::Err(msg) => Err(anyhow::anyhow!("{msg}")),
                SendOutcome::Panic(msg) => panic!("{msg}"),
            }
        }
    }

    fn fake(kind: NotificationBackendKind, outcome: SendOutcome) -> Arc<dyn NotificationBackend> {
        Arc::new(FakeBackend { kind, outcome })
    }

    fn request() -> NotificationRequest {
        NotificationRequest::foreground("test title", "test body")
    }

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Runtime::new().expect("failed to build tokio runtime for test")
    }

    #[test]
    fn fallback_used_when_primary_returns_err() {
        let svc = NotificationService::with_backends(
            Some(fake(
                NotificationBackendKind::UserNotify,
                SendOutcome::Err("primary boom"),
            )),
            fake(NotificationBackendKind::NotifyRust, SendOutcome::Ok),
            None,
            None,
        );
        let result = runtime().block_on(async { svc.send(request(), true).await });
        assert!(result.delivered_natively, "secondary should deliver");
        assert_eq!(result.backend_used, NotificationBackendKind::NotifyRust);
        assert!(
            result.degraded,
            "a primary existed, so the result is degraded"
        );
        let summary = result
            .error_summary
            .expect("should carry the primary error");
        assert!(
            summary.contains("primary boom"),
            "error summary should include the primary failure: {summary}"
        );
    }

    #[test]
    fn degrades_to_ui_only_when_both_backends_fail() {
        let svc = NotificationService::with_backends(
            Some(fake(
                NotificationBackendKind::UserNotify,
                SendOutcome::Err("primary-fail"),
            )),
            fake(
                NotificationBackendKind::NotifyRust,
                SendOutcome::Err("secondary-fail"),
            ),
            None,
            None,
        );
        let result = runtime().block_on(async { svc.send(request(), true).await });
        assert!(!result.delivered_natively);
        assert_eq!(result.backend_used, NotificationBackendKind::UiOnly);
        assert!(result.degraded);
        let summary = result
            .error_summary
            .expect("both backend errors are recorded");
        assert!(
            summary.contains("primary-fail") && summary.contains("secondary-fail"),
            "error summary should include both failures: {summary}"
        );
    }

    #[test]
    fn primary_panic_does_not_bypass_fallback() {
        let svc = NotificationService::with_backends(
            Some(fake(
                NotificationBackendKind::UserNotify,
                SendOutcome::Panic("primary panicked"),
            )),
            fake(NotificationBackendKind::NotifyRust, SendOutcome::Ok),
            None,
            None,
        );
        let result = runtime().block_on(async { svc.send(request(), true).await });
        assert!(
            result.delivered_natively,
            "secondary must deliver despite the primary panic"
        );
        assert_eq!(result.backend_used, NotificationBackendKind::NotifyRust);
        let summary = result
            .error_summary
            .expect("panic must be recorded as an error");
        assert!(
            summary.to_lowercase().contains("panic"),
            "error summary should mention the panic: {summary}"
        );
    }

    #[test]
    fn native_send_skipped_when_disabled_by_user() {
        let svc = NotificationService::with_backends(
            Some(fake(NotificationBackendKind::UserNotify, SendOutcome::Ok)),
            fake(NotificationBackendKind::NotifyRust, SendOutcome::Ok),
            None,
            None,
        );
        let result = runtime().block_on(async { svc.send(request(), false).await });
        assert!(!result.delivered_natively);
        assert_eq!(result.backend_used, NotificationBackendKind::UiOnly);
    }

    /// On Linux the primary is None, so the secondary is the only backend and
    /// its panic must land in the in-app policy like any other failure.
    #[test]
    fn secondary_panic_degrades_to_ui_only() {
        let svc = NotificationService::with_backends(
            None,
            fake(
                NotificationBackendKind::NotifyRust,
                SendOutcome::Panic("secondary panicked"),
            ),
            None,
            None,
        );
        let result = runtime().block_on(async { svc.send(request(), true).await });
        assert!(
            !result.delivered_natively,
            "a secondary panic must not count as delivery"
        );
        assert_eq!(result.backend_used, NotificationBackendKind::UiOnly);
        assert!(result.degraded);
        let summary = result
            .error_summary
            .expect("the secondary panic must be recorded");
        assert!(
            summary.to_lowercase().contains("panic"),
            "error summary should mention the secondary panic: {summary}"
        );
    }

    #[test]
    fn degraded_reason_precedence_is_user_disable_then_daemon_then_error() {
        let svc = NotificationService::with_backends(
            None,
            fake(NotificationBackendKind::NotifyRust, SendOutcome::Ok),
            None,
            None,
        );
        let mut snapshot = NotificationRuntimeSnapshot::new(&svc, true);
        assert_eq!(snapshot.degraded_reason, None);

        snapshot.last_backend_error = Some("send failed".into());
        snapshot.recompute_degraded_reason();
        assert_eq!(snapshot.degraded_reason.as_deref(), Some("send failed"));

        // The probe's fact outranks a stale send error.
        snapshot.daemon_present = false;
        snapshot.recompute_degraded_reason();
        assert_eq!(
            snapshot.degraded_reason.as_deref(),
            Some(NO_DAEMON_REASON),
            "a missing daemon must outrank the last backend error"
        );

        // The user toggle outranks the probe: a late probe completion must not
        // erase the disabled reason.
        snapshot.enabled_by_user = false;
        snapshot.recompute_degraded_reason();
        assert_eq!(
            snapshot.degraded_reason.as_deref(),
            Some(crate::i18n::localize("notifications_degraded_disabled_by_user").as_str()),
            "user-disable must outrank a missing daemon"
        );
    }

    /// Linux skips the user-notify backend (nothing over notify-rust), so
    /// `new()` must leave `primary = None` and report notify-rust as active.
    #[cfg(target_os = "linux")]
    #[test]
    fn linux_selects_notify_rust_as_active_backend() {
        let svc = NotificationService::new(None);
        assert!(svc.primary.is_none(), "primary must be None on Linux");
        assert_eq!(svc.active_backend(), NotificationBackendKind::NotifyRust);
    }

    /// Linux/FreeDesktop has no per-app notification permission model, so the
    /// notify-rust backend must report `Unsupported` (never `Denied`/`Authorized`).
    #[cfg(target_os = "linux")]
    #[test]
    fn linux_notify_rust_reports_unsupported_permission() {
        let backend = NotifyRustBackend::new();
        let state = runtime().block_on(async { backend.refresh_permission_state().await });
        assert_eq!(state, NotificationPermissionState::Unsupported);
    }

    #[cfg(all(feature = "notifications-portal", target_os = "linux"))]
    #[test]
    fn portal_backend_reports_kind_and_capabilities() {
        use crate::services::notifications::backend::PortalBackend;
        let backend = PortalBackend::new();
        assert_eq!(backend.kind(), NotificationBackendKind::Portal);
        let caps = backend.capabilities();
        assert!(caps.can_send_immediate_native);
        assert!(caps.requires_packaged_runtime);
    }
}
