//! Removable canvas showcase page. `CanvasShowcasePage` is the only public
//! surface; everything else stays private to the crate. Removal = delete this
//! directory and revert its seam touches: the `mod`/`pub use` pair in
//! features/pages/mod.rs, the Page::CanvasShowcase arms and all() entry in
//! shell/sidebar.rs, the VALID_HOSTS entry in shell/route.rs, the AppRoot
//! wiring in shell/root/app_root/state.rs, the page count in
//! tests/e2e_navigation.rs, the nav_canvas_showcase/showcase_* keys in both
//! locale files, and the common-table image line in Cargo.toml.

mod bezier;
mod geometry;
mod image;
mod node_graph;
mod particles;
mod shape_edit;
mod text;

use gpui_kit::component::{
    ActiveTheme as _, IconName, h_flex,
    tab::{Tab, TabBar},
    v_flex,
};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;

use bezier::BezierDemo;
use image::ImageDemo;
use node_graph::NodeGraphDemo;
use particles::ParticleFieldDemo;
use shape_edit::ShapesDemo;
use text::TextDemo;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CanvasDemo {
    Shapes,
    Bezier,
    Nodes,
    Particles,
    Text,
    Image,
}

impl CanvasDemo {
    const ALL: [Self; 6] = [
        Self::Shapes,
        Self::Bezier,
        Self::Nodes,
        Self::Particles,
        Self::Text,
        Self::Image,
    ];

    fn tab_key(self) -> &'static str {
        match self {
            Self::Shapes => "showcase_tab_shapes",
            Self::Bezier => "showcase_tab_bezier",
            Self::Nodes => "showcase_tab_nodes",
            Self::Particles => "showcase_tab_particles",
            Self::Text => "showcase_tab_text",
            Self::Image => "showcase_tab_image",
        }
    }

    fn title_key(self) -> &'static str {
        match self {
            Self::Shapes => "showcase_shapes_title",
            Self::Bezier => "showcase_bezier_title",
            Self::Nodes => "showcase_nodes_title",
            Self::Particles => "showcase_particles_title",
            Self::Text => "showcase_text_title",
            Self::Image => "showcase_image_title",
        }
    }

    fn desc_key(self) -> &'static str {
        match self {
            Self::Shapes => "showcase_shapes_desc",
            Self::Bezier => "showcase_bezier_desc",
            Self::Nodes => "showcase_nodes_desc",
            Self::Particles => "showcase_particles_desc",
            Self::Text => "showcase_text_desc",
            Self::Image => "showcase_image_desc",
        }
    }

    fn icon(self) -> IconName {
        match self {
            Self::Shapes => IconName::Square,
            Self::Bezier => IconName::Dash,
            Self::Nodes => IconName::Network,
            Self::Particles => IconName::Asterisk,
            Self::Text => IconName::CaseSensitive,
            Self::Image => IconName::Frame,
        }
    }
}

pub struct CanvasShowcasePage {
    demos: Vec<(CanvasDemo, AnyView)>,
    active_ix: usize,
}

impl CanvasShowcasePage {
    /// Page view; `AppRoot` constructs it once per window. Nothing in the
    /// page needs `Window`, so the constructor takes only the context.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let demos = vec![
            (CanvasDemo::Shapes, cx.new(ShapesDemo::new).into()),
            (CanvasDemo::Bezier, cx.new(BezierDemo::new).into()),
            (CanvasDemo::Nodes, cx.new(NodeGraphDemo::new).into()),
            (CanvasDemo::Particles, cx.new(ParticleFieldDemo::new).into()),
            (CanvasDemo::Text, cx.new(TextDemo::new).into()),
            (CanvasDemo::Image, cx.new(ImageDemo::new).into()),
        ];
        Self {
            demos,
            active_ix: 0,
        }
    }
}

impl Render for CanvasShowcasePage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = crate::i18n::localize("nav_canvas_showcase");
        v_flex()
            .size_full()
            .child(
                v_flex()
                    .p_4()
                    .pb_0()
                    .gap_1()
                    .child(
                        div()
                            .id("canvas-showcase-title")
                            .a11y(Role::Heading, title.clone())
                            .aria_level(1)
                            .text_xl()
                            .font_weight(FontWeight::BOLD)
                            .child(title),
                    )
                    .child(
                        div()
                            .text_color(cx.theme().muted_foreground)
                            .child(crate::i18n::localize("showcase_page_desc")),
                    ),
            )
            .child(
                TabBar::new("showcase-tabs")
                    .selected_index(self.active_ix)
                    .on_click(cx.listener(|this, ix: &usize, _, cx| {
                        if *ix < this.demos.len() {
                            this.active_ix = *ix;
                            cx.notify();
                        }
                    }))
                    .children(CanvasDemo::ALL.map(|kind| {
                        Tab::new()
                            .icon(kind.icon())
                            .label(crate::i18n::localize(kind.tab_key()))
                    })),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .child(self.demos[self.active_ix].1.clone()),
            )
    }
}

fn render_demo_shell(
    id: &str,
    title: String,
    desc: String,
    toolbar: Vec<AnyElement>,
    canvas: impl IntoElement,
    cx: &App,
) -> Div {
    v_flex()
        .size_full()
        .child(
            v_flex()
                .p_4()
                .gap_1()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    div()
                        .id(SharedString::from(format!("{id}-title")))
                        .a11y(Role::Heading, title.clone())
                        .aria_level(2)
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(title),
                )
                .child(div().text_color(cx.theme().muted_foreground).child(desc))
                .child(h_flex().gap_2().mt_2().children(toolbar)),
        )
        .child(
            div().flex_1().min_h_0().p_4().child(
                div()
                    .size_full()
                    .border_1()
                    .rounded_lg()
                    .border_color(cx.theme().border)
                    .overflow_hidden()
                    .child(canvas),
            ),
        )
}

#[cfg(test)]
#[path = "mod.test.rs"]
mod canvas_showcase_test;
