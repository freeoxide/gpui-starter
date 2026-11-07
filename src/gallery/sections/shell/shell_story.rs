//! Shell story punt section: the upstream story renders a gpui-shell script
//! view, and this app does not depend on gpui-shell.

use gpui_kit::component::IconName;
use gpui_kit::*;

use crate::gallery::demo::punt_page;
use crate::gallery::registry::GallerySection;

const HOST_MODULE_SHAPE: &str = "```rust
HostModule::new(\"market\")
    .declarations(MARKET_TYPES)
    .function(\"quotes\", move |_| {
        with_app(|cx| market.read(cx).to_host_value())
    })
```";

pub struct ShellSection;

impl ShellSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self)
    }
}

impl Render for ShellSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        punt_page(
            "shell-why-none",
            "Why there is no script panel here",
            "The story runs a Rust quote board beside a JavaScript one; the \
             script half needs gpui-shell.",
            IconName::SquareTerminal,
            "Both boards read one Entity<Market>: the left drawn by \
             shell_story.rs, the right by a ScriptView whose JavaScript \
             lives in crates/story/js/quotes.",
            "The script reaches the entity only through the market host module the \
             story registers before the runtime starts; nothing else crosses the \
             boundary.",
            "shell-host-module",
            HOST_MODULE_SHAPE,
            &[
                "gpui-shell is at milestone M0, unpublished, and not a dependency of this \
                 app, so the script view cannot run here.",
                "The story's counters carry its claim: a script render runs only when data \
                 the script reads changes, so a repaint-only feed leaves frames climbing \
                 and script renders at zero.",
                "Every control the Rust half uses (Button, the segmented TabBar feed \
                 picker, Label, and the spring behind the motion panel) has its own \
                 section in this gallery.",
            ],
            cx,
        )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "shell",
        "Shell",
        "Why no script panel runs here: the host module a script view would read \
         through, and the counters that show a script draws only when its data \
         changes. The scripting runtime is not a dependency of this app.",
        ShellSection::view(window, cx),
    ));
}
