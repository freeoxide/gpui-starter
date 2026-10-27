use std::collections::HashMap;

use gpui_kit::{prelude::*, *};

use super::controller::TourController;

#[derive(Default)]
pub(crate) struct TargetRegistry {
    bounds: HashMap<&'static str, Bounds<Pixels>>,
    subscriber: Option<WeakEntity<TourController>>,
}

impl Global for TargetRegistry {}

/// Store `bounds` for `id`, notifying the subscribed controller only when the
/// value changed, so steady-state frames schedule no renders. Returns whether
/// the stored value changed.
pub(crate) fn publish(id: &'static str, bounds: Bounds<Pixels>, cx: &mut App) -> bool {
    let subscriber = {
        let registry = cx.default_global::<TargetRegistry>();
        if registry
            .bounds
            .get(&id)
            .is_some_and(|stored| *stored == bounds)
        {
            return false;
        }
        registry.bounds.insert(id, bounds);
        registry.subscriber.clone()
    };
    if let Some(subscriber) = subscriber {
        let _ = subscriber.update(cx, |_, cx| cx.notify());
    }
    true
}

/// Last published bounds for `id`, or None if it has never painted.
pub fn resolve(id: &str, cx: &App) -> Option<Bounds<Pixels>> {
    cx.try_global::<TargetRegistry>()?.bounds.get(id).copied()
}

/// Point change notifications at `subscriber`, replacing any previous one.
pub(crate) fn set_subscriber(subscriber: WeakEntity<TourController>, cx: &mut App) {
    cx.default_global::<TargetRegistry>().subscriber = Some(subscriber);
}

pub struct TourTarget {
    key: &'static str,
    child: Option<AnyElement>,
}

/// Publish the wrapped child's resolved bounds under `key` every drawn frame.
/// Adds no layout node and no element id, so the child keeps its element-id
/// path and a11y node unchanged. `key` must be unique per parent.
pub fn target(key: &'static str, child: impl IntoElement) -> TourTarget {
    TourTarget {
        key,
        child: Some(child.into_any_element()),
    }
}

impl IntoElement for TourTarget {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

// Delegation shape from gpui-pre 0.3.9 elements/animation.rs: returning the
// child's LayoutId makes the bounds passed to this prepaint the child's own.
impl Element for TourTarget {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, AnyElement) {
        let mut child = self
            .child
            .take()
            .expect("TourTarget renders once per frame");
        (child.request_layout(window, cx), child)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) {
        publish(self.key, bounds, cx);
        child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        child: &mut AnyElement,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        child.paint(window, cx);
    }
}

#[cfg(test)]
#[path = "target.test.rs"]
mod target_test;
