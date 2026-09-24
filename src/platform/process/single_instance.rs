// Native: single-instance lock + local-socket forwarding between instances.
// Wasm has one instance per tab, so `preflight()` always reports start.

#[cfg(not(target_family = "wasm"))]
pub use native::{
    Preflight, SingleInstanceRuntime, initial_deep_link, install, preflight, shutdown,
};

// Test-only internals (the ipc round-trip tests exercise these directly).
#[cfg(all(test, not(target_family = "wasm")))]
pub(crate) use native::{
    AcquireOutcome, MAX_LINE_BYTES, SCHEME, append_forwarded_link, drain_forwarded_links,
    read_bounded_line, resolve_ipc_name, retry_until_single, send_forwarded_link_via_ipc,
};

// The Windows deep-link registration derives its registry key name from SCHEME.
#[cfg(all(target_os = "windows", not(test)))]
pub(crate) use native::SCHEME;

#[cfg(not(target_family = "wasm"))]
mod native {
    use std::{
        fs,
        io::{BufRead, BufReader, Read, Write},
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };

    #[cfg(target_family = "unix")]
    use std::os::unix::fs::OpenOptionsExt as _;

    use gpui::{App, Global};
    use interprocess::local_socket::{
        GenericFilePath, GenericNamespaced, ListenerOptions, Stream, prelude::*,
    };
    use single_instance::SingleInstance;
    use std::time::Duration;

    use crate::events::{self, AppEventKind};
    use crate::ipc::{
        ForwardedRequest, ForwardedResponse,
        rpc::{decode_request, encode_line},
    };

    const INSTANCE_NAME: &str = "com.gpui-starter.app.instance";
    const LOG: &str = "gpui_starter::single_instance";
    pub(crate) const SCHEME: &str = "gpui-starter://";
    // Deep links and typed commands are tiny; anything longer is hostile or
    // corrupt, so the reader stops at this cap instead of growing unbounded.
    pub(crate) const MAX_LINE_BYTES: usize = 16 * 1024;
    // Backpressure between the blocking listener thread and the UI task: a
    // local flood must hit a bounded queue, not grow memory.
    const FORWARD_QUEUE_CAP: usize = 64;

    pub struct SingleInstanceRuntime {
        _instance: SingleInstance,
        ipc_name: Option<String>,
        queue_file: Option<PathBuf>,
        /// Filesystem path of the forwarder socket, if a namespaced socket is
        /// unavailable. Stored so a [`Drop`] impl can remove a stale socket.
        socket_path: Option<PathBuf>,
        ipc_running: Arc<AtomicBool>,
    }

    impl Drop for SingleInstanceRuntime {
        fn drop(&mut self) {
            // Wake the listener so it observes the flag, then remove the
            // socket file a leftover would break the next startup with.
            self.ipc_running.store(false, Ordering::SeqCst);
            if let Some(ipc_name) = &self.ipc_name {
                let _ = send_forwarded_link_via_ipc(ipc_name, "__shutdown__");
            }
            if let Some(path) = &self.socket_path
                && path.exists()
            {
                let _ = fs::remove_file(path);
                tracing::debug!(target: LOG, path = %path.display(), "removed stale ipc socket on drop");
            }
        }
    }

    impl Global for SingleInstanceRuntime {}

    pub struct Preflight {
        pub should_start: bool,
        pub runtime: Option<SingleInstanceRuntime>,
        pub initial_deep_link: Option<String>,
    }

    /// One lock-acquisition attempt: `Failed` (lock subsystem error) must not
    /// be retried, while `Busy` (another instance holds the lock) may be.
    pub(crate) enum AcquireOutcome<T> {
        Acquired(T),
        Busy,
        Failed,
    }

    // Restart-handoff budget: 50 tries x 100ms while the dying parent's last
    // mutex handle closes; every other launch stays single-shot.
    const RESTART_LOCK_ATTEMPTS: u32 = 50;
    const RESTART_LOCK_INTERVAL: Duration = Duration::from_millis(100);

    pub(crate) fn retry_until_single<T>(
        mut acquire: impl FnMut() -> AcquireOutcome<T>,
        attempts: u32,
        interval: Duration,
    ) -> AcquireOutcome<T> {
        for attempt in 0..attempts {
            match acquire() {
                AcquireOutcome::Acquired(value) => {
                    if attempt > 0 {
                        tracing::info!(
                            target: LOG,
                            attempt,
                            "instance lock acquired after restart retry"
                        );
                    }
                    return AcquireOutcome::Acquired(value);
                }
                AcquireOutcome::Failed => return AcquireOutcome::Failed,
                AcquireOutcome::Busy => {}
            }
            if attempt + 1 < attempts {
                std::thread::sleep(interval);
            }
        }
        tracing::warn!(target: LOG, "instance lock never released by dying restart parent");
        AcquireOutcome::Busy
    }

    /// The launch's deep-link argument, if any: the first argv entry that
    /// starts with the scheme. Protocol launches pass the URL as its own argv
    /// entry after the exe path (quotes are gone once the command line is
    /// parsed), so argv[0] alone must never match.
    pub fn initial_deep_link(args: &[String]) -> Option<String> {
        args.iter().find(|arg| arg.starts_with(SCHEME)).cloned()
    }

    pub fn preflight() -> Preflight {
        let args: Vec<String> = std::env::args().collect();
        let deep_link = initial_deep_link(&args);
        let ipc_name = ipc_name();
        let queue_file = queue_file_path();

        let acquire = || match SingleInstance::new(INSTANCE_NAME) {
            Ok(instance) if instance.is_single() => AcquireOutcome::Acquired(instance),
            Ok(_) => AcquireOutcome::Busy,
            Err(err) => {
                eprintln!("single-instance init failed: {err}");
                AcquireOutcome::Failed
            }
        };
        let outcome = if std::env::var_os(crate::app::reload::RESTART_HANDOFF_ENV).is_some() {
            retry_until_single(acquire, RESTART_LOCK_ATTEMPTS, RESTART_LOCK_INTERVAL)
        } else {
            acquire()
        };

        match outcome {
            // Degraded unlocked start: the lock subsystem itself failed.
            AcquireOutcome::Failed => Preflight {
                should_start: true,
                runtime: None,
                initial_deep_link: deep_link,
            },
            AcquireOutcome::Acquired(instance) => {
                if let Some(path) = &queue_file {
                    if let Some(parent) = path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let _ = fs::remove_file(path);
                }
                // Probe a filesystem socket left by a crashed run and remove it
                // when nothing answers; namespaced sockets have no file to clean.
                let socket_path = filesystem_socket_path(ipc_name.as_deref());
                if let Some(path) = &socket_path
                    && path.exists()
                    && !is_socket_live(ipc_name.as_deref())
                {
                    let _ = fs::remove_file(path);
                    tracing::warn!(
                        target: LOG,
                        path = %path.display(),
                        "removed stale ipc socket before startup"
                    );
                }
                Preflight {
                    should_start: true,
                    runtime: Some(SingleInstanceRuntime {
                        _instance: instance,
                        ipc_name,
                        queue_file,
                        socket_path,
                        ipc_running: Arc::new(AtomicBool::new(true)),
                    }),
                    initial_deep_link: deep_link,
                }
            }
            AcquireOutcome::Busy => {
                if let Some(link) = deep_link {
                    match ipc_name
                        .as_deref()
                        .map(|name| send_forwarded_link_via_ipc(name, &link))
                    {
                        Some(Ok(())) => {}
                        Some(Err(err)) => {
                            tracing::warn!(
                                target: LOG,
                                error = %err,
                                "ipc forward failed; falling back to queue file"
                            );
                            if let Some(queue) = &queue_file {
                                append_forwarded_link(queue, &link);
                            }
                        }
                        None => {
                            tracing::warn!(
                                target: LOG,
                                link = %truncate_for_log(&link),
                                "no safe ipc socket path available; deep link dropped"
                            );
                        }
                    }
                }
                Preflight {
                    should_start: false,
                    runtime: None,
                    initial_deep_link: None,
                }
            }
        }
    }

    pub fn install(runtime: SingleInstanceRuntime, cx: &mut App) {
        crate::capabilities::set(
            "single_instance",
            crate::capabilities::CapabilityStatus::supported_enabled(),
            cx,
        );
        crate::capabilities::set(
            "second_instance_forwarding",
            crate::capabilities::CapabilityStatus::supported_enabled(),
            cx,
        );
        let queue_file = runtime.queue_file.clone();
        let ipc_name = runtime.ipc_name.clone();
        let ipc_running = runtime.ipc_running.clone();
        cx.set_global(runtime);
        if !start_ipc_forwarder(ipc_name, ipc_running, cx) {
            crate::capabilities::set(
                "second_instance_forwarding",
                crate::capabilities::CapabilityStatus {
                    supported: true,
                    enabled: true,
                    degraded: true,
                    reason: Some(
                        "ipc forwarding unavailable; queue-file fallback only when app dirs exist"
                            .into(),
                    ),
                    last_error: Some("failed to initialize local-socket listener".into()),
                },
                cx,
            );
        }
        start_forwarded_link_poller(queue_file, cx);
    }

    pub fn shutdown(cx: &mut App) {
        if let Some(runtime) = cx.try_global::<SingleInstanceRuntime>() {
            runtime.ipc_running.store(false, Ordering::SeqCst);
            if let Some(ipc_name) = &runtime.ipc_name {
                let _ = send_forwarded_link_via_ipc(ipc_name, "__shutdown__");
            }
        }
    }

    fn start_forwarded_link_poller(queue_file: Option<PathBuf>, cx: &mut App) {
        let Some(queue_file) = queue_file else {
            tracing::info!(target: LOG, "no app dirs; queue-file fallback disabled");
            return;
        };
        tracing::info!(target: LOG, queue = %queue_file.display(), "starting deep-link forwarder poller");
        let bg = cx.background_executor().clone();
        cx.spawn(async move |cx| {
            loop {
                bg.timer(Duration::from_millis(450)).await;
                let links = drain_forwarded_links(&queue_file);
                if links.is_empty() {
                    continue;
                }
                cx.update(move |cx| {
                    for link in links {
                        tracing::info!(target: LOG, link, "received forwarded deep-link payload");
                        events::emit(AppEventKind::DeepLinkReceived(link), cx);
                    }
                });
            }
        })
        .detach();
    }

    fn start_ipc_forwarder(
        ipc_name: Option<String>,
        ipc_running: Arc<AtomicBool>,
        cx: &mut App,
    ) -> bool {
        let Some(ipc_name) = ipc_name else {
            tracing::warn!(target: LOG, "no safe socket path; ipc forwarding disabled");
            return false;
        };
        let (tx, rx) = flume::bounded::<ForwardedPayload>(FORWARD_QUEUE_CAP);
        let ipc_name_for_thread = ipc_name.clone();
        let thread = std::thread::Builder::new()
            .name("gpui-ipc-forwarder".to_string())
            .spawn(move || {
                let name = match resolve_ipc_name(&ipc_name_for_thread) {
                    Ok(name) => name,
                    Err(err) => {
                        tracing::error!(
                            target: LOG,
                            error = %err,
                            "failed to resolve ipc listener name"
                        );
                        return;
                    }
                };

                let listener = match ListenerOptions::new().name(name).create_sync() {
                    Ok(listener) => listener,
                    Err(err) => {
                        tracing::error!(target: LOG, error = %err, "failed to create ipc listener");
                        return;
                    }
                };

                tracing::info!(target: LOG, ipc = %ipc_name_for_thread, "starting ipc deep-link listener");

                for conn in listener.incoming() {
                    if !ipc_running.load(Ordering::SeqCst) {
                        break;
                    }
                    let Ok(mut conn) = conn else {
                        continue;
                    };
                    if !peer_is_same_user(&conn) {
                        tracing::warn!(target: LOG, "ipc client failed same-user check; dropped");
                        continue;
                    }
                    let Some(trimmed) = read_bounded_line(&conn) else {
                        continue;
                    };
                    if trimmed.is_empty() || trimmed == "__shutdown__" {
                        continue;
                    }
                    if let Some(req) = decode_request(&trimmed) {
                        let id = req.id;
                        // A full queue means the UI task is wedged: report the
                        // failure instead of pretending the command was taken.
                        let accepted = tx.try_send(ForwardedPayload::Request(req)).is_ok();
                        let resp = if accepted {
                            ForwardedResponse::ok(id)
                        } else {
                            ForwardedResponse::error(id, "primary instance is busy; retry")
                        };
                        if let Ok(encoded) = encode_line(&resp) {
                            let _ = conn.write_all(encoded.as_bytes());
                        }
                    } else if trimmed.starts_with(SCHEME) {
                        // Legacy raw deep-link line: no response channel exists,
                        // so a full queue is logged, not silently dropped.
                        if let Err(flume::TrySendError::Full(ForwardedPayload::DeepLink(link))) =
                            tx.try_send(ForwardedPayload::DeepLink(trimmed))
                        {
                            tracing::warn!(
                                target: LOG,
                                link = %truncate_for_log(&link),
                                "forward queue full; dropped legacy deep link"
                            );
                        }
                    } else {
                        tracing::debug!(
                            target: LOG,
                            line = %trimmed,
                            "ignoring unrecognized ipc line"
                        );
                    }
                }
            });

        if thread.is_err() {
            return false;
        }

        cx.spawn(async move |cx| {
            loop {
                let payload = match rx.recv_async().await {
                    Ok(payload) => payload,
                    // Sender half dropped (runtime shutting down): exit cleanly.
                    Err(_) => break,
                };
                cx.update(move |cx| match payload {
                    ForwardedPayload::Request(req) => {
                        tracing::info!(
                            target: LOG,
                            id = req.id,
                            command = req.command.label(),
                            "received forwarded ipc command"
                        );
                        events::emit(AppEventKind::RemoteCommand(req.command), cx);
                    }
                    ForwardedPayload::DeepLink(link) => {
                        tracing::info!(target: LOG, link, "received forwarded deep-link payload via ipc");
                        events::emit(AppEventKind::DeepLinkReceived(link), cx);
                    }
                });
            }
        })
        .detach();

        true
    }

    /// Payload pushed from the blocking forwarder thread to the gpui task.
    enum ForwardedPayload {
        Request(ForwardedRequest),
        /// Legacy raw deep-link line (starts with the app scheme).
        DeepLink(String),
    }

    /// Drop connections from other users where the OS exposes peer creds;
    /// no cred support stays accepted (UI commands only, user-owned socket).
    #[cfg(target_family = "unix")]
    fn peer_is_same_user(conn: &Stream) -> bool {
        match conn.peer_creds().ok().and_then(|creds| creds.euid()) {
            Some(euid) => euid == unsafe { libc::geteuid() },
            None => true,
        }
    }

    #[cfg(not(target_family = "unix"))]
    fn peer_is_same_user(_conn: &Stream) -> bool {
        true
    }

    /// Read one newline-terminated frame, capped at [`MAX_LINE_BYTES`].
    /// Returns `None` on EOF, read errors, or an unterminated oversized line.
    pub(crate) fn read_bounded_line(conn: &Stream) -> Option<String> {
        let mut reader = BufReader::new(conn);
        let mut line = String::new();
        let mut limited = (&mut reader).take(MAX_LINE_BYTES as u64 + 1);
        match limited.read_line(&mut line) {
            Ok(n) if n > 0 && line.ends_with('\n') => Some(line.trim().to_string()),
            _ => None,
        }
    }

    pub(crate) fn append_forwarded_link(path: &PathBuf, link: &str) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let mut options = fs::OpenOptions::new();
        options.create(true).append(true);
        // Don't create the queue world-readable; content is app-scheme URLs.
        #[cfg(target_family = "unix")]
        options.mode(0o600);

        match options.open(path) {
            Ok(mut file) => {
                let _ = writeln!(file, "{link}");
            }
            Err(err) => {
                eprintln!("failed forwarding deep-link to primary instance: {err}");
            }
        }
    }

    /// Send a single deep-link to the primary instance over a local socket.
    pub(crate) fn send_forwarded_link_via_ipc(
        ipc_name: &str,
        link: &str,
    ) -> Result<(), std::io::Error> {
        let name = resolve_ipc_name(ipc_name)?;
        let mut stream = Stream::connect(name)?;
        writeln!(stream, "{link}")
    }

    pub(crate) fn drain_forwarded_links(path: &PathBuf) -> Vec<String> {
        let Ok(content) = fs::read_to_string(path) else {
            return Vec::new();
        };
        if content.trim().is_empty() {
            return Vec::new();
        }
        // Remove instead of truncating: a symlink planted on the path must not
        // have its target clobbered; the next append recreates the file.
        let _ = fs::remove_file(path);
        content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect()
    }

    /// Queue lives in the user-owned cache dir; no temp-dir fallback, since a
    /// predictable world-writable path invites symlink attacks.
    fn queue_file_path() -> Option<PathBuf> {
        crate::platform::filesystem::paths::project_dirs().map(|dirs| {
            dirs.cache_dir()
                .join("runtime")
                .join("forwarded-deep-links.queue")
        })
    }

    pub(crate) fn resolve_ipc_name<'a>(
        ipc_name: &'a str,
    ) -> std::io::Result<interprocess::local_socket::Name<'a>> {
        if GenericNamespaced::is_supported() {
            ipc_name.to_ns_name::<GenericNamespaced>()
        } else {
            ipc_name.to_fs_name::<GenericFilePath>()
        }
    }

    fn ipc_name() -> Option<String> {
        if GenericNamespaced::is_supported() {
            return Some("com.gpui-starter.app.forwarder".to_string());
        }
        // Prefer the user-owned cache dir; the no-XDG fallback is an
        // app-owned 0700 runtime dir, never a predictable $TMPDIR socket.
        queue_file_path()
            .map(|queue| queue.with_extension("sock"))
            .or_else(|| fallback_runtime_dir().map(|dir| dir.join("forwarder.sock")))
            .map(|path| path.display().to_string())
    }

    /// 0700 uid-scoped dir under the temp root: another user cannot
    /// pre-create it or write into it. `None` disables forwarding.
    fn fallback_runtime_dir() -> Option<PathBuf> {
        #[cfg(target_family = "unix")]
        let stem = format!("gpui-starter-{}", unsafe { libc::geteuid() });
        #[cfg(not(target_family = "unix"))]
        let stem = "gpui-starter-runtime".to_string();
        let dir = std::env::temp_dir().join(stem);

        #[cfg(target_family = "unix")]
        let created = {
            use std::os::unix::fs::DirBuilderExt as _;
            fs::DirBuilder::new().mode(0o700).create(&dir)
        };
        #[cfg(not(target_family = "unix"))]
        let created = fs::DirBuilder::new().create(&dir);

        match created {
            Ok(()) => Some(dir),
            Err(err)
                if err.kind() == std::io::ErrorKind::AlreadyExists
                    && runtime_dir_is_app_owned(&dir) =>
            {
                Some(dir)
            }
            Err(err) => {
                tracing::warn!(
                    target: LOG,
                    path = %dir.display(),
                    error = %err,
                    "unusable or unsafe temp runtime dir; ipc forwarding disabled"
                );
                None
            }
        }
    }

    /// Must be a real dir (not a symlink) owned by this uid with no
    /// group/other write bits, or it could host a squatted socket.
    #[cfg(target_family = "unix")]
    fn runtime_dir_is_app_owned(dir: &std::path::Path) -> bool {
        use std::os::unix::fs::MetadataExt as _;
        match fs::symlink_metadata(dir) {
            Ok(meta) => {
                meta.is_dir()
                    && meta.uid() == unsafe { libc::geteuid() }
                    && meta.mode() & 0o022 == 0
            }
            Err(_) => false,
        }
    }

    #[cfg(not(target_family = "unix"))]
    fn runtime_dir_is_app_owned(_dir: &std::path::Path) -> bool {
        true
    }

    /// Log-friendly prefix of a forwarded payload; URLs can carry long params.
    fn truncate_for_log(payload: &str) -> String {
        payload.chars().take(128).collect()
    }

    /// Filesystem path of the forwarder socket, but only when namespaced
    /// sockets are unavailable (no file to clean up for abstract sockets).
    fn filesystem_socket_path(ipc_name: Option<&str>) -> Option<PathBuf> {
        if GenericNamespaced::is_supported() {
            None
        } else {
            ipc_name.map(PathBuf::from)
        }
    }

    fn is_socket_live(ipc_name: Option<&str>) -> bool {
        match ipc_name.map(resolve_ipc_name).transpose() {
            Ok(Some(name)) => Stream::connect(name).is_ok(),
            _ => false,
        }
    }
}

#[cfg(target_family = "wasm")]
mod wasm {
    use gpui::{App, Global};

    /// Unit runtime: no lock, no IPC — keeps the uniform `install(runtime, cx)`
    /// shape for callers.
    pub struct SingleInstanceRuntime;

    impl Global for SingleInstanceRuntime {}

    pub struct Preflight {
        pub should_start: bool,
        pub runtime: Option<SingleInstanceRuntime>,
        pub initial_deep_link: Option<String>,
    }

    /// Always start: there is no cross-tab single-instance contract.
    pub fn preflight() -> Preflight {
        Preflight {
            should_start: true,
            runtime: Some(SingleInstanceRuntime),
            initial_deep_link: None,
        }
    }

    /// Record the capability as unsupported-and-disabled; nothing to install.
    pub fn install(_runtime: SingleInstanceRuntime, cx: &mut App) {
        let status = crate::capabilities::CapabilityStatus {
            supported: false,
            enabled: false,
            degraded: false,
            reason: Some("single-instance locking unavailable on wasm".into()),
            last_error: None,
        };
        crate::capabilities::set("single_instance", status.clone(), cx);
        crate::capabilities::set("second_instance_forwarding", status, cx);
    }

    /// No-op: nothing was installed.
    pub fn shutdown(_cx: &mut App) {}
}

#[cfg(target_family = "wasm")]
pub use wasm::{Preflight, SingleInstanceRuntime, install, preflight, shutdown};

#[cfg(test)]
#[path = "single_instance.test.rs"]
mod single_instance_test;
