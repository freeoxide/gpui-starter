use gpui_kit::layer_shell::Anchor;
use gpui_kit::{Edges, TestAppContext, point, px, size};

use super::*;
use crate::ui::floating::{FloatingSpec, FloatingWindowId, LayerShellSpec};

#[test]
fn anchor_bits_cover_all_nine_placements() {
    let cases = [
        (Placement9::TopLeft, Anchor::TOP | Anchor::LEFT),
        (Placement9::Top, Anchor::TOP),
        (Placement9::TopRight, Anchor::TOP | Anchor::RIGHT),
        (Placement9::Left, Anchor::LEFT),
        (Placement9::Center, Anchor::empty()),
        (Placement9::Right, Anchor::RIGHT),
        (Placement9::BottomLeft, Anchor::BOTTOM | Anchor::LEFT),
        (Placement9::Bottom, Anchor::BOTTOM),
        (Placement9::BottomRight, Anchor::BOTTOM | Anchor::RIGHT),
    ];
    for (placement, expected) in cases {
        assert_eq!(anchor_bits(placement), expected, "{placement:?}");
    }
}

#[test]
fn margin_tuple_keeps_css_order() {
    let edges = Edges {
        top: px(1.),
        right: px(2.),
        bottom: px(3.),
        left: px(4.),
    };
    assert_eq!(margin_tuple(edges), (px(1.), px(2.), px(3.), px(4.)));
}

#[test]
fn margin_from_anchor_applies_the_offset_only_on_touched_edges() {
    let cases = [
        (Placement9::TopLeft, (px(11.), px(0.), px(0.), px(7.))),
        (Placement9::Top, (px(11.), px(0.), px(0.), px(0.))),
        (Placement9::TopRight, (px(11.), px(7.), px(0.), px(0.))),
        (Placement9::Left, (px(0.), px(0.), px(0.), px(7.))),
        (Placement9::Center, (px(0.), px(0.), px(0.), px(0.))),
        (Placement9::Right, (px(0.), px(7.), px(0.), px(0.))),
        (Placement9::BottomLeft, (px(0.), px(0.), px(11.), px(7.))),
        (Placement9::Bottom, (px(0.), px(0.), px(11.), px(0.))),
        (Placement9::BottomRight, (px(0.), px(7.), px(11.), px(0.))),
    ];
    for (placement, expected) in cases {
        assert_eq!(
            margin_from_anchor(placement, point(px(7.), px(11.))),
            expected,
            "{placement:?}"
        );
    }
}

#[test]
fn edge_bits_map_to_single_anchor_bits() {
    assert_eq!(edge_bits(Edge4::Top), Anchor::TOP);
    assert_eq!(edge_bits(Edge4::Right), Anchor::RIGHT);
    assert_eq!(edge_bits(Edge4::Bottom), Anchor::BOTTOM);
    assert_eq!(edge_bits(Edge4::Left), Anchor::LEFT);
}

#[test]
fn classifies_layer_shell_rejection() {
    let direct: anyhow::Error = LayerShellNotSupportedError.into();
    assert!(is_layer_shell_unsupported(&direct));
    let wrapped = anyhow::anyhow!("Compositor doesn't support zwlr_layer_shell_v1");
    assert!(is_layer_shell_unsupported(&wrapped));
    assert!(!is_layer_shell_unsupported(&anyhow::anyhow!(
        "other failure"
    )));
}

#[gpui_kit::test]
async fn plan_returns_the_empty_plan_without_a_layer_request(cx: &mut TestAppContext) {
    let plan = cx.update(|cx| {
        plan(
            &FloatingSpec::new(FloatingWindowId("plain".into()), size(px(120.), px(90.))),
            cx,
        )
    });
    assert!(plan.layer_options.is_none());
    assert!(plan.display_id.is_none());
}

#[gpui_kit::test]
async fn plan_declines_a_layer_request_off_wayland_sessions(cx: &mut TestAppContext) {
    let plan = cx.update(|cx| {
        plan(
            &FloatingSpec::new(FloatingWindowId("layered".into()), size(px(120.), px(90.)))
                .layer_shell(LayerShellSpec::new()),
            cx,
        )
    });
    assert!(plan.layer_options.is_none());
    assert!(plan.display_id.is_none());
}
