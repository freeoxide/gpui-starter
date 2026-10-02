//! Nav stack section, ported from the wasm showcase's `nav_stack` demo (the
//! showcase's raw example colors became theme tokens here).

use std::time::Duration;

use gpui_kit::base::motion::{PresencePhase, Transition};
use gpui_kit::base::{NavMotion, NavOperation, NavPage, NavStack, NavStackState};
use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, StyledExt as _, button::Button, h_flex, v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::gallery::registry::GallerySection;

use super::demo::section;

/// One page of the stack. A page knows its depth and holds the stack it lives
/// in, so its own buttons can push over it, replace it, or pop it.
struct NavPageView {
    depth: usize,
    stack: WeakEntity<NavStackState>,
}

impl NavPageView {
    fn new(depth: usize, stack: WeakEntity<NavStackState>) -> Self {
        Self { depth, stack }
    }

    /// A click handler that builds a page at `depth` and hands it to `apply`:
    /// a pushed page sits one deeper, a replacement at the same depth.
    fn navigate(
        &self,
        depth: usize,
        apply: impl Fn(&mut NavStackState, Entity<NavPageView>, &mut Context<NavStackState>) + 'static,
    ) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
        let stack = self.stack.clone();
        move |_, _, cx| {
            _ = stack.update(cx, |state, cx| {
                let page = cx.new(|_| NavPageView::new(depth, stack.clone()));
                apply(state, page, cx);
            });
        }
    }
}

impl Render for NavPageView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let depth = self.depth;
        // The trail is the stack's history: the pages behind this one, then
        // the pages popped off it, which forward brings back one at a time.
        let (behind, ahead) = self
            .stack
            .upgrade()
            .map(|stack| {
                let stack = stack.read(cx);
                (stack.depth(), stack.forward_views().len())
            })
            .unwrap_or((depth, 0));
        let strong = self.stack.clone();

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .bg(if depth % 2 == 1 {
                cx.theme().background
            } else {
                cx.theme().muted
            })
            .child(div().font_semibold().child(format!("Page {depth}")))
            .child(
                h_flex()
                    .gap_1()
                    .text_color(cx.theme().muted_foreground)
                    .children((1..=behind + ahead).map(|page| {
                        div()
                            .px_1()
                            .when(page == depth, |this| {
                                this.text_color(cx.theme().foreground).font_semibold()
                            })
                            // Pages only forward can reach read dimmer than the
                            // ones behind; opacity keeps the single token.
                            .when(page > behind, |this| {
                                this.text_color(cx.theme().muted_foreground.opacity(0.55))
                            })
                            .child(page.to_string())
                    })),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("nav-stack-push")
                            .outline()
                            .small()
                            .label("Push")
                            .on_click(self.navigate(depth + 1, |stack, page, cx| {
                                stack.push(page, NavMotion::Animated, cx)
                            })),
                    )
                    .child(
                        Button::new("nav-stack-replace")
                            .outline()
                            .small()
                            .label("Replace")
                            .on_click(self.navigate(depth, |stack, page, cx| {
                                stack.replace(page, NavMotion::Animated, cx);
                            })),
                    )
                    .when(depth > 1, |this| {
                        this.child(
                            Button::new("nav-stack-pop")
                                .outline()
                                .small()
                                .label("Pop")
                                .on_click(move |_, _, cx| {
                                    _ = strong.update(cx, |stack, cx| {
                                        stack.pop(NavMotion::Animated, cx);
                                    });
                                }),
                        )
                    })
                    .when(ahead > 0, |this| {
                        this.child(
                            Button::new("nav-stack-forward")
                                .outline()
                                .small()
                                .label("Forward")
                                .on_click({
                                    let stack = self.stack.clone();
                                    move |_, _, cx| {
                                        _ = stack.update(cx, |stack, cx| {
                                            stack.forward(NavMotion::Animated, cx);
                                        });
                                    }
                                }),
                        )
                    }),
            )
    }
}

/// A pushed page slides in from the right and slides back out when popped;
/// the page underneath drifts a little to show depth.
fn slide(page: NavPage) -> AnyElement {
    let offset = match (page.phase(), page.operation()) {
        (PresencePhase::Entering, Some(NavOperation::Push | NavOperation::Replace)) => {
            1.0 - page.progress()
        }
        (PresencePhase::Exiting, Some(NavOperation::Pop)) => page.progress(),
        (PresencePhase::Exiting, Some(NavOperation::Push)) => -0.3 * page.progress(),
        (PresencePhase::Entering, Some(NavOperation::Pop)) => -0.3 * (1.0 - page.progress()),
        _ => 0.0,
    };
    page.left(relative(offset)).into_any_element()
}

pub struct NavStackSection {
    stack: Entity<NavStackState>,
    _subscriptions: Vec<Subscription>,
}

impl NavStackSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let stack = cx.new(|_| NavStackState::new());
            stack.update(cx, |state, cx| {
                let root = cx.new(|_| NavPageView::new(1, stack.downgrade()));
                state.push(root, NavMotion::Immediate, cx);
            });
            let subscription = cx.observe(&stack, |_, _, cx| cx.notify());
            Self {
                stack,
                _subscriptions: vec![subscription],
            }
        })
    }
}

impl Render for NavStackSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .child(
                section("nav-stack-box", "Stack navigation")
                    .description(
                        "Push, replace, pop, and forward, with the outgoing page kept mounted until its transition ends.",
                    )
                    .v_flex()
                    .gap_3()
                    .child(
                        NavStack::new(&self.stack)
                            .w_72()
                            .h_40()
                            .overflow_hidden()
                            .border_1()
                            .border_color(cx.theme().border)
                            .transition(Transition::new(Duration::from_millis(220)))
                            .item(|page, _, _| slide(page)),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "nav-stack",
        "Nav Stack",
        "A stack of views with one visible at a time, keeping popped pages for forward.",
        NavStackSection::view(window, cx),
    ));
}
