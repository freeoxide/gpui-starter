use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::{App, TestAppContext};

use super::*;

fn status_of(cx: &App, name: &str) -> crate::capabilities::CapabilityStatus {
    crate::capabilities::snapshot(cx)
        .get(name)
        .cloned()
        .unwrap_or_else(|| panic!("missing capability row {name}"))
}

#[gpui_kit::test]
async fn init_writes_unsupported_rows_on_the_test_platform(cx: &mut TestAppContext) {
    cx.update(init);
    cx.read(|cx| {
        for name in [
            CAPABILITY_LAYER_SHELL_WIDGETS,
            CAPABILITY_LAYER_SHELL_INPUT_PASSTHROUGH,
        ] {
            let status = status_of(cx, name);
            assert!(!status.supported, "{name}");
            assert_eq!(
                status.reason.as_ref().map(|reason| reason.to_string()),
                Some("Wayland layer-shell only".to_string()),
                "{name}"
            );
        }
    });
}

#[gpui_kit::test]
async fn record_unsupported_degrades_the_capability_and_counts_the_fallback(
    cx: &mut TestAppContext,
) {
    cx.update(init);
    cx.update(|cx| {
        record_unsupported(
            &anyhow::anyhow!("Compositor doesn't support zwlr_layer_shell_v1"),
            cx,
        )
    });
    cx.read(|cx| {
        let status = status_of(cx, CAPABILITY_LAYER_SHELL_WIDGETS);
        assert!(status.degraded);
        assert!(status.last_error.is_some());
        let state = cx.global::<LayerShellState>();
        assert_eq!(state.available, Some(false));
        assert_eq!(state.fallbacks, 1);
        assert!(state.last_error.is_some());
    });
}

#[gpui_kit::test]
async fn record_available_restores_supported_enabled(cx: &mut TestAppContext) {
    cx.update(init);
    cx.update(|cx| record_unsupported(&anyhow::anyhow!("Compositor lacks the protocol"), cx));
    cx.update(record_available);
    cx.read(|cx| {
        let status = status_of(cx, CAPABILITY_LAYER_SHELL_WIDGETS);
        assert!(status.supported && status.enabled && !status.degraded);
        assert_eq!(cx.global::<LayerShellState>().available, Some(true));
        assert_eq!(cx.global::<LayerShellState>().last_error, None);
    });
}

#[gpui_kit::test]
async fn record_transitions_notify_global_observers(cx: &mut TestAppContext) {
    let fired = Rc::new(Cell::new(false));
    let flag = fired.clone();
    cx.update(|cx| {
        cx.observe_global::<LayerShellState>(move |_: &mut App| flag.set(true))
            .detach();
    });
    assert!(!fired.get());
    cx.update(record_available);
    assert!(fired.get());
}
