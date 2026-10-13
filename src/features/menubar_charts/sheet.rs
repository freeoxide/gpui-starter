use gpui_kit::component::WindowExt as _;
use gpui_kit::{App, ParentElement, px};

/// Open the charts sheet on the recorded root window. Returns false when no
/// root window exists (logged warn, no panic). No-op returning true when a
/// sheet is already active: `open_sheet_at` replaces the active sheet, so
/// this never swaps out a sheet it did not open.
pub fn open_sheet(cx: &mut App) -> bool {
    let Some(handle) = crate::app::window::root_window(cx) else {
        tracing::warn!(
            target: "gpui_starter::menubar_charts",
            "no root window recorded; charts sheet ignored"
        );
        return false;
    };
    handle
        .update(cx, |_, window, cx| {
            if window.has_active_sheet(cx) {
                return true;
            }
            // open_sheet's builder re-runs on layer rebuilds (gpui-component
            // window_ext.rs), so the panel entity is created once and captured.
            let panel = super::panel::ChartsPanel::view(window, cx);
            window.open_sheet(cx, move |sheet, _, _| {
                sheet
                    .size(px(420.))
                    .overlay(true)
                    .overlay_closable(true)
                    .title(crate::i18n::localize("menubar_charts_title"))
                    .child(panel.clone())
            });
            true
        })
        .unwrap_or(false)
}
