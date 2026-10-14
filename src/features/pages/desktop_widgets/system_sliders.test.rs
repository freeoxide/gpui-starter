use std::time::Duration;

use crate::platform::clock::Instant;

use super::{level_of, percent_of, should_sync};

#[test]
fn percent_of_clamps_both_ends_and_rounds() {
    assert_eq!(percent_of(0.), 0.);
    assert_eq!(percent_of(1.), 100.);
    assert_eq!(percent_of(-0.5), 0.);
    assert_eq!(percent_of(1.5), 100.);
    assert_eq!(percent_of(0.125), 13.);
}

#[test]
fn level_of_clamps_into_the_unit_range() {
    assert_eq!(level_of(0.), 0.);
    assert_eq!(level_of(100.), 1.);
    assert_eq!(level_of(-10.), 0.);
    assert_eq!(level_of(200.), 1.);
    assert_eq!(level_of(50.), 0.5);
}

#[test]
fn should_sync_allows_all_when_no_deadline_is_set() {
    let now = Instant::now();
    assert!(should_sync(now, None));
}

#[test]
fn should_sync_blocks_before_the_deadline_and_passes_at_it() {
    let now = Instant::now();
    let deadline = now + Duration::from_secs(2);
    assert!(!should_sync(now, Some(deadline)));
    assert!(!should_sync(
        deadline - Duration::from_millis(1),
        Some(deadline)
    ));
    assert!(should_sync(deadline, Some(deadline)));
    assert!(should_sync(
        deadline + Duration::from_secs(1),
        Some(deadline)
    ));
}
