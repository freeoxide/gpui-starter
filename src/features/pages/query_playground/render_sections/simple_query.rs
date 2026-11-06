use gpui_kit::component::{
    ActiveTheme as _, Disableable as _,
    button::{Button, ButtonVariants as _},
    h_flex,
};
use gpui_kit::{prelude::*, *};

use super::super::QueryPlaygroundPage;
use super::super::ui_helpers::{
    chip, fetch_label, query_data, query_status, section_card, status_badge,
};

impl QueryPlaygroundPage {
    pub(in super::super) fn render_simple_query(&mut self, cx: &mut Context<Self>) -> Div {
        let status = query_status(self.simple_query.as_ref(), cx);
        let loading = status.is_loading();
        let data_preview = query_data(self.simple_query.as_ref(), cx);
        let cache_age = self.simple_query.as_ref().and_then(|(e, _)| {
            e.read_with(cx, |r, _| {
                let now = crate::platform::clock::SystemTime::now()
                    .duration_since(crate::platform::clock::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis();
                r.last_updated_at_ms()
                    .map(|t| now.saturating_sub(u128::from(t)))
            })
        });
        let bg = cx.theme().background;

        section_card(
            "simple",
            &crate::i18n::localize("query_playground_simple_title"),
            &crate::i18n::localize("query_playground_simple_description"),
            cx,
        )
        .child(
            h_flex()
                .gap_2()
                .flex_wrap()
                .px_4()
                .py_3()
                .child(
                    Button::new("pg-simple-fetch")
                        .primary()
                        .label(fetch_label(loading))
                        .disabled(loading)
                        .on_click(cx.listener(|this, _, _, cx| this.fetch_simple(cx))),
                )
                .child(
                    Button::new("pg-simple-cancel")
                        .outline()
                        .label(crate::i18n::localize("query_playground_cancel"))
                        .disabled(!loading)
                        .on_click(cx.listener(|this, _, _, cx| this.cancel_simple(cx))),
                )
                .child(
                    Button::new("pg-simple-reset")
                        .outline()
                        .label(crate::i18n::localize("query_playground_reset"))
                        .on_click(cx.listener(|this, _, _, cx| this.reset_simple(cx))),
                ),
        )
        .child(
            h_flex()
                .gap_3()
                .items_center()
                .px_4()
                .pb_3()
                .child(status_badge("simple", status, cx))
                .when_some(data_preview, |el, user| {
                    el.child(chip(
                        "simple-user",
                        &format!("{} <{}>", user.name, user.email),
                        bg,
                        cx,
                    ))
                })
                .when_some(cache_age, |el, age| {
                    el.child(chip(
                        "simple-age",
                        &format!(
                            "{}: {}ms",
                            crate::i18n::localize("query_playground_cache_age"),
                            age
                        ),
                        bg,
                        cx,
                    ))
                }),
        )
    }
}
