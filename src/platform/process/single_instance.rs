// Native: single-instance lock + local-socket forwarding between instances.
// Wasm has one instance per tab, so `preflight()` always reports start.

#[cfg(not(target_family = "wasm"))]
pub use native::{
    Preflight, SingleInstanceRuntime, initial_deep_link, install, preflight, shutdown,
};

// Test-only internals (the ipc round-trip tests exercise these directly).
#[cfg(all(test, not(target_family = "wasm")))]
pub(crate) use native::{
    AcquireOutcome, ForwardOutcome, MAX_LINE_BYTES, RUNTIME_DIR_ENV, acquire_instance_lock,
    append_forwarded_link, create_forwarder_listener, drain_forwarded_links, forward_deep_link,
    link_path_for_log, lock_file_path, queue_file_path, read_bounded_line, request_raise,
    resolve_ipc_name, retry_until_single, send_forwarded_link_via_ipc,
};

#[cfg(not(target_family = "wasm"))]
mod native {
    use std::{
        fs,
        io::{BufRead, BufReader, Read, Write},
        path::{Path, PathBuf},
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicU64, Ordering},
        },
    };

    #[cfg(target_family = "unix")]
    use std::os::unix::fs::OpenOptionsExt as _;

    use gpui_kit::{App, Global};
    use interprocess::local_socket::{
        GenericFilePath, GenericNamespaced, Listener, ListenerOptions, Stream, prelude::*,
    };
    use single_instance::SingleInstance;
    use std::time::Duration;

    use crate::events::{self, AppEventKind};
    use crate::ipc::{
        ForwardedCommand, ForwardedRequest, ForwardedResponse,
        rpc::{decode_line, decode_request, encode_line},
    };

    const INSTANCE_NAME: &str = "com.gpui-starter.app.instance";
    const LOG: &str = "gpui_starter::single_instance";
    // Deep links and typed commands are tiny; anything longer is hostile or
    // corrupt, so the reader stops at this cap instead of growing unbounded.
    pub(crate) const MAX_LINE_BYTES: usize = 16 * 1024;
    // Backpressure between the blocking listener thread and the UI task: a
    // local flood must hit a bounded queue, not grow memory.
    const FORWARD_QUEUE_CAP: usize = 64;
    // Past this deadline a peer is stalled and must not wedge the single
    // forwarder thread; Windows named pipes cannot set it and stay blocking.
    const IPC_READ_TIMEOUT: Duration = Duration::from_secs(3);
    /// Tests (tests/e2e_lifecycle.rs) redirect every single-instance artifact
    /// at a throwaway dir through this env var; unset in production.
    pub(crate) const RUNTIME_DIR_ENV: &str = "GPUI_STARTER_SINGLE_INSTANCE_DIR";
    static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

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

    /// Deep-link argument prefix, built from the one scheme constant in
    /// `shell::route` so registration and routing cannot drift apart.
    pub(crate) fn scheme_prefix() -> String {
        format!("{}://", crate::shell::route::APP_URL_SCHEME)
    }

    /// The launch's deep-link argument, if any: the first argv entry that
    /// starts with the scheme. Protocol launches pass the URL as its own argv
    /// entry after the exe path (quotes are gone once the command line is
    /// parsed), so argv[0] alone must never match.
    pub fn initial_deep_link(args: &[String]) -> Option<String> {
        let scheme = scheme_prefix();
        args.iter().find(|arg| arg.starts_with(&scheme)).cloned()
    }

    /// One lock-acquisition attempt. Every platform primitive (flock file,
    /// abstract socket, named mutex) is kernel-reclaimed on holder death, so
    /// a leftover lock file is inert and no pidfile check applies.
    pub(crate) fn acquire_instance_lock(lock_name: &str) -> AcquireOutcome<SingleInstance> {
        match SingleInstance::new(lock_name) {
            Ok(instance) if instance.is_single() => AcquireOutcome::Acquired(instance),
            Ok(_) => AcquireOutcome::Busy,
            Err(err) => {
                eprintln!("single-instance init failed: {err}");
                AcquireOutcome::Failed
            }
        }
    }

    /// Fixed per-user home of the lock file, beside the forwarder artifacts.
    pub(crate) fn lock_file_path() -> Option<PathBuf> {
        queue_file_path()
            .map(|queue| queue.with_file_name(INSTANCE_NAME))
            .or_else(|| fallback_runtime_dir().map(|dir| dir.join(INSTANCE_NAME)))
    }

    fn lock_name() -> Option<String> {
        // A flock name is a file path, so it must be absolute and per-user or
        // launches from two CWDs both win; the test seam isolates it the same way.
        if cfg!(target_os = "macos") || overridden_runtime_dir().is_some() {
            lock_file_path().map(|path| {
                if let Some(parent) = path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                path.display().to_string()
            })
        } else {
            Some(INSTANCE_NAME.to_string())
        }
    }

    pub fn preflight() -> Preflight {
        // args() panics on any non-UTF-8 argv entry; skip those instead.
        let args: Vec<String> = std::env::args_os()
            .filter_map(|arg| arg.into_string().ok())
            .collect();
        let deep_link = initial_deep_link(&args);
        let ipc_name = ipc_name();
        let queue_file = queue_file_path();

        let acquire = || match lock_name() {
            Some(name) => acquire_instance_lock(&name),
            // No per-user location at all: same degrade as a broken lock
            // subsystem; a cache-less machine must not lose the app.
            None => AcquireOutcome::Failed,
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
                // Pre-fix macOS builds left a CWD-relative lock file behind.
                #[cfg(target_os = "macos")]
                let _ = fs::remove_file(INSTANCE_NAME);
                if let Some(path) = &queue_file
                    && let Some(parent) = path.parent()
                {
                    let _ = fs::create_dir_all(parent);
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
                if let Some(link) = deep_link.as_deref() {
                    forward_deep_link(ipc_name.as_deref(), queue_file.as_deref(), link);
                }
                // A relaunch of a running app raises its window, like tray Show.
                if let Some(name) = ipc_name.as_deref() {
                    request_raise(name);
                }
                Preflight {
                    should_start: false,
                    runtime: None,
                    initial_deep_link: None,
                }
            }
        }
    }

    /// How a Busy launch's deep link left this process.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum ForwardOutcome {
        /// Delivered to the primary instance over its socket.
        Sent,
        /// Socket send failed; appended to the queue file the primary drains.
        Queued,
        /// Delivered nowhere (no socket path, or no queue to fall back to).
        Dropped,
    }

    /// Forward a second instance's deep link to the primary instance.
    pub(crate) fn forward_deep_link(
        ipc_name: Option<&str>,
        queue: Option<&Path>,
        link: &str,
    ) -> ForwardOutcome {
        match ipc_name.map(|name| send_forwarded_link_via_ipc(name, link)) {
            Some(Ok(())) => ForwardOutcome::Sent,
            Some(Err(err)) => {
                tracing::warn!(
                    target: LOG,
                    error = %err,
                    "ipc forward failed; falling back to queue file"
                );
                match queue {
                    Some(queue) => {
                        append_forwarded_link(queue, link);
                        ForwardOutcome::Queued
                    }
                    None => ForwardOutcome::Dropped,
                }
            }
            None => {
                tracing::warn!(
                    target: LOG,
                    link = %truncate_for_log(link),
                    "no safe ipc socket path available; deep link dropped"
                );
                ForwardOutcome::Dropped
            }
        }
    }

    pub fn install(runtime: SingleInstanceRuntime, cx: &mut App) {
        crate::capabilities::set(
            crate::capabilities::Capability::SINGLE_INSTANCE,
            crate::capabilities::CapabilityStatus::supported_enabled(),
            cx,
        );
        crate::capabilities::set(
            crate::capabilities::Capability::SECOND_INSTANCE_FORWARDING,
            crate::capabilities::CapabilityStatus::supported_enabled(),
            cx,
        );
        let queue_file = runtime.queue_file.clone();
        let ipc_name = runtime.ipc_name.clone();
        let ipc_running = runtime.ipc_running.clone();
        cx.set_global(runtime);
        if !start_ipc_forwarder(ipc_name, ipc_running, cx) {
            crate::capabilities::set(
                crate::capabilities::Capability::SECOND_INSTANCE_FORWARDING,
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

    pub(crate) fn link_path_for_log(link: &str) -> &str {
        link.split('?').next().unwrap_or(link)
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
                        tracing::info!(
                            target: LOG,
                            path = link_path_for_log(&link),
                            "received forwarded deep-link payload"
                        );
                        events::emit(AppEventKind::DeepLinkReceived(link), cx);
                    }
                });
            }
        })
        .detach();
    }

    /// Bind the forwarder listener with stale-file recovery: EADDRINUSE plus
    /// a refused connect unlinks the leftover and retries once; a live peer
    /// keeps its socket (`try_overwrite` displaces a live listener too).
    pub(crate) fn create_forwarder_listener(ipc_name: &str) -> Option<Listener> {
        let try_bind = || -> std::io::Result<Listener> {
            let name = resolve_ipc_name(ipc_name)?;
            ListenerOptions::new().name(name).create_sync()
        };
        match try_bind() {
            Ok(listener) => Some(listener),
            Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => {
                if is_socket_live(Some(ipc_name)) {
                    tracing::error!(
                        target: LOG,
                        error = %err,
                        "ipc socket owned by a live instance; forwarding degraded"
                    );
                    return None;
                }
                // Only a real file path can be unlinked; a bare namespaced
                // name must never resolve against the CWD.
                if !Path::new(ipc_name).is_absolute() || fs::remove_file(ipc_name).is_err() {
                    tracing::error!(
                        target: LOG,
                        error = %err,
                        "stale ipc socket not recoverable; forwarding degraded"
                    );
                    return None;
                }
                tracing::warn!(
                    target: LOG,
                    path = ipc_name,
                    "removed stale ipc socket left by a crashed run; retrying bind"
                );
                match try_bind() {
                    Ok(listener) => Some(listener),
                    Err(retry) => {
                        tracing::error!(
                            target: LOG,
                            error = %retry,
                            "ipc listener retry failed; forwarding degraded"
                        );
                        None
                    }
                }
            }
            Err(err) => {
                tracing::error!(target: LOG, error = %err, "failed to create ipc listener");
                None
            }
        }
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
        // Bind on the calling thread: a failed bind must degrade the
        // capability, not die silently inside the listener thread.
        let Some(listener) = create_forwarder_listener(&ipc_name) else {
            return false;
        };
        let (tx, rx) = flume::bounded::<ForwardedPayload>(FORWARD_QUEUE_CAP);
        let thread = std::thread::Builder::new()
            .name("gpui-ipc-forwarder".to_string())
            .spawn(move || {
                tracing::info!(target: LOG, ipc = %ipc_name, "starting ipc deep-link listener");

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
                    if let Err(err) = conn.set_recv_timeout(Some(IPC_READ_TIMEOUT))
                        && err.kind() != std::io::ErrorKind::Unsupported
                    {
                        tracing::warn!(target: LOG, error = %err, "cannot set ipc read timeout; client dropped");
                        continue;
                    }
                    let trimmed = match read_bounded_line(&conn) {
                        Ok(Some(line)) => line,
                        Ok(None) => continue,
                        Err(err) => {
                            tracing::warn!(target: LOG, error = %err, "ipc client stalled or errored; dropped");
                            continue;
                        }
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
                    } else if trimmed.starts_with(&scheme_prefix()) {
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
                        tracing::info!(
                            target: LOG,
                            path = link_path_for_log(&link),
                            "received forwarded deep-link payload via ipc"
                        );
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

    /// The socket name is world-bindable: both ends check the peer's euid so
    /// a squatter neither harvests links nor feeds us commands. No creds => accept.
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
    /// `Ok(None)` is EOF or an unterminated oversized line; `Err` is a read
    /// failure, deadline expiry included.
    pub(crate) fn read_bounded_line(conn: &Stream) -> std::io::Result<Option<String>> {
        let mut reader = BufReader::new(conn);
        let mut line = String::new();
        let mut limited = (&mut reader).take(MAX_LINE_BYTES as u64 + 1);
        match limited.read_line(&mut line) {
            Ok(n) if n > 0 && line.ends_with('\n') => Ok(Some(line.trim().to_string())),
            Ok(_) => Ok(None),
            Err(err) => Err(err),
        }
    }

    pub(crate) fn append_forwarded_link(path: &Path, link: &str) {
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

    /// Connect to the primary's socket, verifying the listener belongs to
    /// this app's user before any payload is written to it.
    fn connect_verified(ipc_name: &str) -> Result<Stream, std::io::Error> {
        let name = resolve_ipc_name(ipc_name)?;
        let stream = Stream::connect(name)?;
        if !peer_is_same_user(&stream) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "ipc listener failed the same-user check",
            ));
        }
        Ok(stream)
    }

    /// Send a single deep-link to the primary instance over a local socket.
    pub(crate) fn send_forwarded_link_via_ipc(
        ipc_name: &str,
        link: &str,
    ) -> Result<(), std::io::Error> {
        let mut stream = connect_verified(ipc_name)?;
        writeln!(stream, "{link}")
    }

    /// Ask the running primary to raise its window; true once it acked.
    pub(crate) fn request_raise(ipc_name: &str) -> bool {
        let mut stream = match connect_verified(ipc_name) {
            Ok(stream) => stream,
            Err(err) => {
                tracing::warn!(target: LOG, error = %err, "cannot reach the primary to raise its window");
                return false;
            }
        };
        let id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
        let request = ForwardedRequest::new(id, ForwardedCommand::ShowWindow);
        let Ok(frame) = encode_line(&request) else {
            return false;
        };
        if let Err(err) = stream.write_all(frame.as_bytes()) {
            tracing::warn!(target: LOG, error = %err, "raise request write failed");
            return false;
        }
        // Wait for the ack so the raise lands before exit; pipes cannot set
        // a deadline but a healthy primary answers instantly.
        let _ = stream.set_recv_timeout(Some(IPC_READ_TIMEOUT));
        let ack = match read_bounded_line(&stream) {
            Ok(Some(line)) => decode_line::<ForwardedResponse>(&line).ok().flatten(),
            Ok(None) => None,
            Err(err) => {
                tracing::warn!(target: LOG, error = %err, "no raise ack before the deadline");
                return false;
            }
        };
        match ack {
            Some(resp) if resp.id == id && resp.ok => true,
            Some(resp) => {
                tracing::warn!(
                    target: LOG,
                    error = resp.error.as_deref().unwrap_or("unknown"),
                    "primary refused the raise request"
                );
                false
            }
            None => {
                tracing::warn!(target: LOG, "primary closed without a raise ack");
                false
            }
        }
    }

    pub(crate) fn drain_forwarded_links(path: &Path) -> Vec<String> {
        let drained = path.with_extension("drained");
        // Windows rename refuses an existing target; a crashed run's leftover
        // must not wedge every later drain.
        let _ = fs::remove_file(&drained);
        if fs::rename(path, &drained).is_err() {
            return Vec::new();
        }
        // Rename claims the queue atomically (a racing append recreates it)
        // and moves a planted symlink aside instead of writing through it.
        let content = fs::read_to_string(&drained).unwrap_or_default();
        let _ = fs::remove_file(&drained);
        content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect()
    }

    fn overridden_runtime_dir() -> Option<PathBuf> {
        std::env::var_os(RUNTIME_DIR_ENV).map(PathBuf::from)
    }

    /// Queue lives in the user-owned cache dir, never a predictable temp dir;
    /// the test seam env var replaces that base dir wholesale.
    pub(crate) fn queue_file_path() -> Option<PathBuf> {
        if let Some(dir) = overridden_runtime_dir() {
            return Some(dir.join("runtime").join("forwarded-deep-links.queue"));
        }
        crate::platform::filesystem::paths::project_dirs().map(|dirs| {
            dirs.cache_dir()
                .join("runtime")
                .join("forwarded-deep-links.queue")
        })
    }

    pub(crate) fn resolve_ipc_name<'a>(
        ipc_name: &'a str,
    ) -> std::io::Result<interprocess::local_socket::Name<'a>> {
        // An absolute path is a filesystem socket by definition; only bare
        // names may use the abstract/named-pipe namespace.
        if GenericNamespaced::is_supported() && !Path::new(ipc_name).is_absolute() {
            ipc_name.to_ns_name::<GenericNamespaced>()
        } else {
            ipc_name.to_fs_name::<GenericFilePath>()
        }
    }

    fn ipc_name() -> Option<String> {
        if GenericNamespaced::is_supported() && overridden_runtime_dir().is_none() {
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
    use gpui_kit::{App, Global};

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
        crate::capabilities::set(
            crate::capabilities::Capability::SINGLE_INSTANCE,
            status.clone(),
            cx,
        );
        crate::capabilities::set(
            crate::capabilities::Capability::SECOND_INSTANCE_FORWARDING,
            status,
            cx,
        );
    }

    /// No-op: nothing was installed.
    pub fn shutdown(_cx: &mut App) {}
}

#[cfg(target_family = "wasm")]
pub use wasm::{Preflight, SingleInstanceRuntime, install, preflight, shutdown};

#[cfg(test)]
#[path = "single_instance.test.rs"]
mod single_instance_test;
