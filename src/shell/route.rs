use serde::{Deserialize, Serialize};
use url::Url;

use crate::{errors::AppError, sidebar::Page};

pub const APP_URL_SCHEME: &str = "gpui-starter";

/// Valid deep-link hosts, derived from [`Page::host`](crate::sidebar::Page::host)
/// so the parser, `to_url`, and foundation validation cannot drift.
pub(crate) const VALID_HOSTS: &[&str] = &[
    Page::Home.host(),
    Page::Form.host(),
    Page::Settings.host(),
    Page::Notifications.host(),
    Page::Diagnostics.host(),
    Page::ErrorPlayground.host(),
    Page::QueryPlayground.host(),
    Page::QueryDevToolsV2.host(),
    Page::About.host(),
    Page::Gallery.host(),
];

const INVALID_SEGMENT_CHARS: &[char] = &['/', '\\', '\0', '<', '>', '|', '"'];

fn validate_path_segment(segment: &str) -> Result<(), AppError> {
    if segment.is_empty() {
        return Ok(());
    }

    if segment == ".." || segment == "." || segment.contains("..") {
        return Err(AppError::invalid_deep_link(
            segment,
            "path traversal detected in URL segment",
        ));
    }

    for &forbidden in INVALID_SEGMENT_CHARS {
        if segment.contains(forbidden) {
            return Err(AppError::invalid_deep_link(
                segment,
                format!(
                    "path segment contains forbidden character `{:?}`",
                    forbidden
                ),
            ));
        }
    }

    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppRoute {
    Page(Page),
    SettingsNotifications,
}

impl AppRoute {
    pub fn page(page: Page) -> Self {
        Self::Page(page)
    }

    pub fn page_for_render(&self) -> Page {
        match self {
            Self::Page(page) => *page,
            Self::SettingsNotifications => Page::Settings,
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::Page(page) => page.title(),
            Self::SettingsNotifications => "Settings",
        }
    }

    pub fn to_url(&self) -> String {
        match self {
            // The host segment comes from `Page::host` — the enum is the
            // single source of truth for the route set.
            Self::Page(page) => format!("{}://{}", APP_URL_SCHEME, page.host()),
            Self::SettingsNotifications => {
                format!(
                    "{}://{}/notifications",
                    APP_URL_SCHEME,
                    Page::Settings.host()
                )
            }
        }
    }

    /// Hash-URL form for the web (`#/settings`) — same layout as [`to_url`](Self::to_url)
    /// behind a `#`; only the wasm router feeds this to the History API.
    pub fn to_hash(&self) -> String {
        match self {
            Self::Page(page) => format!("#/{}", page.host()),
            Self::SettingsNotifications => format!("#/{}/notifications", Page::Settings.host()),
        }
    }

    /// Parse a `location.hash` fragment; empty fragments resolve to the
    /// default route, the rest goes through [`parse_deep_link`](Self::parse_deep_link).
    pub fn from_hash(hash: &str) -> Result<Self, AppError> {
        let trimmed = hash.trim();
        let path = trimmed.strip_prefix('#').unwrap_or(trimmed);
        if path.is_empty() || path == "/" {
            return Ok(Self::default());
        }
        let url = format!("{}://{}", APP_URL_SCHEME, path.trim_start_matches('/'));
        Self::parse_deep_link(&url)
    }

    pub fn parse_deep_link(input: &str) -> Result<Self, AppError> {
        let url =
            Url::parse(input).map_err(|err| AppError::invalid_deep_link(input, err.to_string()))?;

        if url.scheme() != APP_URL_SCHEME {
            return Err(AppError::invalid_deep_link(
                input,
                format!(
                    "unsupported scheme `{}`, expected `{}`",
                    url.scheme(),
                    APP_URL_SCHEME
                ),
            ));
        }

        let host = url.host_str().unwrap_or_default();
        if !VALID_HOSTS.contains(&host) {
            return Err(AppError::invalid_deep_link(
                input,
                format!("unexpected host `{}`", host),
            ));
        }

        let segments: Vec<&str> = url
            .path_segments()
            .map(|segs| segs.filter(|s| !s.is_empty()).collect::<Vec<_>>())
            .unwrap_or_default();

        for segment in &segments {
            validate_path_segment(segment)?;
        }

        // Query parameters are ignored: matching consumes host + path only,
        // so unsanitized query data can never reach dispatch.
        match (host, segments.as_slice()) {
            (host, []) => match Page::from_host(host) {
                Some(page) => Ok(Self::Page(page)),
                None => Err(AppError::invalid_deep_link(input, "unknown route")),
            },
            (h, ["notifications"]) if h == Page::Settings.host() => Ok(Self::SettingsNotifications),
            _ => Err(AppError::invalid_deep_link(input, "unknown route")),
        }
    }
}

impl Default for AppRoute {
    fn default() -> Self {
        Self::Page(Page::Home)
    }
}

#[cfg(test)]
#[path = "route.test.rs"]
mod route_test;
