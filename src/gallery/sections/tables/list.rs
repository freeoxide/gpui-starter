//! List section, ported from the upstream `ListStory`: a searchable,
//! sectioned, virtualized list with lazy loading and drag-to-reorder.

use std::{rc::Rc, time::Duration};

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, IndexPath, Selectable, StyledExt as _, ThemeStyled as _,
    button::{Button, DropdownButton},
    h_flex,
    label::Label,
    list::{List, ListDelegate, ListItem, ListState},
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};
use serde::Deserialize;

use crate::gallery::registry::GallerySection;

use super::demo::{demo_toolbar, rng_f64, rng_usize, section};

actions!(gallery_tables, [SelectedCompany]);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_tables, no_json)]
enum ListOption {
    Selectable,
    Searchable,
    Loading,
    LazyLoad,
    Draggable,
}

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_tables, no_json)]
enum ListGoTo {
    Top,
    Selected,
    Row,
    Bottom,
}

// Word lists the deterministic generator composes company names from, in
// place of the `fake` crate the upstream story drew them from.
const COMPANY_PREFIXES: &[&str] = &[
    "Apex",
    "Beacon",
    "Cedar",
    "Dunhill",
    "Everline",
    "Fathom",
    "Granite",
    "Harbor",
    "Ironwood",
    "Juniper",
    "Kestrel",
    "Lakeshore",
];
const COMPANY_SUFFIXES: &[&str] = &[
    "Systems",
    "Labs",
    "Holdings",
    "Logistics",
    "Energy",
    "Robotics",
    "Foods",
    "Media",
    "Materials",
    "Transport",
    "Analytics",
    "Outfitters",
];
const INDUSTRIES: &[&str] = &[
    "Software",
    "Retail",
    "Finance",
    "Healthcare",
    "Manufacturing",
    "Energy",
    "Transport",
    "Media",
    "Agriculture",
    "Real Estate",
];

#[derive(Clone, Default)]
struct Company {
    name: SharedString,
    industry: SharedString,
    last_done: f64,
    prev_close: f64,

    change_percent: f64,
    change_percent_str: SharedString,
    last_done_str: SharedString,
    prev_close_str: SharedString,
}

impl Company {
    fn prepare(mut self) -> Self {
        self.change_percent = (self.last_done - self.prev_close) / self.prev_close;
        self.change_percent_str = format!("{:.2}%", self.change_percent).into();
        self.last_done_str = format!("{:.2}", self.last_done).into();
        self.prev_close_str = format!("{:.2}", self.prev_close).into();
        self
    }
}

/// Where the dragged item will be inserted relative to the drop target row.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DropPosition {
    Before,
    After,
}

impl DropPosition {
    /// The insertion gap index (0..=len) this position resolves to for `row`.
    fn gap(self, row: usize) -> usize {
        match self {
            DropPosition::Before => row,
            DropPosition::After => row + 1,
        }
    }
}

#[derive(Clone)]
struct DragCompany {
    ix: IndexPath,
    name: SharedString,
}

impl Render for DragCompany {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .text_sm()
            .bg(cx.theme().accent)
            .text_color(cx.theme().accent_foreground)
            .rounded(cx.theme().radius)
            .shadow_md()
            .child(self.name.clone())
    }
}

#[derive(IntoElement)]
struct CompanyListItem {
    base: ListItem,
    company: Rc<Company>,
    selected: bool,
    drop_position: Option<DropPosition>,
}

impl CompanyListItem {
    pub fn new(id: impl Into<ElementId>, company: Rc<Company>, selected: bool) -> Self {
        CompanyListItem {
            company,
            base: ListItem::new(id).selected(selected),
            selected,
            drop_position: None,
        }
    }

    /// Make this item draggable for reordering, by using the
    /// `InteractiveElement` methods on the inner [`ListItem`].
    pub fn draggable(
        mut self,
        ix: IndexPath,
        drop_position: Option<DropPosition>,
        on_drag_start: impl Fn(&mut App) + 'static,
        on_drag_move: impl Fn(&DragMoveEvent<DragCompany>, &mut Window, &mut App) + 'static,
        on_drop: impl Fn(&DragCompany, &mut Window, &mut App) + 'static,
    ) -> Self {
        let drag = DragCompany {
            ix,
            name: self.company.name.clone(),
        };

        self.drop_position = drop_position;
        self.base = self
            .base
            .on_drag(drag, move |drag, _, _, cx| {
                on_drag_start(cx);
                cx.new(|_| drag.clone())
            })
            .on_drag_move(on_drag_move)
            .on_drop(on_drop);
        self
    }
}

impl Selectable for CompanyListItem {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl RenderOnce for CompanyListItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let text_color = if self.selected {
            cx.theme().accent_foreground
        } else {
            cx.theme().foreground
        };

        let trend_color = match self.company.change_percent {
            change if change > 0.0 => cx.theme().green,
            change if change < 0.0 => cx.theme().red,
            _ => cx.theme().foreground,
        };

        self.base
            .px_2()
            .py_1()
            .overflow_x_hidden()
            .border_1()
            .rounded(cx.theme().radius)
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .text_color(text_color)
                    .when_some(
                        self.drop_position.filter(|_| cx.has_active_drag()),
                        |this, position| {
                            let line = div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .h(rems(0.125))
                                .rounded_full_style(cx)
                                .bg(cx.theme().blue);

                            this.child(match position {
                                DropPosition::Before => line.top(rems(-0.3125)),
                                DropPosition::After => line.bottom(rems(-0.3125)),
                            })
                        },
                    )
                    .child(
                        h_flex().gap_2().child(
                            v_flex()
                                .gap_1()
                                .max_w(rems(31.25))
                                .overflow_x_hidden()
                                .flex_nowrap()
                                .child(Label::new(self.company.name.clone()).whitespace_nowrap()),
                        ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .justify_end()
                            .child(
                                div()
                                    .w(rems(4.0625))
                                    .text_color(text_color)
                                    .child(self.company.last_done_str.clone()),
                            )
                            .child(
                                h_flex().w(rems(4.0625)).justify_end().child(
                                    div()
                                        .rounded(cx.theme().radius)
                                        .whitespace_nowrap()
                                        .text_size(rems(0.75))
                                        .px_1()
                                        .text_color(trend_color)
                                        .child(self.company.change_percent_str.clone()),
                                ),
                            ),
                    ),
            )
    }
}

struct CompanyListDelegate {
    industries: Vec<SharedString>,
    companies: Vec<Rc<Company>>,
    matched_companies: Vec<Vec<Rc<Company>>>,
    selected_index: Option<IndexPath>,
    query: SharedString,
    loading: bool,
    eof: bool,
    lazy_load: bool,
    draggable: bool,
    drop_target: Option<(IndexPath, DropPosition)>,
}

impl CompanyListDelegate {
    fn prepare(&mut self, query: impl Into<SharedString>) {
        self.query = query.into();
        for companies in &mut self.matched_companies {
            companies.clear();
        }
        let companies: Vec<Rc<Company>> = self
            .companies
            .iter()
            .filter(|company| {
                company
                    .name
                    .to_lowercase()
                    .contains(&self.query.to_lowercase())
            })
            .cloned()
            .collect();
        for company in companies.into_iter() {
            if let Some(ix) = self.industries.iter().position(|s| s == &company.industry) {
                self.matched_companies[ix].push(company);
            } else {
                self.industries.push(company.industry.clone());
                self.matched_companies.push(vec![company]);
            }
        }
    }

    fn extend_more(&mut self, len: usize) {
        self.companies
            .extend((0..len).map(|_| Rc::new(random_company())));
        self.prepare(self.query.clone());
    }

    fn selected_company(&self) -> Option<Rc<Company>> {
        let ix = self.selected_index?;

        self.matched_companies
            .get(ix.section)
            .and_then(|c| c.get(ix.row))
            .cloned()
    }

    /// Record the pending drop target row, returns true if it changed.
    fn update_drop_target(&mut self, ix: IndexPath, position: Option<DropPosition>) -> bool {
        match position {
            Some(position) => {
                if self.drop_target != Some((ix, position)) {
                    self.drop_target = Some((ix, position));
                    true
                } else {
                    false
                }
            }
            None => {
                if self.drop_target.is_some_and(|(target, _)| target == ix) {
                    self.drop_target = None;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Move the company at `from` to before or after the company at `to`.
    fn move_company(&mut self, from: IndexPath, to: IndexPath, position: DropPosition) {
        self.drop_target = None;
        if from == to {
            return;
        }

        let Some(company) = self
            .matched_companies
            .get_mut(from.section)
            .filter(|companies| from.row < companies.len())
            .map(|companies| companies.remove(from.row))
        else {
            return;
        };

        let mut row = position.gap(to.row);
        if from.section == to.section && from.row < row {
            row -= 1;
        }

        if let Some(companies) = self.matched_companies.get_mut(to.section) {
            let row = row.min(companies.len());
            companies.insert(row, company);
            self.selected_index = Some(IndexPath::new(row).section(to.section));
        }
    }
}

impl ListDelegate for CompanyListDelegate {
    type Item = CompanyListItem;

    fn sections_count(&self, _: &App) -> usize {
        self.industries.len()
    }

    fn items_count(&self, section: usize, _: &App) -> usize {
        if matches!(section, 0 | 2 | 3) {
            // Return some empty sections for testing.
            return 0;
        }

        self.matched_companies[section].len()
    }

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        self.prepare(query.to_owned());
        Task::ready(())
    }

    fn confirm(
        &mut self,
        _secondary: bool,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        window.dispatch_action(Box::new(SelectedCompany), cx);
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }

    fn render_section_header(
        &mut self,
        section: usize,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<impl IntoElement> {
        let industry = self.industries.get(section)?;

        Some(
            h_flex()
                .pb_1()
                .px_2()
                .gap_2()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(Icon::new(IconName::Folder))
                .child(industry.clone())
                .child(format!("(section: {})", section)),
        )
    }

    fn render_section_footer(
        &mut self,
        section: usize,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<impl IntoElement> {
        self.industries.get(section)?;

        Some(
            div()
                .pt_1()
                .pb_5()
                .px_2()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(format!(
                    "Total {} items in section.",
                    self.matched_companies[section].len()
                )),
        )
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let selected = Some(ix) == self.selected_index;
        if let Some(company) = self.matched_companies[ix.section].get(ix.row) {
            let item =
                CompanyListItem::new(ix, company.clone(), selected).when(self.draggable, |this| {
                    // Normalize the drop target to an insertion gap, so the
                    // indicator renders at the same edge of the gap either way.
                    let count = self.matched_companies[ix.section].len();
                    let drop_position = self.drop_target.and_then(|(target, position)| {
                        if target.section != ix.section {
                            return None;
                        }

                        let gap = position.gap(target.row);
                        if gap == ix.row {
                            Some(DropPosition::Before)
                        } else if gap == count && ix.row + 1 == count {
                            Some(DropPosition::After)
                        } else {
                            None
                        }
                    });

                    let state = cx.entity().downgrade();
                    this.draggable(
                        ix,
                        drop_position,
                        // A drag may end without a drop (e.g. released outside the
                        // list), clear the stale drop target when a new one starts.
                        move |cx| {
                            _ = state.update(cx, |this, cx| {
                                if this.delegate_mut().drop_target.take().is_some() {
                                    cx.notify();
                                }
                            });
                        },
                        cx.listener(move |this, e: &DragMoveEvent<DragCompany>, _, cx| {
                            let bounds = e.bounds;
                            let from = e.drag(cx).ix;
                            let position = if bounds.contains(&e.event.position) {
                                let position = if e.event.position.y < bounds.center().y {
                                    DropPosition::Before
                                } else {
                                    DropPosition::After
                                };

                                // Gaps adjacent to the dragged row are no-op moves.
                                let gap = position.gap(ix.row);
                                if from.section == ix.section
                                    && (gap == from.row || gap == from.row + 1)
                                {
                                    None
                                } else {
                                    Some(position)
                                }
                            } else {
                                None
                            };

                            if this.delegate_mut().update_drop_target(ix, position) {
                                cx.notify();
                            }
                        }),
                        cx.listener(move |this, drag: &DragCompany, _, cx| {
                            let Some(position) = this
                                .delegate()
                                .drop_target
                                .filter(|(target, _)| *target == ix)
                                .map(|(_, position)| position)
                            else {
                                return;
                            };

                            this.delegate_mut().move_company(drag.ix, ix, position);
                            cx.notify();
                        }),
                    )
                });

            return Some(item);
        }

        None
    }

    fn loading(&self, _: &App) -> bool {
        self.loading
    }

    fn has_more(&self, _: &App) -> bool {
        if self.loading {
            return false;
        }

        !self.eof
    }

    fn load_more_threshold(&self) -> usize {
        150
    }

    fn load_more(&mut self, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        if !self.lazy_load {
            return;
        }

        cx.spawn_in(window, async move |view, window| {
            // Simulate network request, delay 1s to load data.
            window
                .background_executor()
                .timer(Duration::from_secs(1))
                .await;

            _ = view.update_in(window, move |view, window, cx| {
                let query = view.delegate().query.clone();
                view.delegate_mut().extend_more(200);
                _ = view.delegate_mut().perform_search(&query, window, cx);
                view.delegate_mut().eof = view.delegate().companies.len() >= 6000;
            });
        })
        .detach();
    }
}

fn random_company() -> Company {
    let name = format!(
        "{} {}",
        COMPANY_PREFIXES[rng_usize(0, COMPANY_PREFIXES.len())],
        COMPANY_SUFFIXES[rng_usize(0, COMPANY_SUFFIXES.len())]
    );
    let last_done = rng_f64(0.0, 999.0);
    let prev_close = last_done * rng_f64(-0.1, 0.1);

    Company {
        name: name.into(),
        industry: INDUSTRIES[rng_usize(0, INDUSTRIES.len())].into(),
        last_done,
        prev_close,
        ..Default::default()
    }
    .prepare()
}

pub struct ListSection {
    focus_handle: FocusHandle,
    company_list: Entity<ListState<CompanyListDelegate>>,
    selected_company: Option<Rc<Company>>,
    selectable: bool,
    searchable: bool,
}

impl ListSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut delegate = CompanyListDelegate {
            industries: vec![],
            matched_companies: vec![],
            companies: vec![],
            selected_index: Some(IndexPath::default()),
            query: "".into(),
            loading: false,
            eof: false,
            lazy_load: false,
            draggable: false,
            drop_target: None,
        };
        delegate.extend_more(100);

        let company_list = cx.new(|cx| ListState::new(delegate, window, cx).searchable(true));

        // One refresh pass, as if the quotes had just streamed in.
        cx.spawn(async move |this, cx| {
            this.update(cx, |this, cx| {
                this.company_list.update(cx, |picker, _| {
                    picker
                        .delegate_mut()
                        .companies
                        .iter_mut()
                        .for_each(|company| {
                            let mut new_company = random_company();
                            new_company.name = company.name.clone();
                            new_company.industry = company.industry.clone();
                            *company = Rc::new(new_company);
                        });
                    picker.delegate_mut().prepare("");
                });
                cx.notify();
            })
            .ok();
        })
        .detach();

        Self {
            focus_handle: cx.focus_handle(),
            searchable: true,
            selectable: true,
            company_list,
            selected_company: None,
        }
    }

    fn selected_company(&mut self, _: &SelectedCompany, _: &mut Window, cx: &mut Context<Self>) {
        let picker = self.company_list.read(cx);
        if let Some(company) = picker.delegate().selected_company() {
            self.selected_company = Some(company);
            cx.notify();
        }
    }

    fn toggle_selectable(&mut self, selectable: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.selectable = selectable;
        self.company_list.update(cx, |list, cx| {
            list.set_selectable(self.selectable, cx);
        })
    }

    fn toggle_searchable(&mut self, searchable: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.searchable = searchable;
        self.company_list.update(cx, |list, cx| {
            list.set_searchable(self.searchable, cx);
        })
    }
}

impl Render for ListSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let lazy_load = self.company_list.read(cx).delegate().lazy_load;
        let draggable = self.company_list.read(cx).delegate().draggable;
        let loading = self.company_list.read(cx).delegate().loading;
        let selectable = self.selectable;
        let searchable = self.searchable;

        v_flex()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::selected_company))
            .on_action(
                cx.listener(|this, action: &ListOption, window, cx| match action {
                    ListOption::Selectable => {
                        this.toggle_selectable(!this.selectable, window, cx);
                    }
                    ListOption::Searchable => {
                        this.toggle_searchable(!this.searchable, window, cx);
                    }
                    ListOption::Loading => {
                        this.company_list.update(cx, |list, cx| {
                            list.delegate_mut().loading = !list.delegate().loading;
                            cx.notify();
                        });
                    }
                    ListOption::LazyLoad => {
                        this.company_list.update(cx, |list, cx| {
                            list.delegate_mut().lazy_load = !list.delegate().lazy_load;
                            cx.notify();
                        });
                    }
                    ListOption::Draggable => {
                        this.company_list.update(cx, |list, cx| {
                            list.delegate_mut().draggable = !list.delegate().draggable;
                            cx.notify();
                        });
                    }
                }),
            )
            .on_action(
                cx.listener(|this, action: &ListGoTo, window, cx| match action {
                    ListGoTo::Top => {
                        this.company_list.update(cx, |list, cx| {
                            list.scroll_to_item(
                                IndexPath::default(),
                                ScrollStrategy::Top,
                                window,
                                cx,
                            );
                        });
                    }
                    ListGoTo::Selected => {
                        this.company_list.update(cx, |list, cx| {
                            list.scroll_to_selected_item(window, cx);
                        });
                    }
                    ListGoTo::Row => {
                        this.company_list.update(cx, |list, cx| {
                            list.scroll_to_item(
                                IndexPath::new(1).section(5),
                                ScrollStrategy::Center,
                                window,
                                cx,
                            );
                        });
                    }
                    ListGoTo::Bottom => {
                        this.company_list.update(cx, |list, cx| {
                            let last_section = list.delegate().sections_count(cx).saturating_sub(1);
                            let last_row = list
                                .delegate()
                                .items_count(last_section, cx)
                                .saturating_sub(1);
                            list.scroll_to_item(
                                IndexPath::default().section(last_section).row(last_row),
                                ScrollStrategy::Top,
                                window,
                                cx,
                            );
                        });
                    }
                }),
            )
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .child(demo_toolbar(vec![
                DropdownButton::new("list-go-to")
                    .button(Button::new("list-go-to-trigger").label("Go To"))
                    .dropdown_menu(|menu, _, _| {
                        menu.menu("Top", Box::new(ListGoTo::Top))
                            .menu("Selected", Box::new(ListGoTo::Selected))
                            .menu("Section 5, Row 1", Box::new(ListGoTo::Row))
                            .menu("Bottom", Box::new(ListGoTo::Bottom))
                    })
                    .into_any_element(),
                DropdownButton::new("list-options")
                    .button(Button::new("list-options-trigger").label("Options"))
                    .dropdown_menu(move |menu, _, _| {
                        menu.menu_with_check(
                            "Selectable",
                            selectable,
                            Box::new(ListOption::Selectable),
                        )
                        .menu_with_check("Searchable", searchable, Box::new(ListOption::Searchable))
                        .menu_with_check("Loading", loading, Box::new(ListOption::Loading))
                        .menu_with_check("Lazy Load", lazy_load, Box::new(ListOption::LazyLoad))
                        .menu_with_check(
                            "Draggable",
                            draggable,
                            Box::new(ListOption::Draggable),
                        )
                    })
                    .into_any_element(),
            ]))
            .child(
                section("list-companies", "Companies")
                    .description(
                        "Search narrows the list, drag rows to reorder them, and Enter confirms \
                        the selection.",
                    )
                    .w_full()
                    .v_flex()
                    .child(
                        List::new(&self.company_list)
                            .h(rems(30.))
                            .w_full()
                            .p_2()
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius),
                    )
                    .child(
                        h_flex()
                            .min_h_6()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .when_some(self.selected_company.clone(), |this, company| {
                                this.child(format!("Last confirmed: {}", company.name))
                            }),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "list",
        "List",
        "A searchable, sectioned list with lazy loading and drag-to-reorder.",
        ListSection::view(window, cx),
    ));
}
