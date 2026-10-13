//! Removable shared canvas widget library. `ui::canvas` is the only public
//! surface; features mount it as a child element, nothing routes to it.
//! Removal = delete this directory and revert its single seam touch: the
//! `pub mod canvas;` line in src/ui/mod.rs.

mod cache;
mod element;
mod geometry;
mod paint;
mod scene;
mod state;
mod tool;

pub use element::{Canvas, CanvasElement};
pub use geometry::{CanvasTransform, segment_distance};
pub use scene::{
    CanvasDocument, CanvasLayer, CanvasShape, LayerId, ShapeId, ShapeStyle, StrokeStyle,
};
pub use state::{CanvasEvent, CanvasGrid, CanvasState, HIT_TOLERANCE, MAX_ZOOM, MIN_ZOOM};
pub use tool::{
    BrushConfig, BrushTool, BuiltinTool, CanvasPointerEvent, CanvasScrollEvent, CanvasTool,
    PanTool, SelectTool, ToolContext,
};
