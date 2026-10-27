use std::sync::atomic::{AtomicU64, Ordering};

pub(crate) const PAGE_SLOW_RENDER_THRESHOLD_US: u64 = 2_000;

static LAST_PAGE_RENDER_US: AtomicU64 = AtomicU64::new(0);
static LAST_GEN_US: AtomicU64 = AtomicU64::new(0);

pub(crate) fn store_page_render_us(elapsed_us: u64) {
    LAST_PAGE_RENDER_US.store(elapsed_us, Ordering::Relaxed);
}

pub(crate) fn last_page_render_us() -> u64 {
    LAST_PAGE_RENDER_US.load(Ordering::Relaxed)
}

pub(crate) fn store_gen_us(elapsed_us: u64) {
    LAST_GEN_US.store(elapsed_us, Ordering::Relaxed);
}

pub(crate) fn last_gen_us() -> u64 {
    LAST_GEN_US.load(Ordering::Relaxed)
}

pub(crate) fn is_slow_page_render(elapsed_us: u64) -> bool {
    elapsed_us > PAGE_SLOW_RENDER_THRESHOLD_US
}
