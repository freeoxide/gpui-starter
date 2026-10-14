use gpui_kit::layer_shell::{
    Anchor, KeyboardInteractivity, Layer, LayerShellNotSupportedError, LayerShellOptions,
};
use gpui_kit::{App, DisplayId, Edges, Pixels, Point, px};

use super::LOG;
use super::spec::{Edge4, WidgetKeyboardInteractivity, WidgetLayer};
use crate::ui::floating::spec::{FloatAnchor, FloatingSpec, Placement9};

pub(crate) fn anchor_bits(placement: Placement9) -> Anchor {
    use Placement9::*;
    match placement {
        TopLeft => Anchor::TOP | Anchor::LEFT,
        Top => Anchor::TOP,
        TopRight => Anchor::TOP | Anchor::RIGHT,
        Left => Anchor::LEFT,
        Center => Anchor::empty(),
        Right => Anchor::RIGHT,
        BottomLeft => Anchor::BOTTOM | Anchor::LEFT,
        Bottom => Anchor::BOTTOM,
        BottomRight => Anchor::BOTTOM | Anchor::RIGHT,
    }
}

pub(crate) fn margin_tuple(margin: Edges<Pixels>) -> (Pixels, Pixels, Pixels, Pixels) {
    (margin.top, margin.right, margin.bottom, margin.left)
}

fn margin_from_anchor(
    placement: Placement9,
    offset: Point<Pixels>,
) -> (Pixels, Pixels, Pixels, Pixels) {
    use Placement9::*;
    let (top, right, bottom, left) = match placement {
        TopLeft => (true, false, false, true),
        Top => (true, false, false, false),
        TopRight => (true, true, false, false),
        Left => (false, false, false, true),
        Center => (false, false, false, false),
        Right => (false, true, false, false),
        BottomLeft => (false, false, true, true),
        Bottom => (false, false, true, false),
        BottomRight => (false, true, true, false),
    };
    let zero = px(0.);
    (
        if top { offset.y } else { zero },
        if right { offset.x } else { zero },
        if bottom { offset.y } else { zero },
        if left { offset.x } else { zero },
    )
}

fn layer_of(layer: WidgetLayer) -> Layer {
    match layer {
        WidgetLayer::Background => Layer::Background,
        WidgetLayer::Bottom => Layer::Bottom,
        WidgetLayer::Top => Layer::Top,
        WidgetLayer::Overlay => Layer::Overlay,
    }
}

fn keyboard_of(mode: WidgetKeyboardInteractivity) -> KeyboardInteractivity {
    match mode {
        WidgetKeyboardInteractivity::None => KeyboardInteractivity::None,
        WidgetKeyboardInteractivity::Exclusive => KeyboardInteractivity::Exclusive,
        WidgetKeyboardInteractivity::OnDemand => KeyboardInteractivity::OnDemand,
    }
}

fn edge_bits(edge: Edge4) -> Anchor {
    match edge {
        Edge4::Top => Anchor::TOP,
        Edge4::Right => Anchor::RIGHT,
        Edge4::Bottom => Anchor::BOTTOM,
        Edge4::Left => Anchor::LEFT,
    }
}

pub(crate) struct LayerSpawnPlan {
    pub(crate) layer_options: Option<LayerShellOptions>,
    pub(crate) display_id: Option<DisplayId>,
}

pub(crate) fn plan(spec: &FloatingSpec, cx: &App) -> LayerSpawnPlan {
    let empty = LayerSpawnPlan {
        layer_options: None,
        display_id: None,
    };
    let Some(layer_spec) = spec.layer_shell_spec() else {
        return empty;
    };
    if cx.compositor_name() != "Wayland" {
        return empty;
    }
    let FloatAnchor::Screen {
        display, offset, ..
    } = spec.float_anchor()
    else {
        tracing::warn!(
            target: LOG,
            "layer surfaces anchor to output edges; ignoring the request for a window-anchored float"
        );
        return empty;
    };
    let placement = layer_spec
        .requested_anchor()
        .unwrap_or_else(|| spec.float_anchor().placement());
    let anchor = anchor_bits(placement);
    let margin = layer_spec
        .requested_margin()
        .map(margin_tuple)
        .unwrap_or_else(|| margin_from_anchor(placement, *offset));
    let exclusive_edge = layer_spec.requested_exclusive_edge().filter(|edge| {
        let bits = edge_bits(*edge);
        let valid = bits.bits().count_ones() == 1 && anchor.contains(bits);
        if !valid {
            tracing::warn!(
                target: LOG,
                "ignoring exclusive edge {edge:?}: must be a single edge of the surface anchor {anchor:?}"
            );
        }
        valid
    });
    let display_id = match display {
        Some(id) => cx.find_display(*id).map(|found| found.id()),
        None => cx.primary_display().map(|found| found.id()),
    };
    if display_id.is_none() && cx.displays().is_empty() {
        tracing::warn!(
            target: LOG,
            "displays not enumerable yet; the compositor picks the output (zed#46378)"
        );
    }
    LayerSpawnPlan {
        layer_options: Some(LayerShellOptions {
            namespace: layer_spec.requested_namespace().to_string(),
            layer: layer_of(layer_spec.requested_layer()),
            anchor,
            exclusive_zone: layer_spec.requested_exclusive_zone(),
            exclusive_edge: exclusive_edge.map(edge_bits),
            margin: Some(margin),
            keyboard_interactivity: keyboard_of(layer_spec.requested_keyboard()),
        }),
        display_id,
    }
}

/// True when the error is the compositor's layer-shell rejection:
/// downcasts to `LayerShellNotSupportedError`, or matches the Display
/// text in case a future context wrap breaks the downcast chain.
pub fn is_layer_shell_unsupported(err: &anyhow::Error) -> bool {
    err.downcast_ref::<LayerShellNotSupportedError>().is_some()
        || err.to_string().contains("zwlr_layer_shell_v1")
}

#[cfg(test)]
#[path = "wayland.test.rs"]
mod wayland_test;
