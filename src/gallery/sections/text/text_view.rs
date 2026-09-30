//! Text View section, ported from the upstream `text_max_lines` example:
//! rendered Markdown clamped to a line budget with `TextView::max_lines`.

use gpui_kit::component::{
    ActiveTheme as _, IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    slider::{Slider, SliderEvent, SliderState},
    text::{TextView, TextViewState},
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::gallery::registry::GallerySection;

const DEFAULT_MAX_LINES: usize = 5;

// The example's remote image URLs are embedded data URLs here: the app sets
// no GPUI HTTP client, so remote images would silently never load.
const LONG_MARKDOWN: &str = r#"### Quarterly summary

**Revenue** grew by *18%* quarter over quarter, driven by the desktop client
rollout and the new [market data](https://longbridge.com) subscriptions;
legacy plans are ~~discontinued~~ and folded into `pro`.

> The clip must land on a whole line: however you drag the slider, no line of
> glyphs is ever cut in half.

Inline image mix: PNG avatars <img src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACAAAAAgCAIAAAD8GO2jAAAAMUlEQVR42mOwD9xKU8QwasGoBUPYgsq20yShUQtGLRi1YNSCUQuGpwWjVeaoBSPIAgDlIZlMODbZSwAAAABJRU5ErkJggg==" alt="Jason Lee avatar" width="32" height="32" /> and <img src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACAAAAAgCAIAAAD8GO2jAAAAMUlEQVR42mPgLS6nKWIYtWDUgiFsge6qtSShUQtGLRi1YNSCUQuGpwWjVeaoBSPIAgBvJGk9tKhR4QAAAABJRU5ErkJggg==" alt="GitHub avatar" width="32" height="32" /> stay inside the same text flow, and an icon badge ![Rust](data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABwAAAAcCAIAAAD9b0jDAAAALElEQVR42mPQVFWhOmIYNRSObpazkY1GDR01dNTQUUNHDR01lJaGjtb7VEEA/uzBf15TsuYAAAAASUVORK5CYII=) wraps with the text around it.

- Desktop DAU is up **24%**
  - macOS **+31%**, Windows *+19%*
  - Linux ships via `install.sh` now
- The `max_lines` preview lands in this release
- Churn stayed flat at 2.1%

| Segment | QoQ    | Note                 |
| ------- | ------ | -------------------- |
| Desktop | +24%   | new dock layout      |
| Mobile  | +9%    | steady               |
| Web     | -3%    | migrating to desktop |

![Img](data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAeAAAAEOCAIAAADe+FMwAAAFfUlEQVR42u3Ux44VVAAGYB/Ctb1GsRuVjsDQexeYXu+dudMLMzDAAENijIlsTIgb3bjxId0ZNucuzEn+nORLvmf4Xnv9jT3/GRh9DkDQqye/JmgAQQPwf4N+c+8QAEGCBmgu6H1DAAQVg35r3xAAQX2CHgYgSNAArQX99v5hAIIEDSBoAKoE/c7+YQCCBA3QXNAHRgAIKgb97oERAIIEDSBoAKoE/d7BEQCCBA0gaACqBP3+wVEAggQN0FzQh0YBCCoG/cGhUQCCBA0gaACqBP3h4TEAggQNIGgAqgT90eExAIIEDdBc0EfGAAgqBv3xkTEAgvoEPQ5AkKABWgv6cucXAIIEDSBoAKoE/cmP4wAECRqguaCPTgAQVAz606MTAAQJGkDQAFQJes+xCQCCBA0gaACqBP3ZsUkAggQN0FzQxycBCCoG/fnxSQCCBA0gaACqBP3FwBQAQYIGEDQAVYL+cmAKgCBBAzQX9IkpAIKKQX91YgqAoD5BTwMQJGiA1oL++uQ0AEGCBhA0AFWC/ubkNABBggZoLuhTMwAEFYP+9tQMAEGCBhA0AFWCHlz6DYAgQQO0F/QLAIKKQX93ugNAkKABmgv6TAeAoGLQ35/pABAkaABBA1Al6B/OdgEIEjSAoAGoEvTes10AggQN0FzQ57oABBWD3neuC0BQn6BnAQgSNEBrQe8/PwtAkKABBA1AlaAPnJ8FIEjQAM0FfWEOgKBi0AcvzAEQJGgAQQNQJehDF+cACBI0QHtB9wAIKgZ9+GIPgCBBAzQX9KUeAEHFoI9c6gEQJGgAQQNQJejO5u8ABAkaQNAAVAn66OV5AIIEDdBc0FfmAQgqBn3syjwAQX2CXgAgSNAArQV9/OoCAEGCBhA0AFWCHri6AECQoAGaC/raIgBBxaBPXFsEIEjQAIIGoErQJ68vAhAkaID2gl4CIKgY9KnrSwAECRqguaBvLAEQVAz69I0lAIIEDSBoAKoEfebmMgBBggYQNABVgj57cxmAoHLQt5YBCBI0QGtBn7u1DEBQn6BXAAgSNEBrQa/t/AFAkKABBA1AlaAv/LQCQFA56NurAAQJGqC1oC/eXgUgSNAAggagStCX7qwCECRogPaCXgMgqBj05TtrAASVg767BkCQoAFaC/rK3TUAggQNIGgAqgR9dXAdgCBBAwgagCpBXxtcByCoHPTQOgBBggZoLejrQ+sABPUJegOAIEEDtBb0jeENAIIEDSBoAKoEfXN4A4CgctAj9wAIEjRAa0HfGrkHQJCgAQQNQJWgd37+E4AgQQO0F/RfAAQVg749uglAUDnosU0AggQN0FrQd8Y2AQgSNICgAagS9N3xLQCCBA0gaACqBD04vgVAUDnoiS0AggQN0FrQQxNbAAT1Cfo+AEGCBmgt6OHJ+wAECRpA0ABUCXpk8j4AQeWgpx4AECRogNaCHp16AECQoAEEDUCVoMemHwAQJGiA9oLeBiCoGPT49DYAQeWgZ7YBCBI0QGtBT8xsAxAkaABBA1Al6F9f/A1AkKABBA1AlaCnOg8BCCoH3X0IQJCgAVoLerr7CIAgQQMIGoAqQc/MPgIgSNAAggagStCd2UcABJWDnnsMQJCgAVoLujv3GIAgQQMIGoAqQc/2HgMQJGiA9oLeASCoGPRcbweAoHLQ8zsABAkaoLWge/M7AAQJGkDQAFQJen7hCQBBggYQNABVgl5YeAJAUDnoxScABAkaoLWgFxefAhAkaABBA1Al6Jcv/wEgSNAAggagStDLS08BCCoHvfwMgCBBA7QW9MryMwCCBA0gaACqBL268gyAIEEDtBf0LgBBxaDXVnYBCCoHvboLQJCgAVoLen11F4AgQQO0FvTG2nMAgl49+V8YGrMgAN2l3AAAAABJRU5ErkJggg==)

---

```rust
fn main() {
    println!("hidden until expanded");
}
```

Text lines are kept whole; the photo above is cut on the box edge instead, so
the preview never holds blank space it could have filled."#;

const SHORT_MARKDOWN: &str = "A **short** note that fits inside the cap, so it renders at its natural \
     height and no button appears.";

pub struct TextViewSection {
    long: Entity<TextViewState>,
    short: Entity<TextViewState>,
    slider: Entity<SliderState>,
    max_lines: usize,
    expanded: bool,
    _subscriptions: Vec<Subscription>,
}

impl TextViewSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(Self::new)
    }

    fn new(cx: &mut Context<Self>) -> Self {
        let long = cx.new(|cx| TextViewState::markdown(LONG_MARKDOWN, cx));
        let short = cx.new(|cx| TextViewState::markdown(SHORT_MARKDOWN, cx));
        // `is_clamped` is written while the view paints; observe the states so
        // the button follows it without the caller measuring the text again.
        let mut _subscriptions = vec![
            cx.observe(&long, |_, _, cx| cx.notify()),
            cx.observe(&short, |_, _, cx| cx.notify()),
        ];

        let slider = cx.new(|_| {
            SliderState::new()
                .min(1.)
                .max(60.)
                .step(1.)
                .default_value(DEFAULT_MAX_LINES as f32)
        });
        _subscriptions.push(cx.subscribe(&slider, |this, _, event, cx| {
            if let SliderEvent::Change(value) = event {
                this.max_lines = value.start() as usize;
                cx.notify();
            }
        }));

        Self {
            long,
            short,
            slider,
            max_lines: DEFAULT_MAX_LINES,
            expanded: false,
            _subscriptions,
        }
    }
}

/// Caption above a preview, then the preview itself on a hairline surface
/// (the example's own demo-box helper).
fn preview(caption: impl Into<SharedString>, body: Div, cx: &App) -> Div {
    v_flex()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(caption.into()),
        )
        .child(
            body.p_4()
                .rounded(cx.theme().radius)
                .border_1()
                .border_color(cx.theme().border),
        )
}

impl Render for TextViewSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let clamped = self.long.read(cx).is_clamped();
        let expanded = self.expanded;
        let max_lines = self.max_lines;

        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .child(
                h_flex()
                    .w_full()
                    .pb_4()
                    .gap_3()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("Lines"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .max_w(rems(20.))
                            .child(Slider::new(&self.slider)),
                    )
                    // A fixed width keeps the slider still as the value grows.
                    .child(div().w(rems(1.5)).text_sm().child(max_lines.to_string())),
            )
            .child(preview(
                if expanded {
                    "Expanded"
                } else {
                    "Clamped to the line budget"
                },
                v_flex()
                    .gap_3()
                    .child(
                        TextView::new(&self.long)
                            .selectable(true)
                            .when(!expanded, |this| this.max_lines(max_lines)),
                    )
                    .when(clamped || expanded, |this| {
                        this.child(
                            h_flex().child(
                                Button::new("text-view-toggle")
                                    .ghost()
                                    .small()
                                    .icon(if expanded {
                                        IconName::ChevronUp
                                    } else {
                                        IconName::ChevronDown
                                    })
                                    .label(if expanded { "Show less" } else { "Show more" })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.expanded = !this.expanded;
                                        cx.notify();
                                    })),
                            ),
                        )
                    }),
                cx,
            ))
            .child(preview(
                "Shorter than the budget",
                v_flex().child(
                    TextView::new(&self.short)
                        .selectable(true)
                        .max_lines(max_lines),
                ),
                cx,
            ))
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "text-view",
        "Text View",
        "Rendered Markdown bounded to a number of whole lines. Drag the slider, or resize the window to reflow the text.",
        TextViewSection::view(window, cx),
    ));
}
