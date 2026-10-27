use super::{TourProgress, demo_tour};

#[test]
fn progress_saturates_at_both_ends() {
    let mut progress = TourProgress::new(3);
    assert!(
        !progress.go_back(),
        "go_back on the first step must not move"
    );
    assert!(progress.advance());
    assert!(progress.advance());
    assert!(progress.is_last());
    assert!(
        !progress.advance(),
        "advance on the last step must not move"
    );
    assert_eq!(progress.ix(), 2);
}

#[test]
fn progress_reports_ix_total_is_last() {
    let progress = TourProgress::new(5);
    assert_eq!((progress.ix(), progress.total()), (0, 5));
    assert!(!progress.is_last());
    let mut single = TourProgress::new(1);
    assert!(single.is_last());
    assert!(!single.advance());
}

#[test]
fn demo_tour_targets_are_unique_and_nonempty() {
    let tour = demo_tour();
    assert!(!tour.steps.is_empty());
    let mut targets: Vec<_> = tour.steps.iter().map(|step| step.target).collect();
    targets.sort_unstable();
    let unique_len = {
        targets.dedup();
        targets.len()
    };
    assert_eq!(unique_len, demo_tour().steps.len());
}

#[test]
fn demo_tour_i18n_keys_resolve() {
    rust_i18n::set_locale("en");
    for step in demo_tour().steps {
        assert_ne!(
            crate::i18n::localize(step.title_key),
            step.title_key,
            "missing catalog entry for {}",
            step.title_key
        );
        assert_ne!(
            crate::i18n::localize(step.body_key),
            step.body_key,
            "missing catalog entry for {}",
            step.body_key
        );
    }
}
