//! Resizable section, ported from the upstream `ResizableStory`.

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _,
    button::Button,
    h_flex,
    resizable::{ResizableState, h_resizable, resizable_panel, v_resizable},
    v_flex,
};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::section;

fn panel_box(content: impl Into<SharedString>) -> AnyElement {
    div()
        .p_4()
        .size_full()
        .child(content.into())
        .into_any_element()
}

pub struct ResizableSection {
    show_left: bool,
    use_flex_none: bool,
    programmatic_state: Entity<ResizableState>,
}

impl ResizableSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(cx))
    }

    fn new(cx: &mut App) -> Self {
        Self {
            show_left: true,
            use_flex_none: true,
            programmatic_state: cx.new(|_| ResizableState::default()),
        }
    }
}

impl Render for ResizableSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .child(
                section("resizable-nested-box", "Nested Panels")
                    .description(
                        "Combines horizontal and vertical splits with constrained panel sizes.",
                    )
                    .w_full()
                    .child(
                        div()
                            .w_full()
                            .h(rems(37.5))
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                v_resizable("resizable-nested")
                                    .child(
                                        h_resizable("resizable-nested-split")
                                            .child(
                                                resizable_panel()
                                                    .size(px(150.))
                                                    .size_range(px(120.)..px(300.))
                                                    .child(panel_box("Left (120px .. 300px)")),
                                            )
                                            .child(panel_box("Center"))
                                            .child(
                                                resizable_panel()
                                                    .size(px(300.))
                                                    .child(panel_box("Right")),
                                            ),
                                    )
                                    .child(panel_box("Center"))
                                    .child(
                                        resizable_panel()
                                            .size(px(80.))
                                            .size_range(px(80.)..Pixels::MAX)
                                            .child(panel_box("Bottom (80px and up)")),
                                    ),
                            ),
                    ),
            )
            .child(
                section("resizable-growing-box", "Growing Panel")
                    .description(
                        "A flexible panel absorbs the space left by a constrained neighbor.",
                    )
                    .w_full()
                    .child(
                        div()
                            .w_full()
                            .h(rems(25.))
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                h_resizable("resizable-growing")
                                    .child(
                                        resizable_panel()
                                            .size(px(200.))
                                            .size_range(px(200.)..px(400.))
                                            .child(panel_box("Left 2")),
                                    )
                                    .child(panel_box("Right (Grow)")),
                            ),
                    ),
            )
            .child(
                section("resizable-flex-box", "Flex Behavior")
                    .description("Compare fixed and growing panels while toggling visibility.")
                    .w_full()
                    .child(
                        v_flex()
                            .w_full()
                            .gap_2()
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap_1()
                                    .child(
                                        Button::new("resizable-toggle-left")
                                            .outline()
                                            .label(if self.show_left {
                                                "Hide Left"
                                            } else {
                                                "Show Left"
                                            })
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.show_left = !this.show_left;
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Button::new("resizable-toggle-flex-none")
                                            .outline()
                                            .label(if self.use_flex_none {
                                                "Use flex_none: ON"
                                            } else {
                                                "Use flex_none: OFF"
                                            })
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.use_flex_none = !this.use_flex_none;
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .h(rems(12.5))
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .child(
                                        h_resizable("resizable-flex")
                                            .child({
                                                // flex_none keeps a sized panel at its width, so
                                                // hidden-panel slack lands in the center panel.
                                                let mut panel = resizable_panel()
                                                    .visible(self.show_left)
                                                    .size(px(200.))
                                                    .size_range(px(150.)..px(400.))
                                                    .child(panel_box("Left"));
                                                if self.use_flex_none {
                                                    panel = panel.flex_none();
                                                }
                                                panel
                                            })
                                            .child(panel_box("Center"))
                                            .child({
                                                let mut panel = resizable_panel()
                                                    .size(px(280.))
                                                    .size_range(px(200.)..px(400.))
                                                    .child(panel_box("Right"));
                                                if self.use_flex_none {
                                                    panel = panel.flex_none();
                                                }
                                                panel
                                            }),
                                    ),
                            ),
                    ),
            )
            .child(
                section("resizable-programmatic-box", "Programmatic Resize")
                    .description("Panel sizes can be changed by actions as well as dragging.")
                    .w_full()
                    .child(
                        v_flex()
                            .w_full()
                            .gap_2()
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap_1()
                                    .child(
                                        Button::new("resizable-compact-left")
                                            .small()
                                            .label("Compact left → 100")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.programmatic_state.update(cx, |state, cx| {
                                                    state.resize_panel(0, px(100.), window, cx);
                                                });
                                            })),
                                    )
                                    .child(
                                        Button::new("resizable-reset-left")
                                            .small()
                                            .label("Reset left → 200")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.programmatic_state.update(cx, |state, cx| {
                                                    state.resize_panel(0, px(200.), window, cx);
                                                });
                                            })),
                                    )
                                    .child(
                                        Button::new("resizable-compact-right")
                                            .small()
                                            .label("Compact right → 80")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.programmatic_state.update(cx, |state, cx| {
                                                    state.resize_panel(2, px(80.), window, cx);
                                                });
                                            })),
                                    )
                                    .child(
                                        Button::new("resizable-reset-right")
                                            .small()
                                            .label("Reset right → 200")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.programmatic_state.update(cx, |state, cx| {
                                                    state.resize_panel(2, px(200.), window, cx);
                                                });
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .h(rems(12.5))
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .child(
                                        h_resizable("resizable-programmatic")
                                            .with_state(&self.programmatic_state)
                                            .child(
                                                resizable_panel()
                                                    .size(px(200.))
                                                    .child(panel_box("Left")),
                                            )
                                            .child(panel_box("Center (grow)"))
                                            .child(
                                                resizable_panel()
                                                    .size(px(200.))
                                                    .child(panel_box("Right")),
                                            ),
                                    ),
                            ),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "resizable",
        "Resizable",
        "Draggable split panels with size constraints and programmatic resize.",
        ResizableSection::view(window, cx),
    ));
}
