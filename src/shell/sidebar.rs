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
    SystemControls,
    Gallery,
    CanvasShowcase,
    InfiniteCanvas,
    MultiuserCanvas,
    FloatingUi,
    ImageEditor,
    Microphone,
    Audio,
    Analytics,
    Music,
    SignIn,
    DesktopWidgets,
    Wgpu,
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
            Page::SystemControls => "System Controls",
            Page::Gallery => "Gallery",
            Page::CanvasShowcase => "Canvas Showcase",
            Page::InfiniteCanvas => "Infinite Canvas",
            Page::MultiuserCanvas => "Shared Canvas",
            Page::FloatingUi => "Floating UI",
            Page::ImageEditor => "Image Editor",
            Page::Microphone => "Microphone",
            Page::Audio => "Audio",
            Page::Analytics => "Analytics",
            Page::Music => "Music",
            Page::SignIn => "Sign In",
            Page::DesktopWidgets => "Desktop Widgets",
            Page::Wgpu => "WGPU",
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
            Page::SystemControls => "nav_system_controls",
            Page::Gallery => "nav_gallery",
            Page::CanvasShowcase => "nav_canvas_showcase",
            Page::InfiniteCanvas => "nav_infinite_canvas",
            Page::MultiuserCanvas => "nav_multiuser_canvas",
            Page::FloatingUi => "nav_floating_ui",
            Page::ImageEditor => "nav_image_editor",
            Page::Microphone => "nav_microphone",
            Page::Audio => "nav_audio",
            Page::Analytics => "nav_analytics",
            Page::Music => "nav_music",
            Page::SignIn => "nav_sign_in",
            Page::DesktopWidgets => "nav_desktop_widgets",
            Page::Wgpu => "nav_wgpu",
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
            Page::SystemControls => "system-controls",
            Page::Gallery => "gallery",
            Page::CanvasShowcase => "canvas-showcase",
            Page::InfiniteCanvas => "infinite-canvas",
            Page::MultiuserCanvas => "multiuser-canvas",
            Page::FloatingUi => "floating-ui",
            Page::ImageEditor => "image-editor",
            Page::Microphone => "microphone",
            Page::Audio => "audio",
            Page::Analytics => "analytics",
            Page::Music => "music",
            Page::SignIn => "sign-in",
            Page::DesktopWidgets => "desktop-widgets",
            Page::Wgpu => "wgpu",
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
            // SlidersHorizontal is full-catalog and renders blank on native;
            // Settings2 is the shipped stand-in.
            Page::SystemControls => IconName::Settings2,
            Page::Gallery => IconName::GalleryVerticalEnd,
            Page::CanvasShowcase => IconName::Palette,
            Page::InfiniteCanvas => IconName::Frame,
            Page::MultiuserCanvas => IconName::User,
            // CanvasShowcase already claims Palette; panels name this page.
            Page::FloatingUi => IconName::PanelLeft,
            // Full-catalog Image renders blank on native; Frame is the stand-in.
            Page::ImageEditor => IconName::Frame,
            Page::Microphone => IconName::Mic,
            // AudioLines is full-catalog and renders blank on native; Play is
            // the shipped stand-in.
            Page::Audio => IconName::Play,
            Page::Analytics => IconName::ChartPie,
            // Music is full-catalog and renders blank on native; Play is
            // the shipped stand-in.
            Page::Music => IconName::Play,
            Page::SignIn => IconName::User,
            Page::DesktopWidgets => IconName::PanelBottom,
            Page::Wgpu => IconName::Cpu,
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
            Page::SystemControls,
            Page::Gallery,
            Page::CanvasShowcase,
            Page::InfiniteCanvas,
            Page::MultiuserCanvas,
            Page::FloatingUi,
            Page::ImageEditor,
            Page::Microphone,
            Page::Audio,
            Page::Analytics,
            Page::Music,
            Page::SignIn,
            Page::DesktopWidgets,
            Page::Wgpu,
        ]
    }
}
