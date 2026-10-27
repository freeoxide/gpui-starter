use gpui_kit::TestAppContext;

use super::*;
use crate::app_state::{AppConfig, debounce_test_lock, force_save, install_for_tests};
use crate::features::pages::desktop_widgets::catalog::widget_layout;
use crate::ui::floating::{FloatingHandle, floating_snapshot, init as floating_init};

fn boot(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(floating_init);
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
}

fn clock_entry() -> PersistedWidget {
    PersistedWidget {
        kind: "clock".to_string(),
        enabled: true,
        placement: "top_right".to_string(),
        always_on_top: true,
        click_through: false,
        layer: "above".to_string(),
    }
}

#[gpui_kit::test]
async fn enabling_a_widget_spawns_a_native_float_and_persists_it(cx: &mut TestAppContext) {
    let _guard = debounce_test_lock();
    boot(cx);
    assert!(cx.update(|cx| set_widget_enabled(WidgetKind::Clock, true, cx)));
    let snapshot = cx.read(floating_snapshot);
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0].id.0.as_ref(), "widget-clock");
    assert!(matches!(snapshot[0].handle, FloatingHandle::Native(_)));
    let layout = cx.read(widget_layout);
    assert_eq!(layout.len(), 1);
    assert!(layout[0].enabled);
    cx.update(force_save);
}

#[gpui_kit::test]
async fn disabling_a_widget_closes_its_float_and_reports_false_once_closed(
    cx: &mut TestAppContext,
) {
    let _guard = debounce_test_lock();
    boot(cx);
    cx.update(|cx| set_widget_enabled(WidgetKind::Clock, true, cx));
    assert!(cx.update(|cx| set_widget_enabled(WidgetKind::Clock, false, cx)));
    assert!(cx.read(floating_snapshot).is_empty());
    let again = cx.update(|cx| set_widget_enabled(WidgetKind::Clock, false, cx));
    assert!(!again, "closing an already-closed widget reports false");
    let layout = cx.read(widget_layout);
    assert_eq!(layout.len(), 1);
    assert!(!layout[0].enabled);
    cx.update(force_save);
}

#[gpui_kit::test]
async fn enabling_the_battery_renders_headless_without_panic(cx: &mut TestAppContext) {
    let _guard = debounce_test_lock();
    boot(cx);
    assert!(cx.update(|cx| set_widget_enabled(WidgetKind::Battery, true, cx)));
    let snapshot = cx.read(floating_snapshot);
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0].id.0.as_ref(), "widget-battery");
    cx.update(force_save);
}

#[gpui_kit::test]
async fn a_placement_change_respawns_the_single_entry_and_persists(cx: &mut TestAppContext) {
    let _guard = debounce_test_lock();
    boot(cx);
    cx.update(|cx| set_widget_enabled(WidgetKind::Clock, true, cx));
    assert!(
        cx.update(|cx| { set_widget_placement(WidgetKind::Clock, Placement9::BottomLeft, cx) })
    );
    let snapshot = cx.read(floating_snapshot);
    assert_eq!(snapshot.len(), 1, "respawn must swap, not duplicate");
    assert_eq!(cx.read(widget_layout)[0].placement, "bottom_left");
    cx.update(force_save);
}

#[gpui_kit::test]
async fn click_through_returns_the_shim_bool_and_persists_the_choice(cx: &mut TestAppContext) {
    let _guard = debounce_test_lock();
    boot(cx);
    cx.update(|cx| set_widget_enabled(WidgetKind::Clock, true, cx));
    let applied = cx.update(|cx| set_widget_click_through(WidgetKind::Clock, true, cx));
    assert!(!applied, "the test platform has no click-through shim");
    assert!(cx.read(widget_layout)[0].click_through);
    cx.update(force_save);
}

#[gpui_kit::test]
async fn always_on_top_respawns_when_the_runtime_apply_fails(cx: &mut TestAppContext) {
    let _guard = debounce_test_lock();
    boot(cx);
    cx.update(|cx| set_widget_enabled(WidgetKind::Clock, true, cx));
    let result = cx.update(|cx| set_widget_always_on_top(WidgetKind::Clock, false, cx));
    assert!(result, "the respawn path reports overall success");
    assert_eq!(cx.read(floating_snapshot).len(), 1);
    assert!(!cx.read(widget_layout)[0].always_on_top);
    cx.update(force_save);
}

#[gpui_kit::test]
async fn spawn_persisted_respawns_enabled_widgets_and_is_idempotent(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(floating_init);
    let config = AppConfig {
        widgets_layout: vec![clock_entry()],
        ..AppConfig::default()
    };
    cx.update(|cx| install_for_tests(config, cx));
    cx.update(spawn_persisted);
    let snapshot = cx.read(floating_snapshot);
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0].id.0.as_ref(), "widget-clock");
    cx.update(spawn_persisted);
    assert_eq!(
        cx.read(floating_snapshot).len(),
        1,
        "the already-open guard must not duplicate entries"
    );
}

#[gpui_kit::test]
async fn spawn_persisted_with_an_empty_config_spawns_nothing(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(floating_init);
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
    cx.update(spawn_persisted);
    assert!(cx.read(floating_snapshot).is_empty());
}

#[gpui_kit::test]
async fn setters_tolerate_a_closed_widget_and_reject_unknown_layers(cx: &mut TestAppContext) {
    let _guard = debounce_test_lock();
    boot(cx);
    assert!(cx.update(|cx| set_widget_layer(WidgetKind::Metrics, "pinned", cx)));
    assert_eq!(cx.read(widget_layout)[0].layer, "pinned");
    assert!(!cx.update(|cx| set_widget_layer(WidgetKind::Metrics, "weird", cx)));
    assert!(cx.update(|cx| { set_widget_click_through(WidgetKind::Metrics, true, cx) }));
    assert!(cx.update(|cx| set_widget_always_on_top(WidgetKind::Metrics, false, cx)));
    assert_eq!(cx.read(floating_snapshot).len(), 0);
    cx.update(force_save);
}
