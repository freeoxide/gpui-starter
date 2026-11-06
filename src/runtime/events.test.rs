use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::{App, TestAppContext};

use super::*;

#[test]
fn app_event_new_assigns_distinct_ids_and_a_current_timestamp() {
    let before = chrono::Utc::now();
    let first = AppEvent::new(AppEventKind::DeepLinkReceived(
        "gpui-starter://settings".to_string(),
    ));
    let after = chrono::Utc::now();
    let second = AppEvent::new(AppEventKind::DiagnosticsChanged);

    assert_ne!(first.id, second.id);
    assert!(matches!(
        &first.kind,
        AppEventKind::DeepLinkReceived(link) if link == "gpui-starter://settings"
    ));
    assert!(matches!(second.kind, AppEventKind::DiagnosticsChanged));
    assert!(first.emitted_at.0 >= before && first.emitted_at.0 <= after);
}

#[gpui_kit::test]
async fn drain_preserves_emit_order(cx: &TestAppContext) {
    cx.update(|cx| {
        emit(AppEventKind::DiagnosticsChanged, cx);
        emit(
            AppEventKind::AppError {
                message: "oops".to_string(),
                severity: crate::errors::AppErrorSeverity::Error,
            },
            cx,
        );
        emit(
            AppEventKind::DeepLinkReceived("gpui-starter://settings".to_string()),
            cx,
        );
    });
    let drained = cx.update(|cx| drain(cx));
    assert_eq!(drained.len(), 3);
    assert!(matches!(drained[0].kind, AppEventKind::DiagnosticsChanged));
    assert!(matches!(drained[1].kind, AppEventKind::AppError { .. }));
    assert!(matches!(drained[2].kind, AppEventKind::DeepLinkReceived(_)));
    assert!(cx.update(|cx| drain(cx)).is_empty());
}

#[gpui_kit::test]
async fn drain_on_a_fresh_app_returns_empty_and_installs_no_global(cx: &TestAppContext) {
    assert!(cx.update(|cx| drain(cx)).is_empty());
    assert!(cx.read(|cx| cx.try_global::<AppEventQueue>().is_none()));
}

#[gpui_kit::test]
async fn emit_then_drain_yields_each_event_once(cx: &TestAppContext) {
    cx.update(|cx| {
        emit(
            AppEventKind::Test {
                message: "first".to_string(),
            },
            cx,
        );
        emit(AppEventKind::DiagnosticsChanged, cx);
    });
    let drained = cx.update(|cx| drain(cx));
    assert_eq!(drained.len(), 2);
    assert!(matches!(drained[0].kind, AppEventKind::Test { .. }));
    assert!(matches!(drained[1].kind, AppEventKind::DiagnosticsChanged));
    assert!(cx.update(|cx| drain(cx)).is_empty());
}

#[gpui_kit::test]
async fn emit_error_maps_severity_and_message(cx: &TestAppContext) {
    let error =
        crate::errors::AppError::invalid_deep_link("gpui-starter://nowhere", "unknown route");
    let expected_severity = error.severity();
    let expected_message = error.to_string();
    cx.update(|cx| emit_error(error, cx));
    let drained = cx.update(|cx| drain(cx));
    assert_eq!(drained.len(), 1);
    match &drained[0].kind {
        AppEventKind::AppError { message, severity } => {
            assert_eq!(*severity, expected_severity);
            assert_eq!(message, &expected_message);
        }
        other => panic!("expected an AppError event, got {other:?}"),
    }
}

#[gpui_kit::test]
async fn drain_called_from_a_global_observer_terminates(cx: &TestAppContext) {
    let firings = Rc::new(Cell::new(0u32));
    let count = firings.clone();
    cx.update(|cx| {
        cx.observe_global::<AppEventQueue>(move |cx: &mut App| {
            count.set(count.get() + 1);
            drain(cx);
        })
        .detach();
        emit(AppEventKind::DiagnosticsChanged, cx);
    });
    // emit fires the observer once; its drain re-fires it once more before
    // the empty guard stops the effect chain.
    assert_eq!(firings.get(), 2);
    assert!(cx.update(|cx| drain(cx)).is_empty());
}
