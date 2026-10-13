use gpui_kit::AppContext as _;

use super::{CanvasDemo, CanvasShowcasePage};
use crate::sidebar::Page;

#[test]
fn every_showcase_key_resolves_in_the_catalog() {
    let mut keys = vec![
        "nav_canvas_showcase",
        "showcase_page_desc",
        "showcase_reset",
        "showcase_reset_view",
        "showcase_play",
        "showcase_pause",
        "showcase_selected_count",
    ];
    for demo in CanvasDemo::ALL {
        keys.push(demo.tab_key());
        keys.push(demo.title_key());
        keys.push(demo.desc_key());
    }
    for key in keys {
        let localized = crate::i18n::localize(key);
        assert_ne!(
            localized, key,
            "showcase key `{key}` is missing from the catalogs"
        );
    }
}

#[test]
fn the_page_nav_key_is_the_sidebar_title_key() {
    assert_eq!(Page::CanvasShowcase.title_key(), "nav_canvas_showcase");
    assert_eq!(Page::CanvasShowcase.host(), "canvas-showcase");
    assert_ne!(
        crate::i18n::localize(Page::CanvasShowcase.title_key()),
        "nav_canvas_showcase"
    );
}

#[gpui_kit::test]
fn the_page_constructs_with_every_demo_registered(cx: &mut gpui_kit::TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let page = cx.new(|cx| CanvasShowcasePage::new(cx));
    cx.read(|cx| {
        let page = page.read(cx);
        assert_eq!(page.demos.len(), CanvasDemo::ALL.len());
        for (kind, _) in &page.demos {
            assert!(CanvasDemo::ALL.contains(kind));
        }
    });
}
