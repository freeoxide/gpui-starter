//! Shared helpers for the tests/ targets. Each target uses a subset,
//! hence the `#[allow(dead_code)]` on every `mod common;` declaration.

use std::path::{Path, PathBuf};

/// Absolute path for a repo-relative file, immune to the test runner's
/// current working directory.
pub fn manifest_path(relative: &str) -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join(relative)
}

pub fn read_file(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

pub fn repo_file(name: &str) -> String {
    read_file(manifest_path(name))
}
