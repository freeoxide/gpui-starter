use gpui_kit::component::{
    ActiveTheme as _, Sizable as _,
    button::{Button, Toggle, ToggleGroup},
    h_flex, v_flex,
};
use gpui_kit::{prelude::*, *};

pub(crate) const SHADER_ID: &str = "canvas-shader-toy";

const MAX_QUAD_CELLS: usize = 4096;
const CELL_SIZES: [Pixels; 3] = [px(4.), px(6.), px(10.)];

pub(crate) fn shader_hsv(u: f32, v: f32, t: f32) -> Hsla {
    hsla(
        ((u + t * 0.05) % 1.0).max(0.),
        0.85,
        (0.5 + (u * 6. + t).sin() * 0.25 + (v * 4. - t * 1.7).sin() * 0.25).clamp(0.08, 0.92),
        1.,
    )
}

fn ceil_dims(bounds: Bounds<Pixels>, cell: f32) -> (usize, usize) {
    (
        (bounds.size.width.as_f32() / cell).ceil() as usize,
        (bounds.size.height.as_f32() / cell).ceil() as usize,
    )
}

pub(crate) fn grid_dims(bounds: Bounds<Pixels>, cell: Pixels) -> (usize, usize) {
    let mut cell = cell.as_f32().max(1.);
    let (mut cols, mut rows) = ceil_dims(bounds, cell);
    while cols.saturating_mul(rows) > MAX_QUAD_CELLS {
        cell *= 2.;
        (cols, rows) = ceil_dims(bounds, cell);
    }
    (cols.max(1), rows.max(1))
}

pub(crate) fn grid_cell_bounds(
    bounds: Bounds<Pixels>,
    cell: Pixels,
    gx: usize,
    gy: usize,
) -> Bounds<Pixels> {
    let cell = cell.as_f32();
    let origin = point(
        px(bounds.origin.x.as_f32() + cell * gx as f32),
        px(bounds.origin.y.as_f32() + cell * gy as f32),
    );
    let width = cell
        .min(bounds.right().as_f32() - origin.x.as_f32())
        .max(0.);
    let height = cell
        .min(bounds.bottom().as_f32() - origin.y.as_f32())
        .max(0.);
    Bounds {
        origin,
        size: size(px(width), px(height)),
    }
}

pub(crate) struct ShaderPanel {
    frame: usize,
    playing: bool,
    cell: Pixels,
}

impl ShaderPanel {
    pub(crate) fn new_view(_window: &mut Window, cx: &mut App) -> AnyView {
        cx.new(|_| Self {
            frame: 0,
            playing: true,
            cell: CELL_SIZES[1],
        })
        .into()
    }
}

impl Render for ShaderPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.playing && !cx.reduce_motion() {
            window.request_animation_frame();
        }
        self.frame += 1;
        let t = self.frame as f32 / 60.;
        let cell = self.cell;
        v_flex()
            .size_full()
            .child(
                h_flex()
                    .gap_2()
                    .p_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("shader-play")
                            .small()
                            .label(if self.playing { "Pause" } else { "Play" })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.playing = !this.playing;
                                cx.notify();
                            })),
                    )
                    .child(
                        ToggleGroup::new("shader-cell")
                            .small()
                            .segmented()
                            .child(
                                Toggle::new("shader-cell-fine")
                                    .label("Fine")
                                    .checked(self.cell == CELL_SIZES[0]),
                            )
                            .child(
                                Toggle::new("shader-cell-balanced")
                                    .label("Balanced")
                                    .checked(self.cell == CELL_SIZES[1]),
                            )
                            .child(
                                Toggle::new("shader-cell-coarse")
                                    .label("Coarse")
                                    .checked(self.cell == CELL_SIZES[2]),
                            )
                            .on_click(cx.listener(|this, values: &Vec<bool>, _, cx| {
                                if let Some(ix) = values.iter().position(|b| *b)
                                    && let Some(cell) = CELL_SIZES.get(ix)
                                {
                                    this.cell = *cell;
                                    cx.notify();
                                }
                            })),
                    ),
            )
            .child(
                div().relative().flex_1().min_h_0().overflow_hidden().child(
                    canvas(
                        |_bounds, _, _| (),
                        move |bounds, _, window, _| {
                            let (cols, rows) = grid_dims(bounds, cell);
                            for gy in 0..rows {
                                for gx in 0..cols {
                                    let cell_bounds = grid_cell_bounds(bounds, cell, gx, gy);
                                    let color = shader_hsv(
                                        (gx as f32 + 0.5) / cols as f32,
                                        (gy as f32 + 0.5) / rows as f32,
                                        t,
                                    );
                                    window.paint_quad(fill(cell_bounds, color));
                                }
                            }
                        },
                    )
                    .absolute()
                    .size_full(),
                ),
            )
    }
}

#[cfg(test)]
#[path = "shader.test.rs"]
mod shader_test;
