//! The transparency checkerboard behind the color swatches, ported from the
//! upstream story support element of the same name.

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

/// A checkerboard surface that reveals how much alpha a swatch carries. The
/// greys are the demo's data: they must stay fixed and theme-neutral, or the
/// transparency readout stops meaning anything.
#[derive(IntoElement)]
pub(crate) struct Checkerboard {
    children: Vec<AnyElement>,
    is_dark: bool,
}

impl Checkerboard {
    pub(crate) fn new(is_dark: bool) -> Self {
        Self {
            children: Vec::new(),
            is_dark,
        }
    }
}

impl ParentElement for Checkerboard {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Checkerboard {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let square_size = px(12.);
        let (c1, c2) = if self.is_dark {
            (hsla(0., 0., 0.1, 1.), hsla(0., 0., 0.13, 1.))
        } else {
            (hsla(0., 0., 1.0, 1.), hsla(0., 0., 0.95, 1.))
        };

        div()
            .bg(c1)
            .rounded(cx.theme().radius_lg)
            .overflow_hidden()
            .size_full()
            .child(
                canvas(
                    move |_, _, _| (),
                    move |bounds, _, window, _| {
                        let rows = (bounds.size.height / square_size).ceil() as i32;
                        let cols = (bounds.size.width / square_size).ceil() as i32;

                        for row in 0..rows {
                            for col in 0..cols {
                                if (row + col) % 2 == 0 {
                                    let origin = bounds.origin
                                        + point(
                                            square_size * (col as f32),
                                            square_size * (row as f32),
                                        );

                                    window.paint_quad(PaintQuad {
                                        bounds: Bounds {
                                            origin,
                                            size: size(square_size, square_size),
                                        },
                                        corner_radii: Corners::default(),
                                        background: c2.into(),
                                        border_widths: Edges::default(),
                                        border_color: transparent_black(),
                                        border_style: BorderStyle::default(),
                                    });
                                }
                            }
                        }
                    },
                )
                .absolute()
                .size_full(),
            )
            .children(self.children)
    }
}
