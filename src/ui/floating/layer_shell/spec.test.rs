use super::*;

#[test]
fn defaults_place_widgets_in_the_top_layer() {
    let spec = LayerShellSpec::new();
    assert_eq!(spec.requested_layer(), WidgetLayer::Top);
}

#[test]
fn builder_overrides_exactly_the_layer() {
    let spec = LayerShellSpec::new().layer(WidgetLayer::Background);
    assert_eq!(spec.requested_layer(), WidgetLayer::Background);
}

#[test]
fn enum_mirrors_are_copy() {
    fn assert_copy<T: Copy>(value: T) -> T {
        value
    }
    let layer = assert_copy(WidgetLayer::Top);
    let surface = assert_copy(FloatSurface::LayerShell);
    assert_eq!(layer, WidgetLayer::Top);
    assert_eq!(surface, FloatSurface::LayerShell);
}
