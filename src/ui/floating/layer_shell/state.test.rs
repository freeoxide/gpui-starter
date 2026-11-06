use gpui_kit::{App, TestAppContext};

use super::*;

fn status_of(cx: &App, capability: Capability) -> crate::capabilities::CapabilityStatus {
    crate::capabilities::snapshot(cx)
        .get(&capability)
        .cloned()
        .unwrap_or_else(|| panic!("missing capability row {capability}"))
}

#[gpui_kit::test]
async fn init_writes_unsupported_rows_on_the_test_platform(cx: &mut TestAppContext) {
    cx.update(init);
    cx.read(|cx| {
        for capability in [
            Capability::LAYER_SHELL_WIDGETS,
            Capability::LAYER_SHELL_INPUT_PASSTHROUGH,
        ] {
            let status = status_of(cx, capability);
            assert!(!status.supported, "{capability}");
            assert_eq!(
                status.reason.as_ref().map(|reason| reason.to_string()),
                Some("Wayland layer-shell only".to_string()),
                "{capability}"
            );
        }
    });
}

#[gpui_kit::test]
async fn record_unsupported_degrades_the_capability(cx: &mut TestAppContext) {
    cx.update(init);
    cx.update(|cx| {
        record_unsupported(
            &anyhow::anyhow!("Compositor doesn't support zwlr_layer_shell_v1"),
            cx,
        )
    });
    cx.read(|cx| {
        let status = status_of(cx, Capability::LAYER_SHELL_WIDGETS);
        assert!(status.degraded);
        assert_eq!(
            status.last_error.as_deref(),
            Some("Compositor doesn't support zwlr_layer_shell_v1")
        );
    });
}

#[gpui_kit::test]
async fn record_available_restores_supported_enabled(cx: &mut TestAppContext) {
    cx.update(init);
    cx.update(|cx| record_unsupported(&anyhow::anyhow!("Compositor lacks the protocol"), cx));
    cx.update(record_available);
    cx.read(|cx| {
        let status = status_of(cx, Capability::LAYER_SHELL_WIDGETS);
        assert!(status.supported && status.enabled && !status.degraded);
        assert_eq!(status.last_error, None);
    });
}
