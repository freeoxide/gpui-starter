//! Gallery root view: a searchable section navigator plus the active section,
//! patterned on the upstream gpui-kit story gallery.

use gpui_kit::component::{
    ActiveTheme as _, ThemeStyled as _, h_flex,
    input::{Input, InputEvent, InputState},
    sidebar::{Sidebar, SidebarCollapsible, SidebarHeader, SidebarMenu, SidebarMenuItem},
    v_flex,
};
use gpui_kit::{prelude::*, *};

use super::registry::{self, GallerySection};

pub struct GalleryPage {
    search_input: Entity<InputState>,
    sections: Vec<GallerySection>,
    matches: Vec<usize>,
    active_ix: usize,
    _subscriptions: Vec<Subscription>,
}

impl GalleryPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search sections…"));
        let _subscriptions = vec![cx.subscribe(&search_input, |this, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                this.apply_search(cx);
            }
        })];
        let sections = registry::sections(window, cx);
        let mut this = Self {
            search_input,
            sections,
            matches: Vec::new(),
            active_ix: 0,
            _subscriptions,
        };
        this.apply_search(cx);
        this
    }

    fn query(&self, cx: &App) -> String {
        self.search_input.read(cx).value().trim().to_lowercase()
    }

    fn matching_indexes(&self, query: &str) -> Vec<usize> {
        self.sections
            .iter()
            .enumerate()
            .filter(|(_, section)| section.matches_query(query))
            .map(|(ix, _)| ix)
            .collect()
    }

    /// Recompute matches for the current query and notify; keep the active
    /// section while it matches, else jump to the first match.
    fn apply_search(&mut self, cx: &mut Context<Self>) {
        let query = self.query(cx);
        self.matches = self.matching_indexes(&query);
        if !self.matches.contains(&self.active_ix) {
            if let Some(&first) = self.matches.first() {
                self.active_ix = first;
            }
        }
        cx.notify();
    }
}

impl Render for GalleryPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let matches = &self.matches;

        let nav = crate::features::tutorial::target(
            "gallery-nav",
            Sidebar::new("gallery-sidebar")
                .w_64()
                .collapsible(SidebarCollapsible::None)
                .header(
                    SidebarHeader::new()
                        .w_full()
                        .child(crate::features::tutorial::target(
                            "gallery-search",
                            div()
                                .bg(cx.theme().sidebar_accent)
                                .rounded_full_style(cx)
                                .px_2()
                                .child(
                                    Input::new(&self.search_input)
                                        .appearance(false)
                                        .cleanable(true),
                                ),
                        )),
                )
                .child(SidebarMenu::new().children(matches.iter().map(|&ix| {
                    SidebarMenuItem::new(self.sections[ix].title())
                        .active(ix == self.active_ix)
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.active_ix = ix;
                            cx.notify();
                        }))
                }))),
        );

        // `active_ix` always points into `matches` while it is non-empty;
        // apply_search and item clicks only pick matching indexes.
        let content = if matches.is_empty() {
            v_flex()
                .id("gallery-empty")
                .flex_1()
                .min_w_0()
                .h_full()
                .items_center()
                .justify_center()
                .child(
                    v_flex()
                        .id("gallery-empty-state")
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .id("gallery-empty-title")
                                .text_lg()
                                .font_weight(FontWeight::MEDIUM)
                                .child("No matching sections"),
                        )
                        .child(
                            div()
                                .id("gallery-empty-hint")
                                .text_color(cx.theme().muted_foreground)
                                .child("Clear the search to see every section."),
                        ),
                )
        } else {
            let section = &self.sections[self.active_ix];
            v_flex()
                .id(SharedString::from(format!(
                    "gallery-section-{}",
                    section.id()
                )))
                .flex_1()
                .min_w_0()
                .h_full()
                .overflow_x_hidden()
                .child(
                    v_flex()
                        .id("gallery-section-header")
                        .p_4()
                        .gap_1()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .child(
                            div()
                                .id("gallery-section-title")
                                .role(Role::Heading)
                                .text_2xl()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(SharedString::from(section.title())),
                        )
                        .child(
                            div()
                                .id("gallery-section-description")
                                .text_color(cx.theme().muted_foreground)
                                .child(SharedString::from(section.description())),
                        ),
                )
                .child(
                    div()
                        // Keyed by the section slug so element state (scroll
                        // offset) resets when switching sections.
                        .id(SharedString::from(format!(
                            "gallery-section-view-{}",
                            section.id()
                        )))
                        .flex_1()
                        .min_h_0()
                        // Single scroll owner for section content.
                        .overflow_y_scroll()
                        .child(section.view()),
                )
        };

        h_flex()
            .id("gallery-page")
            .size_full()
            // Children are full-height columns; stretch pins their headers
            // instead of cross-axis centering them.
            .items_stretch()
            .child(nav)
            .child(content)
    }
}
