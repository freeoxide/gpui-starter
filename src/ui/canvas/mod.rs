//! Removable shared canvas widget library. `ui::canvas` is the only public
//! surface; features mount it as a child element, nothing routes to it.
//! Removal = delete this directory and revert its single seam touch: the
//! `pub mod canvas;` line in src/ui/mod.rs.

use std::sync::atomic::{AtomicU64, Ordering};

mod cache;
mod element;
mod geometry;
mod paint;
mod scene;
mod state;
mod tool;

static LAST_PAINT_US: AtomicU64 = AtomicU64::new(0);

/// Duration of the most recent `CanvasElement` paint, in microseconds.
#[cfg(test)]
pub(crate) fn last_paint_us() -> u64 {
    LAST_PAINT_US.load(Ordering::Relaxed)
}

pub(crate) fn store_last_paint_us(elapsed_us: u64) {
    LAST_PAINT_US.store(elapsed_us, Ordering::Relaxed);
}

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

/// Thread-local work counters for perf tests: paints driven by one test
/// thread are deterministic; global atomics would race sibling tests.
#[cfg(test)]
pub(crate) mod perf_probe {
    use std::cell::Cell;

    thread_local! {
        pub(crate) static PATH_BUILDS: Cell<u64> = const { Cell::new(0) };
        pub(crate) static KEY_HASHES: Cell<u64> = const { Cell::new(0) };
        pub(crate) static TEXT_SHAPINGS: Cell<u64> = const { Cell::new(0) };
    }

    pub(crate) fn add_path_build() {
        PATH_BUILDS.with(|c| c.set(c.get() + 1));
    }

    pub(crate) fn add_key_hash() {
        KEY_HASHES.with(|c| c.set(c.get() + 1));
    }

    pub(crate) fn add_text_shaping() {
        TEXT_SHAPINGS.with(|c| c.set(c.get() + 1));
    }

    pub(crate) fn reset() {
        PATH_BUILDS.with(|c| c.set(0));
        KEY_HASHES.with(|c| c.set(0));
        TEXT_SHAPINGS.with(|c| c.set(0));
    }

    pub(crate) fn snapshot() -> [u64; 3] {
        [
            PATH_BUILDS.with(Cell::get),
            KEY_HASHES.with(Cell::get),
            TEXT_SHAPINGS.with(Cell::get),
        ]
    }
}
