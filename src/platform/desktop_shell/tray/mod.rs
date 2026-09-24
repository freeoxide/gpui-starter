#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub use macos::setup;
#[cfg(target_os = "windows")]
pub use windows::setup;

#[cfg(test)]
#[path = "tray.test.rs"]
mod tray_test;

const LOG: &str = "gpui_starter::tray";

// Magnifying-glass tray glyph drawn in code to avoid an asset; sized per
// platform (36 for the macOS menu bar, 16 for the Windows tray at 100% DPI).
pub(crate) fn icon(size: usize) -> tray_icon::Icon {
    tray_icon::Icon::from_rgba(icon_pixels(size), size as u32, size as u32)
        .expect("tray icon pixel buffer matches the requested dimensions by construction")
}

pub(crate) fn icon_pixels(size: usize) -> Vec<u8> {
    let mut px = vec![0u8; size * size * 4];

    let cx = size as f32 * 0.42;
    let cy = size as f32 * 0.42;
    let r_outer = size as f32 * 0.30;
    let r_inner = r_outer - 3.2;

    let hx0 = cx + r_outer * 0.65;
    let hy0 = cy + r_outer * 0.65;
    let hx1 = size as f32 * 0.86;
    let hy1 = size as f32 * 0.86;

    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;

            let dx = fx - cx;
            let dy = fy - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let lens_a: f32 = if dist >= r_inner && dist <= r_outer {
                1.0
            } else if dist < r_inner {
                ((dist - (r_inner - 1.0)) / 1.0).clamp(0.0, 1.0)
            } else {
                (1.0 - (dist - r_outer) / 1.0).clamp(0.0, 1.0)
            };

            let ex = hx1 - hx0;
            let ey = hy1 - hy0;
            let len2 = ex * ex + ey * ey;
            let t = ((fx - hx0) * ex + (fy - hy0) * ey) / len2;
            let t = t.clamp(0.0, 1.0);
            let px2 = hx0 + t * ex;
            let py2 = hy0 + t * ey;
            let d_h = ((fx - px2) * (fx - px2) + (fy - py2) * (fy - py2)).sqrt();
            let handle_a: f32 = if d_h <= 1.8 {
                1.0
            } else {
                (1.0 - (d_h - 1.8) / 1.0).clamp(0.0, 1.0)
            };

            let a = (lens_a.max(handle_a) * 255.0) as u8;
            let i = (y * size + x) * 4;
            px[i] = 0;
            px[i + 1] = 0;
            px[i + 2] = 0;
            px[i + 3] = a;
        }
    }

    px
}
