use std::ops::Range;
use std::rc::Rc;

use gpui_kit::component::scroll::{ScrollableElement, ScrollbarAxis};
use gpui_kit::component::{VirtualListScrollHandle, v_flex, v_virtual_list};
use gpui_kit::{prelude::*, *};

/// Width is `px(0.)` so each row's flex layout controls it.
fn uniform_item_sizes(count: usize, height: Pixels) -> Rc<Vec<Size<Pixels>>> {
    Rc::new(vec![size(px(0.), height); count])
}

/// Width is `px(0.)` as above.
fn variable_item_sizes(heights: &[Pixels]) -> Rc<Vec<Size<Pixels>>> {
    Rc::new(heights.iter().map(|&h| size(px(0.), h)).collect())
}

/// A view's memoized size table: hands back the same `Rc` until the
/// requested sizes change, so steady-state renders allocate nothing.
#[derive(Default)]
pub struct ItemSizeCache {
    sizes: Option<Rc<Vec<Size<Pixels>>>>,
}

impl ItemSizeCache {
    /// A uniform table is fully determined by `count` plus `height`, so the
    /// cached first row's height is enough to detect a change.
    pub fn uniform(&mut self, count: usize, height: Pixels) -> Rc<Vec<Size<Pixels>>> {
        if let Some(sizes) = &self.sizes
            && sizes.len() == count
            && sizes.first().is_some_and(|s| s.height == height)
        {
            return sizes.clone();
        }
        let sizes = uniform_item_sizes(count, height);
        self.sizes = Some(sizes.clone());
        sizes
    }

    pub fn variable(&mut self, heights: &[Pixels]) -> Rc<Vec<Size<Pixels>>> {
        if let Some(sizes) = &self.sizes
            && sizes.len() == heights.len()
            && sizes
                .iter()
                .zip(heights.iter())
                .all(|(s, &h)| s.height == h)
        {
            return sizes.clone();
        }
        let sizes = variable_item_sizes(heights);
        self.sizes = Some(sizes.clone());
        sizes
    }
}

pub fn bounded_list_height(item_sizes: &[Size<Pixels>], gap: Pixels, max_height: Pixels) -> Pixels {
    let content_h: f32 = item_sizes.iter().map(|s| s.height.as_f32()).sum();
    let gap_total = gap.as_f32() * item_sizes.len().saturating_sub(1) as f32;
    px((content_h + gap_total).min(max_height.as_f32()))
}

/// Precondition: items pinned to declared heights. `Role::List` sits on an inner
/// node (the kit list has none); items get `Role::ListItem` from [`virtual_list_item`].
pub fn render_virtual_list<R, V>(
    cx: &mut Context<V>,
    id: impl Into<ElementId>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    list_height: Pixels,
    gap: Pixels,
    scroll_handle: &VirtualListScrollHandle,
    show_scrollbar: bool,
    render_items: impl Fn(&mut V, Range<usize>, &mut Window, &mut Context<V>) -> Vec<R> + 'static,
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

/// A `Role::ListItem` row at 0-based `index`; its setsize comes from the nearest
/// `Role::List` ancestor. Text children get no a11y node, so pass that text as `label`.
pub fn virtual_list_item(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    index: usize,
) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::ListItem)
        .aria_label(label)
        // accesskit stores position_in_set 0-based; AT bridges report the
        // stored value +1.
        .aria_position_in_set(index)
}

#[cfg(test)]
#[path = "virtual_list.test.rs"]
mod virtual_list_test;
