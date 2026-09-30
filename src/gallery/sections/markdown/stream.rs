//! Stream Markdown section, ported from the upstream `stream-markdown`
//! example: replay a document that arrives in short chunks, with the fade
//! the text view applies to streamed text.

use std::convert::Infallible;
use std::time::Duration;

use futures_util::stream::unfold;
use gpui_kit::component::{
    button::Button,
    h_flex,
    switch::Switch,
    text::{TextView, TextViewState},
    v_flex,
};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

const EXAMPLE: &str = include_str!("fixtures/test.md");

/// Cadence between chunks, from the upstream example.
const CHUNK_INTERVAL: Duration = Duration::from_millis(50);

/// Seed for the deterministic chunk lengths (5 to 19 characters), standing in
/// for the example's `rand` crate, which this app does not depend on.
const CHUNK_SEED: u64 = 0x9E3779B97F4A7C15;

pub struct StreamSection {
    markdown_state: Entity<TextViewState>,
    stream_fade: bool,
    /// The receiver task of the stream in flight; replacing it drops the
    /// channel, which is what stops the producer.
    _receiver_task: Task<()>,
}

impl StreamSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            markdown_state: cx
                .new(|cx| TextViewState::markdown("# Streaming Markdown Parse\n\n", cx)),
            stream_fade: true,
            _receiver_task: Task::ready(()),
        })
    }

    /// Simulate streaming: one chunk of 5 - 19 characters every 50ms, instead
    /// of the network source a real app would stream from.
    fn replay(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.markdown_state
            .update(cx, |state, cx| state.set_text("", cx));

        let executor = cx.background_executor().clone();
        let chars: Vec<char> = EXAMPLE.chars().collect();
        let stream = unfold((chars, 0, CHUNK_SEED), move |(chars, current, mut seed)| {
            let executor = executor.clone();
            async move {
                let remaining = chars.len() - current;
                if remaining == 0 {
                    return None;
                }
                seed = seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let chunk_len = (5 + ((seed >> 33) % 15) as usize).min(remaining);
                executor.timer(CHUNK_INTERVAL).await;
                let chunk: String = chars[current..current + chunk_len].iter().collect();
                Some((
                    Ok::<_, Infallible>(chunk),
                    (chars, current + chunk_len, seed),
                ))
            }
        });

        let (producer, rx) = crate::services::streaming::spawn_token_stream(cx, stream);
        producer.detach();
        self._receiver_task = cx.spawn(async move |this, cx| {
            while let Ok(chunk) = rx.recv_async().await {
                _ = this.update(cx, |this, cx| {
                    this.markdown_state
                        .update(cx, |state, cx| state.push_str(&chunk, cx));
                });
            }
        });
    }
}

impl Render for StreamSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_3()
            .p_4()
            .child(
                h_flex()
                    .w_full()
                    .gap_4()
                    .child(
                        Button::new("markdown-stream-replay")
                            .outline()
                            .label("Replay")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.replay(window, cx);
                            })),
                    )
                    .child(
                        Switch::new("markdown-stream-fade")
                            .checked(self.stream_fade)
                            .label("Fade in streamed text")
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.stream_fade = *checked;
                                cx.notify();
                            })),
                    ),
            )
            // The gallery pane owns scrolling, so the document grows at its
            // natural height instead of scrolling inside a fixed viewport.
            .child(
                TextView::new(&self.markdown_state)
                    .selectable(true)
                    .stream_fade(self.stream_fade),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "markdown-stream",
        "Stream Markdown",
        "Replay a document that arrives in short chunks and watch each append parse and render as it lands, optionally fading in the streamed words.",
        StreamSection::view(window, cx),
    ));
}
