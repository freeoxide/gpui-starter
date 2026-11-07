//! Removable shared canvas widget library. `ui::canvas` is the only public
//! surface; features mount it as a child element, nothing routes to it.
//! Removal = delete this directory and revert its single seam touch: the
//! `pub mod canvas;` line in src/ui/mod.rs.

#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};

mod cache;
mod element;
mod geometry;
mod paint;
mod scene;
mod state;
mod tool;

#[cfg(test)]
static LAST_PAINT_US: AtomicU64 = AtomicU64::new(0);

/// Duration of the most recent `CanvasElement` paint, in microseconds.
#[cfg(test)]
pub(crate) fn last_paint_us() -> u64 {
    LAST_PAINT_US.load(Ordering::Relaxed)
}

#[cfg(test)]
pub(crate) fn store_last_paint_us(elapsed_us: u64) {
    LAST_PAINT_US.store(elapsed_us, Ordering::Relaxed);
}

pub use element::{Canvas, CanvasElement};
pub use geometry::{CanvasTransform, segment_distance};
pub use scene::{
    CanvasDocument, CanvasLayer, CanvasShape, LayerId, ShapeId, ShapeStyle, StrokeStyle,
    measure_text, rgba_to_render,
};
pub use state::{CanvasEvent, CanvasGrid, CanvasState, HIT_TOLERANCE, MAX_ZOOM, MIN_ZOOM};
pub use tool::{
    BrushConfig, BrushTool, BuiltinTool, CanvasPointerEvent, CanvasScrollEvent, CanvasTool,
    PanTool, SelectTool, ShapeConfig, ShapeKind, ShapeTool, ToolContext,
};

/// Thread-local work counters for perf tests: paints driven by one test
/// thread are deterministic; global atomics would race sibling tests.
#[cfg(test)]
pub(crate) mod perf_probe {
    use std::cell::{Cell, RefCell};

    use gpui_kit::{Path, Pixels};

    thread_local! {
        pub(crate) static PATH_BUILDS: Cell<u64> = const { Cell::new(0) };
        pub(crate) static KEY_HASHES: Cell<u64> = const { Cell::new(0) };
        pub(crate) static TEXT_SHAPINGS: Cell<u64> = const { Cell::new(0) };
        pub(crate) static DOT_PAINTS: Cell<u64> = const { Cell::new(0) };
        // Element state is only reachable mid-paint, so the newest built
        // path is mirrored here for tests to inspect afterwards.
        pub(crate) static LAST_PATH_BUILD: RefCell<Option<Path<Pixels>>> =
            const { RefCell::new(None) };
    }

    pub(crate) fn add_path_build() {
        PATH_BUILDS.with(|c| c.set(c.get() + 1));
    }

    pub(crate) fn set_last_path_build(path: Option<&Path<Pixels>>) {
        LAST_PATH_BUILD.with(|p| *p.borrow_mut() = path.cloned());
    }

    pub(crate) fn last_path_build() -> Option<Path<Pixels>> {
        LAST_PATH_BUILD.with(|p| p.borrow().clone())
    }

    pub(crate) fn add_key_hash() {
        KEY_HASHES.with(|c| c.set(c.get() + 1));
    }

    pub(crate) fn add_text_shaping() {
        TEXT_SHAPINGS.with(|c| c.set(c.get() + 1));
    }

    pub(crate) fn add_dot_paint() {
        DOT_PAINTS.with(|c| c.set(c.get() + 1));
    }

    pub(crate) fn reset() {
        PATH_BUILDS.with(|c| c.set(0));
        KEY_HASHES.with(|c| c.set(0));
        TEXT_SHAPINGS.with(|c| c.set(0));
        DOT_PAINTS.with(|c| c.set(0));
        LAST_PATH_BUILD.with(|p| *p.borrow_mut() = None);
    }

    pub(crate) fn snapshot() -> [u64; 4] {
        [
            PATH_BUILDS.with(Cell::get),
            KEY_HASHES.with(Cell::get),
            TEXT_SHAPINGS.with(Cell::get),
            DOT_PAINTS.with(Cell::get),
        ]
    }
}
