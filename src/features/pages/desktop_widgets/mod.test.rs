use gpui_kit::{AppContext as _, TestAppContext, WindowOptions};

use super::{
    CAPABILITY_DESKTOP_WIDGETS, DesktopWidgetsPage, WidgetKind, catalog, init, open_map, row_kinds,
    set_widget_enabled,
};
use crate::app_state::{AppConfig, debounce_test_lock, force_save, install_for_tests};
use crate::features::pages::desktop_widgets::catalog::PersistedWidget;

#[gpui_kit::test]
async fn row_kinds_mirror_the_catalog(cx: &mut TestAppContext) {
    let kinds = cx.read(row_kinds);
    assert_eq!(kinds.len(), catalog::available_kinds().len());
}

#[gpui_kit::test]
async fn open_map_starts_false_and_flips_with_the_registry(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::ui::floating::init);
    assert_eq!(cx.read(open_map).get(&WidgetKind::Clock), Some(&false));
    let _guard = debounce_test_lock();
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
    cx.update(|cx| set_widget_enabled(WidgetKind::Clock, true, cx));
    assert_eq!(cx.read(open_map).get(&WidgetKind::Clock), Some(&true));
    cx.update(force_save);
}

#[gpui_kit::test]
async fn init_registers_the_desktop_widgets_capability_row(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::capabilities::initialize);
    cx.update(init);
    let status = cx
        .read(|cx| {
            crate::capabilities::snapshot(cx)
                .get(CAPABILITY_DESKTOP_WIDGETS)
                .cloned()
        })
        .expect("capability row registered");
    assert!(status.supported);
    assert!(status.enabled);
    assert!(!status.degraded);
}

#[gpui_kit::test]
async fn init_respawns_enabled_widgets_exactly_once(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::ui::floating::init);
    cx.update(crate::capabilities::initialize);
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
    cx.update(init);
    assert!(cx.read(crate::ui::floating::floating_snapshot).is_empty());

    let config = AppConfig {
        widgets_layout: vec![PersistedWidget {
            kind: "clock".to_string(),
            enabled: true,
            placement: "top_right".to_string(),
            always_on_top: true,
            click_through: false,
            layer: "above".to_string(),
        }],
        ..AppConfig::default()
    };
    cx.update(|cx| install_for_tests(config, cx));
    cx.update(init);
    let snapshot = cx.read(crate::ui::floating::floating_snapshot);
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0].id.0.as_ref(), "widget-clock");
    cx.update(init);
    assert_eq!(
        cx.read(crate::ui::floating::floating_snapshot).len(),
        1,
        "the already-open guard must keep init idempotent"
    );
}

#[gpui_kit::test]
async fn the_page_constructs_in_a_window(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::ui::floating::init);
    let opened = cx.update(|cx| {
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| DesktopWidgetsPage::new(window, cx))
        })
    });
    assert!(opened.is_ok());
}
