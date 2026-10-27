use gpui_kit::TestAppContext;

use crate::ui::canvas::CanvasShape;

use super::{TEXT_SEEDS, TextDemo};

#[gpui_kit::test]
fn seed_builds_six_texts_over_one_backing_rect(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let handle = cx.add_window(|_, cx| TextDemo::new(cx));
    let demo = handle.root(cx).expect("demo root entity");
    let state = cx.read(|cx| demo.read(cx).state.clone());
    let shapes = cx.read(|cx| state.read(cx).document().layers()[0].shapes().to_vec());
    assert_eq!(shapes.len(), 7);
    assert!(matches!(shapes[0].1, CanvasShape::Rect { .. }));
    let texts = &shapes[1..];
    for ((_, shape), (text, _, font_size, _)) in texts.iter().zip(TEXT_SEEDS) {
        let CanvasShape::Text {
            text: seeded_text,
            font_size: seeded_size,
            ..
        } = shape
        else {
            panic!("seeded shape must be text");
        };
        assert_eq!(&**seeded_text, text);
        assert!((seeded_size.as_f32() - font_size).abs() < 0.01);
    }
}

#[gpui_kit::test]
fn seeded_texts_carry_theme_derived_colors(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let handle = cx.add_window(|_, cx| TextDemo::new(cx));
    let demo = handle.root(cx).expect("demo root entity");
    let state = cx.read(|cx| demo.read(cx).state.clone());
    let (foreground, muted_foreground, primary) = cx.read(|cx| {
        let cx = gpui_kit::component::ActiveTheme::theme(cx);
        (cx.foreground, cx.muted_foreground, cx.primary)
    });
    let texts = cx.read(|cx| state.read(cx).document().layers()[0].shapes()[1..].to_vec());
    for ((_, shape), (_, _, _, expected)) in texts.iter().zip(TEXT_SEEDS) {
        let CanvasShape::Text { color, .. } = shape else {
            panic!("seeded shape must be text");
        };
        let expected = match expected {
            super::SeedColor::Foreground => foreground,
            super::SeedColor::MutedForeground => muted_foreground,
            super::SeedColor::Primary => primary,
        };
        assert_eq!(*color, expected);
    }
}
