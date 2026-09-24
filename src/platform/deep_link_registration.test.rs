use std::path::Path;

use super::{command_matches, scheme_key};

#[test]
fn scheme_key_is_the_scheme_minus_separator() {
    assert_eq!(scheme_key(), "gpui-starter");
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
