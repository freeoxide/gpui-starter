//! WebView punt section: the upstream webview example embeds a native Wry
//! web view through the gpui-wry crate, which this app does not depend on.

use gpui_kit::component::text::TextView;
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, StyledExt as _, h_flex, v_flex};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::section;

const WEBVIEW_SHAPE: &str = "```rust
let webview = cx.new(|cx| {
    let native = wry::WebViewBuilder::new().build_as_child(&handle)?;
    WebView::new(native, window, cx)
});

webview.update(cx, |view, _| view.load_url(&url));
```";

pub struct WebViewSection;

impl WebViewSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self)
    }
}

impl Render for WebViewSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let paragraph = |line: &str| {
            div()
                .text_sm()
                .text_color(muted)
                .child(SharedString::from(line))
        };

        v_flex().w_full().items_center().gap_6().p_4().child(
            section("webview-why-none", "Why there is no web view here")
                .description(
                    "A native web view needs the gpui-wry, wry, and raw-window-handle \
                         crates, which this app does not depend on.",
                )
                .w(rems(30.))
                .v_flex()
                .items_start()
                .gap_3()
                .child(
                    h_flex()
                        .gap_3()
                        .child(Icon::new(IconName::Globe).size_6())
                        .child(div().text_sm().child(
                            "On macOS and Windows the child view renders browser \
                                     content inside the window.",
                        )),
                )
                .child(paragraph(
                    "The Linux hosting path is unfinished. The native view also sits above \
                         GPUI content in the same rectangle, including popovers, dialogs, and \
                         menus.",
                ))
                .child(
                    v_flex()
                        .w_full()
                        .gap_2()
                        .child(TextView::markdown("webview-shape", WEBVIEW_SHAPE)),
                )
                .child(paragraph(
                    "For document content, the TextView HTML mode renders markup in place \
                         (see the HTML Render section). For an external page, open_url hands the \
                         link to the default browser.",
                )),
        )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "webview",
        "WebView",
        "Embedding a native web view needs crates this app does not depend on.",
        WebViewSection::view(window, cx),
    ));
}
