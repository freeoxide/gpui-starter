//! Hilbert-curve scene, ported from the `fps_monitor` example: a port of
//! three.js' `webgl_lines_colors` demo, rotating over a black background with
//! its curve count adjustable. The example's `gpui-fps` HUD overlay is not
//! ported: gpui-fps is not an app dependency.

use gpui_kit::base::ElementExt as _;
use gpui_kit::component::{ActiveTheme as _, Sizable as _, button::Button, h_flex, v_flex};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

/// Matches the original demo: a one-iteration Hilbert curve of 64 control
/// points, resampled at six points each.
const HILBERT_SIZE: f32 = 200.;
const HILBERT_ITERATIONS: u32 = 1;
const SUBDIVISIONS: usize = 6;

/// How far the curve reaches from the origin: each recursion centers a
/// sub-cell on a corner and extends half a sub-cell out, 1.5x the half size.
const HILBERT_EXTENT: f32 = HILBERT_SIZE / 2. * 1.5;

/// Fraction of a grid cell the curve may fill; covers the spline's ~8%
/// overshoot and the perspective divide's ~1.32 near-face magnification.
const CELL_FILL: f32 = 0.68;

/// Points per drawn path. A path carries one color, so the gradient is built
/// from short runs; GPUI has no per-vertex colors.
const SEGMENT_POINTS: usize = 6;

const CURVE_STEP: usize = 1;
const MAX_CURVES: usize = 48;

/// Distance from the eye to the origin, for the perspective divide.
const EYE_DISTANCE: f32 = 620.;
/// How quickly the view catches up with the cursor.
const CURSOR_EASING: f32 = 0.08;
/// The spin the original took from wall-clock seconds, advanced per frame
/// instead: std::time::Instant has no wasm32 implementation.
const FRAMES_PER_SECOND: f32 = 60.;

#[derive(Clone, Copy, Default)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

struct HilbertScene {
    /// The spline, shared by every curve on screen.
    points: Vec<Vec3>,
    /// Vertex colors for the demo's three schemes, indexed by scheme.
    palettes: [Vec<Hsla>; 3],
    curves: usize,
    /// Where the view is being pulled to, and where it currently is.
    cursor_tilt: Point<f32>,
    tilt: Point<f32>,
    /// Stand-in clock: render runs once per animation frame.
    frame: usize,
    /// The canvas frame's resolved bounds, for cursor normalization.
    frame_bounds: Bounds<Pixels>,
}

impl HilbertScene {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self::new())
    }

    fn new() -> Self {
        let points = hilbert_spline();
        let palettes = [
            color_scheme(&points, Scheme::CyanByX),
            color_scheme(&points, Scheme::MagentaByY),
            color_scheme(&points, Scheme::Rainbow),
        ];

        Self {
            points,
            palettes,
            // The original lays out six curves in a 2x3 grid.
            curves: 6,
            cursor_tilt: point(0., 0.),
            tilt: point(0., 0.),
            frame: 0,
            frame_bounds: Bounds::default(),
        }
    }

    fn render_curves(&self) -> impl IntoElement {
        let points = self.points.clone();
        let palettes = self.palettes.clone();
        let curves = self.curves;
        let tilt = self.tilt;
        let spin = self.frame as f32 / FRAMES_PER_SECOND * 0.35;

        canvas(
            |_, _, _| (),
            move |bounds: Bounds<Pixels>, _, window: &mut Window, _| {
                // Lay the curves out on the squarest grid that fits them.
                let columns = (curves as f32).sqrt().ceil().max(1.) as usize;
                let rows = curves.div_ceil(columns);
                let cell = size(
                    bounds.size.width / columns as f32,
                    bounds.size.height / rows as f32,
                );
                let scale =
                    (cell.width.min(cell.height).as_f32() / (HILBERT_EXTENT * 2.)) * CELL_FILL;

                for index in 0..curves {
                    let column = index % columns;
                    let row = index / columns;
                    let center = point(
                        (bounds.origin.x + cell.width * (column as f32 + 0.5)).as_f32(),
                        (bounds.origin.y + cell.height * (row as f32 + 0.5)).as_f32(),
                    );
                    // Alternating spin direction, as in the original.
                    let direction = if index % 2 == 0 { 1. } else { -1. };
                    let yaw = spin * direction + index as f32 * 0.4 + tilt.x;
                    let pitch = tilt.y;

                    let projected: Vec<Point<f32>> = points
                        .iter()
                        .map(|vertex| project(*vertex, yaw, pitch, center, scale))
                        .collect();
                    paint_gradient_curve(window, &projected, &palettes[index % palettes.len()]);
                }
            },
        )
        .absolute()
        .size_full()
    }

    fn render_load_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_2()
            .items_center()
            .child(
                Button::new("hilbert-fewer")
                    .outline()
                    .small()
                    .label("− load")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.curves = this.curves.saturating_sub(CURVE_STEP).max(1);
                        cx.notify();
                    })),
            )
            .child(format!("{} curves", self.curves))
            .child(
                Button::new("hilbert-more")
                    .outline()
                    .small()
                    .label("+ load")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.curves = (this.curves + CURVE_STEP).min(MAX_CURVES);
                        cx.notify();
                    })),
            )
    }
}

impl Render for HilbertScene {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The scene has to keep asking for frames to stay animated; each
        // request notifies this view, so render runs once per frame.
        window.request_animation_frame();

        // Ease toward the cursor rather than snapping, the way the original
        // demo drifts its camera.
        self.tilt.x += (self.cursor_tilt.x - self.tilt.x) * CURSOR_EASING;
        self.tilt.y += (self.cursor_tilt.y - self.tilt.y) * CURSOR_EASING;
        // Render runs once per animation frame, so this counts them off.
        self.frame += 1;

        let scene = cx.entity();
        v_flex()
            .w_full()
            .gap_4()
            .p_4()
            .child(self.render_load_controls(cx))
            .child(
                div()
                    .id("hilbert-canvas")
                    .relative()
                    .w_full()
                    .h(rems(25.))
                    // The palette is authored against the demo's black
                    // backdrop, so the backdrop is scene data, not chrome.
                    .bg(black())
                    .overflow_hidden()
                    .rounded(cx.theme().radius_lg)
                    .on_prepaint(move |bounds, _, cx| {
                        scene.update(cx, |this, _| this.frame_bounds = bounds);
                    })
                    .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _, _| {
                        let bounds = this.frame_bounds;
                        // A zero-sized frame would divide to infinity, and
                        // the easing would keep the tilt non-finite forever.
                        if bounds.size.width <= px(0.) || bounds.size.height <= px(0.) {
                            return;
                        }
                        // No notify: the scene already redraws every frame.
                        this.cursor_tilt = point(
                            ((event.position.x - bounds.origin.x) / bounds.size.width - 0.5) * 2.4,
                            ((event.position.y - bounds.origin.y) / bounds.size.height - 0.5) * 1.2,
                        );
                    }))
                    .child(self.render_curves()),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(
                        "Move the cursor over the canvas to tilt the view. The load buttons \
                        change how many curves each frame paints.",
                    ),
            )
    }
}

/// Paints one curve as short runs of constant color, approximating the
/// per-vertex gradient of the original.
fn paint_gradient_curve(window: &mut Window, projected: &[Point<f32>], colors: &[Hsla]) {
    let mut start = 0;
    while start + 1 < projected.len() {
        let end = (start + SEGMENT_POINTS).min(projected.len() - 1);
        let run = &projected[start..=end];
        // The tessellator asserts on non-finite coordinates, so a run that
        // picked one up is dropped rather than aborting the process.
        if run.iter().any(|v| !v.x.is_finite() || !v.y.is_finite()) {
            start = end;
            continue;
        }

        let mut path = PathBuilder::stroke(px(1.));
        path.move_to(point(px(run[0].x), px(run[0].y)));
        for vertex in &run[1..] {
            path.line_to(point(px(vertex.x), px(vertex.y)));
        }
        if let Ok(path) = path.build() {
            window.paint_path(path, colors[(start + end) / 2]);
        }
        // Share the boundary vertex so runs join without a gap.
        start = end;
    }
}

/// Rotates around Y then X and applies a perspective divide.
fn project(vertex: Vec3, yaw: f32, pitch: f32, center: Point<f32>, scale: f32) -> Point<f32> {
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    let x = vertex.x * cos_yaw + vertex.z * sin_yaw;
    let z = vertex.z * cos_yaw - vertex.x * sin_yaw;

    let (sin_pitch, cos_pitch) = pitch.sin_cos();
    let y = vertex.y * cos_pitch - z * sin_pitch;
    let z = z * cos_pitch + vertex.y * sin_pitch;

    // Guard the divide: a vertex level with the eye would blow up.
    let depth = (EYE_DISTANCE + z).max(1.);
    let perspective = EYE_DISTANCE / depth;
    point(
        center.x + x * perspective * scale,
        center.y + y * perspective * scale,
    )
}

#[derive(Clone, Copy)]
enum Scheme {
    CyanByX,
    MagentaByY,
    Rainbow,
}

/// The three vertex color schemes from the original demo.
fn color_scheme(points: &[Vec3], scheme: Scheme) -> Vec<Hsla> {
    points
        .iter()
        .enumerate()
        .map(|(index, vertex)| match scheme {
            Scheme::CyanByX => hsla(0.6, 1., (-vertex.x / 200.).max(0.) + 0.5, 1.),
            Scheme::MagentaByY => hsla(0.9, 1., (-vertex.y / 200.).max(0.) + 0.5, 1.),
            Scheme::Rainbow => hsla(index as f32 / points.len() as f32, 1., 0.5, 1.),
        })
        .collect()
}

/// A Hilbert curve resampled through a Catmull-Rom spline, matching the
/// original demo's geometry.
fn hilbert_spline() -> Vec<Vec3> {
    let mut control = Vec::new();
    hilbert3d(
        Vec3::default(),
        HILBERT_SIZE,
        HILBERT_ITERATIONS,
        [0, 1, 2, 3, 4, 5, 6, 7],
        &mut control,
    );

    let samples = control.len() * SUBDIVISIONS;
    (0..=samples)
        .map(|index| catmull_rom(&control, index as f32 / samples as f32))
        .collect()
}

/// Port of three.js' `hilbert3D`.
fn hilbert3d(center: Vec3, size: f32, iterations: u32, v: [usize; 8], out: &mut Vec<Vec3>) {
    let half = size / 2.;
    let corners = [
        Vec3::new(center.x - half, center.y + half, center.z - half),
        Vec3::new(center.x - half, center.y + half, center.z + half),
        Vec3::new(center.x - half, center.y - half, center.z + half),
        Vec3::new(center.x - half, center.y - half, center.z - half),
        Vec3::new(center.x + half, center.y - half, center.z - half),
        Vec3::new(center.x + half, center.y - half, center.z + half),
        Vec3::new(center.x + half, center.y + half, center.z + half),
        Vec3::new(center.x + half, center.y + half, center.z - half),
    ];
    let vec = v.map(|index| corners[index]);

    let Some(iterations) = iterations.checked_sub(1) else {
        out.extend_from_slice(&vec);
        return;
    };

    let [v0, v1, v2, v3, v4, v5, v6, v7] = v;
    let children = [
        [v0, v3, v4, v7, v6, v5, v2, v1],
        [v0, v7, v6, v1, v2, v5, v4, v3],
        [v0, v7, v6, v1, v2, v5, v4, v3],
        [v2, v3, v0, v1, v6, v7, v4, v5],
        [v2, v3, v0, v1, v6, v7, v4, v5],
        [v4, v3, v2, v5, v6, v1, v0, v7],
        [v4, v3, v2, v5, v6, v1, v0, v7],
        [v6, v5, v2, v1, v0, v3, v4, v7],
    ];
    for (child, order) in vec.iter().zip(children) {
        hilbert3d(*child, half, iterations, order, out);
    }
}

/// Centripetal Catmull-Rom over the control polygon, endpoints clamped.
/// Centripetal, not uniform: uniform overshoots a Hilbert curve's corners.
fn catmull_rom(control: &[Vec3], t: f32) -> Vec3 {
    if control.is_empty() {
        return Vec3::default();
    }
    if control.len() == 1 {
        return control[0];
    }

    let spans = control.len() - 1;
    let scaled = t.clamp(0., 1.) * spans as f32;
    let span = (scaled as usize).min(spans - 1);
    let local = scaled - span as f32;

    let at = |index: isize| control[(index.clamp(0, spans as isize)) as usize];
    let (p0, p1, p2, p3) = (
        at(span as isize - 1),
        at(span as isize),
        at(span as isize + 1),
        at(span as isize + 2),
    );

    // Knots spaced by sqrt(chord). Coincident control points would collapse a
    // span to zero width, so each step is floored well above f32::EPSILON.
    let knot = |from: Vec3, to: Vec3| distance3(from, to).sqrt().max(1e-3);
    let t0 = 0.;
    let t1 = t0 + knot(p0, p1);
    let t2 = t1 + knot(p1, p2);
    let t3 = t2 + knot(p2, p3);
    let t = t1 + (t2 - t1) * local;

    // Barry-Goldman pyramid: three lerps, then two, then one.
    let a1 = lerp3(p0, p1, (t - t0) / (t1 - t0));
    let a2 = lerp3(p1, p2, (t - t1) / (t2 - t1));
    let a3 = lerp3(p2, p3, (t - t2) / (t3 - t2));
    let b1 = lerp3(a1, a2, (t - t0) / (t2 - t0));
    let b2 = lerp3(a2, a3, (t - t1) / (t3 - t1));
    lerp3(b1, b2, (t - t1) / (t2 - t1))
}

fn distance3(a: Vec3, b: Vec3) -> f32 {
    let (dx, dy, dz) = (a.x - b.x, a.y - b.y, a.z - b.z);
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn lerp3(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    Vec3::new(
        a.x + (b.x - a.x) * t,
        a.y + (b.y - a.y) * t,
        a.z + (b.z - a.z) * t,
    )
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "hilbert-curves",
        "Hilbert Curves",
        "The fps_monitor example's rotating spline scene; its gpui-fps HUD is not an app dependency.",
        HilbertScene::view(window, cx),
    ));
}
