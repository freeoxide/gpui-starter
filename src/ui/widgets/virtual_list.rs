//! Virtualized-list helpers around `gpui_component::v_virtual_list`: item-size
//! builders, bounded heights, and a single wired-up render entry point.

use std::ops::Range;
use std::rc::Rc;

use gpui::{prelude::*, *};
use gpui_component::scroll::{ScrollableElement, ScrollbarAxis};
use gpui_component::{VirtualListScrollHandle, v_flex, v_virtual_list};

// ---------------------------------------------------------------------------
// Item-size helpers
// ---------------------------------------------------------------------------

/// Uniform item sizes; width is `px(0.)` so each row's flex layout controls it.
pub fn uniform_item_sizes(count: usize, height: Pixels) -> Rc<Vec<Size<Pixels>>> {
    Rc::new(vec![size(px(0.), height); count])
}

/// Variable item sizes from heights; width is `px(0.)` as above.
pub fn variable_item_sizes(heights: &[Pixels]) -> Rc<Vec<Size<Pixels>>> {
    Rc::new(heights.iter().map(|&h| size(px(0.), h)).collect())
}

// ---------------------------------------------------------------------------
// Bounded list height
// ---------------------------------------------------------------------------

/// Bounded list height: `min(total_content_height + gaps, max_height)`.
pub fn bounded_list_height(item_sizes: &[Size<Pixels>], gap: Pixels, max_height: Pixels) -> Pixels {
    let content_h: f32 = item_sizes.iter().map(|s| s.height.as_f32()).sum();
    let gap_total = gap.as_f32() * item_sizes.len().saturating_sub(1) as f32;
    px((content_h + gap_total).min(max_height.as_f32()))
}

// ---------------------------------------------------------------------------
// render_virtual_list
// ---------------------------------------------------------------------------

/// Render a virtualized list (entity clone, scroll tracking, gap, container,
/// optional scrollbar). Precondition: items pinned to declared heights.
///
/// An inner node carries `Role::List` because the scrollable list element has
/// no role of its own; items get their `Role::ListItem` from
/// [`virtual_list_item`]. The container stays a plain `Div` for callers.
pub fn render_virtual_list<R, V>(
    cx: &mut Context<V>,
    id: impl Into<ElementId>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    list_height: Pixels,
    gap: Pixels,
    scroll_handle: &VirtualListScrollHandle,
    show_scrollbar: bool,
    render_items: impl 'static + Fn(&mut V, Range<usize>, &mut Window, &mut Context<V>) -> Vec<R>,
) -> Div
where
    R: IntoElement + 'static,
    V: Render + 'static,
{
    let entity = cx.entity();
    let list_id: ElementId = id.into();
    let total = item_sizes.len();

    let mut list = v_virtual_list(entity, list_id.clone(), item_sizes, render_items)
        .track_scroll(scroll_handle);

    if gap > px(0.) {
        list = list.gap(gap);
    }

    let mut region = div()
        .id((list_id, "list-region"))
        .role(Role::List)
        .aria_orientation(Orientation::Vertical)
        // AT-SPI derives each item's setsize from the nearest ancestor that
        // declares one; item-level values are ignored.
        .aria_size_of_set(total)
        .size_full()
        .child(list);

    if show_scrollbar {
        region = region.scrollbar(scroll_handle, ScrollbarAxis::Vertical);
    }

    v_flex().relative().w_full().h(list_height).child(region)
}

/// A virtualized-list row: `Role::ListItem` announcing "item N of M". The
/// label is required because plain text children produce no accessibility
/// nodes; render the visible content inside and pass the same text as `label`.
pub fn virtual_list_item(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    index: usize,
    total: usize,
) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::ListItem)
        .aria_label(label)
        // accesskit stores position_in_set 0-based; AT bridges report the
        // stored value +1.
        .aria_position_in_set(index)
        .aria_size_of_set(total)
}

#[cfg(test)]
#[path = "virtual_list.test.rs"]
mod virtual_list_test;
