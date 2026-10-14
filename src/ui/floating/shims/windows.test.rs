use super::*;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOPMOST;

#[test]
fn with_ex_bits_sets_both_bits_on_a_zero_word() {
    assert_eq!(
        with_ex_bits(0, CLICK_THROUGH_BITS.0, true),
        CLICK_THROUGH_BITS.0
    );
}

#[test]
fn with_ex_bits_clears_both_bits_without_touching_neighbors() {
    let topmost = WS_EX_TOPMOST.0;
    let word = with_ex_bits(topmost, CLICK_THROUGH_BITS.0, true);
    assert_eq!(
        with_ex_bits(word, CLICK_THROUGH_BITS.0, false),
        topmost,
        "WS_EX_TOPMOST must survive a click-through clear"
    );
}

#[test]
fn with_ex_bits_is_idempotent() {
    let once = with_ex_bits(0, CLICK_THROUGH_BITS.0, true);
    let twice = with_ex_bits(once, CLICK_THROUGH_BITS.0, true);
    assert_eq!(once, twice);
    let cleared = with_ex_bits(once, CLICK_THROUGH_BITS.0, false);
    assert_eq!(with_ex_bits(cleared, CLICK_THROUGH_BITS.0, false), cleared);
}
