use std::sync::Arc;

use gpui_kit::component::{
    ActiveTheme as _, IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
};
use gpui_kit::{prelude::*, *};

use crate::ui::canvas::{Canvas, *};

const SWATCH: u32 = 96;
const CELL: u32 = 12;

fn checker_swatch() -> image::RgbaImage {
    image::RgbaImage::from_fn(SWATCH, SWATCH, |x, y| {
        let (blue, green, red) = if (x / CELL + y / CELL).is_multiple_of(2) {
            (60, 120, 230)
        } else {
            (170, 90, 40)
        };
        image::Rgba([blue, green, red, 255])
    })
}

fn render_image() -> Arc<RenderImage> {
    Arc::new(RenderImage::new(vec![image::Frame::new(checker_swatch())]))
}

pub(super) struct ImageDemo {
    state: Entity<CanvasState>,
    _subscription: Subscription,
}

impl ImageDemo {
    pub(super) fn new(cx: &mut Context<Self>) -> Self {
        let image = render_image();
        let state = cx.new(|_| CanvasState::new());
        state.update(cx, |s, cx| {
            s.set_builtin_tool(BuiltinTool::Pan, cx);
            s.set_grid(
                CanvasGrid {
                    visible: false,
                    spacing: px(16.),
                },
                cx,
            );
            let layer = s.add_layer(cx);
            let border = cx.theme().border;
            let image = image.clone();
            s.edit_document(cx, |doc| {
                for (origin, side) in [((160., 140.), 192.), ((400., 140.), 96.)] {
                    let bounds = Bounds {
                        origin: point(px(origin.0), px(origin.1)),
                        size: size(px(side), px(side)),
                    };
                    doc.add_shape(layer, CanvasShape::image(bounds, image.clone()));
                    doc.add_shape(
                        layer,
                        CanvasShape::rect(
                            bounds,
                            ShapeStyle::stroked(StrokeStyle {
                                width: px(1.),
                                color: border,
                                dash: None,
                            }),
                        ),
                    );
                }
            });
        });
        let _subscription = cx.observe(&state, |_, _, cx| cx.notify());
        Self {
            state,
            _subscription,
        }
    }
}

impl Render for ImageDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        super::render_demo_shell(
            "image",
            crate::i18n::localize(super::CanvasDemo::Image.title_key()),
            crate::i18n::localize(super::CanvasDemo::Image.desc_key()),
            vec![
                Button::new("image-reset-view")
                    .ghost()
                    .small()
                    .icon(IconName::RotateCw)
                    .label(crate::i18n::localize("showcase_reset_view"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.state.update(cx, |s, cx| s.reset_view(cx));
                    }))
                    .into_any_element(),
            ],
            Canvas::new("image-canvas", self.state.clone()),
            cx,
        )
    }
}

#[cfg(test)]
#[path = "image.test.rs"]
mod image_test;
