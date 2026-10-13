use gpui_kit::*;

/// Document coordinates are logical pixels at zoom 1.0; screen is viewport
/// space (element-local): `screen = doc * zoom + pan`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasTransform {
    pub pan: Point<Pixels>,
    pub zoom: f32,
}

impl CanvasTransform {
    pub const IDENTITY: Self = Self {
        pan: Point::new(px(0.), px(0.)),
        zoom: 1.,
    };

    pub fn new(pan: Point<Pixels>, zoom: f32) -> Self {
        Self { pan, zoom }
    }

    pub fn to_screen(&self, doc: Point<Pixels>) -> Point<Pixels> {
        doc * self.zoom + self.pan
    }

    pub fn to_doc(&self, screen: Point<Pixels>) -> Point<Pixels> {
        Point::new(
            px((screen.x - self.pan.x).as_f32() / self.zoom),
            px((screen.y - self.pan.y).as_f32() / self.zoom),
        )
    }

    pub fn to_screen_bounds(&self, doc: Bounds<Pixels>) -> Bounds<Pixels> {
        Bounds::from_corners(
            self.to_screen(doc.origin),
            self.to_screen(doc.bottom_right()),
        )
    }

    pub fn to_doc_bounds(&self, screen: Bounds<Pixels>) -> Bounds<Pixels> {
        Bounds::from_corners(
            self.to_doc(screen.origin),
            self.to_doc(screen.bottom_right()),
        )
    }

    pub fn scale_length(&self, doc_len: Pixels) -> Pixels {
        doc_len * self.zoom
    }
}

/// Distance from `point` to segment `a`..`b`, in the input units.
pub fn segment_distance(point: Point<Pixels>, a: Point<Pixels>, b: Point<Pixels>) -> Pixels {
    let (px_, py_) = (point.x.as_f32(), point.y.as_f32());
    let (ax, ay) = (a.x.as_f32(), a.y.as_f32());
    let (bx, by) = (b.x.as_f32(), b.y.as_f32());
    let (abx, aby) = (bx - ax, by - ay);
    let length_sq = abx * abx + aby * aby;
    let t = if length_sq == 0. {
        0.
    } else {
        (((px_ - ax) * abx + (py_ - ay) * aby) / length_sq).clamp(0., 1.)
    };
    let (cx, cy) = (ax + t * abx, ay + t * aby);
    px(((px_ - cx) * (px_ - cx) + (py_ - cy) * (py_ - cy)).sqrt())
}

#[cfg(test)]
#[path = "geometry.test.rs"]
mod geometry_test;
