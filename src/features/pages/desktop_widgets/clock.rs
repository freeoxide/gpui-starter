use gpui_kit::component::{ActiveTheme as _, v_flex};
use gpui_kit::{prelude::*, *};

pub(crate) fn clock_parts(now: chrono::DateTime<chrono::Local>) -> (SharedString, SharedString) {
    (
        now.format("%H:%M").to_string().into(),
        now.format("%Y-%m-%d").to_string().into(),
    )
}

pub(crate) struct ClockWidget {
    time: SharedString,
    date: SharedString,
}

impl ClockWidget {
    pub(crate) fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (time, date) = clock_parts(chrono::Local::now());
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(1))
                    .await;
                if this
                    .update(cx, |this, cx| {
                        let (time, date) = clock_parts(chrono::Local::now());
                        this.time = time;
                        this.date = date;
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        Self { time, date }
    }

    pub(crate) fn view(window: &mut Window, cx: &mut App) -> Entity<ClockWidget> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for ClockWidget {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_1()
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .child(self.time.clone()),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(self.date.clone()),
            )
    }
}

#[cfg(test)]
#[path = "clock.test.rs"]
mod clock_test;
