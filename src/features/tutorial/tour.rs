use gpui_kit::SharedString;

use crate::routes::AppRoute;
use crate::sidebar::Page;

/// One tour stop. `route` is emitted as a Navigate event when the step
/// becomes current; None keeps the current page.
#[derive(Clone, Debug)]
pub struct TourStep {
    pub target: &'static str,
    pub title_key: &'static str,
    pub body_key: &'static str,
    pub route: Option<AppRoute>,
}

/// A tour's content. `steps` is non-empty by contract; the controller
/// rejects an empty definition.
#[derive(Clone, Debug)]
pub struct TourDefinition {
    pub id: SharedString,
    pub steps: Vec<TourStep>,
}

/// Pure step machine, saturating at both ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TourProgress {
    step_ix: usize,
    total: usize,
}

impl TourProgress {
    pub fn new(total: usize) -> Self {
        Self { step_ix: 0, total }
    }

    pub fn ix(&self) -> usize {
        self.step_ix
    }

    pub fn total(&self) -> usize {
        self.total
    }

    pub fn is_last(&self) -> bool {
        self.total > 0 && self.step_ix + 1 >= self.total
    }

    /// Whether the index moved; false when already on the last step.
    pub fn advance(&mut self) -> bool {
        if self.is_last() {
            return false;
        }
        self.step_ix += 1;
        true
    }

    /// Whether the index moved; false when already on the first step.
    pub fn go_back(&mut self) -> bool {
        if self.step_ix == 0 {
            return false;
        }
        self.step_ix -= 1;
        true
    }
}

/// Home to Gallery demo tour. "tour-viewport" is reserved for the AppRoot
/// wrap; do not use it as a step target.
pub fn demo_tour() -> TourDefinition {
    TourDefinition {
        id: "demo".into(),
        steps: vec![
            TourStep {
                target: "home-heading",
                title_key: "tutorial_step_home_heading_title",
                body_key: "tutorial_step_home_heading_body",
                route: Some(AppRoute::page(Page::Home)),
            },
            TourStep {
                target: "home-cta",
                title_key: "tutorial_step_home_cta_title",
                body_key: "tutorial_step_home_cta_body",
                route: None,
            },
            TourStep {
                target: "sidebar-gallery",
                title_key: "tutorial_step_sidebar_title",
                body_key: "tutorial_step_sidebar_body",
                route: None,
            },
            TourStep {
                target: "gallery-nav",
                title_key: "tutorial_step_gallery_nav_title",
                body_key: "tutorial_step_gallery_nav_body",
                route: Some(AppRoute::page(Page::Gallery)),
            },
            TourStep {
                target: "gallery-search",
                title_key: "tutorial_step_gallery_search_title",
                body_key: "tutorial_step_gallery_search_body",
                route: None,
            },
        ],
    }
}

#[cfg(test)]
#[path = "tour.test.rs"]
mod tour_test;
