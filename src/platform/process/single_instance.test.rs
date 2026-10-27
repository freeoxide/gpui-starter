use std::{
    io::{BufRead, ErrorKind, Write},
    sync::mpsc,
    time::Duration,
};

use tempfile::tempdir;

use super::{
    AcquireOutcome, MAX_LINE_BYTES, SCHEME, acquire_instance_lock, append_forwarded_link,
    create_forwarder_listener, drain_forwarded_links, link_path_for_log, lock_file_path,
    read_bounded_line, resolve_ipc_name, retry_until_single, send_forwarded_link_via_ipc,
};
use interprocess::local_socket::{GenericNamespaced, ListenerOptions, prelude::*};

#[test]
fn forwarded_links_roundtrip_in_order() {
    let dir = tempdir().expect("tempdir");
    let queue = dir.path().join("forward.queue");

    append_forwarded_link(&queue, "gpui-starter://settings");
    append_forwarded_link(&queue, "gpui-starter://notifications");

    let links = drain_forwarded_links(&queue);
    assert_eq!(
        links,
        vec![
            "gpui-starter://settings".to_string(),
            "gpui-starter://notifications".to_string()
        ]
    );
    assert!(drain_forwarded_links(&queue).is_empty());
}

#[test]
fn forwards_link_over_ipc() {
    if !GenericNamespaced::is_supported() {
        return;
    }
    let unique_name = format!("com.gpui-starter.tests.{}", uuid::Uuid::new_v4());
    let name = resolve_ipc_name(&unique_name).expect("resolve name");
    let listener = ListenerOptions::new()
        .name(name)
        .create_sync()
        .expect("create listener");
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        if let Some(Ok(conn)) = listener.incoming().next() {
            let mut reader = std::io::BufReader::new(conn);
            let mut line = String::new();
            let _ = reader.read_line(&mut line);
            let _ = tx.send(line.trim().to_string());
        }
    });

    let sent = format!("{SCHEME}settings");
    send_forwarded_link_via_ipc(&unique_name, &sent).expect("send via ipc");
    let received = rx
        .recv_timeout(Duration::from_secs(3))
        .expect("receive forwarded link");
    assert_eq!(received, sent);
}

/// Listener plus a connected client on a throwaway name; the reader runs on
/// a thread because `read_bounded_line` blocks until a frame terminates.
fn bounded_line_harness(payload: usize) -> Option<String> {
    let unique_name = format!("com.gpui-starter.tests.{}", uuid::Uuid::new_v4());
    let name = resolve_ipc_name(&unique_name).expect("resolve name");
    let listener = ListenerOptions::new()
        .name(name)
        .create_sync()
        .expect("create listener");
    let (tx, rx) = mpsc::channel::<Option<String>>();
    std::thread::spawn(move || {
        let conn = listener.incoming().next().and_then(|c| c.ok());
        let _ = tx.send(conn.as_ref().and_then(read_bounded_line));
    });

    let mut client = interprocess::local_socket::Stream::connect(
        resolve_ipc_name(&unique_name).expect("resolve client name"),
    )
    .expect("connect client");
    let write = writeln!(client, "{}", "a".repeat(payload));
    // Past the cap the reader closes after MAX+1 bytes, so the write tail
    // surfaces as BrokenPipe on Windows named pipes — still a dropped frame.
    if payload > MAX_LINE_BYTES {
        if let Err(error) = write {
            assert_eq!(error.kind(), ErrorKind::BrokenPipe);
        }
    } else {
        write.expect("write frame");
    }
    rx.recv_timeout(Duration::from_secs(3))
        .expect("reader finished")
}

#[test]
fn ipc_frame_at_bound_is_accepted() {
    if !GenericNamespaced::is_supported() {
        return;
    }
    // Payload of exactly MAX_LINE_BYTES plus newline: the reader's
    // take(MAX + 1) window still sees the terminator.
    let line = bounded_line_harness(MAX_LINE_BYTES).expect("frame at the cap must be accepted");
    assert_eq!(line.len(), MAX_LINE_BYTES);
}

#[test]
fn oversized_ipc_frame_is_dropped() {
    if !GenericNamespaced::is_supported() {
        return;
    }
    let line = bounded_line_harness(MAX_LINE_BYTES + 1);
    assert_eq!(line, None, "frame past the 16 KiB cap must be dropped");
}

#[test]
fn retry_acquires_on_first_attempt_without_sleeping() {
    let mut calls = 0;
    let start = std::time::Instant::now();
    let outcome = retry_until_single(
        || {
            calls += 1;
            AcquireOutcome::Acquired("lock")
        },
        50,
        Duration::from_secs(30),
    );
    assert!(matches!(outcome, AcquireOutcome::Acquired("lock")));
    assert_eq!(calls, 1);
    // A first-try success must not pay the sleep interval.
    assert!(start.elapsed() < Duration::from_secs(30));
}

#[test]
fn retry_acquires_after_busy_attempts() {
    let mut calls = 0;
    let outcome = retry_until_single(
        || {
            calls += 1;
            if calls <= 3 {
                AcquireOutcome::Busy
            } else {
                AcquireOutcome::Acquired(())
            }
        },
        10,
        Duration::from_millis(1),
    );
    assert!(matches!(outcome, AcquireOutcome::Acquired(())));
    assert_eq!(calls, 4);
}

#[test]
fn retry_exhausts_attempts_into_busy() {
    let mut calls = 0;
    let outcome: AcquireOutcome<()> = retry_until_single(
        || {
            calls += 1;
            AcquireOutcome::Busy
        },
        5,
        Duration::from_millis(1),
    );
    assert!(matches!(outcome, AcquireOutcome::Busy));
    assert_eq!(calls, 5);
}

#[test]
fn retry_failed_short_circuits_without_sleeping() {
    let mut calls = 0;
    let start = std::time::Instant::now();
    let outcome: AcquireOutcome<()> = retry_until_single(
        || {
            calls += 1;
            AcquireOutcome::Failed
        },
        50,
        Duration::from_secs(30),
    );
    assert!(matches!(outcome, AcquireOutcome::Failed));
    assert_eq!(calls, 1, "Failed must not consume further attempts");
    assert!(start.elapsed() < Duration::from_secs(30));
}

#[test]
fn log_path_strips_the_query_and_keeps_plain_links() {
    assert_eq!(
        link_path_for_log("gpui-starter://auth/callback?code=x&state=y"),
        "gpui-starter://auth/callback"
    );
    assert_eq!(
        link_path_for_log("gpui-starter://settings"),
        "gpui-starter://settings"
    );
}

#[test]
fn lock_file_is_pinned_to_the_per_user_cache_dir() {
    let path = lock_file_path().expect("a per-user dir must resolve in tests");
    assert!(
        path.is_absolute(),
        "the lock must never resolve against the CWD"
    );
    assert_eq!(
        path.file_name().unwrap().to_str().unwrap(),
        "com.gpui-starter.app.instance",
        "lock naming stays stable so second-instance forwarding keeps working"
    );
    if let Some(dirs) = crate::platform::filesystem::paths::project_dirs() {
        assert_eq!(
            path.parent(),
            Some(dirs.cache_dir().join("runtime").as_path()),
            "the lock sits beside the forwarder artifacts in the cache dir"
        );
    }
}

#[test]
fn a_live_lock_holder_forces_the_busy_outcome() {
    let name = format!("com.gpui-starter.tests.{}", uuid::Uuid::new_v4());
    let first = acquire_instance_lock(&name);
    assert!(
        matches!(first, AcquireOutcome::Acquired(_)),
        "a unique lock name must be acquirable"
    );
    let second = acquire_instance_lock(&name);
    assert!(
        matches!(second, AcquireOutcome::Busy),
        "the owns-it outcome: a live holder must yield Busy, never a second owner"
    );
    drop(first);
}

#[test]
fn a_released_lock_is_reacquired_by_the_next_instance() {
    let name = format!("com.gpui-starter.tests.{}", uuid::Uuid::new_v4());
    let first = acquire_instance_lock(&name);
    assert!(matches!(first, AcquireOutcome::Acquired(_)));
    drop(first);
    let second = acquire_instance_lock(&name);
    assert!(
        matches!(second, AcquireOutcome::Acquired(_)),
        "kernel reclamation must leave no stale lock behind"
    );
}

#[cfg(target_family = "unix")]
#[test]
fn absolute_paths_resolve_as_filesystem_socket_names() {
    let name = resolve_ipc_name("/tmp/definitely-a-path/forwarder.sock").expect("fs name");
    assert!(!name.is_namespaced());
    assert!(name.is_path());
}

#[cfg(target_family = "unix")]
#[test]
fn a_stale_socket_file_is_unlinked_and_the_bind_retried() {
    let dir = tempdir().expect("tempdir");
    let sock = dir.path().join("forwarder.sock");
    let name = sock.display().to_string();

    // std's UnixListener never unlinks on drop: exactly the file a crashed
    // run leaves behind on the fixed cache path.
    let leftover = std::os::unix::net::UnixListener::bind(&sock).expect("bind leftover");
    drop(leftover);
    assert!(sock.exists());

    let recovered = create_forwarder_listener(&name);
    assert!(
        recovered.is_some(),
        "EADDRINUSE with a refused connect must unlink the stale file and retry"
    );
}

#[cfg(target_family = "unix")]
#[test]
fn a_live_listener_keeps_its_socket_under_conflict() {
    let dir = tempdir().expect("tempdir");
    let sock = dir.path().join("forwarder.sock");
    let name = sock.display().to_string();

    let live = create_forwarder_listener(&name).expect("live listener binds a fresh path");
    let outcome = create_forwarder_listener(&name);
    assert!(
        outcome.is_none(),
        "a live owner must degrade forwarding, not double-bind"
    );
    assert!(
        sock.exists(),
        "recovery must never steal a live instance's socket file"
    );
    drop(live);
}
