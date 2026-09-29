use gpui_kit::Action;

/// Navigate directly to a sidebar page by index (0-based).
#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = app, no_json)]
pub struct NavigateToPage(pub usize);

#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = app, no_json)]
pub struct RefreshPage;

#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = app, no_json)]
pub struct ToggleSidebar;

pub(crate) fn is_rtl_locale(locale: &str) -> bool {
    locale
        .split('-')
        .next()
        .map(|primary| matches!(primary, "ar" | "he" | "fa" | "ur"))
        .unwrap_or(false)
}
