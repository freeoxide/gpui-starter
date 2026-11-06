use super::clock_parts;

#[test]
fn clock_parts_formats_24h_time_and_the_iso_date_of_the_same_instant() {
    let now = chrono::DateTime::parse_from_rfc3339("2026-10-14T09:05:07+00:00").unwrap();
    let (time, date) = clock_parts(now);
    assert_eq!(time, "09:05");
    assert_eq!(date, "2026-10-14");
}

#[test]
fn clock_parts_rolls_the_date_at_zone_midnight() {
    let now = chrono::DateTime::parse_from_rfc3339("2026-01-02T23:59:00+00:00").unwrap();
    let (time, date) = clock_parts(now);
    assert_eq!(time, "23:59");
    assert_eq!(date, "2026-01-02");
}
