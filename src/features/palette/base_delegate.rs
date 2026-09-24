//! Selection and filtered-view state for the command palette, independent of GPUI.

/// All mutation helpers are infallible (clamped or ignored).
pub struct BaseDelegate<T: Clone> {
    items: Vec<T>,
    /// Indices into [`BaseDelegate::items`] that are currently visible.
    filtered_indices: Vec<usize>,
    /// Selected position inside the *filtered* view (`None` = empty).
    selected_index: Option<usize>,
}

impl<T: Clone> BaseDelegate<T> {
    /// Create a new base delegate owning `items`, all visible, first selected.
    pub fn new(items: Vec<T>) -> Self {
        let selected_index = if items.is_empty() { None } else { Some(0) };
        let filtered_indices = (0..items.len()).collect();
        Self {
            items,
            filtered_indices,
            selected_index,
        }
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Set the selection without bounds checking; callers ensure validity.
    pub fn set_selected_unchecked(&mut self, index: usize) {
        self.selected_index = Some(index);
    }

    pub fn filtered_count(&self) -> usize {
        self.filtered_indices.len()
    }

    /// Replace the filtered view with `indices` and re-select the first item.
    pub fn apply_filtered_indices(&mut self, indices: Vec<usize>) {
        self.filtered_indices = indices;
        self.selected_index = if self.filtered_indices.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    /// Fetch an item by its *filtered* position.
    pub fn get_filtered_item(&self, filtered_index: usize) -> Option<&T> {
        self.filtered_indices
            .get(filtered_index)
            .and_then(|&item_idx| self.items.get(item_idx))
    }

    /// Move the selection down by one, wrapping at the bottom. No-op when empty.
    pub fn select_down(&mut self) {
        let count = self.filtered_count();
        if count == 0 {
            return;
        }
        let current = self.selected_index.unwrap_or(0);
        let next = if current + 1 >= count { 0 } else { current + 1 };
        self.selected_index = Some(next);
    }

    /// Move the selection up by one, wrapping at the top. No-op when empty.
    pub fn select_up(&mut self) {
        let count = self.filtered_count();
        if count == 0 {
            return;
        }
        let current = self.selected_index.unwrap_or(0);
        let prev = if current == 0 { count - 1 } else { current - 1 };
        self.selected_index = Some(prev);
    }

    pub fn items(&self) -> &[T] {
        &self.items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation_initialises_filter_and_selection() {
        let d = BaseDelegate::new(vec!["a", "b", "c"]);
        assert_eq!(d.filtered_count(), 3);
        assert_eq!(d.selected_index(), Some(0));
    }

    #[test]
    fn empty_set_has_no_selection() {
        let d: BaseDelegate<&str> = BaseDelegate::new(vec![]);
        assert_eq!(d.selected_index(), None);
        assert_eq!(d.filtered_count(), 0);
    }

    #[test]
    fn navigation_wraps() {
        let mut d = BaseDelegate::new(vec!["a", "b", "c"]);
        d.select_down();
        d.select_down();
        assert_eq!(d.selected_index(), Some(2));
        d.select_down();
        assert_eq!(d.selected_index(), Some(0));
        d.select_up();
        assert_eq!(d.selected_index(), Some(2));
    }

    #[test]
    fn apply_filtered_indices_resets_selection() {
        let mut d = BaseDelegate::new(vec!["a", "b", "c", "d"]);
        d.apply_filtered_indices(vec![1, 3]);
        assert_eq!(d.selected_index(), Some(0));
        assert_eq!(d.get_filtered_item(0), Some(&"b"));
        assert_eq!(d.filtered_count(), 2);
    }

    #[test]
    fn get_filtered_item_maps_through_indices() {
        let d = BaseDelegate::new(vec!["a", "b", "c"]);
        assert_eq!(d.get_filtered_item(2), Some(&"c"));
        assert_eq!(d.get_filtered_item(3), None);
    }
}
