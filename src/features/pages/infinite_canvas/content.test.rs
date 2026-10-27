use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::{AppContext as _, Entity, Subscription, TestAppContext, point, px};

use crate::ui::canvas::{CanvasEvent, CanvasState, LayerId};

use super::content::{
    DOT_SIZE, NOTE_H, NOTE_W, NoteField, NoteKind, PALETTE_LEN, build_field, dot_offsets,
};

fn same_seed_same_ix_specs() -> [NoteField; 2] {
    [
        NoteField { seed: 7, count: 64 },
        NoteField { seed: 7, count: 64 },
    ]
}

#[test]
fn spec_is_deterministic_for_the_same_seed_and_index() {
    let [a, b] = same_seed_same_ix_specs();
    for ix in 0..64 {
        let sa = a.spec(ix).unwrap();
        let sb = b.spec(ix).unwrap();
        assert_eq!(
            sa.bounds.origin.x.as_f32().to_bits(),
            sb.bounds.origin.x.as_f32().to_bits()
        );
        assert_eq!(
            sa.bounds.origin.y.as_f32().to_bits(),
            sb.bounds.origin.y.as_f32().to_bits()
        );
        assert_eq!(sa.palette_ix, sb.palette_ix);
        assert_eq!(sa.kind, sb.kind);
        assert_eq!(sa.label, sb.label);
    }
}

#[test]
fn different_seeds_move_note_zero() {
    let a = NoteField { seed: 1, count: 16 }.spec(0).unwrap();
    let b = NoteField { seed: 2, count: 16 }.spec(0).unwrap();
    assert_ne!(
        a.bounds.origin.x.as_f32().to_bits(),
        b.bounds.origin.x.as_f32().to_bits()
    );
    assert_ne!(
        a.bounds.origin.y.as_f32().to_bits(),
        b.bounds.origin.y.as_f32().to_bits()
    );
}

#[test]
fn grid_dimensions_cover_the_count_exactly() {
    for &count in &[1usize, 5_000, 20_000] {
        let field = NoteField { seed: 1, count };
        let (cols, rows) = (field.cols(), field.rows());
        assert!(
            cols * rows >= count,
            "count {count}: {cols}x{rows} too small"
        );
        assert!(
            (cols - 1) * rows < count,
            "count {count}: {cols}x{rows} wastes a row"
        );
    }
}

#[test]
fn every_note_lies_inside_the_field_span() {
    let field = NoteField {
        seed: 42,
        count: 1_000,
    };
    let span = field.span();
    for ix in 0..field.count {
        let spec = field.spec(ix).unwrap();
        assert!(
            spec.bounds.is_contained_within(&span),
            "note {ix} escapes the span"
        );
    }
}

#[test]
fn palette_index_stays_in_range_and_kinds_follow_every_fourth() {
    let field = NoteField {
        seed: 9,
        count: 500,
    };
    for ix in 0..field.count {
        let spec = field.spec(ix).unwrap();
        assert!(spec.palette_ix < PALETTE_LEN);
        let expected = if ix % 4 == 3 {
            NoteKind::DotCard
        } else {
            NoteKind::Sticky
        };
        assert_eq!(spec.kind, expected, "note {ix}");
        assert_eq!(
            spec.label.is_empty(),
            ix % 4 == 3,
            "label presence must follow the kind"
        );
    }
}

#[test]
fn spec_past_the_count_is_none() {
    let field = NoteField { seed: 3, count: 10 };
    assert!(field.spec(10).is_none());
    assert!(field.spec(0).is_some());
}

#[test]
fn anchors_match_spec_geometry_and_cover_every_note() {
    let field = NoteField {
        seed: 21,
        count: 97,
    };
    assert_eq!(field.anchors().count(), 97);
    for (ix, anchor) in field.anchors().enumerate() {
        let spec = field.spec(ix).unwrap();
        assert_eq!(anchor.palette_ix, spec.palette_ix, "note {ix}");
        let center = spec.bounds.center();
        assert!(
            (anchor.center.x - center.x).as_f32().abs() < 0.001
                && (anchor.center.y - center.y).as_f32().abs() < 0.001,
            "note {ix}: anchor {:?} vs spec center {center:?}",
            anchor.center
        );
    }
    let empty = NoteField { seed: 1, count: 0 };
    assert_eq!(empty.anchors().count(), 0);
}

#[test]
fn dot_offsets_stay_inside_a_note() {
    for off in dot_offsets() {
        assert!(off.x + DOT_SIZE <= NOTE_W, "dot at {off:?} overflows width");
        assert!(
            off.y + DOT_SIZE <= NOTE_H,
            "dot at {off:?} overflows height"
        );
        assert!(off.x >= px(0.) && off.y >= px(0.));
    }
}

#[test]
fn span_squares_the_grid_plus_the_outer_gap() {
    let field = NoteField { seed: 1, count: 25 };
    let span = field.span();
    assert_eq!((field.cols(), field.rows()), (5, 5));
    let step = (NOTE_W + px(40.)).as_f32();
    let expected = px(5. * step + 40.);
    assert_eq!(
        span.size.width.as_f32().to_bits(),
        expected.as_f32().to_bits()
    );
    assert_eq!(
        span.size.height.as_f32().to_bits(),
        expected.as_f32().to_bits()
    );
    assert_eq!(span.origin, point(px(0.), px(0.)));
}

fn doc_changed_counter(
    cx: &TestAppContext,
    state: &Entity<CanvasState>,
) -> (Rc<RefCell<Vec<CanvasEvent>>>, Subscription) {
    let events = Rc::new(RefCell::new(Vec::new()));
    let sink = events.clone();
    let subscription = cx.update(|cx| {
        cx.subscribe(state, move |_, event: &CanvasEvent, _| {
            sink.borrow_mut().push(event.clone())
        })
    });
    (events, subscription)
}

fn layer_shape_counts(
    cx: &TestAppContext,
    state: &Entity<CanvasState>,
    bodies: LayerId,
    labels: LayerId,
) -> (usize, usize) {
    cx.read(|cx| {
        let doc = state.read(cx).document();
        let (mut body_count, mut label_count) = (0, 0);
        for (layer, _, _) in doc.shapes() {
            if layer == bodies {
                body_count += 1;
            } else if layer == labels {
                label_count += 1;
            }
        }
        (body_count, label_count)
    })
}

#[gpui_kit::test]
fn build_field_batch_adds_exact_shape_counts_in_one_revision(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let state = cx.new(|_| CanvasState::new());
    let (events, _subscription) = doc_changed_counter(cx, &state);
    let field = NoteField {
        seed: 1,
        count: 100,
    };
    let (bodies, labels) = state.update(cx, |s, cx| build_field(s, &field, cx));
    assert_eq!(&*events.borrow(), &[CanvasEvent::DocumentChanged]);
    assert_eq!(cx.read(|cx| state.read(cx).document_revision()), 1);
    assert_eq!(cx.read(|cx| state.read(cx).document().layers().len()), 2);
    assert_eq!(layer_shape_counts(cx, &state, bodies, labels), (250, 75));
}

#[gpui_kit::test]
fn rebuild_with_a_new_seed_emits_exactly_one_more_document_changed(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let state = cx.new(|_| CanvasState::new());
    let (events, _subscription) = doc_changed_counter(cx, &state);
    let first = NoteField { seed: 1, count: 40 };
    state.update(cx, |s, cx| build_field(s, &first, cx));
    events.borrow_mut().clear();
    let second = NoteField { seed: 2, count: 40 };
    state.update(cx, |s, cx| build_field(s, &second, cx));
    assert_eq!(&*events.borrow(), &[CanvasEvent::DocumentChanged]);
    assert_eq!(cx.read(|cx| state.read(cx).document_revision()), 2);
    let counts = cx.read(|cx| {
        let doc = state.read(cx).document();
        doc.shapes().count()
    });
    assert_eq!(counts, 40 + 6 * (40 / 4) + (40 - 40 / 4));
}
