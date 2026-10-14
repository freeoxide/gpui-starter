use gpui_kit::{AnyWindowHandle, App, Bounds, DisplayId, Pixels, Point, SharedString, Size, point};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FloatingWindowId(pub SharedString);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatLevel {
    /// Above the app's normal windows. Not OS-topmost on Windows/X11.
    Floating,
    /// PopUp window kind: NSPopUp level + all spaces (macOS),
    /// WS_EX_TOPMOST (Windows), notification type (X11). Implies
    /// non-activating on macOS and non-resizable on Windows.
    AlwaysOnTop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement9 {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

#[derive(Clone, Debug)]
pub enum FloatAnchor {
    /// Place relative to a window's current bounds (usually `root_window(cx)`).
    Window {
        window: AnyWindowHandle,
        placement: Placement9,
        offset: Point<Pixels>,
    },
    /// Place relative to a display's visible bounds; None = primary display.
    Screen {
        display: Option<DisplayId>,
        placement: Placement9,
        offset: Point<Pixels>,
    },
}

impl FloatAnchor {
    pub fn placement(&self) -> Placement9 {
        match self {
            FloatAnchor::Window { placement, .. } | FloatAnchor::Screen { placement, .. } => {
                *placement
            }
        }
    }

    pub fn offset(&self) -> Point<Pixels> {
        match self {
            FloatAnchor::Window { offset, .. } | FloatAnchor::Screen { offset, .. } => *offset,
        }
    }
}

pub struct FloatingSpec {
    id: FloatingWindowId,
    size: Size<Pixels>,
    title: SharedString,
    level: FloatLevel,
    anchor: FloatAnchor,
    focus: bool,
    resizable: bool,
    movable: bool,
    click_through: bool,
    min_size: Option<Size<Pixels>>,
}

impl FloatingSpec {
    pub fn new(id: FloatingWindowId, size: Size<Pixels>) -> Self {
        let title = id.0.clone();
        Self {
            id,
            size,
            title,
            level: FloatLevel::Floating,
            anchor: FloatAnchor::Screen {
                display: None,
                placement: Placement9::Center,
                offset: Point::default(),
            },
            focus: false,
            resizable: false,
            movable: true,
            click_through: false,
            min_size: None,
        }
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    pub fn level(mut self, level: FloatLevel) -> Self {
        self.level = level;
        self
    }

    pub fn anchor(mut self, anchor: FloatAnchor) -> Self {
        self.anchor = anchor;
        self
    }

    pub fn focus(mut self, focus: bool) -> Self {
        self.focus = focus;
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn movable(mut self, movable: bool) -> Self {
        self.movable = movable;
        self
    }

    /// Spawn-time click-through: the float ignores all pointer input.
    /// Applied over the platform shim right after the native window opens;
    /// advisory where no shim exists (wasm overlays, test windows).
    pub fn click_through(mut self, enabled: bool) -> Self {
        self.click_through = enabled;
        self
    }

    pub fn min_size(mut self, min: Size<Pixels>) -> Self {
        self.min_size = Some(min);
        self
    }

    pub fn id(&self) -> &FloatingWindowId {
        &self.id
    }

    pub fn size(&self) -> Size<Pixels> {
        self.size
    }

    pub fn window_title(&self) -> SharedString {
        self.title.clone()
    }

    pub fn float_level(&self) -> FloatLevel {
        self.level
    }

    pub fn float_anchor(&self) -> &FloatAnchor {
        &self.anchor
    }

    pub fn is_focus(&self) -> bool {
        self.focus
    }

    pub fn is_resizable(&self) -> bool {
        self.resizable
    }

    pub fn is_movable(&self) -> bool {
        self.movable
    }

    pub fn is_click_through(&self) -> bool {
        self.click_through
    }

    pub(crate) fn set_level(&mut self, level: FloatLevel) {
        self.level = level;
    }

    pub fn min_window_size(&self) -> Option<Size<Pixels>> {
        self.min_size
    }
}

pub(crate) fn resolve_bounds(
    anchor_rect: Bounds<Pixels>,
    clamp_rect: Bounds<Pixels>,
    size: Size<Pixels>,
    placement: Placement9,
    offset: Point<Pixels>,
) -> Bounds<Pixels> {
    let anchor_center = anchor_rect.center();
    let origin = match placement {
        Placement9::TopLeft => anchor_rect.origin,
        Placement9::Top => point(anchor_center.x - size.width / 2.0, anchor_rect.origin.y),
        Placement9::TopRight => point(
            anchor_rect.origin.x + anchor_rect.size.width - size.width,
            anchor_rect.origin.y,
        ),
        Placement9::Left => point(anchor_rect.origin.x, anchor_center.y - size.height / 2.0),
        Placement9::Center => point(
            anchor_center.x - size.width / 2.0,
            anchor_center.y - size.height / 2.0,
        ),
        Placement9::Right => point(
            anchor_rect.origin.x + anchor_rect.size.width - size.width,
            anchor_center.y - size.height / 2.0,
        ),
        Placement9::BottomLeft => point(
            anchor_rect.origin.x,
            anchor_rect.origin.y + anchor_rect.size.height - size.height,
        ),
        Placement9::Bottom => point(
            anchor_center.x - size.width / 2.0,
            anchor_rect.origin.y + anchor_rect.size.height - size.height,
        ),
        Placement9::BottomRight => point(
            anchor_rect.origin.x + anchor_rect.size.width - size.width,
            anchor_rect.origin.y + anchor_rect.size.height - size.height,
        ),
    } + offset;

    let x = clamp_axis(
        origin.x,
        size.width,
        clamp_rect.origin.x,
        clamp_rect.size.width,
    );
    let y = clamp_axis(
        origin.y,
        size.height,
        clamp_rect.origin.y,
        clamp_rect.size.height,
    );
    Bounds {
        origin: point(x, y),
        size,
    }
}

fn clamp_axis(value: Pixels, size: Pixels, min: Pixels, available: Pixels) -> Pixels {
    let max = min + available - size;
    if max < min {
        min + available / 2.0 - size / 2.0
    } else {
        value.max(min).min(max)
    }
}

pub(crate) fn anchor_rect(
    anchor: &FloatAnchor,
    cx: &mut App,
) -> Option<(Bounds<Pixels>, Bounds<Pixels>)> {
    match anchor {
        FloatAnchor::Window { window, .. } => {
            let bounds = window
                .update(cx, |_, window, _| window.window_bounds().get_bounds())
                .ok()?;
            let clamp = cx
                .displays()
                .into_iter()
                .map(|display| display.visible_bounds())
                .find(|display_bounds| display_bounds.contains(&bounds.center()))
                .or_else(|| cx.primary_display().map(|display| display.visible_bounds()))?;
            Some((bounds, clamp))
        }
        FloatAnchor::Screen { display, .. } => {
            let platform_display = match display {
                Some(id) => cx.find_display(*id),
                None => cx.primary_display(),
            }?;
            let bounds = platform_display.visible_bounds();
            Some((bounds, bounds))
        }
    }
}

#[cfg(test)]
#[path = "spec.test.rs"]
mod spec_test;
