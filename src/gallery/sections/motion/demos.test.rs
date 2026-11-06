use super::{END_MINUTES, START_MINUTES, advance_digit, time_digits};

#[test]
fn derives_time_digits() {
    assert_eq!(time_digits(START_MINUTES), [0, 8, 0, 0]);
    assert_eq!(time_digits(END_MINUTES), [2, 0, 0, 0]);
}

#[test]
fn rolls_forward_across_zero() {
    assert_eq!(advance_digit(8., 0), 10.);
    assert_eq!(advance_digit(19., 2), 22.);
}
