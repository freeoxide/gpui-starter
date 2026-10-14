use gpui_kit::{Edges, Pixels};

use crate::ui::floating::Placement9;

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

/// Keyboard delivery for a layer surface. `None` is the default: widgets
/// do not steal keys.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetKeyboardInteractivity {
    #[default]
    None,
    Exclusive,
    OnDemand,
}

/// A single screen edge, for the exclusive-zone edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge4 {
    Top,
    Right,
    Bottom,
    Left,
}

/// Which surface kind actually backs a float: a plain OS window or a
/// Wayland layer-shell surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FloatSurface {
    #[default]
    Window,
    LayerShell,
}

/// Caller's request for a Wayland layer-shell surface. Pure data; compiles
/// on every platform and is inert (ignored, one info log per process) off
/// Wayland sessions.
#[derive(Clone, Debug)]
pub struct LayerShellSpec {
    layer: WidgetLayer,
    anchor: Option<Placement9>,
    margin: Option<Edges<Pixels>>,
    exclusive_zone: Option<Pixels>,
    exclusive_edge: Option<Edge4>,
    keyboard: WidgetKeyboardInteractivity,
    namespace: String,
}

impl Default for LayerShellSpec {
    fn default() -> Self {
        Self {
            layer: WidgetLayer::Top,
            anchor: None,
            margin: None,
            exclusive_zone: None,
            exclusive_edge: None,
            keyboard: WidgetKeyboardInteractivity::None,
            namespace: "gpui-starter.widgets".to_string(),
        }
    }
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

    /// Overrides the anchor derived from `FloatAnchor`. Center means
    /// unanchored; the compositor centers the surface.
    pub fn anchor(mut self, placement: Placement9) -> Self {
        self.anchor = Some(placement);
        self
    }

    /// CSS order top/right/bottom/left, matching gpui's
    /// `LayerShellOptions::margin`.
    pub fn margin(mut self, margin: Edges<Pixels>) -> Self {
        self.margin = Some(margin);
        self
    }

    pub fn exclusive_zone(mut self, zone: Pixels) -> Self {
        self.exclusive_zone = Some(zone);
        self
    }

    /// Ignored with a warning unless the anchor includes this edge; any
    /// other value is a compositor protocol error.
    pub fn exclusive_edge(mut self, edge: Edge4) -> Self {
        self.exclusive_edge = Some(edge);
        self
    }

    pub fn keyboard(mut self, mode: WidgetKeyboardInteractivity) -> Self {
        self.keyboard = mode;
        self
    }

    pub fn namespace(mut self, ns: impl Into<String>) -> Self {
        self.namespace = ns.into();
        self
    }

    pub(crate) fn requested_layer(&self) -> WidgetLayer {
        self.layer
    }

    pub(crate) fn requested_anchor(&self) -> Option<Placement9> {
        self.anchor
    }

    pub(crate) fn requested_margin(&self) -> Option<Edges<Pixels>> {
        self.margin
    }

    pub(crate) fn requested_exclusive_zone(&self) -> Option<Pixels> {
        self.exclusive_zone
    }

    pub(crate) fn requested_exclusive_edge(&self) -> Option<Edge4> {
        self.exclusive_edge
    }

    pub(crate) fn requested_keyboard(&self) -> WidgetKeyboardInteractivity {
        self.keyboard
    }

    pub(crate) fn requested_namespace(&self) -> &str {
        &self.namespace
    }
}

#[cfg(test)]
#[path = "spec.test.rs"]
mod spec_test;
