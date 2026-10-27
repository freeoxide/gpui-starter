use gpui_kit::{TestAppContext, Window, prelude::*};

use super::DetailSource;
use crate::features::pages::music::{AlbumId, Library, MusicPage, PlayerEngine, PlaylistId};

struct PageHost {
    page: gpui_kit::Entity<MusicPage>,
}

impl Render for PageHost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.page.clone()
    }
}

fn open_page_window(cx: &mut TestAppContext) -> gpui_kit::Entity<MusicPage> {
    let mut page = None;
    cx.add_window(|window, cx| {
        let engine = cx.new(|cx| PlayerEngine::new(Library::catalog(), cx));
        let entity = cx.new(|cx| MusicPage::new(engine, window, cx));
        page = Some(entity.clone());
        PageHost { page: entity }
    });
    page.unwrap()
}

#[gpui_kit::test]
fn library_renders_without_a_detail_open(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let page = open_page_window(cx);
    cx.read(|cx| {
        let page = page.read(cx);
        assert!(page.detail.is_none());
        assert!(!page.queue_panel_open);
        assert!(page.orders.is_empty());
        assert_eq!(page.detail_order().len(), 0);
    });
}

#[gpui_kit::test]
fn open_detail_swaps_the_rendered_source(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let page = open_page_window(cx);
    cx.update(|cx| {
        page.update(cx, |page, cx| {
            page.open_detail(DetailSource::Playlist(PlaylistId(1)), cx)
        })
    });
    cx.add_window(|_window, _cx| PageHost { page: page.clone() });
    cx.read(|cx| {
        let page = page.read(cx);
        assert_eq!(page.detail, Some(DetailSource::Playlist(PlaylistId(1))));
        assert_eq!(page.detail_order().len(), 120);
    });
}

#[gpui_kit::test]
fn detail_order_follows_catalog_until_a_move_lands(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let page = open_page_window(cx);
    cx.update(|cx| {
        page.update(cx, |page, cx| {
            page.open_detail(DetailSource::Playlist(PlaylistId(0)), cx)
        })
    });
    let catalog_order = cx.read(|cx| page.read(cx).detail_order());
    assert_eq!(catalog_order.len(), 12);

    cx.update(|cx| {
        page.update(cx, |page, cx| page.move_track(1, -1, cx));
    });
    let moved = cx.read(|cx| page.read(cx).detail_order());
    assert_eq!(moved[0], catalog_order[1]);
    assert_eq!(moved[1], catalog_order[0]);
    assert_eq!(&moved[2..], &catalog_order[2..]);

    cx.update(|cx| {
        page.update(cx, |page, cx| page.move_track(0, -1, cx));
    });
    let unchanged = cx.read(|cx| page.read(cx).detail_order());
    assert_eq!(unchanged, moved);
}

#[gpui_kit::test]
fn album_detail_keeps_orders_empty(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let page = open_page_window(cx);
    cx.update(|cx| {
        page.update(cx, |page, cx| {
            page.open_detail(DetailSource::Album(AlbumId(0)), cx);
            page.move_track(1, -1, cx);
        })
    });
    cx.read(|cx| {
        let page = page.read(cx);
        assert!(page.orders.is_empty());
        assert_eq!(
            page.detail_order().len(),
            page.library.albums()[0].track_ids.len()
        );
    });
}

#[gpui_kit::test]
fn queue_panel_toggles_and_renders_queue_rows(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let page = open_page_window(cx);
    let engine = cx.read(|cx| page.read(cx).player.clone());
    cx.update(|cx| {
        engine.update(cx, |engine, cx| {
            let order = Library::catalog()
                .playlist(PlaylistId(0))
                .unwrap()
                .track_ids
                .clone();
            engine.play_queue(order, 0, cx);
        });
        page.update(cx, |page, cx| page.toggle_queue_panel(cx));
    });
    cx.add_window(|_window, _cx| PageHost { page: page.clone() });
    cx.read(|cx| {
        let page = page.read(cx);
        assert!(page.queue_panel_open);
        assert_eq!(engine.read(cx).queue().len(), 12);
    });
    cx.update(|cx| {
        page.update(cx, |page, cx| page.toggle_queue_panel(cx));
    });
    cx.read(|cx| assert!(!page.read(cx).queue_panel_open));
}
