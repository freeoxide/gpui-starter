//! Wasm-only browser integration bridges behind the app's existing service
//! abstractions (house rule: no parallel web-only service layer).

pub mod clipboard;
pub mod connectivity;
pub mod dispatch;
pub mod document_meta;
pub mod router;

use gpui_kit::App;

/// Install every browser integration at app init; [`dispatch`] must come
/// first because router/connectivity push work through its queue.
pub fn install(cx: &mut App) {
    dispatch::install(cx);
    router::install();
    document_meta::install(cx);
    connectivity::install_listeners();
}
