// No `use super::*`: gpui's `test` proc-macro glob would make bare `#[test]`
// recurse infinitely. Import only what we need.
use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use gpui_kit::component::VirtualListScrollHandle;
use gpui_kit::{
    AnyWindowHandle, Context, IntoElement, ParentElement, Pixels, Render, Size, Styled,
    TestAppContext, Window, div, px, size,
};

use super::{
    ItemSizeCache, bounded_list_height, render_virtual_list, uniform_item_sizes,
    variable_item_sizes, virtual_list_item,
};

#[test]
fn uniform_item_sizes_correct_count() {
    let sizes = uniform_item_sizes(5, px(30.));
    assert_eq!(sizes.len(), 5);
}

#[test]
fn uniform_item_sizes_zero_count() {
    let sizes = uniform_item_sizes(0, px(30.));
    assert!(sizes.is_empty());
}

#[test]
fn uniform_item_sizes_each_element_height() {
    let height = px(42.);
    let sizes = uniform_item_sizes(3, height);
    for s in sizes.iter() {
        assert_eq!(s.height, height);
        assert_eq!(s.width, px(0.));
    }
}

#[test]
fn variable_item_sizes_matches_input_lengths() {
    let heights = vec![px(10.), px(20.), px(30.)];
    let sizes = variable_item_sizes(&heights);
    assert_eq!(sizes.len(), heights.len());
}

#[test]
fn variable_item_sizes_per_element_heights() {
    let heights = vec![px(10.), px(25.), px(40.)];
    let sizes = variable_item_sizes(&heights);
    for (s, &h) in sizes.iter().zip(heights.iter()) {
        assert_eq!(s.height, h);
        assert_eq!(s.width, px(0.));
    }
}

#[test]
fn bounded_list_height_no_gap() {
    let items = vec![size(px(0.), px(100.)), size(px(0.), px(200.))];
    let result = bounded_list_height(&items, px(0.), px(500.));
    assert_eq!(result, px(300.));
}

#[test]
fn bounded_list_height_with_gap() {
    let items = vec![
        size(px(0.), px(100.)),
        size(px(0.), px(100.)),
        size(px(0.), px(100.)),
    ];
    let result = bounded_list_height(&items, px(10.), px(500.));
    assert_eq!(result, px(320.));
}

#[test]
fn bounded_list_height_clamps_to_max() {
    let items = vec![size(px(0.), px(200.)), size(px(0.), px(200.))];
    let result = bounded_list_height(&items, px(0.), px(150.));
    assert_eq!(result, px(150.));
}

#[test]
fn bounded_list_height_single_item() {
    let items = vec![size(px(0.), px(75.))];
    let result = bounded_list_height(&items, px(10.), px(500.));
    assert_eq!(result, px(75.));
}

#[test]
fn bounded_list_height_empty() {
    let items: Vec<Size<Pixels>> = vec![];
    let result = bounded_list_height(&items, px(10.), px(500.));
    assert_eq!(result, px(0.));
}

#[test]
fn item_size_cache_hands_back_the_same_rc_while_uniform_sizes_hold() {
    let mut cache = ItemSizeCache::default();
    let first = cache.uniform(4, px(24.));
    let second = cache.uniform(4, px(24.));
    assert!(Rc::ptr_eq(&first, &second));
}

#[test]
fn item_size_cache_rebuilds_when_uniform_count_or_height_changes() {
    let mut cache = ItemSizeCache::default();
    let base = cache.uniform(4, px(24.));
    let grown = cache.uniform(5, px(24.));
    let retaller = cache.uniform(5, px(30.));
    assert!(!Rc::ptr_eq(&base, &grown));
    assert!(!Rc::ptr_eq(&grown, &retaller));
    assert_eq!(grown.len(), 5);
    assert_eq!(retaller[0].height, px(30.));
}

#[test]
fn item_size_cache_hands_back_the_same_rc_while_heights_hold() {
    let mut cache = ItemSizeCache::default();
    let heights = [px(10.), px(20.), px(30.)];
    let first = cache.variable(&heights);
    let second = cache.variable(&heights);
    assert!(Rc::ptr_eq(&first, &second));

    let retuned = cache.variable(&[px(10.), px(25.), px(30.)]);
    assert!(!Rc::ptr_eq(&first, &retuned));
    let shorter = cache.variable(&heights[..2]);
    assert!(!Rc::ptr_eq(&retuned, &shorter));
    assert_eq!(shorter.len(), 2);
}

struct ListHarness {
    scroll_handle: VirtualListScrollHandle,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    list_height: Pixels,
    gap: Pixels,
    show_scrollbar: bool,
    requested: Rc<RefCell<Vec<Range<usize>>>>,
}

impl Render for ListHarness {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let row_heights = self.item_sizes.clone();
        let requested = self.requested.clone();
        render_virtual_list(
            cx,
            "vlist-harness",
            self.item_sizes.clone(),
            self.list_height,
            self.gap,
            &self.scroll_handle,
            self.show_scrollbar,
            move |_, visible, _, _| {
                requested.borrow_mut().push(visible.clone());
                visible
                    .map(|ix| {
                        virtual_list_item(ix, format!("row {ix}"), ix)
                            .h(row_heights[ix].height)
                            .child(div())
                    })
                    .collect()
            },
        )
    }
}

// The kit measures item 0 before prepaint, so the last recorded range is the
// one actually painted.
fn draw_list(
    cx: &mut TestAppContext,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    list_height: Pixels,
    gap: Pixels,
    show_scrollbar: bool,
) -> Rc<RefCell<Vec<Range<usize>>>> {
    let requested = Rc::new(RefCell::new(Vec::new()));
    let scroll_handle = VirtualListScrollHandle::new();
    let window: AnyWindowHandle = cx
        .open_window(size(px(300.), px(300.)), {
            let requested = requested.clone();
            move |_, _| ListHarness {
                scroll_handle,
                item_sizes,
                list_height,
                gap,
                show_scrollbar,
                requested,
            }
        })
        .into();
    // An untyped handle: a typed one would lease ListHarness for the whole
    // closure, and the list's prepaint re-enters that entity to request rows.
    cx.update(|cx| {
        window
            .update(cx, |_, window, cx| window.draw(cx).clear(cx))
            .unwrap();
    });
    requested
}

#[gpui_kit::test]
async fn render_virtual_list_requests_every_row_of_a_short_list_with_gap(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let item_sizes = uniform_item_sizes(6, px(20.));
    // The same gap feeds the height math and the list, as at the call sites.
    let gap = px(4.);
    let list_height = bounded_list_height(&item_sizes, gap, px(200.));
    assert_eq!(list_height, px(140.));

    let requested = draw_list(cx, item_sizes, list_height, gap, true);
    assert_eq!(
        requested.borrow().last(),
        Some(&(0..6)),
        "a list taller than its content must paint every row"
    );
}

#[gpui_kit::test]
async fn render_virtual_list_without_gap_or_scrollbar_still_requests_rows(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let item_sizes = uniform_item_sizes(4, px(24.));
    let list_height = bounded_list_height(&item_sizes, px(0.), px(200.));

    let requested = draw_list(cx, item_sizes, list_height, px(0.), false);
    assert_eq!(
        requested.borrow().last(),
        Some(&(0..4)),
        "the gapless, scrollbar-free path must still materialize rows"
    );
}

#[gpui_kit::test]
async fn render_virtual_list_with_no_items_requests_no_rows(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let item_sizes = uniform_item_sizes(0, px(20.));
    let list_height = bounded_list_height(&item_sizes, px(4.), px(200.));
    assert_eq!(list_height, px(0.));

    let requested = draw_list(cx, item_sizes, list_height, px(4.), true);
    assert!(
        requested.borrow().is_empty(),
        "an empty list must draw without requesting rows"
    );
}
