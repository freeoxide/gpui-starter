use gpui_kit::component::ActiveTheme as _;
use gpui_kit::{App, Bounds, Context, Hsla, Pixels, Point, point, px, size};

use crate::ui::canvas::{
    CanvasDocument, CanvasGrid, CanvasShape, CanvasState, LayerId, ShapeStyle, StrokeStyle,
};

pub(crate) const NOTE_W: Pixels = px(140.);
pub(crate) const NOTE_H: Pixels = px(90.);
pub(crate) const NOTE_GAP: Pixels = px(40.);
pub(crate) const PALETTE_LEN: usize = 5;
pub(crate) const DOT_SIZE: Pixels = px(10.);

pub(crate) fn splitmix64(x: u64) -> u64 {
    let x = x.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NoteKind {
    Sticky,
    DotCard,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NoteSpec {
    pub bounds: Bounds<Pixels>,
    pub palette_ix: usize,
    pub kind: NoteKind,
    pub label: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct NoteAnchor {
    pub center: Point<Pixels>,
    pub palette_ix: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NoteField {
    pub seed: u64,
    pub count: usize,
}

fn mixed_hash(seed: u64, ix: usize) -> u64 {
    splitmix64(seed ^ splitmix64(ix as u64 + 1))
}

fn note_origin(cols: usize, ix: usize, step: f32, h: u64) -> Point<Pixels> {
    let rx = (h as u32) as f32 / u32::MAX as f32;
    let ry = ((h >> 32) as u32) as f32 / u32::MAX as f32;
    let (col, row) = (ix % cols, ix / cols);
    point(
        px(col as f32 * step + NOTE_GAP.as_f32() + rx * NOTE_GAP.as_f32()),
        px(row as f32 * step + NOTE_GAP.as_f32() + ry * NOTE_GAP.as_f32()),
    )
}

fn note_palette_ix(h: u64) -> usize {
    ((h >> 48) as usize) % PALETTE_LEN
}

fn note_kind(ix: usize) -> NoteKind {
    if ix % 4 == 3 {
        NoteKind::DotCard
    } else {
        NoteKind::Sticky
    }
}

impl NoteField {
    pub(crate) fn cols(&self) -> usize {
        ((self.count as f32).sqrt().ceil() as usize).max(1)
    }

    pub(crate) fn rows(&self) -> usize {
        self.count.div_ceil(self.cols())
    }

    pub(crate) fn step(&self) -> f32 {
        (NOTE_W + NOTE_GAP).as_f32()
    }

    pub(crate) fn span(&self) -> Bounds<Pixels> {
        let step = self.step();
        Bounds {
            origin: Point::default(),
            size: size(
                px(self.cols() as f32 * step + NOTE_GAP.as_f32()),
                px(self.rows() as f32 * step + NOTE_GAP.as_f32()),
            ),
        }
    }

    pub(crate) fn spec(&self, ix: usize) -> Option<NoteSpec> {
        if ix >= self.count {
            return None;
        }
        let h = mixed_hash(self.seed, ix);
        let kind = note_kind(ix);
        Some(NoteSpec {
            bounds: Bounds {
                origin: note_origin(self.cols(), ix, self.step(), h),
                size: size(NOTE_W, NOTE_H),
            },
            palette_ix: note_palette_ix(h),
            kind,
            label: match kind {
                NoteKind::Sticky => format!("Note {ix}"),
                NoteKind::DotCard => String::new(),
            },
        })
    }

    pub(crate) fn anchors(&self) -> impl Iterator<Item = NoteAnchor> + '_ {
        let cols = self.cols();
        let step = self.step();
        (0..self.count).map(move |ix| {
            let h = mixed_hash(self.seed, ix);
            let origin = note_origin(cols, ix, step, h);
            NoteAnchor {
                center: origin + point(NOTE_W * 0.5, NOTE_H * 0.5),
                palette_ix: note_palette_ix(h),
            }
        })
    }
}

pub(crate) fn dot_offsets() -> [Point<Pixels>; 6] {
    let xs = [12., 65., 118.];
    let ys = [12., 65.];
    let mut offsets = [Point::default(); 6];
    let mut ix = 0;
    for &y in &ys {
        for &x in &xs {
            offsets[ix] = point(px(x), px(y));
            ix += 1;
        }
    }
    offsets
}

pub(crate) fn note_palette(cx: &App) -> [Hsla; PALETTE_LEN] {
    let t = cx.theme();
    [t.accent, t.primary, t.secondary, t.info, t.success]
}

/// Rebuilds the whole note document in one `edit_document` batch: one
/// revision bump, one `DocumentChanged`, one notify.
pub(crate) fn build_field(
    state: &mut CanvasState,
    field: &NoteField,
    cx: &mut Context<CanvasState>,
) -> (LayerId, LayerId) {
    let palette = note_palette(cx);
    let border = cx.theme().border;
    let text_color = cx.theme().foreground.opacity(0.9);
    let (bodies, labels) = state.edit_document(cx, |doc: &mut CanvasDocument| {
        doc.clear();
        let bodies = doc.add_layer();
        let labels = doc.add_layer();
        for ix in 0..field.count {
            let spec = field.spec(ix).unwrap();
            doc.add_shape(
                bodies,
                CanvasShape::rect(
                    spec.bounds,
                    ShapeStyle::both(
                        palette[spec.palette_ix].opacity(0.85),
                        StrokeStyle {
                            width: px(1.),
                            color: border,
                            dash: None,
                        },
                    ),
                ),
            );
            match spec.kind {
                NoteKind::Sticky => {
                    doc.add_shape(
                        labels,
                        CanvasShape::text(
                            spec.bounds.origin + point(px(8.), px(6.)),
                            spec.label.clone(),
                            px(12.),
                            text_color,
                        ),
                    );
                }
                NoteKind::DotCard => {
                    for off in dot_offsets() {
                        doc.add_shape(
                            bodies,
                            CanvasShape::rect(
                                Bounds {
                                    origin: spec.bounds.origin + off,
                                    size: size(DOT_SIZE, DOT_SIZE),
                                },
                                ShapeStyle::filled(palette[spec.palette_ix]),
                            ),
                        );
                    }
                }
            }
        }
        (bodies, labels)
    });
    state.set_grid(
        CanvasGrid {
            visible: true,
            spacing: px(64.),
        },
        cx,
    );
    (bodies, labels)
}
