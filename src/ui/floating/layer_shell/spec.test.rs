use gpui_kit::{Edges, px};

use super::*;

#[test]
fn defaults_place_widgets_in_the_top_layer_without_keyboard() {
    let spec = LayerShellSpec::new();
    assert_eq!(spec.requested_layer(), WidgetLayer::Top);
    assert_eq!(spec.requested_keyboard(), WidgetKeyboardInteractivity::None);
    assert_eq!(spec.requested_namespace(), "gpui-starter.widgets");
    assert_eq!(spec.requested_anchor(), None);
    assert_eq!(spec.requested_margin(), None);
    assert_eq!(spec.requested_exclusive_zone(), None);
    assert_eq!(spec.requested_exclusive_edge(), None);
}

#[test]
fn builders_override_exactly_their_field() {
    let spec = LayerShellSpec::new()
        .layer(WidgetLayer::Background)
        .anchor(Placement9::Bottom)
        .margin(Edges {
            top: px(4.),
            right: px(8.),
            bottom: px(12.),
            left: px(16.),
        })
        .exclusive_zone(px(32.))
        .exclusive_edge(Edge4::Top)
        .keyboard(WidgetKeyboardInteractivity::Exclusive)
        .namespace("test.widgets");
    assert_eq!(spec.requested_layer(), WidgetLayer::Background);
    assert_eq!(spec.requested_anchor(), Some(Placement9::Bottom));
    let margin = spec.requested_margin().expect("margin set");
    assert_eq!(
        margin,
        Edges {
            top: px(4.),
            right: px(8.),
            bottom: px(12.),
            left: px(16.),
        }
    );
    assert_eq!(spec.requested_exclusive_zone(), Some(px(32.)));
    assert_eq!(spec.requested_exclusive_edge(), Some(Edge4::Top));
    assert_eq!(
        spec.requested_keyboard(),
        WidgetKeyboardInteractivity::Exclusive
    );
    assert_eq!(spec.requested_namespace(), "test.widgets");
}

#[test]
fn one_builder_leaves_the_rest_at_defaults() {
    let spec = LayerShellSpec::new().layer(WidgetLayer::Overlay);
    assert_eq!(spec.requested_layer(), WidgetLayer::Overlay);
    assert_eq!(spec.requested_anchor(), None);
    assert_eq!(spec.requested_margin(), None);
    assert_eq!(spec.requested_exclusive_zone(), None);
    assert_eq!(spec.requested_exclusive_edge(), None);
    assert_eq!(spec.requested_keyboard(), WidgetKeyboardInteractivity::None);
    assert_eq!(spec.requested_namespace(), "gpui-starter.widgets");
}

#[test]
fn enum_mirrors_are_copy() {
    fn assert_copy<T: Copy>(value: T) -> T {
        value
    }
    let layer = assert_copy(WidgetLayer::Top);
    let keyboard = assert_copy(WidgetKeyboardInteractivity::OnDemand);
    let edge = assert_copy(Edge4::Left);
    let surface = assert_copy(FloatSurface::LayerShell);
    assert_eq!(layer, WidgetLayer::Top);
    assert_eq!(keyboard, WidgetKeyboardInteractivity::OnDemand);
    assert_eq!(edge, Edge4::Left);
    assert_eq!(surface, FloatSurface::LayerShell);
}
