use super::*;
use crate::platform::clock::Duration;

#[test]
fn padded_bytes_per_row_aligns_to_256() {
    assert_eq!(padded_bytes_per_row(512), 2048);
    assert_eq!(padded_bytes_per_row(513), 2304);
}

#[test]
fn rgba_to_bgra_swaps_channels_and_drops_padding() {
    let width: u32 = 2;
    let height: u32 = 2;
    let padded = padded_bytes_per_row(width);
    let mut rows = vec![0u8; padded * height as usize];
    rows[0..8].copy_from_slice(&[10, 20, 30, 40, 50, 60, 70, 80]);
    rows[padded..padded + 8].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);

    let out = rgba_to_bgra(&rows, width, height);

    assert_eq!(
        out,
        vec![30, 20, 10, 40, 70, 60, 50, 80, 3, 2, 1, 4, 7, 6, 5, 8]
    );
}

#[cfg(not(target_family = "wasm"))]
#[test]
fn bgra_to_render_image_round_trips_bytes() {
    let image = bgra_to_render_image(2, 1, vec![1, 2, 3, 4, 5, 6, 7, 8]);

    assert_eq!(image.size(0).width.0, 2);
    assert_eq!(image.size(0).height.0, 1);
    assert_eq!(image.as_bytes(0), Some(&[1, 2, 3, 4, 5, 6, 7, 8][..]));
}

#[cfg(not(target_family = "wasm"))]
#[test]
fn rgba_to_render_image_swaps_channel_zero_and_two() {
    let image = rgba_to_render_image(1, 1, vec![10, 20, 30, 40]);

    assert_eq!(image.as_bytes(0), Some(&[30, 20, 10, 40][..]));
}

#[test]
fn fps_tracker_scales_window_to_one_second() {
    let now = Instant::now();
    let mut tracker = FpsTracker::new();
    assert_eq!(tracker.fps(), 0);

    for _ in 0..75 {
        tracker.push(now);
    }
    assert_eq!(tracker.fps(), 100);

    tracker.push(now + Duration::from_millis(800));
    assert_eq!(tracker.fps(), 1);
}
