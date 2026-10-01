//! Shell story punt section: the upstream story renders a gpui-shell script
//! view, and this app does not depend on gpui-shell.

use gpui_kit::component::text::TextView;
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, StyledExt as _, h_flex, v_flex};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::section;

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
        let muted = cx.theme().muted_foreground;
        let paragraph = |line: &str| {
            div()
                .text_sm()
                .text_color(muted)
                .child(SharedString::from(line))
        };

        v_flex().w_full().items_center().gap_6().p_4().child(
            section("shell-why-none", "Why there is no script panel here")
                .description(
                    "The story runs a Rust quote board beside a JavaScript one; the \
                     script half needs gpui-shell.",
                )
                .w(rems(30.))
                .v_flex()
                .items_start()
                .gap_3()
                .child(
                    h_flex()
                        .gap_3()
                        .child(Icon::new(IconName::SquareTerminal).size_6())
                        .child(div().text_sm().child(
                            "Both boards read one Entity<Market>: the left drawn by \
                                 shell_story.rs, the right by a ScriptView whose JavaScript \
                                 lives in crates/story/js/quotes.",
                        )),
                )
                .child(paragraph(
                    "The script reaches the entity only through the market host module the \
                     story registers before the runtime starts; nothing else crosses the \
                     boundary.",
                ))
                .child(
                    v_flex()
                        .w_full()
                        .gap_2()
                        .child(TextView::markdown("shell-host-module", HOST_MODULE_SHAPE)),
                )
                .child(paragraph(
                    "gpui-shell is at milestone M0, unpublished, and not a dependency of this \
                     app, so the script view cannot run here.",
                ))
                .child(paragraph(
                    "The story's counters carry its claim: a script render runs only when data \
                     the script reads changes, so a repaint-only feed leaves frames climbing \
                     and script renders at zero.",
                ))
                .child(paragraph(
                    "Every control the Rust half uses (Button, the segmented TabBar feed \
                     picker, Label, and the spring behind the motion panel) has its own \
                     section in this gallery.",
                )),
        )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "shell",
        "Shell",
        "The gpui-shell quote board story needs a scripting runtime this app does not \
         depend on.",
        ShellSection::view(window, cx),
    ));
}
