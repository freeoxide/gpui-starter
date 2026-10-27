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
fn previous_window_is_empty_or_the_equal_length_stretch_before() {
    let data = load();
    assert!(data.previous_window(DateRange::Last90).is_empty());
    assert_eq!(data.previous_window(DateRange::Last7).len(), 7);
    let window = data.window(DateRange::Last7);
    let previous = data.previous_window(DateRange::Last7);
    assert_eq!(previous.len(), window.len());
}

#[test]
fn kpis_carry_deltas_only_when_history_exists() {
    let data = load();
    assert!(data.kpis(DateRange::Last30)[0].delta.is_some());
    assert!(data.kpis(DateRange::Last90)[0].delta.is_none());
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
