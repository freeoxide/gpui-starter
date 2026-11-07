use gpui_kit::TestAppContext;
use gpui_kit::component::{IndexPath, select::SelectDelegate as _};

use super::{COUNTRIES, country_groups};

/// Åland Islands splits the A run, so consecutive-prefix grouping opens
/// with groups sized 1, 1, 14 — not one group per distinct prefix.
#[gpui_kit::test]
fn country_groups_merge_only_consecutive_equal_prefixes(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let groups = country_groups();
        assert_eq!(groups.sections_count(cx), 27);

        let listed: usize = (0..groups.sections_count(cx))
            .map(|section| groups.items_count(section))
            .sum();
        assert_eq!(listed, COUNTRIES.len(), "every country stays listed");

        assert_eq!(groups.items_count(0), 1);
        assert_eq!(groups.items_count(1), 1);
        assert_eq!(groups.items_count(2), 14);
        assert_eq!(
            groups
                .item(IndexPath::default().section(0).row(0))
                .unwrap()
                .name,
            "Afghanistan"
        );
        assert_eq!(
            groups
                .item(IndexPath::default().section(1).row(0))
                .unwrap()
                .name,
            "Åland Islands"
        );
        assert_eq!(
            groups
                .item(IndexPath::default().section(2).row(0))
                .unwrap()
                .name,
            "Albania"
        );
    });
}
