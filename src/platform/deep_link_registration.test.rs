use std::path::Path;

use super::{command_matches, ensure_deep_link_registered};
use crate::shell::route::APP_URL_SCHEME;

#[test]
fn registration_writes_the_protocol_key_hierarchy() {
    let key_path = format!("Software\\Classes\\{APP_URL_SCHEME}");
    // A leftover key from an earlier run would satisfy the read-back without
    // this run writing anything, so start and end from a clean hive.
    let _ = windows_registry::CURRENT_USER.remove_tree(&key_path);

    ensure_deep_link_registered().expect("ensure registers the scheme");

    let scheme = windows_registry::CURRENT_USER
        .open(&key_path)
        .expect("scheme key");
    let command = windows_registry::CURRENT_USER
        .open(format!("{key_path}\\shell\\open\\command"))
        .expect("command key")
        .get_string("");
    let exe = std::env::current_exe().expect("current_exe");
    let observed = (
        scheme.get_string(""),
        scheme.get_string("URL Protocol"),
        command,
    );
    let _ = windows_registry::CURRENT_USER.remove_tree(&key_path);

    assert_eq!(
        observed.0.expect("key default"),
        "URL:GPUI Starter",
        "the scheme key default names the protocol"
    );
    assert_eq!(
        observed.1.expect("URL Protocol marker"),
        "",
        "the empty URL Protocol value marks the key a protocol handler"
    );
    assert_eq!(
        observed.2.expect("stored command"),
        format!("\"{}\" \"%1\"", exe.display()),
        "the command must launch the current exe with the %1 placeholder"
    );
}

#[test]
fn command_matches_current_exe_ignoring_case() {
    let stored = "\"C:\\Bin\\gpui-starter.EXE\" \"%1\"";
    assert!(command_matches(
        stored,
        Path::new("c:\\bin\\GPUI-STARTER.exe")
    ));
}

#[test]
fn command_rewritten_for_a_different_exe() {
    let stored = "\"C:\\target\\debug\\gpui-starter.exe\" \"%1\"";
    assert!(!command_matches(
        stored,
        Path::new("C:\\target\\release\\gpui-starter.exe")
    ));
}

#[test]
fn command_rewritten_when_placeholder_is_missing_or_altered() {
    let exe = Path::new("C:\\bin\\gpui-starter.exe");
    assert!(!command_matches("\"C:\\bin\\gpui-starter.exe\"", exe));
    assert!(!command_matches(
        "\"C:\\bin\\gpui-starter.exe\" \"%9\"",
        exe
    ));
    assert!(!command_matches("\"C:\\bin\\gpui-starter.exe\" %1", exe));
}

#[test]
fn command_rewritten_for_unquoted_or_garbage_values() {
    let exe = Path::new("C:\\bin\\gpui-starter.exe");
    assert!(!command_matches("C:\\bin\\gpui-starter.exe \"%1\"", exe));
    assert!(!command_matches("", exe));
    assert!(!command_matches("URL:GPUI Starter", exe));
}
