use gpui_kit::layer_shell::{
    Anchor, KeyboardInteractivity, Layer, LayerShellNotSupportedError, LayerShellOptions,
};
use gpui_kit::{App, DisplayId, Pixels, Point, px};

use super::LOG;
use super::spec::WidgetLayer;
use crate::ui::floating::spec::{FloatAnchor, FloatingSpec, Placement9};

/// Compositor rule key for widget surfaces (the layer-shell namespace).
const NAMESPACE: &str = "gpui-starter.widgets";

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

pub(crate) struct LayerSpawnPlan {
    pub(crate) layer_options: Option<LayerShellOptions>,
    pub(crate) display_id: Option<DisplayId>,
}

/// The layer-surface options for a widget float: namespace, z-order band,
/// anchor and margins from the placement. Pure: testable off a live session.
pub(crate) fn layer_surface_options(
    layer: WidgetLayer,
    placement: Placement9,
    offset: Point<Pixels>,
) -> LayerShellOptions {
    LayerShellOptions {
        namespace: NAMESPACE.to_string(),
        layer: layer_of(layer),
        anchor: anchor_bits(placement),
        exclusive_zone: None,
        exclusive_edge: None,
        margin: Some(margin_from_anchor(placement, offset)),
        keyboard_interactivity: KeyboardInteractivity::None,
    }
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
    let placement = spec.float_anchor().placement();
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
        layer_options: Some(layer_surface_options(
            layer_spec.requested_layer(),
            placement,
            *offset,
        )),
        display_id,
    }
}

/// True when the error is the compositor's layer-shell rejection; the Display-text
/// match backs up the downcast in case a future context wrap breaks the chain.
pub fn is_layer_shell_unsupported(err: &anyhow::Error) -> bool {
    err.downcast_ref::<LayerShellNotSupportedError>().is_some()
        || err.to_string().contains("zwlr_layer_shell_v1")
}

#[cfg(test)]
#[path = "wayland.test.rs"]
mod wayland_test;
