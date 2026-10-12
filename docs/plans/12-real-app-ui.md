# 12 Real App Screen (Analytics Workspace) Plan

## Goal

A new "Analytics" sidebar page that reads as a production analytics workspace:
KPI header cards, date-range and segment filters driving two charts plus a
sessions bar chart, a sortable/row-selectable `DataTable` drill-down, and a
detail `Sheet` per row with a `DescriptionList` and a 90-day sparkline. Every
region (KPIs, charts, table) has loading/skeleton, empty, and error states, a
density toggle switches the whole page between compact and comfortable, and the
full workflow is keyboard-operable (Tab through controls, arrow keys in the
table, Enter opens the detail sheet, Esc closes it). Pure composition of
existing gpui-kit components over fixture JSON — no new crates, no new
infrastructure, no new config fields.

## Dependencies

No plan ids consumed. This plan is a leaf over the repo's existing seams:

- Page seam (survey `docs/plans/_survey/seams.md` §1, re-verified this session):
  `src/shell/sidebar.rs`, `src/shell/route.rs`,
  `src/shell/root/app_root/state.rs`, `src/features/pages/mod.rs`,
  `tests/e2e_navigation.rs`, both locale catalogs.
- gpui-kit 0.7.1 component surface only (`gpui_kit::component::*`,
  `gpui_kit::*`); no `use gpui::` anywhere (`tests/qa_migration.rs:386-405`
  rule, survey §9).

This plan is itself a foundation: its Interface Contract is the `AnalyticsPage`
type + `Page::Analytics` navigation identity, which sibling showcase plans may
embed or deep-link to. All 20 plans are now on disk (`ls docs/plans/`); none of
13-20 references `AnalyticsPage|Page::Analytics|nav_analytics` (grep-verified),
so this contract has no current consumer to keep in sync.

## Research Digest

Crate sources on disk:
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/{gpui-kit-0.7.1,
gpui-component-0.7.1, gpui-base-0.7.1, gpui-pre-0.3.8, gpui-kit-assets-0.7.1}`
(located with `ls ~/.cargo/registry/src/*/ | grep -E 'gpui-pre-0.3.8|gpui-kit-0.7.1'`).
Repo files relative to `/home/thragg/fo/gpui-starter`. No cargo gate was run in
this planning session (doc-only scope); every API claim below is source-read.

Page seam (all read this session):

- `Page` enum + `title`/`title_key`/`host`/`icon`/`all` exhaustive matches:
  `src/shell/sidebar.rs:4-105`. `Page::Gallery` is last in `all()`
  (`sidebar.rs:92-105`); appending keeps every existing `secondary-N` digit
  binding stable (`NavigateToPage(usize)` is positional,
  `src/shell/root/app_root/state.rs:144-166` binds only `i < 9`).
- `VALID_HOSTS` list: `src/shell/route.rs:10-21`; parse match falls back to
  `Page::from_host` (`route.rs:154-161`).
- AppRoot wiring: field (`state.rs:22-31`), construction (`state.rs:47-56`),
  view arm (`state.rs:211-223`), imports (`state.rs:6-9`).
- Page count assertion: `tests/e2e_navigation.rs:14-19` (`assert_eq!(all.len(),
  10, ...)`, with the inline comment "nine feature pages + Gallery"); the
  `expected` page array at `:24-34`; deep-link case table at `:59-77`.
- Locale facade: `crate::i18n::localize("key")` / `crate::i18n::localize_with_args(
  "key", &[("name", value)])` with `%{name}` placeholders
  (`src/services/i18n.rs:3-13`). Catalog key sets must match
  (`tests/qa_localization.rs:188-198` via survey §5). Existing `nav_*` block:
  `locales/en.yml:53-62`, `locales/zh-CN.yml:53-62`.
- The shell already renders the page H1 and scrolls the page region
  (`src/shell/root/app_root/render.rs:140-170`: `#page-title` heading level 1,
  `#page` region with `overflow_y_scroll()`) — the page must not add its own H1
  or its own scroll container.
- Serde is a common dependency (`Cargo.toml:25-26`, `:158-159`); fixture
  parsing needs no manifest change.

Charts (gpui-component 0.7.1):

- `LineChart<T, X, Y>` builders verified:
  `new(impl IntoIterator<Item = T>)` (`chart/line_chart.rs:57`),
  `x(fn)` (`:176`), `y(fn)` (`:181`), `stroke(Hsla)` (`:186`),
  `linear()` (`:196`), `dot()` (`:206`), `x_axis(bool)` (`:219`),
  `grid(bool)` (`:224`), `y_domain(min, max)` (`:236`), `point_count` (`:249`),
  `y_axis(bool)` (`:257`), `y_tick_count` (`:279`), `y_tick_format` (`:285`),
  `x_tick_count` (`:299`), `interactive(bool)` (`:100`), `id` (`:88`).
- `AreaChart` mirrors those and takes repeated `.y()/.stroke()/.fill()` triples
  for multi-series stacking; repo usage `src/gallery/sections/charts/chart.rs:523-534`.
  Gradient fill precedent: `system_monitor.rs:289-293`.
- `BarChart` builders (plan-04 digest, verified against source):
  `band(fn)`/`value(fn)` (`chart/bar_chart.rs:223-229`), `name` (`:172`),
  `fill` (`:253-255`, closure arity 4:
  `Fn(&T, Bounds<f32>, Bounds<f32>, BarAlignment) -> Bg`; gallery passes
  `|_, _, _, _|`, `chart.rs:918,923`), `value_axis(bool)` (`:352`),
  `value_tick_count` (`:365`), `value_tick_format` (`:382`),
  `band_tick_count` (`:407`), `id` (`:134`); horizontal bars via
  `.alignment(BarAlignment::Left)` (usage `chart.rs:919-929`).
- Chart color tokens `cx.theme().chart_1..chart_5`
  (`gpui-component-0.7.1/src/theme/theme_color.rs:135` region; usage
  `system_monitor.rs:289-292`).
- Kit chart appear animation is built in (`ChartAppear`, survey §f); no plan
  work needed for motion.

Table (gpui-component 0.7.1, `table/`):

- `TableState::new(delegate, window, cx)` (`table/state.rs:286`); pub flags
  `row_selectable` (default `true`, `state.rs:308`), `col_selectable`,
  `col_movable`, `sortable` (`state.rs:224-254`); `delegate()`/`delegate_mut()`
  (`:324-329`), `refresh` (`:407`), `scroll_to_row` (`:412`),
  `selected_row() -> Option<usize>` (`state.rs:496`, row-mode only),
  `set_selected_row(row_ix, cx)` (`state.rs:501` — sets `SelectionMode::Row`,
  clears the right-click mark, and scrolls the row into view),
  `selected_cell() -> Option<(usize, usize)>` (`:571`),
  `set_selected_cell` (`:590`), `clear_selection` (`:617`).
- `impl EventEmitter<TableEvent> for TableState<D>` (`state.rs:2413`) —
  `TableEvent::{SelectRow(usize), DoubleClickedRow(usize), RightClickedRow(
  Option<usize>), ClearSelection, ...}` (`state.rs:80-120`). `impl Focusable`
  (`state.rs:2405`).
- Subscribing with a `Window` in the handler needs `subscribe_in`:
  `Context::subscribe` handlers are 4-param `(this, emitter, event, cx)` —
  no window (`gpui-pre-0.3.8/src/app/context.rs:94-97`); the 5-param
  `cx.subscribe_in(&emitter, window, |this, _, event, window, cx| ..)` is at
  `context.rs:341-354`; repo precedent `src/gallery/sections/shell/todo_list.rs:76`.
  Window-free subscriptions (the two `SelectEvent::Confirm` listeners) stay
  plain `cx.subscribe` (`pickers/select.rs:371` precedent).
- `TableDelegate` hooks used: `columns_count`, `rows_count`, `column`,
  `render_th`, `render_tr`, `render_td`, `perform_sort`, `context_menu`,
  `loading`. All verified as implemented by the repo's own delegates
  (`src/gallery/sections/tables/data_table.rs:464-890`, system_monitor
  delegate `src/gallery/sections/charts/system_monitor.rs:115-179`).
- Keyboard: `DataTable` binds up/down/left/right/home/end/pageup/pagedown/tab
  and escape under key context `"DataTable"` (`table/data_table.rs:15-28`,
  `.key_context(CONTEXT)` at `:156`) — full grid navigation is free.
- `DataTable::new(&Entity<TableState<D>>)` with `.bordered(bool)`, `.stripe(
  bool)`, `.small()`/`.with_size(Size)` (usage `data_table.rs:1369-1371`,
  `system_monitor.rs:320-323`).
- `Column::new(key, name)` + `.width(f64)`, `.sortable()`, `.text_right()`,
  `.p_0()`, `.fixed(ColumnFixed::Left)` (usage `data_table.rs:336-356`).
- App-scoped extra key binding on the table context is safe: upstream binds no
  `enter` (`data_table.rs:18-28` is the complete binding list), and GPUI
  dispatches an action along the focus chain, so a handler on the page wrapper
  receives it only while focus is inside this page's table
  (`on_action`-on-ancestor precedent: `data_table.rs:1082-1134` handles
  gallery actions fired from inside the table).

Select (gpui-component 0.7.1):

- `SelectState::new(items: Vec<SharedString>, selected: Option<IndexPath>,
  window, cx)` (repo usage `src/gallery/sections/pickers/select.rs:398-404`);
  `SelectEvent::Confirm(Option<Value>)` (`select.rs:71-75`); subscribe
  precedent `pickers/select.rs:371`; `selected_index(cx) -> Option<IndexPath>`
  with pub `row` field (`select.rs:367`, `gpui-base-0.7.1/src/index_path.rs:9-54`).
- `Select::new(&Entity<SelectState<D>>)` builders: `.placeholder`, `.icon`,
  `.title_prefix`, `.cleanable`, `.disabled` (`select.rs:659-735`);
  `.with_size` via `impl Sizable` (`select.rs:741-748`).

Sheet (gpui-component 0.7.1):

- `WindowExt for Window`: `open_sheet(cx, build)` (default right placement,
  `window_ext.rs:15-20`, `:94-98`), `has_active_sheet(cx)` (`:25`),
  `close_sheet(cx)` (`:28`); build closure `Fn(Sheet, &mut Window, &mut App)
  -> Sheet` (`root.rs:325-349`).
- `Sheet::new(&mut Window, &mut App)` + `.title`/`.footer`/`.size`/`.resizable`/
  `.overlay`/`.overlay_closable` (`sheet.rs:57-113`); focus is trapped and the
  previous focus handle saved on open and restored on close
  (`root.rs:331-347`, `:353-366`). `open_sheet_at` replaces any active sheet
  (`root.rs:325-349`) — hence the has_active_sheet guard from plan 04.
- `.size(px(420.))` width precedent: gallery `overlays/sheet.rs` (plan-04
  digest `:252`).

State components:

- `Skeleton::new()` + `Styled` (`size_12`, `w_full`, `h_4`, `w_2_3`) +
  `ThemeStyled::rounded_full_style(cx)` / `.rounded(cx.theme().radius)`
  (`src/gallery/sections/loading/skeleton.rs:3-7, 36-47`).
- `Spinner::new().xsmall()` (`src/gallery/sections/tables/data_table.rs:1394`).
- `Empty::new().header(EmptyHeader::new().media(EmptyMedia::new()
  .with_variant(EmptyMediaVariant::Icon).child(Icon::new(..))).title(
  EmptyTitle::new().child(..)).description(EmptyDescription::new().child(..)))
  .content(EmptyContent::new().child(Button..))`
  (`src/gallery/sections/loading/empty.rs:108-158`, imports `:3-15`).
- `DescriptionList::new().columns(n).layout(Axis::Horizontal).bordered(bool)
  .with_size(Size)` + `DescriptionItem::new(label).value(element).span(n)` /
  `DescriptionItem::Separator`
  (`src/gallery/sections/text/description_list.rs:186-207`; `Axis` re-exported
  through `gpui_kit::*` — `gpui-component-0.7.1/src/description_list.rs:4`).
- Segmented `TabBar::new(id).segmented().selected_index(n).on_click(listener)
  .child(Tab::new().label(..))` (`system_monitor.rs:413-423`).
- `Button::new(id).outline().icon(IconName::Close).label(..).tooltip(..)`
  (`tooltip` on buttons: `src/gallery/sections/editor/workspace.rs:444`;
  outline+icon: `empty.rs:227-231`).
- `DropdownButton::new(id).button(Button..).dropdown_menu(|menu, _, _| menu
  .menu_with_check(label, checked, Box::new(Action)))`
  (`data_table.rs:1024-1050, 1233-1257`).

Actions:

- `actions!(ns, [Unit, Variants])` and `#[derive(Action, Clone, PartialEq, Eq,
  Deserialize)] #[action(namespace = ns, no_json)] enum ...` with unit variants
  (`data_table.rs:22-94`); `.on_action(cx.listener(..))` on the page wrapper
  (`data_table.rs:1082-1134`).
- Context-scoped binding precedent: `KeyBinding::new("up", SelectUp,
  Some(CONTEXT))` in kit (`data_table.rs:19`); app-level `cx.bind_keys` batch
  (`app_root/state.rs:145-166`).

A11y:

- `crate::accessibility::A11yExt` (`src/services/accessibility.rs:23`, re-export
  `src/lib.rs:60`): `.a11y(Role::Heading, label)`, `.a11y_live(
  accesskit::Live::Polite)`, `.aria_level(n)`, `.aria_size_of_set(n)`,
  `.aria_position_in_set(n)` (usage `src/features/pages/notifications.rs:48-58,
  84-124`; `Role`/`accesskit` arrive via the `gpui_kit::*` glob).

Icons: the component enum the app registers (`gpui_kit::component::IconName`)
is generated ONLY from the default subset,
`gpui-kit-assets-0.7.1/default-icons.txt` (default 106; `chart-pie`,
`rotate-cw`, `arrow-up`, `arrow-down`, `inbox`, `triangle-alert`, `search` all
verified present there and in the generated enum at
`target/debug/build/gpui-kit-assets-*/out/icon_name.rs`). `ChartColumn`,
`TrendingUp`, `TrendingDown`, and `Table2` exist only in the full
`gpui_kit::assets::IconName` catalog whose bytes the default asset source does
not register — unusable here. This plan therefore uses `IconName::ChartPie`
(section + sidebar), `ArrowUp`/`ArrowDown` (KPI deltas), `Inbox` (table empty
state), plus `RotateCw`, `TriangleAlert`, `Search`. No `filter` icon exists in
either enum; do not use one.

Fixture pattern: `fixture::<T: Deserialize>(include_str!("fixtures/x.json"))`
(`chart.rs:1479-1481`), plain `#[derive(Clone, Deserialize)]` row structs
(`chart.rs:57-83`), `include_str!` paths relative to the source file.

Crate picks: none. Zero new dependencies; serde/serde_json already common
(`Cargo.toml:25-26`).

## Architecture

Entities and state ownership (one `Render` view per coding guide
"Entity boundaries"):

- `WorkspaceData` — plain owned value (`Rc` shared into sub-views): parsed
  `daily-series.json` (90 rows) + `channels.json` (48 rows). Immutable after
  `AnalyticsPage::new`. Owns all slicing/filtering/KPI math as pure methods.
- `AnalyticsPage` — the only entity. Owns:
  - `data: Rc<WorkspaceData>`;
  - `range: DateRange` (enum), `segment: Segment` (enum) — the filter pair;
  - `range_select` / `segment_select: Entity<SelectState<Vec<SharedString>>>`
    — kit-owned dropdown state; page reads the confirmed index and writes the
    filter enums (controlled value; the enums are the source of truth);
  - `density: Density` (`Compact` | `Comfortable`) — view-local, not persisted;
  - `preview: PreviewState` (`Live` | `Loading` | `Empty` | `Error`) —
    state-preview override for all three regions at once, default `Live`;
  - `phase: Phase` (`Initial` | `Refreshing` | `Ready`) — natural load phase,
    shared by all regions because a filter change reloads everything;
  - `committed: bool` — true once `commit_reload` has run once (distinguishes
    the skeleton `Initial` phase from a `Refreshing` reload that keeps content);
  - `table: Entity<TableState<ChannelTableDelegate>>` — kit-owned selection,
    sort, scroll; delegate owns the filtered row list;
  - `selected_key: Option<ChannelKey>` — domain identity of the drill-down
    row, survives filtering/reordering (design guide requirement). On reload
    the page remaps the key to its new row index and restores kit selection
    with `TableState::set_selected_row(row_ix, cx)` (`state.rs:501`); when the
    key no longer exists, `clear_selection` runs instead;
  - `_subscriptions: Vec<Subscription>` (2 selects, 1 table),
    `_reload_task: Task<()>`.
- `ChannelDetail` — short-lived `Render` entity created inside the sheet build
  closure; owns its `Phase` + the `ChannelRow` + `Rc<WorkspaceData>` for the
  sparkline. Dies with the sheet.

Events and data flow:

1. User commits a Select → `SelectEvent::Confirm` → page listener maps
   `selected_index(cx).row` back to the enum, updates `range`/`segment`,
   calls `reload(cx)`.
2. `reload(cx)`: sets `phase = Refreshing` (or `Initial` when no committed data
   yet), sets delegate `loading = true`, notifies, spawns
   `cx.spawn` → `background_executor().timer(350 ms)` → `this.update` →
   `commit_reload(cx)` (timer/detach shape copied from
   `system_monitor.rs:217-230`; wasm-safe).
3. `commit_reload(cx)`: recompute window + segment rows from `data`, push rows
   into the delegate (`set_rows` + `table.refresh`), then restore selection:
   `selected_key` remapped to its new row index → `set_selected_row(ix, cx)`;
   key gone → `clear_selection(cx)`. `phase = Ready`, notify. Tests call
   `commit_reload` directly (no timer dependency).
4. Row click / arrow move → `TableEvent::SelectRow(ix)` → page records
   `selected_key` from the delegate row (domain identity).
5. Row double-click, row context-menu "View details…", or Enter (action
   `OpenChannelDetails`, bound under context `"DataTable"`) →
   `open_detail(window, cx)`: resolves the key from `selected_key` (fallback:
   the delegate's recorded `menu_row`), then `window.open_sheet(...)` guarded
   by `has_active_sheet` with a `ChannelDetail` view.
6. `ChannelDetail` runs its own 300 ms skeleton phase → Ready; "Try again"
   re-runs it after the sheet's Failed override is irrelevant (detail has no
   natural error; see Risks).
7. Refresh button → `reload(cx)` again.

Module tree:

```text
src/features/pages/analytics/
├── mod.rs            AnalyticsPage view, actions!, Enter binding, region render
├── mod.test.rs       page construction, commit_reload, sheet open, overrides
├── model.rs          fixtures structs, Segment/DateRange/Density, WorkspaceData
├── model.test.rs     window slicing, segment math, KPI deltas, row counts
├── format.rs         compact/money/percent/area_gradient helpers
├── format.test.rs
├── table.rs          ChannelKey, ChannelTableDelegate
├── table.test.rs     sort per column, counts, key lookup
├── detail.rs         ChannelDetail sheet view (DescriptionList + sparkline)
├── detail.test.rs
└── fixtures/
    ├── daily-series.json   90 days x {date, organic, paid, social, direct, sessions, revenue}
    └── channels.json       48 rows x {page, channel, visitors, sessions, revenue, conversion, avg_session_secs}
```

## Interface Contract

Public surface consumers get (everything else `pub(crate)` inside the module):

```rust
// Path: gpui_starter::views::AnalyticsPage
// (features/pages/mod.rs: `mod analytics; pub use analytics::AnalyticsPage;`)
pub struct AnalyticsPage { /* private fields */ }

impl AnalyticsPage {
    /// Full analytics workspace view. Parses the fixture JSON once (panics on
    /// malformed fixtures, same contract as gallery `fixture()`), starts in
    /// the Initial skeleton phase, and kicks the first reload.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self;
}

// Navigation identity (crate::sidebar::Page):
//   Page::Analytics appended last in Page::all() (index 10)
//   .title()      == "Analytics"
//   .title_key()  == "nav_analytics"   (catalog key in BOTH locales)
//   .host()       == "analytics"       (deep link gpui-starter://analytics)
//   .icon()       == gpui_kit::component::IconName::ChartPie
// AppRoot wires Entity<AnalyticsPage> and serves it for Page::Analytics.
```

Guarantees for dependents: constructing `AnalyticsPage::new` never performs I/O
beyond `include_str!` (compile-time embedded); no globals, no events on
`AppEventQueue`, no config reads/writes, no capability rows; removing the
module (Removal Recipe) leaves zero residue. Deep link
`gpui-starter://analytics` routes to the page on every platform including wasm
(hash `#/analytics`, `route.rs:99-118`).

## Module Layout

Sibling tests wired with the repo convention `#[cfg(test)] #[path =
"<name>.test.rs"] mod <name>_test;` (`src/shell/route.rs:171-173`).

| File | Purpose |
| --- | --- |
| `src/features/pages/analytics/mod.rs` | `AnalyticsPage`, `actions!`, Enter binding, filter toolbar, KPI/chart/table region rendering with skeleton/empty/error branches |
| `src/features/pages/analytics/model.rs` | `DailyRow`, `ChannelRow`, `Segment`, `DateRange`, `Density`, `ChannelKey`, `Kpi`, `WorkspaceData::load` + pure slice/filter/KPI/series methods |
| `src/features/pages/analytics/format.rs` | `compact`, `money`, `percent`, `area_gradient` — local copies of the gallery helpers (gallery is removable; no cross-import) |
| `src/features/pages/analytics/table.rs` | `ChannelTableDelegate` (7 columns, sort, context menu, stable row ids) |
| `src/features/pages/analytics/detail.rs` | `ChannelDetail` sheet body: skeleton phase, `DescriptionList`, 90-day `LineChart` sparkline |
| `src/features/pages/analytics/fixtures/daily-series.json` | 90-day per-segment visitor/session/revenue series |
| `src/features/pages/analytics/fixtures/channels.json` | 48 page×channel drill-down rows |
| `src/features/pages/analytics/*.test.rs` | Unit tests per module (see phases) |

## Seam Touches

| # | File | Change | Why minimal |
| --- | --- | --- | --- |
| 1 | `src/features/pages/mod.rs` | `mod analytics;` beside the other page `mod` lines (after `mod about;`; no order is enforced) + `pub use analytics::AnalyticsPage;` in the re-export block | Two lines, the standard page mount (CONTRIBUTING.md:63-69) |
| 2 | `src/shell/sidebar.rs` | `Analytics` variant (after `Gallery`, `sidebar.rs:15`); arms in `title` (`:30`), `title_key` (`:47`), `host` (`:68`), `icon` (`:88`), `all()` (`:103`) | The exhaustive matches force exactly these five edits |
| 3 | `src/shell/route.rs` | `Page::Analytics.host(),` appended to `VALID_HOSTS` (`route.rs:10-21`) | One line; parser handles the rest via `Page::from_host` |
| 4 | `src/shell/root/app_root/state.rs` | `AnalyticsPage` in the `views` import (`:6-9`); field `analytics_page: Entity<AnalyticsPage>` (`:31`); construct `cx.new(|cx| AnalyticsPage::new(window, cx))` (`:56`); arm `Page::Analytics => self.analytics_page.clone().into()` (`:222`) | The four spots every page touches |
| 5 | `tests/e2e_navigation.rs` | count `10` → `11` (`:14-19`) and update the count's inline comment ("nine feature pages + Gallery") to match; add `Page::Analytics` to the `expected` array (`:24-34`); add deep-link case `("gpui-starter://analytics", AppRoute::Page(Page::Analytics))` to `test_route_parsing` (`:59-77`) | Mechanical test updates the page-count rule requires (survey §9) |
| 6 | `locales/en.yml`, `locales/zh-CN.yml` | 56 new `analytics_*`/`nav_analytics` keys (i18n section) appended in the `nav_*` + page-key region of both files | Lockstep rule `tests/qa_localization.rs:188-198` |

Untouched on purpose: `Cargo.toml` / `Cargo.lock` (no deps), `src/lib.rs`
(`pub use features::pages as views` is automatic, `lib.rs:45`), command palette
(no command — the sidebar page is the only entry point), capability registry
(no platform capability), `src/app/init.rs`, `src/ui/` (nothing reusable beyond
this page yet — gallery-removal independence beats premature sharing).

### Removal Recipe

Mirror of `src/gallery/mod.rs:2-7`:

1. Delete `src/features/pages/analytics/` entirely.
2. Revert `src/features/pages/mod.rs`: drop `mod analytics;` and the
   `pub use analytics::AnalyticsPage;` line.
3. Revert `src/shell/sidebar.rs`: remove the `Analytics` variant and its five
   match arms plus the `all()` entry (exhaustive matches point at each).
4. Revert `src/shell/route.rs`: remove the `VALID_HOSTS` entry.
5. Revert `src/shell/root/app_root/state.rs`: drop the import, the
   `analytics_page` field, its construction, and the
   `unchecked_active_page_view` arm.
6. Revert `tests/e2e_navigation.rs`: count back to 10, remove `Page::Analytics`
   from `expected`, remove the deep-link case.
7. Delete the 56 keys from both `locales/en.yml` and `locales/zh-CN.yml`.
8. Run `cargo fmt --all -- --check`, `cargo clippy --workspace
   --all-features`, `cargo test --workspace --all-features` — no runtime state
   outlives the module (the sheet dies with its window; no globals touched).

## Implementation Phases

## Phase 1: Fixtures, model, formatting

### Step 1.1: Locale keys

Files: `locales/en.yml`, `locales/zh-CN.yml`.
Build: append all 56 keys from the i18n section to both files (quoted
scalars, top-level snake_case, parser rules `tests/qa_localization.rs:20-71`);
place `nav_analytics` inside the existing `nav_*` block (`en.yml:53-62`), the
rest after it. No nesting.
Tests: `cargo test -p gpui-starter --test qa_localization`.
Done-check: that command exits 0 and both files have identical key counts
(`diff <(grep -c ':' locales/en.yml) <(grep -c ':' locales/zh-CN.yml)`).

### Step 1.2: Fixture JSON

Files: `src/features/pages/analytics/fixtures/daily-series.json` (new),
`.../channels.json` (new).
Build:
- `daily-series.json`: JSON array of exactly 90 objects, dates
  `"2025-07-01".."2025-09-28"` inclusive, strictly increasing, fields
  `{ "date": string, "organic": number, "paid": number, "social": number,
  "direct": number, "sessions": number, "revenue": number }`. Generate the
  numbers with a deterministic formula (write the file by hand or with a
  throwaway script; the committed JSON is the artifact):
  for day index `n` (0-based), `wave(n, base) = round(base * (1 + 0.35 *
  sin(2πn/30) + 0.12 * sin(2πn/7)))`; `organic = wave(n, 4200)`,
  `paid = wave(n, 2300)`, `social = wave(n, 1500)`, `direct = wave(n, 3100)`,
  `visitors_total = organic + paid + social + direct`, `sessions = round(
  visitors_total * 1.6)`, `revenue = round(visitors_total * 9.2 * 100) / 100`.
  All values ≥ 0, one decimal max on `revenue`.
- `channels.json`: JSON array of exactly 48 objects — 12 pages × 4 channels
  (`organic`, `paid`, `social`, `direct`). Pages (exact strings):
  `"/dashboard"`, `"/pricing"`, `"/docs/getting-started"`, `"/docs/charts"`,
  `"/blog/analytics-101"`, `"/blog/gpui-deep-dive"`, `"/changelog"`,
  `"/download"`, `"/enterprise"`, `"/integrations"`, `"/plugins"`,
  `"/support"`. Fields
  `{ "page": string, "channel": string, "visitors": number, "sessions":
  number, "revenue": number, "conversion": number, "avg_session_secs":
  number }` with plausible descending magnitudes per page (page index 0
  largest): `visitors = wave(i*4 + channel_ix, 2600 - 180*i)` (i = page
  index), `sessions = round(visitors * (1.4 + 0.1 * channel_ix))`,
  `revenue = round(visitors * (7 + channel_ix) * 100) / 100`,
  `conversion = round((2.0 + 0.4 * channel_ix + 0.1 * (11 - i)) * 10) / 10`,
  `avg_session_secs = 120 + 9 * (11 - i) + 15 * channel_ix`.
Tests: none here (model tests in Step 1.4 pin the schema).
Done-check: `python3 -m json.tool` (or `jq length`) on both files succeeds;
`jq length` prints 90 and 48.

### Step 1.3: `format.rs`

Files: `src/features/pages/analytics/format.rs` (new),
`.../analytics/mod.rs` (new, minimal: doc comment naming the removal seams +
`pub(crate) mod format; pub(crate) mod model;` — add later modules as created).
Build: `pub(crate) fn compact(f64) -> String` (thresholds 1M/10K/1K exactly as
gallery `chart.rs:205-216` — note `≥10_000` formats `{:.0}K`), `pub(crate) fn
money(f64) -> String` (`-$`/`$` + compact, gallery `chart.rs:219-225`),
`pub(crate) fn percent(value: f64, digits: usize) -> String`
(`format!("{:.*}%", digits, value)`), and `pub(crate) fn area_gradient(color:
Hsla) -> Background` — `linear_gradient(0., linear_color_stop(
color.opacity(0.45), 1.), linear_color_stop(color.opacity(0.), 0.))`, the
gallery helper verbatim (`chart.rs:484-490`); `Hsla`, `Background`,
`linear_gradient`, `linear_color_stop` all arrive via the `gpui_kit::*` glob
(gallery imports them the same way, `chart.rs:22-27`).
Tests (`format.test.rs`, plain `#[test]`): `compact(1234.)=="1.2K"`,
`compact(12345678.)=="12.3M"`, `compact(312.)=="312"`, `money(-45600.)==
"-$46K"` (45.6 rounds to 46 under the `{:.0}K` branch), `percent(3.456, 1)
=="3.5%"`. `area_gradient` gets no equality test (`Background` carries a
gradient; asserting on it is noise) — the visitors chart rendering it in a
window in `mod.test.rs` is the compile-level check.
Done-check: `cargo test -p gpui-starter --lib pages::analytics::format` green.

### Step 1.4: `model.rs`

Files: `src/features/pages/analytics/model.rs` (new).
Build:
- `#[derive(Clone, Deserialize)] pub(crate) struct DailyRow { pub date:
  SharedString, pub organic: f64, pub paid: f64, pub social: f64, pub direct:
  f64, pub sessions: f64, pub revenue: f64 }`
- `#[derive(Clone, Deserialize)] pub(crate) struct ChannelRow { pub page:
  SharedString, pub channel: SharedString, pub visitors: f64, pub sessions:
  f64, pub revenue: f64, pub conversion: f64, pub avg_session_secs: f64 }`
- `#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub(crate) enum Segment {
  All, Organic, Paid, Social, Direct }` with `const ALL: [Segment; 5]`,
  `fn from_index(usize) -> Self` (0-4, `_ => All`), `fn key(self) ->
  &'static str` (`"all"`, `"organic"`, ... — matches fixture channel values),
  `fn label_key(self) -> &'static str` (`analytics_segment_all`, ...),
  `fn matches(self, channel: &str) -> bool` (`All` → true).
- `#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub(crate) enum DateRange {
  Last7, Last30, Last90 }` with `const ALL: [DateRange; 3]`,
  `fn from_index(usize)`, `fn days(self) -> usize` (7/30/90),
  `fn label_key(self) -> &'static str` (`analytics_range_7`, ...).
- `#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub(crate) enum Density {
  Compact, Comfortable }` + `fn size(self) -> Size` (`Compact → Size::Small`,
  `Comfortable → Size::Medium`; `Size` from `gpui_kit::component`).
- `#[derive(Clone, PartialEq, Eq, Hash, Debug)] pub(crate) struct ChannelKey {
  pub page: SharedString, pub channel: SharedString }`
- `pub(crate) struct Kpi { pub label_key: &'static str, pub value: String,
  pub delta: Option<f64> }`
- `pub(crate) struct WorkspaceData { daily: Vec<DailyRow>, channels:
  Vec<ChannelRow> }` with:
  - `fn load() -> Self` — `serde_json::from_str(include_str!(
    "fixtures/daily-series.json")).expect(...)` twice (gallery `fixture()`
    contract `chart.rs:1479-1481`).
  - `fn window(&self, range: DateRange) -> &[DailyRow]` — tail slice
    `daily[daily.len().saturating_sub(range.days())..]`.
  - `fn previous_window(&self, range: DateRange) -> &[DailyRow]` — the
    equal-length window ending where `window` starts; empty when insufficient
    history.
  - `fn segment_series(&self, range: DateRange, segment: Segment) ->
    Vec<(SharedString, f64)>` — per-day visitor sum for the segment
    (`All` sums all four columns).
  - `fn kpis(&self, range: DateRange) -> [Kpi; 4]` — Visitors
    (`analytics_kpi_visitors`), Sessions (`analytics_kpi_sessions`), Revenue
    (`analytics_kpi_revenue`, `money`), Revenue per visitor
    (`analytics_kpi_revenue_per_visitor`, `money` of revenue/visitors); each
    `delta = Some(pct)` vs `previous_window` sums, `None` when the previous
    window is empty (delta formula gallery `chart.rs:228-234`).
  - `fn channels(&self, segment: Segment) -> Vec<ChannelRow>` — clone-filtered
    by `segment.matches(row.channel)`, original order.
  - `fn channel_series(&self, channel: &str) -> Vec<DailyRow>` — full 90 days
    filtered is wrong shape; return `Vec<(SharedString, f64)>` of
    `(date, column_for(channel))`; unknown channel → empty vec.
Tests (`model.test.rs`, plain `#[test]`): `load()` gives 90 daily rows and 48
channel rows; dates strictly increasing; `window(Last7).len() == 7`,
`window(Last90).len() == 90`; `window(Last30)` is the tail of
`window(Last90)[60..]`; `previous_window(Last90)` is empty and
`previous_window(Last7).len() == 7`; `segment_series(Last7, Social)` sums only
the `social` column (spot-check one day by hand from the fixture);
`kpis(Last30)[0].delta.is_some()` and `kpis(Last90)[0].delta.is_none()`;
`channels(Social).len() == 12`, `channels(All).len() == 48`;
`channel_series("organic").len() == 90`, `channel_series("nope").is_empty()`.
Done-check: `cargo test -p gpui-starter --lib pages::analytics::model` green.

## Phase 2: Table delegate

### Step 2.1: `ChannelTableDelegate`

Files: `src/features/pages/analytics/table.rs` (new),
`.../analytics/mod.rs` (`pub(crate) mod table;`).
Build:
- `pub(crate) struct ChannelTableDelegate { rows: Vec<ChannelRow>, columns:
  Vec<Column>, sort_key: SortKey, sort_order: ColumnSort, loading: bool,
  menu_row: Option<usize> }`
- `#[derive(Clone, Copy, PartialEq)] enum SortKey { Page, Channel, Visitors,
  Sessions, Revenue, Conversion, AvgSession }` mapping column index ↔ key.
- `ChannelTableDelegate::new(rows: Vec<ChannelRow>, cx: &App) -> Self` —
  columns (all labels via `crate::i18n::localize`, widths from the gallery
  precedent `data_table.rs:336-356`):
  - `("page", analytics_col_page).width(200.)` + `.fixed(ColumnFixed::Left)`
    + `.sortable()`
  - `("channel", analytics_col_channel).width(110.).sortable()`
  - `("visitors", analytics_col_visitors).width(90.).sortable().text_right().p_0()`
  - `("sessions", analytics_col_sessions).width(90.).sortable().text_right().p_0()`
  - `("revenue", analytics_col_revenue).width(100.).sortable().text_right().p_0()`
  - `("conversion", analytics_col_conversion).width(80.).sortable().text_right().p_0()`
  - `("avg_session", analytics_col_avg_session).width(100.).sortable().text_right().p_0()`
  default sort `Revenue` descending (`.sort(ColumnSort::Descending)` on the
  revenue column, gallery `system_monitor.rs:73-86`), then `sort_rows()`.
- `fn set_rows(&mut self, rows: Vec<ChannelRow>)` (sorts with the current
  sort, clears `loading`) and `fn set_loading(&mut self, bool)`.
- `fn key_at(&self, row_ix: usize) -> Option<ChannelKey>` and
  `fn row_ix_for_key(&self, key: &ChannelKey) -> Option<usize>` (first match
  in the current sort order — the reload remap input).
- `impl TableDelegate` (signatures from `data_table.rs:464-890`):
  `columns_count`/`rows_count`/`column` (clone); `render_th` = plain
  `div().child(col.name.clone())` with the same `text_right` handling as
  gallery `render_th` (`data_table.rs:570-591`); `render_tr` = `div().id(
  ElementId::Name(key string))` — stable domain id only; row selection,
  hover, and focus painting are entirely the kit's (selection restoration is
  `TableState::set_selected_row`, not a delegate highlight);
  `render_td` — page cell `div().truncate().child(page)`,
  channel cell muted `text_color(muted_foreground)`, numeric cells
  `value_cell` shape from gallery (`data_table.rs:427-437`: `h_full().h_flex()
  .items_center().when(col paddings).table_cell_size(..).when(right,
  justify_end)`) with `compact()` for visitors/sessions, `money()` for
  revenue, `percent(conversion, 1)`, `format!("{}m {}s", secs/60, secs%60)`
  for avg session; `perform_sort(col_ix, sort, ..)` sets `sort_key/sort_order`
  and `sort_rows()` (gallery `system_monitor.rs:161-178` pattern — f64 via
  `partial_cmp`, strings via `cmp`); `context_menu(row_ix, menu, ..)`
  records `menu_row = Some(row_ix)` and returns
  `menu.menu(localize("analytics_open_details"), Box::new(
  crate::features::pages::analytics::OpenChannelDetails))` (menu-with-action
  form, gallery `data_table.rs:612-616`); `loading(&self, _) -> bool`
  returns `self.loading` (in-table loading UI, gallery `:775-777`).
- `actions!(analytics, [OpenChannelDetails]);` goes in `mod.rs` now (unit
  variant, `data_table.rs:22` pattern) so the delegate can reference it.
Tests (`table.test.rs`, `#[gpui_kit::test]` with `cx: &TestAppContext`
precedent `src/state/config_store.test.rs:280-281`): build delegate with the
real fixture via `WorkspaceData::load()`; assert `columns_count == 7`,
`rows_count == 48` (All); `perform_sort` on revenue asc → first row's revenue ≤
last row's; sort on page asc → first page string ≤ last; `channels(Social)`
delegate has 12 rows; `key_at(0)` round-trips into `channel_series`; `set_rows
` preserves the active sort; `set_loading(true)` makes `loading(_, ..)` true.
Done-check: `cargo test -p gpui-starter --lib pages::analytics::table` green;
`cargo check -p gpui-starter` green.

## Phase 3: Page module + seam wiring

### Step 3.1: `AnalyticsPage` skeleton with KPI region

Files: `src/features/pages/analytics/mod.rs` (grows),
`src/features/pages/mod.rs`, `src/shell/sidebar.rs`, `src/shell/route.rs`,
`src/shell/root/app_root/state.rs`, `tests/e2e_navigation.rs`.
Build:
- Seam edits exactly as Seam Touches #1-5 (sidebar variant `Analytics` after
  `Gallery`; `title_key` arm `Page::Analytics => "nav_analytics"`; host
  `"analytics"`; icon `IconName::ChartPie`; `all()` entry appended;
  `VALID_HOSTS` entry; AppRoot import/field/construct/arm; test count 11 +
  expected + deep-link case).
- `mod.rs`:
  - `#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
    #[action(namespace = analytics, no_json)] enum PreviewStateToggle {
    Live, Loading, Empty, Error }` (gallery enum-action shape
    `data_table.rs:46-66` — a derived enum action is declared on its own and
    never listed inside `actions!`; the `actions!` list stays exactly as
    Step 2.1 wrote it, `[OpenChannelDetails]`).
  - `enum Phase { Initial, Refreshing, Ready }` and `enum PreviewState {
    Live, Loading, Empty, Error }` (crate-private), plus `impl
    PreviewStateToggle { fn into_state(self) -> PreviewState }` mapping the
    action variants 1:1.
  - `pub struct AnalyticsPage { ... }` fields exactly as Architecture;
    `pub fn new(window, cx)`: `let data = Rc::new(WorkspaceData::load());`
    two `SelectState`s — range items the three `analytics_range_*` labels with
    `selected: Some(IndexPath::default().row(1))` (Last 30 default), segment
    items the five `analytics_segment_*` labels with `row(0)`; table entity
    `TableState::new(ChannelTableDelegate::new(data.channels(Segment::All),
    cx), window, cx).col_selectable(false).col_movable(false)`
    (gallery `system_monitor.rs:199-203`); subscriptions — the two window-free
    selects use plain `cx.subscribe` (4-param handler,
    `pickers/select.rs:371` precedent):
    `cx.subscribe(&range_select, |this, _, _: &SelectEvent<_>, cx| {
    this.on_range_committed(cx); })`, same for segment; the table handler
    needs `&mut Window`, so it is `cx.subscribe_in(&self.table, window, |this,
    _, event: &TableEvent, window, cx| { this.on_table_event(event, window,
    cx); })` (5-param `subscribe_in`, `context.rs:341-354`, repo precedent
    `todo_list.rs:76`); then `this.reload(cx)` via
    the constructor tail; `cx.bind_keys([KeyBinding::new("enter",
    OpenChannelDetails, Some("DataTable"))])`.
  - `impl Render`: root `v_flex().min_h_full().p_6().gap_4().on_action(
    cx.listener(|this, a: &OpenChannelDetails, window, cx| {
    this.open_detail(window, cx); })).on_action(cx.listener(|this, a:
    &PreviewStateToggle, _, cx| { this.preview = a.into_state(); cx.notify();
    }))` (actions on the wrapper reach focus inside the table,
    `data_table.rs:1082` precedent) with:
    1. filter toolbar (`render_toolbar`),
    2. KPI region (`render_kpis`),
    3. charts region (`render_charts`, Phase 5),
    4. table region (`render_table`, Phase 4).
  - `render_kpis`: section heading `div().a11y(Role::Heading,
    localize("analytics_section_overview")).aria_level(2).text_lg().
    font_weight(FontWeight::SEMIBOLD)` followed by one muted `text_xs()` line
    `localize("analytics_note_fixture")`; body `h_flex().flex_wrap().gap_3()`
    with four cards `v_flex().flex_1().min_w(rems(14.)).border_1().
    border_color(cx.theme().border).rounded(cx.theme().radius).
    p_3/p_4 by density` containing: muted `text_xs().uppercase()` label
    (restrained eyebrow, design guides "Capitalization"), `text_xl()
    font_semibold()` value, delta row = `Icon::new(ArrowUp/ArrowDown by
    sign).size_4()` tinted `cx.theme().success`/`danger` + text
    `format!("{:+.1}%", delta)` + muted `localize("analytics_vs_previous")`
    — sign text present so state is never color-only; `delta == None` → muted
    `"—"`. Region states: `Initial` (or preview `Loading`) → four skeleton
    cards `v_flex()...child(Skeleton::new().w_2_3().h_4()...).child(
    Skeleton::new().w_1_3().h_6()...)`; preview `Empty`/`Error` → the shared
    region state element (Step 4.2).
  - Density application: `let size = self.density.size();` — KPI card padding
    `p_3()` compact / `p_4()` comfortable; later regions consume `size` the
    same way.
Tests (`mod.test.rs`, `#[gpui_kit::test]`): inside `cx.update`, open a window
with `gpui_kit::open_window(WindowOptions::default(), cx, |window, cx|
cx.new(|cx| AnalyticsPage::new(window, cx)))` (test platform opens windows,
`gpui-pre-0.3.8/src/platform/test/platform.rs:493-506`, plan-04 digest) and
keep the returned entity; assert the result is `Ok`; then
`entity.update(cx, |page, cx| page.commit_reload(cx))` → `page.phase ==
Phase::Ready` (direct commit, no timer wait).
Done-check: `cargo check -p gpui-starter` green; `cargo test -p gpui-starter
--test e2e_navigation` green (count 11); sidebar shows Analytics at the end
in a manual launch.

## Phase 4: Filters, reload pipeline, region states

### Step 4.1: Reload pipeline

Files: `src/features/pages/analytics/mod.rs`.
Build:
- `const RELOAD_DELAY: Duration = Duration::from_millis(350);`
- `fn on_range_committed(&mut self, cx)`: read
  `self.range_select.read(cx).selected_index(cx).map(|p| p.row)` →
  `DateRange::from_index`; if changed → `reload(cx)`.
- `fn on_segment_committed(&mut self, cx)`: same over `Segment::from_index`.
- `fn reload(&mut self, cx: &mut Context<Self>)`: `self.phase = if
  !self.committed { Phase::Initial } else { Phase::Refreshing };`
  `self.table.update(cx, |t, _| t.delegate_mut().set_loading(true));`
  `cx.notify();` then `self._reload_task = cx.spawn(
  async move |this, cx| { cx.background_executor().timer(RELOAD_DELAY).await;
  let _ = this.update(cx, |this, cx| this.commit_reload(cx)); });`
  (`system_monitor.rs:217-230` shape).
- `fn commit_reload(&mut self, cx: &mut Context<Self>)`: `let rows =
  self.data.channels(self.segment); let key = self.selected_key.clone();`
  then one table update: `self.table.update(cx, |t, cx| {
  t.delegate_mut().set_rows(rows); t.refresh(cx);
  match key.as_ref().and_then(|k| t.delegate().row_ix_for_key(k)) {
  Some(ix) => t.set_selected_row(ix, cx), None => t.clear_selection(cx) } });`
  (`set_selected_row` restores row-mode selection and scrolls the row into
  view, `state.rs:501-515`; the key is cloned out first because the closure
  cannot borrow `self` behind `self.table.update`); then
  `if self.table.read(cx).selected_row().is_none() { self.selected_key =
  None; }` ; `self.phase = Phase::Ready; self.committed = true;
  cx.notify();`
Tests (append `mod.test.rs`): construct page; drive the segment commit
directly with `page.on_segment_committed(cx)` — programmatic
`set_selected_index` does NOT emit `SelectEvent::Confirm` (`set_selected_index`
`select.rs:309-330` → `list._set_selected_index` `list.rs:167-178` only sets
state, notifies the delegate, and scrolls; it never invokes the on_confirm
callback that emits Confirm), so the shipped UI path (user commits in the
popover → Confirm → listener) is exercised here through its handler. Assert
`page.phase == Refreshing`; call `commit_reload` → delegate
rows == 12 (Social) and `phase == Ready`; with a `selected_key` still present
in the new rows, `table.read(cx).selected_row() == Some(new_ix)`; with a key
the new segment excludes, `selected_row()` is `None` and `selected_key ==
None`.
Done-check: `cargo test -p gpui-starter --lib pages::analytics` green.

### Step 4.2: Region state branches + preview toolbar

Files: `src/features/pages/analytics/mod.rs`.
Build:
- `enum RegionView { Skeleton, Content, Empty, Error }` (crate-private) plus
  `fn region_view(&self) -> RegionView` — one resolver shared by all three
  regions: `preview == Loading` → Skeleton; `preview == Empty` → Empty;
  `preview == Error` → Error; `Live` → `Phase::Initial` → Skeleton,
  `Phase::Refreshing | Phase::Ready` → Content (Refreshing keeps content and
  adds a `Spinner::new().xsmall()` in the region header —
  preserve-previous-data rule, coding guide "Async work"). Each region's
  `render_*` matches on its `region_view()` and renders skeleton/empty/error
  via the shared helpers below instead of its content branch.
- `fn render_state(&self, empty: bool, cx: &mut Context<Self>) -> impl
  IntoElement` — the shared Empty/Error element (`empty` picks the variant):
  - Empty: `Empty::new().header(EmptyHeader::new().media(EmptyMedia::new()
    .with_variant(EmptyMediaVariant::Icon).child(Icon::new(IconName::Inbox)))
    .title(EmptyTitle::new().child(localize("analytics_empty_title")))
    .description(EmptyDescription::new().child(localize(
    "analytics_empty_description"))))` (`empty.rs:108-158` composition).
  - Error: same shape, `IconName::TriangleAlert`, `analytics_error_title`
    /`_description`, `.content(EmptyContent::new().child(Button::new(
    "analytics-retry").outline().label(localize("analytics_retry")).icon(
    IconName::RotateCw).on_click(cx.listener(|this, _, _, cx| {
    this.preview = PreviewStateToggle::Live.into_state(); this.reload(cx);
    }))))` — retry clears the override and reloads (visible action near the
    failure, design guides "Feedback and overlays").
- `render_toolbar`: `h_flex().flex_wrap().items_center().gap_3()`:
  - `Select::new(&self.range_select).title_prefix(localize(
    "analytics_filter_range")).w(rems(15.)).with_size(size)`
  - `Select::new(&self.segment_select).title_prefix(localize(
    "analytics_filter_segment")).w(rems(15.)).with_size(size)`
  - refresh `Button::new("analytics-refresh").outline().icon(IconName::RotateCw)
    .label(localize("analytics_refresh")).on_click(cx.listener(|this, _, _, cx|
    this.reload(cx)))`
  - density segmented control: `TabBar::new("analytics-density").segmented()
    .selected_index(self.density as usize).on_click(cx.listener(|this, ix:
    &usize, _, cx| { this.density = if *ix == 0 { Density::Compact } else {
    Density::Comfortable }; cx.notify(); })).child(Tab::new().label(localize(
    "analytics_density_compact"))).child(Tab::new().label(localize(
    "analytics_density_comfortable")))` wrapped in `div().a11y(Role::Group,
    localize("analytics_density"))` — persistent selected state, design
    guides "Selection is part of the information model".
  - preview `DropdownButton::new("analytics-preview").button(Button::new(
    "analytics-preview-trigger").ghost().label(localize(
    "analytics_preview_label"))).dropdown_menu(move |menu, _, _| menu
    .menu_with_check(localize("analytics_preview_live"), preview ==
    PreviewState::Live, Box::new(PreviewStateToggle::Live)) ... Loading/Empty/
    Error)` (`data_table.rs:1233-1257` shape). Marked as a demo affordance in
    the section's muted description line.
Tests: `PreviewStateToggle` → `into_state()` mapping unit test; override
`Loading` renders skeletons — assert via state not pixels: `page.preview ==
PreviewState::Loading` after dispatching the action listener logic through a
direct method call `page.set_preview(PreviewStateToggle::Empty, cx)`.
Done-check: `cargo test -p gpui-starter --lib pages::analytics` green.

### Step 4.3: Table region

Files: `src/features/pages/analytics/mod.rs`.
Build:
- `render_table`: section heading level 2 `analytics_section_channels`; header
  row `h_flex().justify_between().items_center()` with left: muted summary
  `div().a11y(Role::StatusBar, text).a11y_live(accesskit::Live::Polite).
  text_xs().text_color(muted_foreground).child(localize_with_args(
  "analytics_rows_summary", &[("count", rows.to_string()), ("segment",
  segment_label), ("range", range_label)]))` (announces filter results) and
  `when_some(selected_key)` → `localize_with_args("analytics_selected_summary",
  &[("page", ..), ("channel", ..)])` chip; right: `when(phase == Refreshing)`
  → `Spinner::new().xsmall()`.
- states: Initial → a bordered box holding `v_flex().gap_2()` with six
  `Skeleton::new().h_6().w_full()` rows; preview Empty/Error →
  `render_state`; otherwise
  `div().h(rems(30.)).w_full().child(DataTable::new(&self.table).bordered(
  true).stripe(true).with_size(size))` (fixed height so the table owns its
  scrollbar inside the shell's page scroll — gallery `data_table.rs:1368-1372`
  nested-in-scrollable precedent).
- `fn on_table_event(&mut self, event: &TableEvent, window: &mut Window, cx)`:
  `SelectRow(ix)` → `selected_key = delegate.key_at(ix)`; `DoubleClickedRow(
  ix)` → `selected_key = key_at(ix)` then `open_detail(window, cx)`;
  `ClearSelection` → leave `selected_key` (domain identity survives);
  others ignored.
Tests: simulate `SelectRow` by calling `on_table_event(&TableEvent::SelectRow(
0), window, cx)` → `selected_key == delegate key_at(0)`; `DoubleClickedRow`
sets `has_active_sheet` (assert inside the same test window via
`window.has_active_sheet(cx)`, `window_ext.rs:25`).
Done-check: `cargo test -p gpui-starter --lib pages::analytics` green.

## Phase 5: Charts region

### Step 5.1: Three charts

Files: `src/features/pages/analytics/mod.rs`.
Build:
- `render_charts`: the KPI region owns the "Overview" heading; the charts
  region adds no heading of its own — each card's title row carries its
  `analytics_chart_*` label. Layout `h_flex().flex_wrap().gap_3()` with two
  half-width cards + one full-width:
  1. Visitors by segment (`flex_1().min_w(rems(24.))`): `AreaChart::new(
     window_rows.clone())` with four `.y(|d| d.<col>).stroke(theme.chart_1..4)
     .fill(area_gradient(chart_n))` triples + `.name(segment_label)` each
     (`area_gradient` from `format.rs`, Step 1.3; multi-series stacking,
     `chart.rs:523-534`), `.x(|d| d.date.clone())`,
     `.y_axis(true).y_tick_count(3).y_tick_format(compact).x_tick_count(
     4).grid(true)` — `compact` passes as a plain fn item (gallery passes
     `money` identically at `chart.rs:812`).
  2. Revenue (`flex_1().min_w(rems(24.))`): `LineChart::new(window_rows).x(
     date).y(revenue).stroke(cx.theme().chart_5).name(localize(
     analytics_chart_revenue)).y_axis(true).y_tick_count(3).y_tick_format(
     money).x_tick_count(4).id("analytics-revenue")`.
  3. Sessions by channel (`w_full().min_h(rems(10.))`): aggregate current
     channel rows: `BarChart::new(top6).band(|r| r.page.clone()).value(|r|
     r.sessions).name(localize("analytics_chart_sessions")).fill(move |_, _,
     _, _| theme.chart_2).alignment(BarAlignment::Left).value_axis(true).
     value_tick_count(3).value_tick_format(compact).id(
     "analytics-sessions")` — top 6 channel rows by sessions for the active
     segment (horizontal bars, labels beside, `chart.rs:919-929`).
  Cards share one wrapper shape: `v_flex().flex_1().min_w_0().border_1().
  border_color(border).rounded(theme.radius).p_3/p_4(density)` with a header
  row (title `text_sm`, trailing current-value text `text_sm` colored by
  series — the visible numeric reading for a11y) and chart in
  `div().flex_1().min_h(rems(10.|12. by density))`.
- Region states as Step 4.2: Initial/Loading → `Skeleton::new().w_full().h(
  rems(10.))` + two `h_4` lines per card; Empty/Error via `render_state`;
  Refreshing → previous charts + header `Spinner`.
Tests: none beyond existing page tests (chart construction is exercised by the
window render in `mod.test.rs`; no pixel assertions — kit test-support feature
is not enabled, survey Open Questions).
Done-check: `cargo test -p gpui-starter --lib pages::analytics` green;
`cargo check -p gpui-starter` green.

## Phase 6: Detail sheet

### Step 6.1: `ChannelDetail`

Files: `src/features/pages/analytics/detail.rs` (new),
`.../analytics/mod.rs` (`pub(crate) mod detail;`).
Build:
- `pub(crate) struct ChannelDetail { row: ChannelRow, series: Vec<(
  SharedString, f64)>, phase: DetailPhase, _task: Task<()> }` with
  `enum DetailPhase { Loading, Ready }`.
- `ChannelDetail::view(row: ChannelRow, data: Rc<WorkspaceData>, window:
  &mut Window, cx: &mut App) -> Entity<Self>`: builds with `Loading`, spawns a
  300 ms timer that flips to `Ready` (`cx.spawn` + `this.update` + notify;
  same shape as Phase 4). `series = data.channel_series(&row.channel)`.
- `impl Render`:
  - Loading: 6 `DescriptionList`-shaped skeleton rows (`Skeleton::new().h_4()
    .w_1_2()` / `.w_1_3()` pairs, `skeleton.rs:36-47`) + `Skeleton::new().h_12
    ().w_full()` for the sparkline slot + a muted `analytics_loading` line.
  - Ready: `DescriptionList::new().layout(Axis::Horizontal).bordered(true).
    columns(2).with_size(size)` with `DescriptionItem::new(localize(key)).
    value(div().child(text)).span(1)` rows: page, channel, visitors
    (`compact`), sessions (`compact`), revenue (`money`), conversion
    (`percent(.., 1)`), avg session (`{}m {}s`), plus
    `DescriptionItem::Separator` then `analytics_detail_trend` span 2 holding
    the sparkline: `LineChart::new(series).x(|d| d.0.clone()).y(|d| d.1).
    stroke(cx.theme().chart_2).x_axis(false).y_axis(false).grid(false).
    interactive(false).id("analytics-detail-spark")` in `div().h_16().w_full()`
    (axis-less chart = sparkline; no kit sparkline type exists, plan-04
    digest).
- `open_detail(&mut self, window: &mut Window, cx: &mut Context<Self>)` in
  `mod.rs`: resolve key from `selected_key` else delegate `menu_row` →
  `key_at`; find the full `ChannelRow` in `data.channels(Segment::All)`; guard
  `if window.has_active_sheet(cx) { return; }` (open-only, plan-04 mitigation
  for `root.rs:325-349` sheet replacement); `window.open_sheet(cx, |sheet,
  window, cx| sheet.size(px(420.)).overlay(true).overlay_closable(true).
  title(localize_with_args("analytics_detail_title", &[("page",
  row.page.to_string())])).child(ChannelDetail::view(row.clone(), data.clone(),
  window, cx)))` — `row`/`data` moved into the closure; the sheet traps and
  restores focus itself (`root.rs:331-366`). Import `WindowExt as _` from
  `gpui_kit::component` (`empty.rs:4` style).
Tests (`detail.test.rs`, `#[gpui_kit::test]`): open a window; build
`ChannelDetail::view(row, Rc::new(WorkspaceData::load()), window, cx)`;
assert entity exists and `phase == Loading`; drive the flip by calling the
entity's `fn finish_loading(&mut self, cx)` directly (the task's callback
target) → `Ready`; assert `series.len() == 90`.
Tests (`mod.test.rs` append): with a sheet already open (open one), calling
`open_detail` again leaves exactly one active sheet (`has_active_sheet` still
true, no replace — assert by sheet title equality if observable, else assert
no panic + still true).
Done-check: `cargo test -p gpui-starter --lib pages::analytics` green.

## Phase 7: Gates

### Step 7.1: Full verification pass

Files: none new.
Build/commands, in order (AGENTS.md gates, exact):

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-features`
3. `cargo test --workspace --all-features`
4. `just wasm-check` (lib-only wasm32 nightly check, `justfile:24-25`; the
   page is pure kit composition + `include_str!` + `background_executor`
   timers, all wasm-safe — no `std::time::Instant`, no native deps)

Done-check: all four exit 0; `git status` shows only the Seam Touches files
plus the new module; `grep -rn "use gpui::" src/features/pages/analytics/`
returns nothing (qa_migration rule).

## Platform & Wasm Gating

cfg gates: none. The module compiles everywhere:

- No new dependencies; serde/serde_json are common (`Cargo.toml:25-26`).
- No `std::time::Instant`/`SystemTime` anywhere (survey §6 clock rule); the
  only clock is `cx.background_executor().timer(Duration)`, the exact shape
  `system_monitor.rs:219` uses on wasm today.
- `include_str!` fixtures are compile-time; no filesystem access.
- Sheet, Select, DataTable, charts, Skeleton, Empty are kit components
  already exercised on the wasm lane by the gallery.

Capability-registry entries: none. This feature introduces no platform
capability; the diagnostics page is untouched. (Pure composition per the
feature spec "no new infra".)

Platform behavior matrix:

| Behavior | macOS | Windows | Linux | wasm |
| --- | --- | --- | --- | --- |
| Sidebar entry + page | yes | yes | yes | yes (`#/analytics` hash route, `route.rs:99-118`) |
| Deep link `gpui-starter://analytics` | yes | yes | yes | n/a (hash router instead) |
| Charts / table / sheet | yes | yes | yes | yes; rendering-only differences from the wasm renderer |
| Fixture load | compile-time | compile-time | compile-time | compile-time |
| Anything hidden | nothing | nothing | nothing | nothing |

Native-only deps added: none (the
`[target.'cfg(not(target_family = "wasm"))'.dependencies]` table is
untouched).

## i18n

Facade: `crate::i18n::localize` / `localize_with_args` with `%{name}`
placeholders (`src/services/i18n.rs:3-13`). Numbers/units stay locale-neutral
`format!` through `format.rs`; no sentence is assembled from fragments —
summary strings are single keys with placeholders. Both catalogs in lockstep
(`tests/qa_localization.rs:188-198`). Keys are snake_case top-level scalars
(parser `qa_localization.rs:20-71`).

| Key | en | zh-CN |
| --- | --- | --- |
| `nav_analytics` | "Analytics" | "数据分析" |
| `analytics_section_overview` | "Overview" | "概览" |
| `analytics_section_channels` | "Pages and channels" | "页面与渠道" |
| `analytics_kpi_visitors` | "Visitors" | "访客数" |
| `analytics_kpi_sessions` | "Sessions" | "会话数" |
| `analytics_kpi_revenue` | "Revenue" | "收入" |
| `analytics_kpi_revenue_per_visitor` | "Revenue per visitor" | "单访客收入" |
| `analytics_vs_previous` | "vs previous period" | "较上一周期" |
| `analytics_range_7` | "Last 7 days" | "过去 7 天" |
| `analytics_range_30` | "Last 30 days" | "过去 30 天" |
| `analytics_range_90` | "Last 90 days" | "过去 90 天" |
| `analytics_filter_range` | "Date range" | "日期范围" |
| `analytics_filter_segment` | "Segment" | "细分" |
| `analytics_segment_all` | "All visitors" | "全部访客" |
| `analytics_segment_organic` | "Organic" | "自然流量" |
| `analytics_segment_paid` | "Paid" | "付费流量" |
| `analytics_segment_social` | "Social" | "社交流量" |
| `analytics_segment_direct` | "Direct" | "直接流量" |
| `analytics_refresh` | "Refresh" | "刷新" |
| `analytics_density` | "Density" | "密度" |
| `analytics_density_compact` | "Compact" | "紧凑" |
| `analytics_density_comfortable` | "Comfortable" | "舒适" |
| `analytics_chart_visitors` | "Visitors by segment" | "各细分访客趋势" |
| `analytics_chart_revenue` | "Revenue" | "收入趋势" |
| `analytics_chart_sessions` | "Sessions by page" | "各页面会话数" |
| `analytics_rows_summary` | "%{count} rows · %{segment} · %{range}" | "%{count} 行 · %{segment} · %{range}" |
| `analytics_selected_summary` | "Selected: %{page} (%{channel})" | "已选：%{page}（%{channel}）" |
| `analytics_col_page` | "Page" | "页面" |
| `analytics_col_channel` | "Channel" | "渠道" |
| `analytics_col_visitors` | "Visitors" | "访客数" |
| `analytics_col_sessions` | "Sessions" | "会话数" |
| `analytics_col_revenue` | "Revenue" | "收入" |
| `analytics_col_conversion` | "Conv." | "转化率" |
| `analytics_col_avg_session` | "Avg session" | "平均会话时长" |
| `analytics_open_details` | "View details…" | "查看详情…" |
| `analytics_loading` | "Loading analytics…" | "正在加载分析数据…" |
| `analytics_empty_title` | "No rows for this segment" | "当前细分没有数据" |
| `analytics_empty_description` | "Switch to All visitors or pick another date range." | "切换到全部访客，或选择其他日期范围。" |
| `analytics_error_title` | "Couldn't load analytics" | "无法加载分析数据" |
| `analytics_error_description` | "The fixture data didn't load. Try again." | "示例数据加载失败，请重试。" |
| `analytics_retry` | "Try again" | "重试" |
| `analytics_preview_label` | "Preview states" | "状态预览" |
| `analytics_preview_live` | "Live data" | "实时数据" |
| `analytics_preview_loading` | "Loading" | "加载中" |
| `analytics_preview_empty` | "Empty" | "空数据" |
| `analytics_preview_error` | "Error" | "错误" |
| `analytics_detail_title` | "%{page} details" | "%{page} 详情" |
| `analytics_detail_page` | "Page" | "页面" |
| `analytics_detail_channel` | "Channel" | "渠道" |
| `analytics_detail_visitors` | "Visitors" | "访客数" |
| `analytics_detail_sessions` | "Sessions" | "会话数" |
| `analytics_detail_revenue` | "Revenue" | "收入" |
| `analytics_detail_conversion` | "Conversion" | "转化率" |
| `analytics_detail_avg_session` | "Avg session" | "平均会话时长" |
| `analytics_detail_trend` | "Visitors, last 90 days" | "近 90 天访客趋势" |
| `analytics_note_fixture` | "Demo workspace over fixture data" | "基于示例数据的演示工作台" |

The last key is the one-line muted note under the section heading stating the
data source — the reference screen must not present fixture numbers as a live
integration.

Note on the scanner blind spot (survey §5): `localize(self.title_key())` in
`sidebar.rs:52` is invisible to `qa_localization.rs`; `nav_analytics` must be
added by hand to both catalogs (Step 1.1 does).

## Design Compliance

- **Data-heavy rules** ("Designing data-heavy interfaces"): header + identity
  column (`Page`, fixed left) stable; comparable values right-aligned on
  stable columns (`Column::text_right()` + `value_cell` `justify_end`,
  "Alignment details": right-align comparable numbers); focus/hover/selection
  distinguished by kit DataTable states plus the persistent domain-key
  highlight; sorting visible and reversible via sortable column headers;
  selection preserved by domain identity (`ChannelKey`) across filtering;
  virtualization owned by DataTable without changing keyboard semantics
  (arrow nav, `data_table.rs:18-28`); progressive disclosure via the detail
  sheet; empty state names the next action ("Switch to All visitors…").
- **Density tiers** ("Density tiers", "Radius, spacing, and density"): the
  toggle moves the whole local context (table `with_size`, KPI/card padding,
  DescriptionList size, chart min heights) between compact (tables/repeated
  data) and medium (default chrome) — never one isolated control; no custom
  row heights.
- **Tokens** ("Color and themes", "Theme and styling"): every color from
  `cx.theme()` semantic roles (`chart_1..5`, `primary`, `success`, `danger`,
  `border`, `muted_foreground`, `background`); zero raw hex/rgb/hsla; radii
  from `cx.theme().radius`; the only `px()` values are the sheet width
  `px(420.)` (surface size, `overlays/sheet.rs:252` precedent) — everything
  else uses rem helpers (`p_3`, `gap_3`, `rems(..)`, `text_sm`).
- **Spatial grammar**: `gap_1` inside delta rows (parts of one control),
  `gap_3` between KPI cards and toolbar controls (closely related controls),
  `gap_4` between regions (separate sections); cards draw the border, the
  page adds no second inset around them (no double padding).
- **Layout shell** ("Choose a stable shell", "Responsive desktop windows"):
  the page lives in the shell's existing sidebar workspace and `#page` scroll
  owner (`render.rs:166`); the table gets a fixed rem height so exactly one
  region scrolls; `flex_1` + `min_w_0`/`min_w(rems(24.))` cards wrap instead
  of squeezing charts under readability.
- **Interaction states** ("Interaction states" table): rest/hover/pressed/
  focus-visible/selected come from kit components (Button, Select, TabBar,
  DataTable); loading = skeletons (initial) or content + Spinner (refresh,
  preserving previous data); error states state what happened and how to
  recover (title + description + "Try again"); disabled — nothing is
  disabled in v1 (no async in flight blocks filters; reload coalesces).
- **Command surfaces** ("Prefer desktop command surfaces over hover
  toolbars"): drill-down is reachable three ways — double-click, row context
  menu ("View details…", kit-owned menu keyboard/focus contract), and Enter —
  plus selection summary text; nothing is hover-only.
- **A11y checklist**: every action keyboard-reachable (Tab toolbar, arrows +
  Home/End/PgUp/PgDn in the table, Enter opens the sheet, Esc closes — sheet
  focus trap/restore is kit-owned `root.rs:331-366`); section headings carry
  `Role::Heading` + `aria_level(2)` under the shell's H1; the rows-summary is
  a polite live region announcing filter results; controls are labeled
  (Select `title_prefix`, TabBar group role, Button labels — no icon-only
  buttons); chart values duplicated as visible text; status never color-only
  (signed delta text next to every trend icon); pointer targets are kit-sized
  even in compact density.
- **Interface language**: nouns for destinations/sections ("Overview",
  "Pages and channels"), verbs for commands ("Refresh", "Try again"), states
  as short phrases; sentence case; single `…` on "View details…" (opens a
  sheet) and nothing else; no terminal punctuation on labels; zh-CN uses
  full-width punctuation only in sentences; `%{...}` placeholders — no
  concatenated fragments ("Write each language, do not translate its shape").
- **Motion**: kit chart appear defaults only; no added animation ("Motion").

## Verification

Repo gates (AGENTS.md, run exactly as named in Step 7.1):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

`just wasm-check` proves the whole page compiles for wasm32 (no cfg fences to
hide behind).

Feature-specific manual QA (dev flow `bash scripts/macos-dev-app.sh`,
CONTRIBUTING.md:32-38; or Linux debug build):

1. Sidebar shows "Analytics" last; `secondary-1..9` still land on the previous
   nine pages (positional-binding gotcha, survey §1).
2. Deep link `gpui-starter://analytics` opens the page; `#/analytics` works in
   the wasm build.
3. First paint: KPI skeletons, chart skeletons, table skeletons; ~350 ms later
   all regions show fixture data.
4. Change date range or segment: regions keep previous content with a spinner,
   then update; rows summary announces the new count (screen-reader check with
   the OS AT if available); `48 rows` for All, `12` for any single segment.
5. Sort every column asc/desc; column order and widths follow the spec
   (identity column fixed left).
6. Click a row: selected summary shows `page (channel)`; after a filter
   change the row keeps its kit selection when still present (restored via
   `set_selected_row`) and selection clears for a segment that excludes it.
7. Double-click a row, right-click → "View details…", and Enter (with the
   table focused) each open the right-side sheet: skeletons ~300 ms, then
   DescriptionList + sparkline; Esc closes and focus returns to the table;
   opening while a sheet is already open is a no-op.
8. Density toggle: whole page switches compact/comfortable (table row height,
   card padding, chart heights, detail list size) — not one control in
   isolation.
9. Preview states menu: Loading shows skeletons everywhere; Empty shows the
   guidance empty state; Error shows the error state whose "Try again"
   returns to live data and reloads; "Live data" is checked by default.
10. Both locales (Settings → Language): every label localized, no key strings
    visible; zh-CN layout holds with the longer labels (wrap, not clip).
11. Dark and light themes + one custom theme JSON: no unreadable contrast,
    no raw-color artifacts.
12. Window resize to minimum: toolbar wraps, chart cards wrap, table keeps
    its own scrollbar; no clipped focus rings.

Acceptance criteria:

- All four gates pass with only the Seam Touches files + the new module
  changed; `Cargo.lock` byte-identical (no dependency edits).
- `Page::all().len() == 11`; `AppRoute::parse_deep_link(
  "gpui-starter://analytics") == Ok(AppRoute::Page(Page::Analytics))`
  (pinned by `tests/e2e_navigation.rs`).
- Both locale catalogs carry the identical 56-key set (pinned by
  `tests/qa_localization.rs`).
- No `use gpui::`/`gpui_component::` imports in the new module (pinned by
  `tests/qa_migration.rs`).
- Unit tests green: model (counts/slices/KPIs), format, table (sort/counts),
  detail (load phases), page (reload commit, sheet open/guard, preview).

## Risks & Open Questions

Ranked:

1. **`TableEvent::SelectRow` on right-click is not verified.** Tried: source
   read of `table/state.rs:80-120` (docs say "Single click or move to selected
   row"); the right-click dispatch path was not traced. Mitigation: the
   delegate's `context_menu` records `menu_row` before the menu opens, and
   `open_detail` falls back to it, so "View details…" works regardless.
   (An earlier draft of this plan also claimed `set_selected_row` did not
   exist; re-verification found it at `state.rs:501` and the restore path now
   uses it directly — the lesson being the risk below about grep-negative
   claims.)
2. **Chart accessibility.** The kit plots may not expose per-point accesskit
   nodes (not verified; plot internals in `gpui-base-0.7.1/src/plot/`). The
   exact numbers are always available as visible text (KPI values, region
   header values, the table) and the sparkline's series is summarized by the
   DescriptionList row, so no information is chart-only.
3. **Natural Empty/Error states are unreachable with shipped fixtures** (the
   loader always yields Ready; fixture parse errors panic at construction like
   the gallery's `fixture()`). The preview menu exists precisely so every
   state is demonstrable; the error state's retry path is real (clears the
   override + reloads). If product later wires a real backend, `Phase` grows a
   `Failed` variant and the preview menu shrinks — the state elements already
   exist.
4. **Element-level assertions are state-based, not pixel-based.** The repo
   does not enable the kit's own `test-support` feature, so the
   `gpui_kit::test` module (TestWindowExt, ElementSnapshot) is cfg'd out and
   element snapshots are unavailable; `#[gpui_kit::test]` itself — the macro
   and `TestAppContext` — does work here because the dev-dep gpui enables the
   gpui crate's `test-support` (`Cargo.toml:212-216`) and the macro rides the
   kit's `pub use ::gpui::*` glob. `mod.test.rs`/`detail.test.rs` therefore
   assert entity state and `has_active_sheet` rather than rendered geometry;
   alignment/density claims rest on manual QA 8-12.
5. **Fixture numeric values are formula-pinned, not byte-pinned, in this
   plan.** Schema, row counts, page/channel sets, and the generator formulas
   are exact (Step 1.2); the committed JSON is the artifact and model tests
   pin counts + monotonic dates, so drift is caught structurally. Tried:
   inlining 138 numbers into the plan — rejected as unmaintainable.
6. **Enter bound under context `"DataTable"` is app-global once registered.**
   The binding is registered in `AnalyticsPage::new` but matches any focused
   DataTable (gallery included); the handler lives on this page's wrapper, so
   the action is a no-op elsewhere (dispatch finds no handler). Verified the
   upstream table binds no `enter` itself (`data_table.rs:18-28`). Residual
   risk: a future kit `enter` binding would collide — recheck on kit upgrade.
7. **`set_selected_row` scrolls the viewport on restore.** The setter calls
   `scroll_to_item` internally (`state.rs:501-515`), so committing a filter
   reload with a live selection scrolls that row into view. That is the
   desired "keep the selected row visible" behavior, but if a future iteration
   wants reload to preserve scroll position exactly, the restore must switch
   to `set_selected_cell`-free state juggling — single call site
   (`commit_reload`) either way.

Open Questions (review notes deliberately not acted on):

- Em-dash density in this planning doc (~70) is left as-is: AGENTS.md prose
  rules target shipped copy; trimming happens when text moves into commit
  messages or the repo docs.
- Design-guide section names quoted in Design Compliance ("Designing
  data-heavy interfaces", "Density tiers", etc.) were not re-read this
  revision round; the claims themselves are concrete and were written against
  the full guide read during planning.
