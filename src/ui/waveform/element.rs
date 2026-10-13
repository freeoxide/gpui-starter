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

/// Mounts the waveform. Fills its parent. Non-interactive: no hitbox, no
/// pointer handling, no a11y node in v1. The mounting view must
/// `cx.observe(&state, |_, _, cx| cx.notify())` so refreshes repaint.
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

    /// Overrides the theme colorway for every lane and the spectrum bars.
    /// Defaults: mono lane and bars `theme.primary`, stereo lanes
    /// `theme.chart_1`/`theme.chart_2`.
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

fn lane_grid(
    lane: Bounds<Pixels>,
    bounds: Bounds<Pixels>,
    window_secs: f32,
    scale: f32,
    window: &mut Window,
    cx: &App,
) {
    let grid = cx.theme().chart_grid;
    let thickness = px(1. / scale);
    window.paint_quad(hairline(
        point(lane.origin.x, amplitude_y(0., lane)),
        lane.size.width,
        false,
        thickness,
        grid,
    ));
    let step = nice_time_step(window_secs, GRID_TARGET_LINES);
    for x in time_gridline_xs(bounds, window_secs, step, scale) {
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

fn paint_lane_line(
    lane: Bounds<Pixels>,
    window_samples: usize,
    color: Hsla,
    scale: f32,
    samples: &[f32],
    window: &mut Window,
) {
    let points = oscilloscope_line_points(samples, window_samples, lane, scale);
    if points.is_empty() {
        return;
    }
    let mut builder = PathBuilder::stroke(px(1.));
    builder.move_to(points[0]);
    for &p in &points[1..] {
        builder.line_to(p);
    }
    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}

fn snap_split(bounds: Bounds<Pixels>, scale: f32) -> Pixels {
    let mid = bounds.origin.y + bounds.size.height * 0.5;
    px((mid.as_f32() * scale).round() / scale)
}

fn stereo_lane_bounds(bounds: Bounds<Pixels>, channels: usize, scale: f32) -> Vec<Bounds<Pixels>> {
    if channels <= 1 {
        return vec![bounds];
    }
    let divider = px(1. / scale);
    let split = snap_split(bounds, scale);
    vec![
        Bounds {
            origin: bounds.origin,
            size: size(bounds.size.width, split - divider * 0.5 - bounds.origin.y),
        },
        Bounds {
            origin: point(bounds.origin.x, split + divider * 0.5),
            size: size(bounds.size.width, bounds.bottom() - split - divider * 0.5),
        },
    ]
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
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            let scale = window.scale_factor();
            window.paint_quad(fill(bounds, cx.theme().background));
            let columns = ((bounds.size.width.as_f32() * scale).round() as usize).max(1);
            let state = self.state.read(cx);
            let window_secs = state.window().as_secs_f32();
            let mono_color = self.accent.unwrap_or(cx.theme().primary);

            match state.mode() {
                WaveformMode::Oscilloscope => {
                    let channels = state.channels();
                    let lanes = stereo_lane_bounds(bounds, channels, scale);
                    for (ch, lane) in lanes.iter().enumerate() {
                        lane_grid(*lane, bounds, window_secs, scale, window, cx);
                        let color = match self.accent {
                            Some(accent) => accent,
                            None if channels == 2 && ch == 0 => cx.theme().chart_1,
                            None if channels == 2 => cx.theme().chart_2,
                            None => mono_color,
                        };
                        match state.channel_view(ch, columns) {
                            WaveformSamples::Envelope(cols) => {
                                paint_lane_envelope(*lane, columns, color, scale, &cols, window);
                            }
                            WaveformSamples::Line(samples) => paint_lane_line(
                                *lane,
                                state.window_samples(),
                                color,
                                scale,
                                &samples,
                                window,
                            ),
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
                            cx.theme().chart_grid,
                        ));
                    }
                }
                WaveformMode::Spectrum => {
                    lane_grid(bounds, bounds, window_secs, scale, window, cx);
                    let baseline = bounds.bottom() - px(1. / scale);
                    window.paint_quad(hairline(
                        point(bounds.origin.x, baseline),
                        bounds.size.width,
                        false,
                        px(1. / scale),
                        cx.theme().chart_grid,
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
    }
}

#[cfg(test)]
#[path = "element.test.rs"]
mod element_test;
