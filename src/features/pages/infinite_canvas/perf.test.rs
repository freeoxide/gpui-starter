use super::perf::{
    PAGE_SLOW_RENDER_THRESHOLD_US, is_slow_page_render, last_gen_us, last_page_render_us,
    store_gen_us, store_page_render_us,
};

#[test]
fn render_and_gen_atomics_roundtrip() {
    store_page_render_us(1_234);
    assert_eq!(last_page_render_us(), 1_234);
    store_gen_us(56_789);
    assert_eq!(last_gen_us(), 56_789);
}

#[test]
fn slow_render_boundary_is_exclusive() {
    assert!(!is_slow_page_render(PAGE_SLOW_RENDER_THRESHOLD_US));
    assert!(is_slow_page_render(PAGE_SLOW_RENDER_THRESHOLD_US + 1));
}
