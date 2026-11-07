use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

use super::history::ColumnAgg;
use super::paint::{
    amplitude_y, bar_bounds, envelope_bounds, nice_time_step, oscilloscope_line_points,
    time_gridline_xs,
};
use super::state::{WaveformMode, WaveformSamples, WaveformState};

const GRID_TARGET_LINES: usize = 6;
const PEAK_OPACITY: f32 = 0.45;
const RMS_OPACITY: f32 = 0.9;

/// Fills its parent; no hitbox, pointer handling, or a11y node. The mounting view
/// must `cx.observe(&state, |_, _, cx| cx.notify())` so refreshes repaint.
pub struct Waveform {
    id: ElementId,
    state: Entity<WaveformState>,
    accent: Option<Hsla>,
}

impl Waveform {
    pub fn new(id: impl Into<ElementId>, state: Entity<WaveformState>) -> Self {
        Self {
            id: id.into(),
            state,
            accent: None,
        }
    }

    /// Overrides the theme colorway for every lane and the spectrum bars. Defaults:
    /// mono lane and bars `theme.primary`, stereo lanes `theme.chart_1`/`theme.chart_2`.
    pub fn accent(mut self, color: impl Into<Hsla>) -> Self {
        self.accent = Some(color.into());
        self
    }
}

impl IntoElement for Waveform {
    type Element = WaveformElement;

    fn into_element(self) -> Self::Element {
        WaveformElement {
            id: self.id,
            state: self.state,
            accent: self.accent,
        }
    }
}

pub struct WaveformElement {
    id: ElementId,
    state: Entity<WaveformState>,
    accent: Option<Hsla>,
}

impl IntoElement for WaveformElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

fn hairline(
    origin: Point<Pixels>,
    length: Pixels,
    vertical: bool,
    thickness: Pixels,
    color: Hsla,
) -> PaintQuad {
    let bounds = if vertical {
        Bounds {
            origin,
            size: size(thickness, length),
        }
    } else {
        Bounds {
            origin,
            size: size(length, thickness),
        }
    };
    fill(bounds, color)
}

fn lane_grid(lane: Bounds<Pixels>, grid: Hsla, xs: &[Pixels], scale: f32, window: &mut Window) {
    let thickness = px(1. / scale);
    window.paint_quad(hairline(
        point(lane.origin.x, amplitude_y(0., lane)),
        lane.size.width,
        false,
        thickness,
        grid,
    ));
    for &x in xs {
        window.paint_quad(hairline(
            point(x, lane.origin.y),
            lane.size.height,
            true,
            thickness,
            grid,
        ));
    }
}

fn paint_lane_envelope(
    lane: Bounds<Pixels>,
    columns: usize,
    color: Hsla,
    scale: f32,
    cols: &[ColumnAgg],
    window: &mut Window,
) {
    for (ix, agg) in cols.iter().enumerate() {
        if let Some((peak, rms)) = envelope_bounds(agg, ix, columns, lane, scale) {
            window.paint_quad(fill(peak, color.opacity(PEAK_OPACITY)));
            window.paint_quad(fill(rms, color.opacity(RMS_OPACITY)));
        }
    }
}

fn snap_split(bounds: Bounds<Pixels>, scale: f32) -> Pixels {
    let mid = bounds.origin.y + bounds.size.height * 0.5;
    px((mid.as_f32() * scale).round() / scale)
}

fn stereo_lane_bounds(
    bounds: Bounds<Pixels>,
    channels: usize,
    scale: f32,
) -> (Bounds<Pixels>, Option<Bounds<Pixels>>) {
    if channels <= 1 {
        return (bounds, None);
    }
    let divider = px(1. / scale);
    let split = snap_split(bounds, scale);
    (
        Bounds {
            origin: bounds.origin,
            size: size(bounds.size.width, split - divider * 0.5 - bounds.origin.y),
        },
        Some(Bounds {
            origin: point(bounds.origin.x, split + divider * 0.5),
            size: size(bounds.size.width, bounds.bottom() - split - divider * 0.5),
        }),
    )
}

/// Gridline xs depend only on the left edge, width, window and scale, so they
/// hash to a key instead of being recomputed per lane per frame.
fn gridline_key(bounds: Bounds<Pixels>, window_secs: f32, scale: f32) -> u64 {
    let mut hasher = DefaultHasher::new();
    for bits in [
        bounds.origin.x.as_f32().to_bits(),
        bounds.size.width.as_f32().to_bits(),
        window_secs.to_bits(),
        scale.to_bits(),
    ] {
        bits.hash(&mut hasher);
    }
    hasher.finish()
}

/// The tessellation inputs of one lane's stroke: geometry, scale, the sample
/// window it is spread across, and the samples themselves.
fn line_path_key(lane: Bounds<Pixels>, scale: f32, window_samples: usize, samples: &[f32]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for bits in [
        lane.origin.x.as_f32().to_bits(),
        lane.origin.y.as_f32().to_bits(),
        lane.size.width.as_f32().to_bits(),
        lane.size.height.as_f32().to_bits(),
        scale.to_bits(),
    ] {
        bits.hash(&mut hasher);
    }
    window_samples.hash(&mut hasher);
    samples.len().hash(&mut hasher);
    for &sample in samples {
        sample.to_bits().hash(&mut hasher);
    }
    hasher.finish()
}

fn build_line_path(
    samples: &[f32],
    window_samples: usize,
    lane: Bounds<Pixels>,
    scale: f32,
) -> Option<Path<Pixels>> {
    let points = oscilloscope_line_points(samples, window_samples, lane, scale);
    if points.is_empty() {
        return None;
    }
    let mut builder = PathBuilder::stroke(px(1.));
    builder.move_to(points[0]);
    for &p in &points[1..] {
        builder.line_to(p);
    }
    builder.build().ok()
}

/// Per-element paint scratch held in window keyed state so it outlives the element
/// value; gridline xs and line paths rebuild only when their key changes.
#[derive(Default)]
struct PaintCaches {
    grid_key: Option<u64>,
    grid_xs: Vec<Pixels>,
    line_keys: Vec<Option<u64>>,
    line_paths: Vec<Option<Path<Pixels>>>,
}

impl PaintCaches {
    fn for_paint(key: impl Into<ElementId>, window: &mut Window, cx: &mut App) -> Entity<Self> {
        window.use_keyed_state(key, cx, |_, _| Self::default())
    }

    fn gridline_xs(&mut self, key: u64, rebuild: impl FnOnce() -> Vec<Pixels>) -> &[Pixels] {
        if self.grid_key != Some(key) {
            self.grid_xs = rebuild();
            self.grid_key = Some(key);
        }
        &self.grid_xs
    }

    fn line_path(
        &mut self,
        lane: usize,
        key: u64,
        build: impl FnOnce() -> Option<Path<Pixels>>,
    ) -> Option<Path<Pixels>> {
        if self.line_keys.len() <= lane {
            self.line_keys.resize(lane + 1, None);
            self.line_paths.resize(lane + 1, None);
        }
        if self.line_keys[lane] != Some(key) {
            #[cfg(test)]
            test_probe::add_line_path_build();
            self.line_paths[lane] = build();
            self.line_keys[lane] = Some(key);
        }
        self.line_paths[lane].clone()
    }
}

/// Thread-local work counters for tests: paints driven by one test thread are
/// deterministic; global atomics would race sibling tests.
#[cfg(test)]
pub(crate) mod test_probe {
    use std::cell::Cell;

    thread_local! {
        static LINE_PATH_BUILDS: Cell<u64> = const { Cell::new(0) };
    }

    pub(crate) fn add_line_path_build() {
        LINE_PATH_BUILDS.with(|c| c.set(c.get() + 1));
    }

    pub(crate) fn reset() {
        LINE_PATH_BUILDS.with(|c| c.set(0));
    }

    pub(crate) fn line_path_builds() -> u64 {
        LINE_PATH_BUILDS.with(Cell::get)
    }
}

impl Element for WaveformElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style {
            size: Size::full(),
            ..Default::default()
        };
        (window.request_layout(style, None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Window,
        _: &mut App,
    ) -> Self::PrepaintState {
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.state.update(cx, |state, cx| {
            window.with_content_mask(Some(ContentMask { bounds }), |window| {
                let scale = window.scale_factor();
                window.paint_quad(fill(bounds, cx.theme().background));
                let columns = ((bounds.size.width.as_f32() * scale).round() as usize).max(1);
                let caches = PaintCaches::for_paint((self.id.clone(), "caches"), window, cx);
                caches.update(cx, |caches, cx| {
                    let window_secs = state.window().as_secs_f32();
                    let theme = cx.theme();
                    let grid_color = theme.chart_grid;
                    let chart_1 = theme.chart_1;
                    let chart_2 = theme.chart_2;
                    let mono_color = self.accent.unwrap_or(theme.primary);
                    let step = nice_time_step(window_secs, GRID_TARGET_LINES);
                    let grid_key = gridline_key(bounds, window_secs, scale);
                    let xs = caches.gridline_xs(grid_key, || {
                        time_gridline_xs(bounds, window_secs, step, scale)
                    });

                    match state.mode() {
                        WaveformMode::Oscilloscope => {
                            let channels = state.channels();
                            let (top, bottom) = stereo_lane_bounds(bounds, channels, scale);
                            let lanes = [Some(top), bottom];
                            for lane in lanes.into_iter().flatten() {
                                lane_grid(lane, grid_color, xs, scale, window);
                            }
                            for (ch, lane) in lanes.into_iter().flatten().enumerate() {
                                let color = match self.accent {
                                    Some(accent) => accent,
                                    None if channels == 2 && ch == 0 => chart_1,
                                    None if channels == 2 => chart_2,
                                    None => mono_color,
                                };
                                let window_samples = state.window_samples();
                                match state.channel_view(ch, columns) {
                                    WaveformSamples::Envelope(cols) => {
                                        paint_lane_envelope(
                                            lane, columns, color, scale, cols, window,
                                        );
                                    }
                                    WaveformSamples::Line(samples) => {
                                        let key =
                                            line_path_key(lane, scale, window_samples, samples);
                                        if let Some(path) = caches.line_path(ch, key, || {
                                            build_line_path(samples, window_samples, lane, scale)
                                        }) {
                                            window.paint_path(path, color);
                                        }
                                    }
                                }
                            }
                            if channels == 2 {
                                let divider = px(1. / scale);
                                let split = snap_split(bounds, scale) - divider * 0.5;
                                window.paint_quad(hairline(
                                    point(bounds.origin.x, split),
                                    bounds.size.width,
                                    false,
                                    divider,
                                    grid_color,
                                ));
                            }
                        }
                        WaveformMode::Spectrum => {
                            lane_grid(bounds, grid_color, xs, scale, window);
                            let baseline = bounds.bottom() - px(1. / scale);
                            window.paint_quad(hairline(
                                point(bounds.origin.x, baseline),
                                bounds.size.width,
                                false,
                                px(1. / scale),
                                grid_color,
                            ));
                            if let Some(frame) = state.spectrum() {
                                for (ix, &height) in frame.bars().iter().enumerate() {
                                    window.paint_quad(fill(
                                        bar_bounds(height, ix, frame.bars().len(), bounds, scale),
                                        mono_color,
                                    ));
                                }
                            }
                        }
                    }
                });
            });
        });
    }
}

#[cfg(test)]
#[path = "element.test.rs"]
mod element_test;
