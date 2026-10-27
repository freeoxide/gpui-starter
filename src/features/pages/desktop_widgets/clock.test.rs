use super::clock_parts;

#[test]
fn clock_parts_formats_a_five_char_time_and_iso_date_shape() {
    let now = chrono::DateTime::parse_from_rfc3339("2026-10-14T09:05:07+00:00")
        .unwrap()
        .with_timezone(&chrono::Local);
    let (time, date) = clock_parts(now);
    assert_eq!(time.len(), 5);
    assert!(time.contains(':'));
    assert_eq!(date.len(), 10);
    let bytes = date.as_bytes();
    assert_eq!(bytes[4], b'-');
    assert_eq!(bytes[7], b'-');
    assert!(
        date.split('-')
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
    );
}

#[test]
fn clock_parts_reformats_a_new_timestamp() {
    let now = chrono::DateTime::parse_from_rfc3339("2026-01-02T23:59:00+00:00")
        .unwrap()
        .with_timezone(&chrono::Local);
    let (time, date) = clock_parts(now);
    assert_eq!(time.len(), 5);
    assert_eq!(date.len(), 10);
}
