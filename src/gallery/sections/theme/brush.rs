//! Brush section, ported from the upstream brush example: a freehand drawing
//! canvas driven by brush size, opacity, and color controls.

use std::rc::Rc;

use gpui_kit::component::{
    ActiveTheme as _, Colorize as _, ElementExt, IconName, Sizable as _,
    button::Button,
    checkbox::Checkbox,
    h_flex,
    slider::{Slider, SliderState},
    v_flex,
};
use gpui_kit::{
    App, AppContext, Bounds, Context, Entity, Hsla, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Path, PathBuilder, Pixels, Point,
    Render, Styled, Window, black, blue, canvas, div, green, hsla, prelude::FluentBuilder as _, px,
    red, relative, rems, white, yellow,
};

use crate::gallery::registry::GallerySection;

use crate::gallery::demo::section;

#[derive(Clone, Debug)]
struct Stroke {
    points: Vec<Point<Pixels>>,
    color: Hsla,
    size: f32,
}

/// The palette is the demo's data, like the checkerboard greys: swatch colors
/// stay literal so the brush does not follow the app theme.
const PALETTE: [Hsla; 8] = [
    black(),
    white(),
    red(),
    green(),
    blue(),
    yellow(),
    hsla(0.58, 1.0, 0.5, 1.0),
    hsla(0.083, 1.0, 0.5, 1.0),
];

pub struct BrushSection {
    brush_size: Entity<SliderState>,
    brush_opacity: Entity<SliderState>,
    brush_color: Hsla,
    strokes: Rc<Vec<Stroke>>,
    /// Tessellated committed strokes at a zero origin, rebuilt only when the
    /// stroke set changes, then translated to the canvas origin per paint.
    stroke_paths: Rc<Vec<(Path<Pixels>, Hsla)>>,
    stroke_paths_source: Rc<Vec<Stroke>>,
    current_stroke: Option<Stroke>,
    is_drawing: bool,
    show_grid: bool,
    canvas_bounds: Option<Bounds<Pixels>>,
}

impl BrushSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let brush_size = cx.new(|_| {
            SliderState::new()
                .min(1.)
                .max(50.)
                .default_value(5.)
                .step(1.)
        });

        let brush_opacity = cx.new(|_| {
            SliderState::new()
                .min(0.1)
                .max(1.0)
                .default_value(1.0)
                .step(0.05)
        });

        Self {
            brush_size,
            brush_opacity,
            brush_color: black(),
            strokes: Rc::new(vec![]),
            stroke_paths: Rc::new(vec![]),
            stroke_paths_source: Rc::new(vec![]),
            current_stroke: None,
            is_drawing: false,
            show_grid: false,
            canvas_bounds: None,
        }
    }

    fn handle_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button == MouseButton::Left {
            self.is_drawing = true;
            let brush_size = self.brush_size.read(cx).value().start();
            let brush_opacity = self.brush_opacity.read(cx).value().start();
            let color = self.brush_color.opacity(brush_opacity);

            let local_pos = if let Some(bounds) = self.canvas_bounds {
                Point::new(
                    event.position.x - bounds.origin.x,
                    event.position.y - bounds.origin.y,
                )
            } else {
                event.position
            };

            self.current_stroke = Some(Stroke {
                points: vec![local_pos],
                color,
                size: brush_size,
            });
            cx.notify();
        }
    }

    fn handle_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.is_drawing {
            return;
        }
        // Releasing the button outside the canvas never delivers mouse-up.
        if !event.dragging() {
            self.finish_stroke(cx);
            return;
        }
        if let Some(ref mut stroke) = self.current_stroke {
            let local_pos = if let Some(bounds) = self.canvas_bounds {
                Point::new(
                    event.position.x - bounds.origin.x,
                    event.position.y - bounds.origin.y,
                )
            } else {
                event.position
            };

            let should_add = if let Some(last) = stroke.points.last() {
                let dx = (local_pos.x - last.x).abs();
                let dy = (local_pos.y - last.y).abs();
                dx >= px(1.0) || dy >= px(1.0)
            } else {
                true
            };

            if should_add {
                stroke.points.push(local_pos);
                cx.notify();
            }
        }
    }

    fn handle_mouse_up(&mut self, _event: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.finish_stroke(cx);
    }

    // Also reached from mouse-move when no button is held, so a release
    // outside the canvas cannot leave a stroke dangling.
    fn finish_stroke(&mut self, cx: &mut Context<Self>) {
        if !self.is_drawing {
            return;
        }
        self.is_drawing = false;
        if let Some(stroke) = self.current_stroke.take() {
            // Even a one-point stroke is a mark: a click leaves a dot.
            let mut new_strokes = (*self.strokes).clone();
            new_strokes.push(stroke);
            self.strokes = Rc::new(new_strokes);
        }
        cx.notify();
    }

    /// Rebuild the tessellated paths only when the committed stroke set
    /// changed; repaints then translate them instead of re-tessellating.
    fn refresh_stroke_paths(&mut self) {
        if Rc::ptr_eq(&self.stroke_paths_source, &self.strokes) {
            return;
        }
        let paths = self
            .strokes
            .iter()
            .filter_map(|stroke| {
                build_stroke_path(stroke, Point::default()).map(|path| (path, stroke.color))
            })
            .collect();
        self.stroke_paths = Rc::new(paths);
        self.stroke_paths_source = self.strokes.clone();
    }

    fn clear_canvas(&mut self, cx: &mut Context<Self>) {
        self.strokes = Rc::new(vec![]);
        self.current_stroke = None;
        self.is_drawing = false;
        cx.notify();
    }

    fn set_brush_color(&mut self, color: Hsla, cx: &mut Context<Self>) {
        self.brush_color = color;
        cx.notify();
    }

    fn color_button(&self, color: Hsla, cx: &Context<Self>) -> impl IntoElement {
        let is_selected = self.brush_color.to_hex() == color.to_hex();
        let theme = cx.theme();

        div()
            .size(rems(2.5))
            .rounded(theme.radius)
            .bg(color)
            .border_2()
            .when(is_selected, |this| {
                this.border_color(theme.primary).shadow_md()
            })
            .when(!is_selected, |this| this.border_color(theme.border))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.set_brush_color(color, cx);
                }),
            )
    }

    fn render_canvas(&mut self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        let current_stroke_for_prepaint = self.current_stroke.clone();
        let show_grid_for_prepaint = self.show_grid;
        let theme_for_prepaint = theme.clone();

        let state_entity = cx.entity().clone();
        let canvas_entity = state_entity.clone();

        div()
            .id("brush-canvas")
            .w_full()
            // The GroupBox content area auto-sizes, so a percentage height
            // collapses to zero; the canvas needs an explicit frame.
            .h(rems(24.))
            .bg(theme.background)
            .cursor_crosshair()
            .relative()
            .on_mouse_down(MouseButton::Left, cx.listener(Self::handle_mouse_down))
            .on_mouse_move(cx.listener(Self::handle_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::handle_mouse_up))
            // on_mouse_up fires only while hovered, so a release outside the
            // canvas needs its own binding to end and commit the stroke.
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::handle_mouse_up))
            .on_prepaint(move |bounds, _window, cx| {
                state_entity.update(cx, |state, _| {
                    state.canvas_bounds = Some(bounds);
                })
            })
            .child(
                canvas(
                    move |_, _window, cx| {
                        let paths = canvas_entity.update(cx, |state, _| {
                            state.refresh_stroke_paths();
                            state.stroke_paths.clone()
                        });
                        (
                            paths,
                            current_stroke_for_prepaint,
                            show_grid_for_prepaint,
                            theme_for_prepaint,
                        )
                    },
                    move |bounds, (paths, current_stroke, show_grid, theme), window, _| {
                        let origin = bounds.origin;
                        let size = bounds.size;

                        if show_grid {
                            let grid_color = theme.border.opacity(0.2);
                            let grid_size = 40.0;

                            // One subpath per line, so the whole grid
                            // tessellates into a single painted path.
                            let mut builder = PathBuilder::stroke(px(1.0));
                            let mut x = 0.0;
                            while px(x) <= size.width {
                                builder.move_to(Point::new(origin.x + px(x), origin.y));
                                builder
                                    .line_to(Point::new(origin.x + px(x), origin.y + size.height));
                                x += grid_size;
                            }

                            let mut y = 0.0;
                            while px(y) <= size.height {
                                builder.move_to(Point::new(origin.x, origin.y + px(y)));
                                builder
                                    .line_to(Point::new(origin.x + size.width, origin.y + px(y)));
                                y += grid_size;
                            }
                            if let Ok(path) = builder.build() {
                                window.paint_path(path, grid_color);
                            }
                        }

                        for (path, color) in paths.iter() {
                            window.paint_path(translated(path, origin), *color);
                        }

                        if let Some(ref stroke) = current_stroke {
                            if let Some(path) = build_stroke_path(stroke, origin) {
                                window.paint_path(path, stroke.color);
                            }
                        }
                    },
                )
                .absolute()
                .size_full(),
            )
    }
}

/// `path` moved by `offset`: a finished path is only its bounds and vertices,
/// so shifting them costs a copy, not a re-tessellation.
fn translated(path: &Path<Pixels>, offset: Point<Pixels>) -> Path<Pixels> {
    let mut path = path.clone();
    path.bounds.origin += offset;
    for vertex in &mut path.vertices {
        vertex.xy_position += offset;
    }
    path
}

fn build_stroke_path(stroke: &Stroke, origin: Point<Pixels>) -> Option<Path<Pixels>> {
    let first = stroke.points.first()?;

    let mut builder = PathBuilder::stroke(px(stroke.size));

    // A lone point strokes as a short centered segment, which renders as a dot.
    if stroke.points.len() == 1 {
        let radius = stroke.size / 2.0;
        builder.move_to(Point::new(
            origin.x + first.x - px(radius),
            origin.y + first.y,
        ));
        builder.line_to(Point::new(
            origin.x + first.x + px(radius),
            origin.y + first.y,
        ));
        return builder.build().ok();
    }

    builder.move_to(origin + *first);
    for point in stroke.points.iter().skip(1) {
        builder.line_to(origin + *point);
    }

    builder.build().ok()
}

impl Render for BrushSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let brush_size = self.brush_size.read(cx).value().start();
        let brush_opacity = self.brush_opacity.read(cx).value().start();

        v_flex()
            .w_full()
            .h(rems(40.))
            .gap_6()
            .p_4()
            .child(
                section("brush-controls", "Controls").child(
                    h_flex()
                        .gap_8()
                        .w_full()
                        .items_start()
                        .child(
                            v_flex()
                                .gap_4()
                                .flex_1()
                                .w(relative(0.5))
                                .child(
                                    h_flex()
                                        .gap_4()
                                        .items_center()
                                        .child("Size:")
                                        .child(
                                            Slider::new(&self.brush_size)
                                                .w(rems(12.5))
                                                .bg(cx.theme().primary)
                                                .text_color(cx.theme().primary_foreground),
                                        )
                                        .child(format!("{:.0}px", brush_size)),
                                )
                                .child(
                                    h_flex()
                                        .gap_4()
                                        .items_center()
                                        .child("Opacity:")
                                        .child(
                                            Slider::new(&self.brush_opacity)
                                                .w(rems(12.5))
                                                .bg(cx.theme().primary)
                                                .text_color(cx.theme().primary_foreground),
                                        )
                                        .child(format!("{:.0}%", brush_opacity * 100.0)),
                                )
                                .child(
                                    h_flex()
                                        .gap_3()
                                        .items_center()
                                        .child(
                                            Button::new("brush-clear-canvas")
                                                .icon(IconName::Close)
                                                .label("Clear Canvas")
                                                .small()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.clear_canvas(cx);
                                                })),
                                        )
                                        .child(
                                            Checkbox::new("brush-show-grid")
                                                .label("Show Grid")
                                                .checked(self.show_grid)
                                                .on_click(cx.listener(|this, checked, _, cx| {
                                                    this.show_grid = *checked;
                                                    cx.notify();
                                                })),
                                        ),
                                ),
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .flex_1()
                                .w(relative(0.5))
                                .child(h_flex().gap_2().items_center().child("Color:"))
                                .child(h_flex().gap_3().flex_wrap().children(
                                    PALETTE.iter().map(|color| self.color_button(*color, cx)),
                                )),
                        ),
                ),
            )
            .child(section("brush-canvas-box", "Drawing Canvas").child(self.render_canvas(cx)))
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "brush",
        "Brush",
        "A freehand drawing canvas with brush size, opacity, and color controls.",
        BrushSection::view(window, cx),
    ));
}

#[cfg(test)]
#[path = "brush.test.rs"]
mod brush_test;
