use std::hash::{Hash, Hasher};
use std::rc::Rc;

use gpui::{prelude::*, *};
use gpui_component::{ActiveTheme as _, VirtualListScrollHandle, h_flex, v_flex};

use crate::accessibility::A11yExt as _;
use crate::ui::widgets::{
    bounded_list_height, render_virtual_list, variable_item_sizes, virtual_list_item,
};
use gpui_query::client::{ClientDiagnostic, QueryDiagnostic};
use gpui_query::core::QueryStatus;

use super::dashboard::QueryDevToolsV2Page;
use super::helpers::{QuerySort, filter_button, format_cache_age, rems_from_px, sort_button};

// `v_virtual_list` positions items purely from `item_sizes`, so rows/details
// are pinned to these heights — keep in sync with the row renderers.
const REGISTRY_ROW_H: f32 = 38.0; // px_3/py_2 single-line row
const REGISTRY_DETAIL_H: f32 = 156.0; // 6-field expanded detail (p_3 + gap_1)
const REGISTRY_ITEM_GAP: f32 = 4.0; // gap_1 between row and detail (expanded)
const REGISTRY_LIST_GAP: f32 = 2.0; // gap_0p5 between registry items
const REGISTRY_MAX_LIST_H: f32 = 480.0; // cap before the list itself scrolls

/// Memoized registry rows: cached against a signature of the diagnostic plus
/// sort/filter; per-row strings precomputed to avoid per-frame `format!()`.
struct RegistryRow {
    query: QueryDiagnostic,
    /// Stable stateful element id (`"v2-query-row-{key}"`).
    element_id: SharedString,
    /// Pre-formatted `cache_hits` Display string.
    cache_hits_str: SharedString,
    /// Pre-formatted `retry_count` Display string.
    retry_count_str: SharedString,
}

/// Memoization cache for the filtered+sorted registry rows.
#[derive(Default)]
struct RegistryRowCache {
    signature: u64,
    sort: Option<QuerySort>,
    filter: Option<String>,
    rows: Rc<Vec<RegistryRow>>,
}

impl Global for RegistryRowCache {}

/// Cheap signature of the diagnostic's query slice; equal signatures mean the
/// cached sort/filter result is still valid.
fn diagnostic_signature(d: &ClientDiagnostic) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    d.query_count.hash(&mut h);
    for q in &d.queries {
        q.key.hash(&mut h);
        std::mem::discriminant(&q.status).hash(&mut h);
        q.cache_policy.hash(&mut h);
        q.cache_hits.hash(&mut h);
        q.retry_count.hash(&mut h);
        q.cache_age_ms.hash(&mut h);
    }
    h.finish()
}

/// Return the memoized filtered+sorted row list, rebuilding only when the
/// diagnostic signature, sort, or filter has changed.
fn cached_registry_rows(
    diagnostic: &Option<ClientDiagnostic>,
    sort_by: QuerySort,
    status_filter: &Option<String>,
    cx: &mut Context<QueryDevToolsV2Page>,
) -> Rc<Vec<RegistryRow>> {
    let signature = diagnostic.as_ref().map(diagnostic_signature).unwrap_or(0);
    let cache = cx.default_global::<RegistryRowCache>();
    let hit = cache.signature == signature
        && cache.sort == Some(sort_by)
        && cache.filter.as_deref() == status_filter.as_deref();
    if hit {
        return cache.rows.clone();
    }

    // Miss: recompute the filtered+sorted rows from scratch.
    let mut queries: Vec<QueryDiagnostic> = diagnostic
        .as_ref()
        .map(|d| d.queries.clone())
        .unwrap_or_default();

    // Apply status filter; unknown values are logged and ignored.
    if let Some(filter) = status_filter {
        let filter_status = match filter.as_str() {
            "Idle" => Some(QueryStatus::Idle),
            "LoadingEmpty" => Some(QueryStatus::LoadingEmpty),
            "LoadingWithData" => Some(QueryStatus::LoadingWithData),
            "Success" => Some(QueryStatus::Success),
            "Failure" => Some(QueryStatus::Failure),
            "Cancelled" => Some(QueryStatus::Cancelled),
            _ => {
                tracing::warn!(
                    "QueryDevToolsV2: unknown status_filter value {:?}; clearing filter",
                    filter
                );
                None
            }
        };
        if let Some(fs) = filter_status {
            queries.retain(|q| q.status == fs);
        }
    }

    // Apply sort.
    match sort_by {
        QuerySort::Key => queries.sort_by(|a, b| a.key.cmp(&b.key)),
        QuerySort::Status => queries.sort_by(|a, b| a.status.cmp(&b.status)),
        QuerySort::CacheAge => queries.sort_by(|a, b| {
            b.cache_age_ms
                .unwrap_or(0)
                .cmp(&a.cache_age_ms.unwrap_or(0))
        }),
        QuerySort::CacheHits => queries.sort_by(|a, b| b.cache_hits.cmp(&a.cache_hits)),
    }

    // Precompute per-row strings so the virtual-list closure only clones
    // SharedStrings (Arc bumps) instead of formatting per visible row.
    let rows: Vec<RegistryRow> = queries
        .into_iter()
        .map(|q| {
            let element_id = SharedString::from(format!("v2-query-row-{}", q.key));
            let cache_hits_str = SharedString::from(q.cache_hits.to_string());
            let retry_count_str = SharedString::from(q.retry_count.to_string());
            RegistryRow {
                query: q,
                element_id,
                cache_hits_str,
                retry_count_str,
            }
        })
        .collect();

    cache.signature = signature;
    cache.sort = Some(sort_by);
    cache.filter = status_filter.clone();
    cache.rows = Rc::new(rows);
    cache.rows.clone()
}

pub(super) fn render_query_registry(
    diagnostic: &Option<ClientDiagnostic>,
    expanded_key: &Option<String>,
    sort_by: QuerySort,
    status_filter: &Option<String>,
    scroll_handle: &VirtualListScrollHandle,
    cx: &mut Context<QueryDevToolsV2Page>,
) -> Div {
    let theme = cx.theme();
    let radius_lg = theme.radius_lg;
    let border = theme.border;
    let muted = theme.muted;
    let muted_foreground = theme.muted_foreground;

    // Sort controls
    let sort_controls = h_flex()
        .id("v2-sort-group")
        .a11y(Role::Group, "Sort")
        .gap_2()
        .children(vec![
            sort_button("By Key", QuerySort::Key, sort_by, cx),
            sort_button("By Status", QuerySort::Status, sort_by, cx),
            sort_button("By Cache Age", QuerySort::CacheAge, sort_by, cx),
            sort_button("By Cache Hits", QuerySort::CacheHits, sort_by, cx),
        ]);

    // Status filter options matching v2 QueryStatus variants
    let status_options: Vec<Option<&str>> = vec![
        None,
        Some("Idle"),
        Some("LoadingEmpty"),
        Some("LoadingWithData"),
        Some("Success"),
        Some("Failure"),
        Some("Cancelled"),
    ];

    let filter_controls = h_flex()
        .id("v2-filter-group")
        .a11y(Role::Group, "Status filter")
        .gap_2()
        .children(
            status_options
                .into_iter()
                .map(|opt| filter_button(opt, status_filter, cx)),
        );

    // Memoized rows: rebuilt only when the diagnostic, sort, or filter changes.
    let queries: Rc<Vec<RegistryRow>> =
        cached_registry_rows(diagnostic, sort_by, status_filter, cx);

    // Header row
    let header = h_flex().gap_3().px_3().py_2().children(vec![
        div().w(rems_from_px(16.0)).child(div()),
        div()
            .text_xs()
            .font_weight(FontWeight::BOLD)
            .flex_1()
            .child("Key"),
        div()
            .text_xs()
            .font_weight(FontWeight::BOLD)
            .flex_1()
            .child("Status"),
        div()
            .text_xs()
            .font_weight(FontWeight::BOLD)
            .flex_1()
            .child("Cache Policy"),
        div()
            .text_xs()
            .font_weight(FontWeight::BOLD)
            .flex_1()
            .child("Cache Age"),
        div()
            .text_xs()
            .font_weight(FontWeight::BOLD)
            .flex_1()
            .child("Cache Hits"),
        div()
            .text_xs()
            .font_weight(FontWeight::BOLD)
            .flex_1()
            .child("Retry Count"),
    ]);

    // Virtualized: only visible rows are laid out and painted, so adding
    // queries never forces a full-tree relayout on scroll.
    let item_heights: Vec<Pixels> = queries
        .iter()
        .map(|row| {
            let is_expanded = expanded_key.as_deref() == Some(row.query.key.as_str());
            if is_expanded {
                px(REGISTRY_ROW_H + REGISTRY_ITEM_GAP + REGISTRY_DETAIL_H)
            } else {
                px(REGISTRY_ROW_H)
            }
        })
        .collect();

    let item_sizes = variable_item_sizes(&item_heights);
    let list_h = bounded_list_height(&item_sizes, px(REGISTRY_LIST_GAP), px(REGISTRY_MAX_LIST_H));

    // Empty state within registry
    let registry_content = if queries.is_empty() {
        div().py_6().flex().justify_center().child(
            div()
                .id("v2-registry-empty")
                .a11y(Role::Paragraph, "No queries match the current filter.")
                .text_sm()
                .text_color(muted_foreground)
                .child("No queries match the current filter."),
        )
    } else {
        let scroll_handle = scroll_handle.clone();
        let rows = queries.clone();
        let expanded = expanded_key.clone();
        let total = queries.len();

        render_virtual_list(
            cx,
            "v2-registry-rows",
            item_sizes,
            list_h,
            px(REGISTRY_LIST_GAP),
            &scroll_handle,
            true,
            move |_this, range, _window, cx| {
                range
                    .map(|ix| {
                        let row = &rows[ix];
                        let is_expanded = expanded.as_deref() == Some(row.query.key.as_str());
                        // Drop the theme borrow before query_row takes `&mut cx`.
                        let (radius, secondary) = {
                            let theme = cx.theme();
                            (theme.radius, theme.secondary)
                        };
                        let mut item = virtual_list_item(
                            SharedString::from(format!("{}-item", row.element_id)),
                            registry_row_label(row),
                            ix,
                            total,
                        )
                        .gap_1()
                        .child(query_row(row, is_expanded, cx).h(px(REGISTRY_ROW_H)));
                        if is_expanded {
                            item = item.child(
                                query_expanded_detail(row, radius, secondary)
                                    .h(px(REGISTRY_DETAIL_H)),
                            );
                        }
                        item
                    })
                    .collect::<Vec<_>>()
            },
        )
    };

    div()
        .rounded(radius_lg)
        .border_1()
        .border_color(border)
        .bg(muted)
        .p_4()
        .child(
            div()
                .id("v2-registry-title")
                .a11y(Role::Heading, "Query Registry")
                .aria_level(2)
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .mb_2()
                .child("Query Registry"),
        )
        .child(
            v_flex()
                .gap_2()
                .mb_3()
                .child(
                    v_flex()
                        .gap_1()
                        .child(div().text_xs().text_color(muted_foreground).child("Sort:"))
                        .child(sort_controls),
                )
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted_foreground)
                                .child("Status:"),
                        )
                        .child(filter_controls),
                ),
        )
        .child(header)
        .child(registry_content)
}

fn query_row(
    row: &RegistryRow,
    is_expanded: bool,
    cx: &mut Context<QueryDevToolsV2Page>,
) -> Stateful<Div> {
    let q = &row.query;
    let theme = cx.theme();
    let muted_foreground = theme.muted_foreground;
    let primary = theme.primary;
    let danger = theme.danger;
    let secondary = theme.secondary;
    let radius = theme.radius;

    let key = q.key.clone();

    let status_color = match q.status {
        QueryStatus::Idle | QueryStatus::Cancelled => muted_foreground,
        QueryStatus::LoadingEmpty | QueryStatus::LoadingWithData => primary,
        QueryStatus::Success => primary,
        QueryStatus::Failure => danger,
    };

    let status_label = status_label(&q.status);

    let chevron = if is_expanded { "\u{25BE}" } else { "\u{25B8}" };

    let cache_age_str = format_cache_age(q.cache_age_ms);

    let element_id = row.element_id.clone();
    let cache_hits_str = row.cache_hits_str.clone();
    let retry_count_str = row.retry_count_str.clone();

    div()
        .id(element_id)
        .a11y(Role::Button, q.key.clone())
        .aria_expanded(is_expanded)
        .cursor_pointer()
        .rounded(radius)
        .px_3()
        .py_2()
        .hover(move |s| s.bg(secondary))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.expanded_key = if this.expanded_key.as_deref() == Some(key.as_str()) {
                None
            } else {
                Some(key.clone())
            };
            cx.notify();
        }))
        .child(
            // Chevron column width matches the header placeholder so the six
            // flex_1 data columns align.
            h_flex().gap_3().items_center().children(vec![
                div()
                    .w(rems_from_px(16.0))
                    .text_xs()
                    .text_color(muted_foreground)
                    .child(chevron.to_string()),
                div()
                    .text_sm()
                    .font_family("monospace")
                    .flex_1()
                    .child(q.key.clone()),
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .px_1()
                    .text_color(status_color)
                    .child(status_label.to_string()),
                div()
                    .text_xs()
                    .px_1()
                    .rounded(radius)
                    .bg(secondary)
                    .child(q.cache_policy.clone()),
                div()
                    .text_xs()
                    .text_color(muted_foreground)
                    .child(cache_age_str),
                div()
                    .text_xs()
                    .text_color(muted_foreground)
                    .child(cache_hits_str),
                div()
                    .text_xs()
                    .text_color(muted_foreground)
                    .child(retry_count_str),
            ]),
        )
}

fn status_label(status: &QueryStatus) -> &'static str {
    match status {
        QueryStatus::Idle => "Idle",
        QueryStatus::LoadingEmpty | QueryStatus::LoadingWithData => "Loading",
        QueryStatus::Success => "Success",
        QueryStatus::Failure => "Failure",
        QueryStatus::Cancelled => "Cancelled",
    }
}

/// The text an assistive reader announces for one registry row; row children
/// are plain text divs that produce no a11y nodes on their own.
fn registry_row_label(row: &RegistryRow) -> String {
    let q = &row.query;
    format!(
        "{}: status {}, policy {}, cache age {}, {} hits, {} retries",
        q.key,
        status_label(&q.status),
        q.cache_policy,
        format_cache_age(q.cache_age_ms),
        row.cache_hits_str,
        row.retry_count_str,
    )
}

fn query_expanded_detail(row: &RegistryRow, radius: Pixels, secondary: Hsla) -> Div {
    let q = &row.query;
    let fields: Vec<(&str, SharedString)> = vec![
        ("Key", SharedString::from(q.key.clone())),
        ("Status", SharedString::from(format!("{:?}", q.status))),
        ("Cache Policy", SharedString::from(q.cache_policy.clone())),
        (
            "Cache Age",
            SharedString::from(format_cache_age(q.cache_age_ms)),
        ),
        ("Cache Hits", row.cache_hits_str.clone()),
        ("Retry Count", row.retry_count_str.clone()),
    ];

    div()
        .ml_4()
        .border_1()
        .border_color(secondary)
        .rounded(radius)
        .p_3()
        .gap_1()
        .children(fields.into_iter().map(|(label, value)| {
            h_flex().gap_2().child(
                div().text_xs().child(
                    h_flex()
                        .gap_1()
                        .child(
                            div()
                                .font_weight(FontWeight::BOLD)
                                .child(format!("{}:", label)),
                        )
                        .child(div().child(value)),
                ),
            )
        }))
}
