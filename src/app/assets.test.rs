use super::*;

#[test]
fn embedded_app_icons_decode_at_advertised_dimensions() {
    for size in [16u16, 32, 48] {
        let png = app_icon_png(size).expect("size is in the committed hicolor set");
        assert_eq!(png.dimensions(), (u32::from(size), u32::from(size)));
        assert!(png.pixels().any(|px| px.0[3] > 0));
    }
}

#[test]
fn app_icon_size_outside_the_set_is_none() {
    assert!(app_icon_png(20).is_none());
}

#[cfg(target_os = "linux")]
#[test]
fn window_icon_is_the_48px_brand_png() {
    let icon = window_icon().expect("48px brand png in the embedded set");
    assert_eq!(icon.dimensions(), (48, 48));
    assert!(icon.pixels().any(|px| px.0[3] > 0));
}
