use super::*;

fn load() -> WorkspaceData {
    WorkspaceData::load()
}

#[test]
fn load_yields_90_daily_rows_and_48_channel_rows() {
    let data = load();
    assert_eq!(data.daily.len(), 90);
    assert_eq!(data.channels.len(), 48);
}

#[test]
fn daily_dates_are_strictly_increasing() {
    let data = load();
    let dates: Vec<&str> = data.daily.iter().map(|row| &*row.date).collect();
    assert!(dates.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(dates[0], "2025-07-01");
    assert_eq!(dates[89], "2025-09-28");
}

#[test]
fn window_slices_the_tail_by_range() {
    let data = load();
    assert_eq!(data.window(DateRange::Last7).len(), 7);
    assert_eq!(data.window(DateRange::Last90).len(), 90);
    fn dates(rows: &[DailyRow]) -> Vec<&str> {
        rows.iter().map(|row| &*row.date).collect()
    }
    let last90 = data.window(DateRange::Last90);
    assert_eq!(dates(data.window(DateRange::Last30)), dates(&last90[60..]));
}

#[test]
fn previous_window_is_the_stretch_of_days_directly_before_the_window() {
    let data = load();
    assert!(data.previous_window(DateRange::Last90).is_empty());
    let window = data.window(DateRange::Last7);
    let previous = data.previous_window(DateRange::Last7);
    assert_eq!(previous.len(), window.len());
    assert_eq!(previous.last().unwrap().date, "2025-09-21");
    assert_eq!(window.first().unwrap().date, "2025-09-22");
    let window_dates: std::collections::HashSet<_> =
        window.iter().map(|row| row.date.clone()).collect();
    assert!(
        previous.iter().all(|row| !window_dates.contains(&row.date)),
        "the two stretches must be disjoint"
    );
}

#[test]
fn kpis_carry_deltas_only_when_history_exists() {
    let data = load();
    assert!(data.kpis(DateRange::Last30)[0].delta.is_some());
    assert!(data.kpis(DateRange::Last90)[0].delta.is_none());
}

#[test]
fn visitors_delta_is_computed_from_the_two_window_sums() {
    let data = load();
    let kpis = data.kpis(DateRange::Last30);
    assert_eq!(kpis[0].label_key, "analytics_kpi_visitors");
    let delta = kpis[0].delta.expect("Last30 has a previous stretch");
    // Visitors over the two fixture stretches: 331125 vs 334873.
    assert!((delta - (331125. - 334873.) / 334873. * 100.).abs() < 1e-9);
}

#[test]
fn change_percent_handles_zero_previous_and_signs() {
    assert_eq!(change_percent(5., 0.), 0.);
    assert_eq!(change_percent(0., 0.), 0.);
    assert_eq!(change_percent(150., 100.), 50.);
    assert_eq!(change_percent(-10., -5.), -100.);
}

#[test]
fn channels_filter_by_segment_in_original_order() {
    let data = load();
    assert_eq!(data.channels(Segment::All).len(), 48);
    assert_eq!(data.channels(Segment::Social).len(), 12);
    let social = data.channels(Segment::Social);
    assert!(social.iter().all(|row| row.channel == "social"));
}

#[test]
fn channel_series_covers_90_days_and_rejects_unknown_channels() {
    let data = load();
    assert_eq!(data.channel_series("organic").len(), 90);
    assert!(data.channel_series("nope").is_empty());
}

#[test]
fn channel_row_resolves_a_domain_key() {
    let data = load();
    let row = &data.channels(Segment::All)[0];
    let key = ChannelKey::of(row);
    let found = data.channel_row(&key).expect("key must resolve");
    assert_eq!(found.page, row.page);
    assert_eq!(found.channel, row.channel);
}

#[test]
fn top_pages_returns_unique_pages_sorted_by_session_total() {
    let data = load();
    let top = data.top_pages(Segment::All, 6);
    assert_eq!(top.len(), 6);
    let pages: Vec<&str> = top.iter().map(|row| &*row.page).collect();
    let unique: std::collections::HashSet<&str> = pages.iter().copied().collect();
    assert_eq!(unique.len(), pages.len(), "band keys must not collide");
    assert!(
        top.windows(2)
            .all(|pair| pair[0].sessions >= pair[1].sessions)
    );

    let by_hand: f64 = data
        .channels(Segment::All)
        .iter()
        .filter(|row| row.page == top[0].page)
        .map(|row| row.sessions)
        .sum();
    assert_eq!(top[0].sessions, by_hand);
}

#[test]
fn top_pages_honors_the_segment_filter() {
    let data = load();
    let top = data.top_pages(Segment::Social, 6);
    let social = data.channels(Segment::Social);
    let social_pages: std::collections::HashSet<&str> =
        social.iter().map(|row| &*row.page).collect();
    assert!(top.iter().all(|row| social_pages.contains(&*row.page)));
}
