use gpui_kit::component::{
    ActiveTheme as _, IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
};
use gpui_kit::{prelude::*, *};

use crate::ui::canvas::{Canvas, *};

pub(super) struct TextDemo {
    state: Entity<CanvasState>,
    _subscription: Subscription,
}

enum SeedColor {
    Foreground,
    MutedForeground,
    Primary,
}

const TEXT_SEEDS: [(&str, (f32, f32), f32, SeedColor); 6] = [
    (
        "The quick brown fox jumps over",
        (80., 100.),
        32.,
        SeedColor::Foreground,
    ),
    (
        "Scale, pan, and zoom",
        (80., 170.),
        20.,
        SeedColor::MutedForeground,
    ),
    (
        "Canvas text stays crisp at every zoom level",
        (80., 210.),
        14.,
        SeedColor::Foreground,
    ),
    ("0123456789 !@#$%", (80., 260.), 28., SeedColor::Primary),
    (
        "aligned baseline sample",
        (80., 310.),
        16.,
        SeedColor::MutedForeground,
    ),
    (
        "sizes 32, 20, 14, 28, 16 px",
        (80., 360.),
        12.,
        SeedColor::MutedForeground,
    ),
];

impl TextDemo {
    pub(super) fn new(cx: &mut Context<Self>) -> Self {
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
            let (muted, foreground, muted_foreground, primary) = {
                let theme = cx.theme();
                (
                    theme.muted,
                    theme.foreground,
                    theme.muted_foreground,
                    theme.primary,
                )
            };
            s.edit_document(cx, |doc| {
                doc.add_shape(
                    layer,
                    CanvasShape::rect(
                        Bounds {
                            origin: point(px(64.), px(88.)),
                            size: size(px(500.), px(52.)),
                        },
                        ShapeStyle::filled(muted.opacity(0.5)),
                    ),
                );
                for (text, (x, y), font_size, color) in TEXT_SEEDS {
                    let color = match color {
                        SeedColor::Foreground => foreground,
                        SeedColor::MutedForeground => muted_foreground,
                        SeedColor::Primary => primary,
                    };
                    doc.add_shape(
                        layer,
                        CanvasShape::text(point(px(x), px(y)), text, px(font_size), color),
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

impl Render for TextDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        super::render_demo_shell(
            "text",
            crate::i18n::localize(super::CanvasDemo::Text.title_key()),
            crate::i18n::localize(super::CanvasDemo::Text.desc_key()),
            vec![
                Button::new("text-reset-view")
                    .ghost()
                    .small()
                    .icon(IconName::RotateCw)
                    .label(crate::i18n::localize("showcase_reset_view"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.state.update(cx, |s, cx| s.reset_view(cx));
                    }))
                    .into_any_element(),
            ],
            Canvas::new("text-canvas", self.state.clone()),
            cx,
        )
    }
}

#[cfg(test)]
#[path = "text.test.rs"]
mod text_test;
