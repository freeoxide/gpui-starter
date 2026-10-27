use gpui_kit::component::WindowExt as _;
use gpui_kit::{TestAppContext, WindowOptions};

use super::ChartsPanel;
use crate::app::window::RootWindow;
use crate::features::menubar_charts::{
    CAPABILITY_MENUBAR_CHARTS, CAPABILITY_MENUBAR_CHARTS_ANCHORED, open_sheet,
};

fn boot(cx: &mut gpui_kit::App) {
    gpui_kit::init(cx);
    crate::ui::floating::init(cx);
    crate::features::menubar_charts::init(cx);
}

fn open_root_window(cx: &gpui_kit::TestAppContext) -> gpui_kit::AnyWindowHandle {
    let (handle, _) = cx
        .update(|cx| gpui_kit::open_window(WindowOptions::default(), cx, ChartsPanel::view))
        .expect("test window must open");
    cx.update(|cx| cx.set_global(RootWindow(handle)));
    handle
}

#[gpui_kit::test]
async fn panel_opens_in_a_window_and_samples_on_direct_tick(cx: &TestAppContext) {
    cx.update(gpui_kit::init);
    let opened =
        cx.update(|cx| gpui_kit::open_window(WindowOptions::default(), cx, ChartsPanel::view));
    assert!(opened.is_ok());
    let (_, panel) = opened.unwrap();
    cx.update(|cx| {
        panel.update(cx, |panel, cx| {
            panel.metrics().update(cx, |m, cx| m.tick(cx));
        })
    });
    assert!(
        cx.read(|cx| panel.read(cx).metrics().read(cx).samples().len()) >= 1,
        "one direct tick must land a sample without waiting on the timer"
    );
}

#[gpui_kit::test]
async fn open_sheet_hosts_one_active_sheet_on_the_root_window(cx: &TestAppContext) {
    cx.update(boot);
    let handle = open_root_window(cx);

    assert!(cx.update(|cx| open_sheet(cx)));
    assert!(cx.update(|cx| {
        handle
            .update(cx, |_, window, cx| window.has_active_sheet(cx))
            .unwrap()
    }));

    assert!(
        cx.update(|cx| open_sheet(cx)),
        "a second open must no-op, not replace the active sheet"
    );
    assert!(cx.update(|cx| {
        handle
            .update(cx, |_, window, cx| window.has_active_sheet(cx))
            .unwrap()
    }));
}

#[gpui_kit::test]
async fn open_sheet_without_a_root_window_returns_false(cx: &TestAppContext) {
    cx.update(boot);
    assert!(!cx.update(|cx| open_sheet(cx)));
}

#[gpui_kit::test]
async fn init_registers_both_capability_rows(cx: &TestAppContext) {
    cx.update(boot);
    let snapshot = cx.read(|cx| crate::capabilities::snapshot(cx));
    assert_eq!(
        snapshot
            .get(CAPABILITY_MENUBAR_CHARTS)
            .expect("menubar_charts row")
            .supported,
        cfg!(not(target_family = "wasm"))
    );
    assert_eq!(
        snapshot
            .get(CAPABILITY_MENUBAR_CHARTS_ANCHORED)
            .expect("menubar_charts_anchored row")
            .supported,
        cfg!(target_os = "macos")
    );
}
