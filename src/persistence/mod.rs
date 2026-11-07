// SQLite (bundled C libsqlite3-sys) has no wasm32-unknown-unknown support —
// the whole sqlite module tree is native-only.
#[cfg(not(target_family = "wasm"))]
pub mod sqlite;
