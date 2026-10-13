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
fn percent_formats_with_the_requested_digits() {
    assert_eq!(percent(3.456, 1), "3.5%");
    assert_eq!(percent(2., 0), "2%");
}
