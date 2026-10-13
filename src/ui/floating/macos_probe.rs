use gpui_kit::{Bounds, Edges, Pixels, point, px, size};
use objc2::MainThreadMarker;
use objc2_app_kit::NSScreen;

use super::probe::NotchGeometry;

pub(crate) fn notch_geometry() -> Option<NotchGeometry> {
    let mtm = MainThreadMarker::new()?;
    let screen = NSScreen::mainScreen(mtm)?;
    let insets = screen.safeAreaInsets();
    let left_aux = screen.auxiliaryTopLeftArea();
    let right_aux = screen.auxiliaryTopRightArea();
    let safe_area = Edges {
        top: px(insets.top as f32),
        right: px(insets.right as f32),
        bottom: px(insets.bottom as f32),
        left: px(insets.left as f32),
    };
    let gap_start = left_aux.origin.x + left_aux.size.width;
    let gap_end = right_aux.origin.x;
    let camera_housing = (gap_end > gap_start).then(|| Bounds {
        origin: point(px(gap_start as f32), px(left_aux.origin.y as f32)),
        size: size(
            px((gap_end - gap_start) as f32),
            px(left_aux.size.height as f32),
        ),
    });
    (safe_area.top > px(0.) || camera_housing.is_some()).then_some(NotchGeometry {
        safe_area,
        camera_housing,
    })
}
