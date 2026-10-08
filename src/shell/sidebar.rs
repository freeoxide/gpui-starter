use gpui_kit::component::IconName;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Page {
    Home,
    Form,
    Settings,
    Notifications,
    Diagnostics,
    QueryPlayground,
    QueryDevToolsV2,
    ErrorPlayground,
    About,
    Gallery,
}

impl Page {
    pub fn title(&self) -> &'static str {
        match self {
            Page::Home => "Home",
            Page::Form => "Form",
            Page::Settings => "Settings",
            Page::Notifications => "Notifications",
            Page::Diagnostics => "Diagnostics",
            Page::QueryPlayground => "Query Playground",
            Page::QueryDevToolsV2 => "Query DevTools V2",
            Page::ErrorPlayground => "Error Playground",
            Page::About => "About",
            Page::Gallery => "Gallery",
        }
    }

    /// Catalog key for the localized display title; [`title`](Self::title)
    /// stays the English label for logs and not-yet-localized surfaces.
    pub const fn title_key(self) -> &'static str {
        match self {
            Page::Home => "nav_home",
            Page::Form => "nav_form",
            Page::Settings => "nav_settings",
            Page::Notifications => "nav_notifications",
            Page::Diagnostics => "nav_diagnostics",
            Page::QueryPlayground => "nav_query_playground",
            Page::QueryDevToolsV2 => "nav_query_devtools_v2",
            Page::ErrorPlayground => "nav_error_playground",
            Page::About => "nav_about",
            Page::Gallery => "nav_gallery",
        }
    }

    pub fn localized_title(&self) -> String {
        crate::i18n::localize(self.title_key())
    }

    /// Deep-link host segment; the single source of truth for the `Page` ↔
    /// host mapping (exhaustive match forces updates on new variants).
    pub const fn host(self) -> &'static str {
        match self {
            Page::Home => "home",
            Page::Form => "form",
            Page::Settings => "settings",
            Page::Notifications => "notifications",
            Page::Diagnostics => "diagnostics",
            Page::ErrorPlayground => "error-playground",
            Page::QueryPlayground => "query-playground",
            Page::QueryDevToolsV2 => "query-devtools-v2",
            Page::About => "about",
            Page::Gallery => "gallery",
        }
    }

    /// Inverse of [`host`](Self::host); `None` for an unrecognized host.
    pub fn from_host(host: &str) -> Option<Self> {
        Self::all().iter().copied().find(|p| p.host() == host)
    }

    pub fn icon(&self) -> IconName {
        match self {
            Page::Home => IconName::Inbox,
            Page::Form => IconName::File,
            Page::Settings => IconName::Settings2,
            Page::Notifications => IconName::Bell,
            Page::Diagnostics => IconName::Info,
            Page::QueryPlayground => IconName::Play,
            Page::QueryDevToolsV2 => IconName::LayoutDashboard,
            Page::ErrorPlayground => IconName::TriangleAlert,
            Page::About => IconName::Info,
            Page::Gallery => IconName::GalleryVerticalEnd,
        }
    }

    pub fn all() -> &'static [Page] {
        &[
            Page::Home,
            Page::Form,
            Page::Settings,
            Page::Notifications,
            Page::Diagnostics,
            Page::QueryPlayground,
            Page::QueryDevToolsV2,
            Page::ErrorPlayground,
            Page::About,
            Page::Gallery,
        ]
    }
}
