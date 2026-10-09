#[cfg(any(target_os = "macos", target_os = "windows"))]
pub mod tray;
// No Linux tray: tray-icon 0.24 needs a GTK loop gpui does not run. The
// ksni (D-Bus SNI) candidate is assessed in docs/research/icon-surfaces.md.
