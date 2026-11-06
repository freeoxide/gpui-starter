// Device-pixel raster tests: the canvas is drawn through the real headless
// wgpu renderer and the readback is asserted pixel by pixel, herdr-style.
use std::sync::Arc;

use gpui_kit::{
    AppContext as _, Bounds, Context, Entity, HeadlessAppContext, IntoElement, Pixels,
    PlatformTextSystem, Render, Size, Window, WindowHandle, point, px, rgb, size,
};

use super::paint::grid_step;
use super::{Canvas, CanvasGrid, CanvasShape, CanvasState, ShapeStyle};

struct RasterHost {
    state: Entity<CanvasState>,
}

impl RasterHost {
    fn new(state: Entity<CanvasState>, cx: &mut Context<Self>) -> Self {
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        Self { state }
    }
}

impl Render for RasterHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Canvas::new("raster-canvas", self.state.clone())
    }
}

thread_local! {
    // One platform per thread: each current_platform(true) call leaks its
    // worker threads on Linux, so every raster window shares one text system.
    static TEXT_SYSTEM: Arc<dyn PlatformTextSystem> =
        gpui_kit::platform::current_platform(true).text_system();
}

pub(crate) fn headless_text_system() -> Arc<dyn PlatformTextSystem> {
    TEXT_SYSTEM.with(|system| system.clone())
}

/// True (with a note) when this host has no headless wgpu adapter, e.g. the
/// GPU-less CI runners; raster tests skip rather than fail there.
pub(crate) fn skip_without_headless_gpu() -> bool {
    let available = matches!(gpui_kit::platform::current_headless_renderer(), Ok(Some(_)));
    if !available {
        eprintln!("skipping: no headless wgpu adapter on this host");
    }
    !available
}

pub(crate) struct RasterCanvas {
    cx: HeadlessAppContext,
    state: Entity<CanvasState>,
    handle: WindowHandle<RasterHost>,
}

impl RasterCanvas {
    /// Opens a `viewport`-sized canvas window at `scale_factor` device pixels
    /// per logical pixel, seeded before its first draw. The headless platform
    /// always sizes its target at 2x, so a 1.0 pass renders into the window's
    /// top-left quadrant on a true 1x device grid.
    pub(crate) fn open(
        viewport: Size<Pixels>,
        scale_factor: f32,
        seed: impl FnOnce(&mut CanvasState, &mut Context<CanvasState>),
    ) -> Self {
        let mut cx = HeadlessAppContext::with_platform(
            headless_text_system(),
            Arc::new(()),
            gpui_kit::platform::current_headless_renderer,
        );
        cx.update(|cx| gpui_kit::component::theme::init(cx));
        let state = cx.new(|_| CanvasState::new());
        state.update(&mut cx, seed);
        let handle = cx
            .open_window(viewport, {
                let state = state.clone();
                |_, cx| cx.new(|cx| RasterHost::new(state, cx))
            })
            .unwrap();
        cx.update_window(handle.into(), |_, window, _| {
            window.set_scale_factor(scale_factor)
        })
        .unwrap();
        Self { cx, state, handle }
    }

    pub(crate) fn edit(&mut self, edit: impl FnOnce(&mut CanvasState, &mut Context<CanvasState>)) {
        self.state.update(&mut self.cx, edit);
    }

    /// Draws one frame and returns its device pixels.
    pub(crate) fn draw(&mut self) -> image::RgbaImage {
        self.cx
            .update_window(self.handle.clone().into(), |_, window, cx| {
                window.draw(cx).clear(cx);
                window
                    .render_to_image()
                    .expect("the adapter probe succeeded, so a renderer exists")
            })
            .unwrap()
    }
}

fn channel_diff(a: [u8; 4], b: [u8; 4]) -> u8 {
    a.iter()
        .zip(b)
        .map(|(x, y)| x.abs_diff(y))
        .max()
        .unwrap_or(0)
}

const FILL: [u8; 4] = [255, 0, 0, 255];

fn is_fill(pixel: [u8; 4]) -> bool {
    channel_diff(pixel, FILL) <= 40
}

fn pixel_at(image: &image::RgbaImage, along_x: bool, fixed: u32, at: u32) -> [u8; 4] {
    if along_x {
        image.get_pixel(at, fixed).0
    } else {
        image.get_pixel(fixed, at).0
    }
}

/// Runs of the first `len` positions whose pixel differs from `bg`, as
/// (start, end) ranges.
fn line_runs(
    image: &image::RgbaImage,
    along_x: bool,
    fixed: u32,
    len: u32,
    bg: [u8; 4],
) -> Vec<(u32, u32)> {
    let mut runs = Vec::new();
    let mut start: Option<u32> = None;
    for at in 0..len {
        if channel_diff(pixel_at(image, along_x, fixed, at), bg) >= 6 {
            start.get_or_insert(at);
        } else if let Some(begin) = start.take() {
            runs.push((begin, at));
        }
    }
    if let Some(begin) = start.take() {
        runs.push((begin, len));
    }
    runs
}

/// The solid interior of a fill shape along the first `len` positions of one
/// raster line. Anti-aliased edge pixels fall between fill and background
/// and drop out of the runs.
fn fill_runs(image: &image::RgbaImage, along_x: bool, fixed: u32, len: u32) -> Vec<(u32, u32)> {
    (0..len)
        .filter(|&at| is_fill(pixel_at(image, along_x, fixed, at)))
        .collect::<Vec<_>>()
        .split(|a, b| *b != *a + 1)
        .filter(|run| run.len() >= 3)
        .map(|run| (run[0], run[run.len() - 1] + 1))
        .collect()
}

#[test]
fn ellipse_fill_hugs_its_doc_bounds_across_zooms_and_scales() {
    if skip_without_headless_gpu() {
        return;
    }
    for (zoom, scale) in [(1.0f32, 2.0f32), (2., 2.), (1.5, 2.), (0.5, 2.), (1., 1.)] {
        // 240x160 keeps the whole ellipse on screen at every zoom below.
        let mut canvas = RasterCanvas::open(size(px(240.), px(160.)), scale, |s, cx| {
            s.set_grid(
                CanvasGrid {
                    visible: false,
                    ..Default::default()
                },
                cx,
            );
            let layer = s.add_layer(cx);
            s.add_shape(
                layer,
                CanvasShape::ellipse(
                    Bounds {
                        origin: point(px(30.), px(15.)),
                        size: size(px(80.), px(40.)),
                    },
                    ShapeStyle::filled(rgb(0xff_00_00).into()),
                ),
                cx,
            )
            .unwrap();
        });
        canvas.edit(|s, cx| s.set_zoom(zoom, cx));
        let image = canvas.draw();

        // Doc ellipse: center (70, 35), radii 40 x 20, pan zero.
        let (cx_d, cy_d) = ((70. * zoom * scale) as i32, (35. * zoom * scale) as i32);
        let (rx_d, ry_d) = ((40. * zoom * scale) as i32, (20. * zoom * scale) as i32);
        let bg = image.get_pixel(2, 2).0;
        assert!(
            channel_diff(bg, FILL) > 100,
            "background must differ from fill"
        );
        assert!(
            channel_diff(image.get_pixel(cx_d as u32, cy_d as u32).0, FILL) <= 12,
            "zoom {zoom} scale {scale}: ellipse center is not the fill color"
        );

        // One solid run through each axis: the extents pin the doc bounds and
        // a seam between the four bezier segments would split a run.
        for (along_x, center, radius) in [(true, cx_d, rx_d), (false, cy_d, ry_d)] {
            let fixed = (if along_x { cy_d } else { cx_d }) as u32;
            let len = if along_x {
                image.width()
            } else {
                image.height()
            };
            let runs = fill_runs(&image, along_x, fixed, len);
            assert_eq!(
                runs.len(),
                1,
                "zoom {zoom} scale {scale}: the {} through the center must be one solid \
                 run, got {runs:?}",
                if along_x { "row" } else { "column" }
            );
            let (start, end) = runs[0];
            assert!(
                ((center - radius - 3)..=(center - radius + 3)).contains(&(start as i32)),
                "zoom {zoom} scale {scale}: run starts at {start}, expected ~{}",
                center - radius
            );
            assert!(
                ((center + radius - 3)..=(center + radius + 3)).contains(&(end as i32 - 1)),
                "zoom {zoom} scale {scale}: run ends at {end}, expected ~{}",
                center + radius
            );
        }

        // A wrong kappa bends the diagonals while the axis extremes stay
        // exact, so scan each quadrant's 45-degree ray for the fill boundary.
        for (sx, sy) in [(1.0f32, 1.0f32), (1., -1.), (-1., 1.), (-1., -1.)] {
            let (dx, dy) = (rx_d as f32 * 0.707_106_78, ry_d as f32 * 0.707_106_78);
            let ray_len = (dx * dx + dy * dy).sqrt();
            let mut last_inside = 0.;
            let mut u = 0.6f32;
            while u <= 1.3 {
                let x = (cx_d as f32 + sx * u * dx)
                    .round()
                    .clamp(0., image.width() as f32 - 1.) as u32;
                let y = (cy_d as f32 + sy * u * dy)
                    .round()
                    .clamp(0., image.height() as f32 - 1.) as u32;
                if is_fill(image.get_pixel(x, y).0) {
                    last_inside = u;
                }
                u += 0.5 / ray_len;
            }
            let error_px = (1. - last_inside).abs() * ray_len;
            assert!(
                error_px <= 2.5,
                "zoom {zoom} scale {scale} quadrant ({sx},{sy}): fill boundary {error_px:.1}px \
                 from the true ellipse on the diagonal"
            );
        }
    }
}

#[test]
fn grid_lines_stay_crisp_and_evenly_spaced_across_zooms_and_pan() {
    if skip_without_headless_gpu() {
        return;
    }
    let (viewport_w, viewport_h) = (160i32, 96i32);
    for (zoom, scale) in [(1.0f32, 2.0f32), (2., 2.), (1.5, 2.), (0.5, 2.), (1., 1.)] {
        let mut canvas = RasterCanvas::open(
            size(px(viewport_w as f32), px(viewport_h as f32)),
            scale,
            |_| {},
        );
        canvas.edit(|s, cx| s.set_zoom(zoom, cx));
        let image = canvas.draw();

        let spacing = (grid_step(px(16.), zoom).as_f32() * zoom * scale).round() as i32;
        let line_width = scale as i32;
        // The platform target is always 2x-sized; at window scale 1.0 only
        // the top-left quadrant holds canvas content, so scan just that.
        let (content_w, content_h) = (
            (viewport_w as f32 * scale) as i32,
            (viewport_h as f32 * scale) as i32,
        );
        let mid_gap = (spacing / 2) as u32;
        let bg = image.get_pixel(mid_gap, mid_gap).0;

        for (along_x, extent) in [(true, content_w), (false, content_h)] {
            let expected: Vec<i32> = (0..)
                .map(|k| k * spacing)
                .take_while(|&at| at < extent)
                .collect();
            let runs = line_runs(&image, along_x, mid_gap, extent as u32, bg);
            assert_eq!(
                runs.len(),
                expected.len(),
                "zoom {zoom} scale {scale}: expected lines at {expected:?}, got {runs:?}"
            );
            for ((start, end), center) in runs.iter().zip(&expected) {
                assert_eq!(
                    *start as i32, *center,
                    "zoom {zoom} scale {scale}: {runs:?}"
                );
                let drawn = *end as i32 - *start as i32;
                // Grid quads land on whole device pixels, so a width drift is
                // a rounding seam, not anti-aliasing.
                assert_eq!(
                    drawn, line_width,
                    "zoom {zoom} scale {scale}: runs {runs:?}"
                );
            }
        }

        // k=0 is a major line (0.8 opacity), k=1 minor (0.4): the major pixel
        // must sit further from the background than the minor one.
        let major = channel_diff(image.get_pixel(0, mid_gap).0, bg);
        let minor = channel_diff(image.get_pixel(spacing as u32, mid_gap).0, bg);
        assert!(
            major >= minor + 4,
            "zoom {zoom} scale {scale}: major line barely stronger than minor ({major} vs {minor})"
        );
    }

    // An odd logical pan shifts every line by exactly one logical pixel; the
    // width and spacing must survive the rounding.
    let mut canvas = RasterCanvas::open(size(px(160.), px(96.)), 2., |_| {});
    canvas.edit(|s, cx| s.pan_by(point(px(1.), px(0.)), cx));
    let image = canvas.draw();
    let bg = image.get_pixel(33, 33).0;
    let runs = line_runs(&image, true, 33, image.width(), bg);
    let expected: Vec<i32> = (0..)
        .map(|k| 64 * k + 2)
        .take_while(|&at| at < image.width() as i32)
        .collect();
    assert_eq!(
        runs.len(),
        expected.len(),
        "panned: {runs:?} vs {expected:?}"
    );
    for ((start, end), center) in runs.iter().zip(&expected) {
        assert_eq!(*start as i32, *center, "panned: {runs:?}");
        assert_eq!(*end - *start, 2, "panned line width: {runs:?}");
    }
}
