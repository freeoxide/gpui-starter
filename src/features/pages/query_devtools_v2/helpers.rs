use gpui_kit::component::{Selectable, button::Button};
use gpui_kit::*;
use gpui_query::core::QueryStatus;

use super::dashboard::QueryDevToolsV2Page;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum QuerySort {
    Key,
    Status,
    CacheAge,
    CacheHits,
}

pub(super) fn sort_button(
    label: &str,
    target: QuerySort,
    current: QuerySort,
    cx: &mut Context<QueryDevToolsV2Page>,
) -> Button {
    let active = current == target;
    let mut btn = Button::new(format!("v2-sort-{:?}", target))
        .outline()
        .label(label)
        .toggled(active);
    if active {
        btn = btn.selected(true);
    }
    btn.on_click(cx.listener(move |this, _, _, _cx| {
        this.sort_by = target;
        this.expanded_key = None;
        _cx.notify();
    }))
}

pub(super) fn filter_button(
    target: Option<QueryStatus>,
    current: &Option<QueryStatus>,
    cx: &mut Context<QueryDevToolsV2Page>,
) -> Button {
    let active = *current == target;
    let label = match target {
        None => crate::i18n::localize("query_devtools_filter_all"),
        Some(status) => status_label(status),
    };
    let id = format!(
        "v2-filter-{}",
        target
            .map(|status| format!("{status:?}"))
            .unwrap_or_else(|| "all".to_string())
    );
    let mut btn = Button::new(id).outline().label(label).toggled(active);
    if active {
        btn = btn.selected(true);
    }
    btn.on_click(cx.listener(move |this, _, _, _cx| {
        this.status_filter = target;
        this.expanded_key = None;
        _cx.notify();
    }))
}

pub(super) fn status_label(status: QueryStatus) -> String {
    let key = match status {
        QueryStatus::Idle => "query_devtools_status_idle",
        QueryStatus::LoadingEmpty | QueryStatus::LoadingWithData => "query_devtools_status_loading",
        QueryStatus::Success => "query_devtools_status_success",
        QueryStatus::Failure => "query_devtools_status_failure",
        QueryStatus::Cancelled => "query_devtools_status_cancelled",
    };
    crate::i18n::localize(key)
}

pub(super) fn format_cache_age(age_ms: Option<u64>) -> String {
    match age_ms {
        None => "n/a".to_string(),
        Some(ms) => {
            if ms < 1000 {
                format!("{}ms", ms)
            } else if ms < 60_000 {
                format!("{:.1}s", ms as f64 / 1000.0)
            } else if ms < 3_600_000 {
                format!("{:.1}m", ms as f64 / 60_000.0)
            } else {
                format!("{:.1}h", ms as f64 / 3_600_000.0)
            }
        }
    }
}
