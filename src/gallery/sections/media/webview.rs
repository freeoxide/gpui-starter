//! WebView punt section: the upstream webview example embeds a native Wry
//! web view through the gpui-wry crate, which this app does not depend on.

use gpui_kit::component::IconName;
use gpui_kit::*;

use crate::gallery::demo::punt_page;
use crate::gallery::registry::GallerySection;

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
        punt_page(
            "webview-why-none",
            "Why there is no web view here",
            "A native web view needs the gpui-wry, wry, and raw-window-handle \
             crates, which this app does not depend on.",
            IconName::Globe,
            "On macOS and Windows the child view renders browser \
             content inside the window.",
            "The Linux hosting path is unfinished. The native view also sits above \
             GPUI content in the same rectangle, including popovers, dialogs, and \
             menus.",
            "webview-shape",
            WEBVIEW_SHAPE,
            &[
                "For document content, the TextView HTML mode renders markup in place \
                 (see the HTML Render section). For an external page, open_url hands the \
                 link to the default browser.",
            ],
            cx,
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
