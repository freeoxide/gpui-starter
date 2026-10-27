use gpui_kit::component::{ActiveTheme as _, ElementExt as _};
use gpui_kit::{
    App, Bounds, Entity, Hitbox, HitboxBehavior, InteractiveElement, IntoElement, MouseDownEvent,
    ParentElement, Pixels, Point, Role, Styled, Window, canvas, div, fill, point, px, size,
};

use crate::accessibility::A11yExt as _;
use crate::ui::canvas::CanvasState;

use super::InfiniteCanvasPage;
use super::viewport::local_viewport;

pub(crate) const MINIMAP_MAX_DOTS: usize = 20_000;

pub(crate) struct MiniProjection {
    pub(crate) scale: f32,
    pub(crate) offset: Point<Pixels>,
}

pub(crate) fn fit_projection(field: Bounds<Pixels>, minimap: Bounds<Pixels>) -> MiniProjection {
    let scale = (minimap.size.width.as_f32() / field.size.width.as_f32())
        .min(minimap.size.height.as_f32() / field.size.height.as_f32());
    let leftover_w = (minimap.size.width - field.size.width * scale) * 0.5;
    let leftover_h = (minimap.size.height - field.size.height * scale) * 0.5;
    MiniProjection {
        scale,
        offset: point(minimap.origin.x + leftover_w, minimap.origin.y + leftover_h),
    }
}

pub(crate) fn project_doc_to_minimap(p: Point<Pixels>, pr: &MiniProjection) -> Point<Pixels> {
    pr.offset + p * pr.scale
}

pub(crate) fn unproject_minimap_to_doc(p: Point<Pixels>, pr: &MiniProjection) -> Point<Pixels> {
    point(
        px((p.x - pr.offset.x).as_f32() / pr.scale),
        px((p.y - pr.offset.y).as_f32() / pr.scale),
    )
}

pub(crate) fn minimap(
    page: Entity<InfiniteCanvasPage>,
    state: Entity<CanvasState>,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let (border, popover, radius_lg) = (theme.border, theme.popover, theme.radius_lg);
    div()
        .id("canvas-minimap")
        .absolute()
        .bottom_4()
        .right_4()
        .w_40()
        .h_24()
        .border_1()
        .border_color(border)
        .rounded(radius_lg)
        .bg(popover)
        .overflow_hidden()
        .a11y(Role::Image, crate::i18n::localize("canvas_minimap_label"))
        .on_prepaint({
            let page = page.clone();
            move |bounds, _, cx| {
                page.update(cx, |this, _| this.store_minimap_bounds(bounds));
            }
        })
        .child(
            canvas(
                move |bounds, window: &mut Window, _: &mut App| {
                    window.insert_hitbox(bounds, HitboxBehavior::BlockMouse)
                },
                move |bounds, hitbox, window: &mut Window, cx: &mut App| {
                    paint_minimap(&page, &state, bounds, hitbox, window, cx);
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
}

fn paint_minimap(
    page: &Entity<InfiniteCanvasPage>,
    state: &Entity<CanvasState>,
    _bounds: Bounds<Pixels>,
    hitbox: Hitbox,
    window: &mut Window,
    cx: &mut App,
) {
    let (field, palette, minimap_bounds, viewport) = {
        let page = page.read(cx);
        (page.field, page.palette, page.minimap_bounds, page.viewport)
    };
    let pr = fit_projection(field.span(), minimap_bounds);
    let dot_step = field.count.div_ceil(MINIMAP_MAX_DOTS).max(1);
    for anchor in field.anchors().step_by(dot_step) {
        let center = project_doc_to_minimap(anchor.center, &pr);
        window.paint_quad(fill(
            Bounds {
                origin: center - point(px(1.), px(1.)),
                size: size(px(2.), px(2.)),
            },
            palette[anchor.palette_ix],
        ));
    }
    let transform = state.read(cx).transform();
    let visible_doc = transform
        .to_doc_bounds(local_viewport(viewport))
        .intersect(&field.span());
    let rect = Bounds {
        origin: project_doc_to_minimap(visible_doc.origin, &pr),
        size: size(
            visible_doc.size.width * pr.scale,
            visible_doc.size.height * pr.scale,
        ),
    };
    let color = cx.theme().primary;
    let edges = [
        Bounds {
            origin: rect.origin,
            size: size(rect.size.width, px(1.)),
        },
        Bounds {
            origin: point(rect.origin.x, rect.origin.y + rect.size.height - px(1.)),
            size: size(rect.size.width, px(1.)),
        },
        Bounds {
            origin: rect.origin,
            size: size(px(1.), rect.size.height),
        },
        Bounds {
            origin: point(rect.origin.x + rect.size.width - px(1.), rect.origin.y),
            size: size(px(1.), rect.size.height),
        },
    ];
    for edge in edges {
        window.paint_quad(fill(edge.intersect(&minimap_bounds), color));
    }

    let page = page.clone();
    let state = state.clone();
    window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
        if !phase.bubble() || !hitbox.is_hovered(window) {
            return;
        }
        cx.stop_propagation();
        let doc = unproject_minimap_to_doc(event.position, &pr);
        let viewport = page.read(cx).viewport;
        state.update(cx, |state, cx| {
            let local_center = point(viewport.size.width * 0.5, viewport.size.height * 0.5);
            let target_pan = local_center - doc * state.zoom();
            let delta = target_pan - state.transform().pan;
            state.pan_by(delta, cx);
        });
    });
}
