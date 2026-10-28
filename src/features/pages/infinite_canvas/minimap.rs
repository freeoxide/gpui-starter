use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::{ActiveTheme as _, ElementExt as _};
use gpui_kit::{
    App, Bounds, Corners, Entity, Hitbox, HitboxBehavior, Hsla, InteractiveElement, IntoElement,
    MouseDownEvent, ParentElement, Pixels, Point, RenderImage, Role, Size, Styled, Window, canvas,
    div, fill, point, px, size,
};

use crate::accessibility::A11yExt as _;
use crate::ui::canvas::CanvasState;

use super::InfiniteCanvasPage;
use super::content::{NoteField, PALETTE_LEN};
use super::viewport::local_viewport;

pub(crate) const MINIMAP_MAX_DOTS: usize = 20_000;
const DOT_PX: f32 = 2.;

#[cfg(test)]
pub(crate) mod minimap_probe {
    use std::cell::Cell;

    thread_local! {
        pub(crate) static DOT_ANCHOR_PASSES: Cell<u64> = const { Cell::new(0) };
    }

    pub(crate) fn add_dot_anchor_passes(count: u64) {
        DOT_ANCHOR_PASSES.with(|c| c.set(c.get() + count));
    }

    pub(crate) fn dot_anchor_passes() -> u64 {
        DOT_ANCHOR_PASSES.with(Cell::get)
    }

    pub(crate) fn reset() {
        DOT_ANCHOR_PASSES.with(|c| c.set(0));
    }
}

/// Dot raster retained across frames (field/palette/geometry keyed). Held
/// as page scratch: an entity created mid-paint re-measures on pass two.
#[derive(Default)]
pub(crate) struct MinimapDots {
    key: Option<u64>,
    image: Option<Arc<RenderImage>>,
}

impl MinimapDots {
    fn entry(
        &mut self,
        field: &NoteField,
        palette: &[Hsla; PALETTE_LEN],
        bounds: Bounds<Pixels>,
        pr: &MiniProjection,
        scale: f32,
    ) -> &Arc<RenderImage> {
        let key = dots_key(field, palette, bounds.size, scale);
        if self.key != Some(key) {
            self.image = Some(rasterize_dots(
                field,
                palette,
                pr,
                bounds.origin,
                bounds.size,
                scale,
            ));
            self.key = Some(key);
        }
        self.image.as_ref().expect("key set with image")
    }
}

fn dots_key(
    field: &NoteField,
    palette: &[Hsla; PALETTE_LEN],
    size: Size<Pixels>,
    scale: f32,
) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher as _};
    let mut hasher = DefaultHasher::new();
    field.seed.hash(&mut hasher);
    field.count.hash(&mut hasher);
    for color in palette {
        color.h.to_bits().hash(&mut hasher);
        color.s.to_bits().hash(&mut hasher);
        color.l.to_bits().hash(&mut hasher);
        color.a.to_bits().hash(&mut hasher);
    }
    size.width.as_f32().to_bits().hash(&mut hasher);
    size.height.as_f32().to_bits().hash(&mut hasher);
    scale.to_bits().hash(&mut hasher);
    hasher.finish()
}

/// Paints every anchor as a solid square into a transparent BGRA buffer; the
/// border-box-relative projection keeps dots aligned with the viewport rect.
pub(crate) fn rasterize_dots(
    field: &NoteField,
    palette: &[Hsla; PALETTE_LEN],
    pr: &MiniProjection,
    origin: Point<Pixels>,
    size: Size<Pixels>,
    scale: f32,
) -> Arc<RenderImage> {
    let width = ((size.width.as_f32() * scale).floor() as u32).max(1);
    let height = ((size.height.as_f32() * scale).floor() as u32).max(1);
    let dot = ((DOT_PX * scale).round() as u32).max(1);
    let dot_step = field.count.div_ceil(MINIMAP_MAX_DOTS).max(1);
    #[cfg(test)]
    minimap_probe::add_dot_anchor_passes(field.count.div_ceil(dot_step) as u64);
    let mut buffer = image::RgbaImage::new(width, height);
    for anchor in field.anchors().step_by(dot_step) {
        let center = (project_doc_to_minimap(anchor.center, pr) - origin) * scale;
        let rgba = palette[anchor.palette_ix].to_rgb();
        // RenderImage data is BGRA, not the Rgba type's RGBA order.
        let pixel = image::Rgba([
            (rgba.b * 255.).round() as u8,
            (rgba.g * 255.).round() as u8,
            (rgba.r * 255.).round() as u8,
            (rgba.a * 255.).round() as u8,
        ]);
        let x0 = center.x.as_f32().round() as i32;
        let y0 = center.y.as_f32().round() as i32;
        for dy in 0..dot as i32 {
            for dx in 0..dot as i32 {
                let (x, y) = (x0 + dx, y0 + dy);
                if (0..width as i32).contains(&x) && (0..height as i32).contains(&y) {
                    buffer.put_pixel(x as u32, y as u32, pixel);
                }
            }
        }
    }
    Arc::new(RenderImage::new([image::Frame::new(buffer)]))
}

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
    dots: Rc<RefCell<MinimapDots>>,
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
                    paint_minimap(&page, &state, &dots, bounds, hitbox, window, cx);
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
    dots: &Rc<RefCell<MinimapDots>>,
    bounds: Bounds<Pixels>,
    hitbox: Hitbox,
    window: &mut Window,
    cx: &mut App,
) {
    let (field, palette, minimap_bounds, viewport) = {
        let page = page.read(cx);
        (page.field, page.palette, page.minimap_bounds, page.viewport)
    };
    let pr = fit_projection(field.span(), minimap_bounds);
    let scale = window.scale_factor();
    let image = dots
        .borrow_mut()
        .entry(&field, &palette, bounds, &pr, scale)
        .clone();
    if let Err(err) = window.paint_image(bounds, bounds, Corners::default(), image, 0, false) {
        tracing::warn!("minimap image paint failed: {err}");
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
