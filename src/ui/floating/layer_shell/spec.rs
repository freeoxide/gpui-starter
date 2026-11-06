/// Which z-order band a layer surface occupies; converted to gpui's
/// Wayland `Layer` on Linux only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetLayer {
    Background,
    Bottom,
    /// Above app windows, below docks/overlays. Default for widgets; gpui's
    /// own default is Overlay, this diverges deliberately.
    #[default]
    Top,
    Overlay,
}

/// Which surface kind actually backs a float: a plain OS window or a
/// Wayland layer-shell surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FloatSurface {
    #[default]
    Window,
    LayerShell,
}

/// Caller's request for a Wayland layer-shell surface. Compiles everywhere; off
/// Wayland sessions it is ignored with one info log per process.
#[derive(Clone, Debug, Default)]
pub struct LayerShellSpec {
    layer: WidgetLayer,
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
impl LayerShellSpec {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn layer(mut self, layer: WidgetLayer) -> Self {
        self.layer = layer;
        self
    }

    pub(crate) fn requested_layer(&self) -> WidgetLayer {
        self.layer
    }
}

#[cfg(test)]
#[path = "spec.test.rs"]
mod spec_test;
