use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

/// Spotlight rect: `target` expanded by `pad` on every side.
pub(crate) fn cutout_bounds(target: Bounds<Pixels>, pad: Pixels) -> Bounds<Pixels> {
    Bounds {
        origin: point(target.origin.x - pad, target.origin.y - pad),
        size: size(target.size.width + pad * 2., target.size.height + pad * 2.),
    }
}

/// Tooltip anchor: below the target when it fits, otherwise above, then
/// clamped into the viewport. `card_w`/`card_h_est` are the card's fixed
/// width and a conservative height estimate used only for placement.
pub(crate) fn place_card(
    target: Bounds<Pixels>,
    viewport: Bounds<Pixels>,
    card_w: Pixels,
    card_h_est: Pixels,
    gap: Pixels,
) -> Bounds<Pixels> {
    let below_y = target.bottom() + gap;
    let y = if below_y + card_h_est > viewport.bottom() {
        target.top() - gap - card_h_est
    } else {
        below_y
    };
    let x = target
        .left()
        .max(viewport.left())
        .min((viewport.right() - card_w).max(viewport.left()));
    let y = y
        .max(viewport.top())
        .min((viewport.bottom() - card_h_est).max(viewport.top()));
    Bounds {
        origin: point(x, y),
        size: size(card_w, card_h_est),
    }
}

/// The eight points a rounded-rect subpath passes through, clockwise from the
/// top edge: each straight edge ends `radius` short of a corner and each arc
/// lands `radius` past it.
pub(crate) fn rounded_rect_corners(rect: Bounds<Pixels>, radius: Pixels) -> [Point<Pixels>; 8] {
    [
        point(rect.origin.x + radius, rect.origin.y),
        point(rect.right() - radius, rect.origin.y),
        point(rect.right(), rect.origin.y + radius),
        point(rect.right(), rect.bottom() - radius),
        point(rect.right() - radius, rect.bottom()),
        point(rect.origin.x + radius, rect.bottom()),
        point(rect.origin.x, rect.bottom() - radius),
        point(rect.origin.x, rect.origin.y + radius),
    ]
}

/// Rounded-rect subpath (four arc_to corners) appended to `builder`.
pub(crate) fn rounded_rect_path(builder: &mut PathBuilder, rect: Bounds<Pixels>, radius: Pixels) {
    let [
        start,
        top_right,
        right_top,
        right_bottom,
        bottom_right,
        bottom_left,
        left_bottom,
        left_top,
    ] = rounded_rect_corners(rect, radius);
    let radii = point(radius, radius);
    builder.move_to(start);
    builder.line_to(top_right);
    builder.arc_to(radii, px(0.), false, true, right_top);
    builder.line_to(right_bottom);
    builder.arc_to(radii, px(0.), false, true, bottom_right);
    builder.line_to(bottom_left);
    builder.arc_to(radii, px(0.), false, true, left_bottom);
    builder.line_to(left_top);
    builder.arc_to(radii, px(0.), false, true, start);
    builder.close();
}

/// Paint the even-odd dim mask (viewport rect minus cutout) with the themed
/// overlay scrim, then the glow ring quad in the theme's primary color.
pub(crate) fn paint_spotlight(
    window: &mut Window,
    cx: &App,
    viewport: Bounds<Pixels>,
    cutout: Bounds<Pixels>,
    rem: Pixels,
) {
    let radius = cx.theme().radius_lg;
    // EvenOdd is gpui's default fill rule; restating it keeps the
    // hole-punching intent visible at the only site that depends on it.
    let mut builder = PathBuilder::fill().with_style(PathStyle::Fill(FillOptions {
        fill_rule: FillRule::EvenOdd,
    }));
    builder.move_to(viewport.origin);
    builder.line_to(point(viewport.right(), viewport.origin.y));
    builder.line_to(point(viewport.right(), viewport.bottom()));
    builder.line_to(point(viewport.origin.x, viewport.bottom()));
    builder.close();
    rounded_rect_path(&mut builder, cutout, radius);
    match builder.build() {
        Ok(path) => window.paint_path(path, cx.theme().overlay),
        Err(error) => tracing::warn!(
            target: super::LOG,
            %error,
            "spotlight mask path failed to build; skipping paint"
        ),
    }
    let border_w = rems(0.125).to_pixels(rem);
    window.paint_quad(quad(
        cutout,
        Corners::all(radius),
        transparent_black(),
        Edges::all(border_w),
        cx.theme().primary,
        BorderStyle::Solid,
    ));
}

#[cfg(test)]
#[path = "mask.test.rs"]
mod mask_test;
