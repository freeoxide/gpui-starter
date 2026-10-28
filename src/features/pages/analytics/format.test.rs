use super::*;

#[test]
fn compact_formats_thousands_and_millions() {
    assert_eq!(compact(1234.), "1.2K");
    assert_eq!(compact(12345678.), "12.3M");
    assert_eq!(compact(312.), "312");
}

#[test]
fn money_keeps_the_sign_in_front_of_the_dollar() {
    assert_eq!(money(-45600.), "-$46K");
    assert_eq!(money(1234.), "$1.2K");
}

#[test]
fn money_keeps_cents_for_small_amounts() {
    assert_eq!(money(9.2), "$9.20");
    assert_eq!(money(-0.5), "-$0.50");
    assert_eq!(money(999.99), "$999.99");
    assert_eq!(money(1000.), "$1.0K");
}

#[test]
fn percent_formats_with_the_requested_digits() {
    assert_eq!(percent(3.456, 1), "3.5%");
    assert_eq!(percent(2., 0), "2%");
}

#[test]
fn duration_formats_minutes_and_seconds() {
    assert_eq!(duration(0.), "0m 0s");
    assert_eq!(duration(75.), "1m 15s");
    assert_eq!(duration(6042.), "100m 42s");
}
