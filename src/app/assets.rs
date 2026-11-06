//! Project assets win on lookup, shadowing same-named component assets.

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{HashSet, VecDeque};

use gpui_kit::{AssetSource, Result, SharedString};

/// The folder is deliberately relative: an absolute `$CARGO_MANIFEST_DIR`
/// path breaks embed-at-compile-time builds.
#[derive(rust_embed::RustEmbed)]
#[folder = "themes"]
struct ProjectAssets;

/// Wasm-only fonts, icon SVGs, and the 64px PNG the favicon canvas draws
/// (`document_meta::BASE_ICON_PATH`); other app icons and licenses stay out.
#[cfg(target_family = "wasm")]
#[derive(rust_embed::RustEmbed)]
#[folder = "assets"]
#[include = "app-icon/hicolor/64x64/apps/gpui-starter.png"]
#[include = "fonts/*.ttf"]
#[include = "fonts/*.otf"]
#[include = "icons/*.svg"]
struct WasmAssets;

// Icons the app uses but the wasm icon folder does not embed; none are in
// the kit's default bundle, so they resolve nowhere natively without this.
gpui_kit::assets::icon_assets!(
    AppIcons,
    [
        AtSign,
        Ban,
        ChartNoAxesCombined,
        Image,
        Mic,
        RefreshCw,
        Save,
        Sparkles,
        Square
    ]
);

/// Branded app-icon PNGs for the Windows tray and the X11 window icon; the
/// macOS tray stays a template glyph, so macOS is excluded.
#[cfg(any(target_os = "windows", target_os = "linux"))]
#[derive(rust_embed::RustEmbed)]
#[folder = "assets/app-icon/hicolor"]
struct AppIconPngs;

/// Decoded `<size>x<size>` branded app-icon PNG, or None when the size is
/// absent from the embedded set or the bytes fail to decode.
#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn app_icon_png(size: u16) -> Option<image::RgbaImage> {
    let png = AppIconPngs::get(&format!("{size}x{size}/apps/gpui-starter.png"))?;
    image::load_from_memory(&png.data)
        .ok()
        .map(|img| img.into_rgba8())
}

/// 48px branded PNG for `WindowOptions.icon` (X11 `_NET_WM_ICON`); Wayland has
/// no icon path in gpui-pre 0.3.8 and resolves it from the desktop entry.
#[cfg(target_os = "linux")]
pub fn window_icon() -> Option<std::sync::Arc<image::RgbaImage>> {
    app_icon_png(48).map(std::sync::Arc::new)
}

/// Bytes of every embedded font (wasm-only; see [`WasmAssets`]). Only real
/// font files: the folder's OFL license text makes fontdb error out.
#[cfg(target_family = "wasm")]
pub fn embedded_font_bytes() -> Vec<Cow<'static, [u8]>> {
    WasmAssets::iter()
        .filter(|path| {
            (path.ends_with(".ttf") || path.ends_with(".otf")) && path.starts_with("fonts/")
        })
        .filter_map(|path| WasmAssets::get(&path).map(|file| file.data))
        .collect()
}

/// (file name, JSON) of every theme embedded from themes/. Release builds
/// have no themes/ checkout on disk, so this is the bundle's theme source.
pub fn embedded_themes() -> Vec<(SharedString, String)> {
    ProjectAssets::iter()
        .filter(|path| path.ends_with(".json"))
        .filter_map(|path| {
            ProjectAssets::get(&path).map(|file| {
                (
                    SharedString::from(path.into_owned()),
                    String::from_utf8_lossy(&file.data).into_owned(),
                )
            })
        })
        .collect()
}

pub struct CombinedAssets {
    component: gpui_kit::assets::Assets,
}

impl CombinedAssets {
    pub fn new() -> Self {
        Self {
            // Native: unit struct (icons embedded). Wasm: on-demand CDN source.
            #[cfg(not(target_family = "wasm"))]
            component: gpui_kit::assets::Assets,
            #[cfg(target_family = "wasm")]
            component: gpui_kit::assets::Assets::default(),
        }
    }
}

impl AssetSource for CombinedAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        // Debug rust-embed reads the live fs; reject traversal segments so no
        // file outside the embedded folders can ever be served as an asset.
        if path.is_empty() || path.split('/').any(|seg| seg == "..") {
            return Ok(None);
        }
        if let Some(file) = ProjectAssets::get(path) {
            return Ok(Some(file.data));
        }
        // Before the kit source: its fallback errors on a miss instead of
        // returning None, so untried overlay icons would never load.
        if let Some(bytes) = AppIcons.load(path)? {
            return Ok(Some(bytes));
        }
        // Wasm: embedded fonts + icons, checked before the CDN fallback.
        #[cfg(target_family = "wasm")]
        if let Some(file) = WasmAssets::get(path) {
            return Ok(Some(file.data));
        }
        // A miss is reported once per path: GPUI probes paths it may not
        // need, so a miss stays Ok(None) instead of failing the render.
        match self.component.load(path) {
            Ok(Some(bytes)) => Ok(Some(bytes)),
            Ok(None) => {
                report_missing(path);
                Ok(None)
            }
            // Native fallback: its only error is "not found".
            #[cfg(not(target_family = "wasm"))]
            Err(_) => {
                report_missing(path);
                Ok(None)
            }
            // Wasm fallback errors while a CDN download is pending; GPUI
            // retries on the error, so it propagates.
            #[cfg(target_family = "wasm")]
            Err(err) => Err(err),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        if path.split('/').any(|seg| seg == "..") {
            return Ok(Vec::new());
        }
        let mut entries: Vec<SharedString> = ProjectAssets::iter()
            .filter(|p| p.starts_with(path))
            .map(Into::into)
            .collect();
        let mut component_entries = self.component.list(path)?;
        entries.append(&mut component_entries);
        Ok(entries)
    }
}

const MAX_REPORTED_MISSING_ASSETS: usize = 256;

#[derive(Default)]
struct MissingAssetReports {
    paths: HashSet<String>,
    order: VecDeque<String>,
}

impl MissingAssetReports {
    fn insert(&mut self, path: String) -> bool {
        if self.paths.contains(&path) {
            return false;
        }
        if self.paths.len() == MAX_REPORTED_MISSING_ASSETS
            && let Some(oldest) = self.order.pop_front()
        {
            self.paths.remove(&oldest);
        }
        self.order.push_back(path.clone());
        self.paths.insert(path)
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.paths.len()
    }

    #[cfg(test)]
    fn contains(&self, path: &str) -> bool {
        self.paths.contains(path)
    }
}

thread_local! {
    static REPORTED_MISSING: RefCell<MissingAssetReports> =
        RefCell::new(MissingAssetReports::default());
}

fn report_missing(path: &str) {
    let first_time =
        REPORTED_MISSING.with(|reported| reported.borrow_mut().insert(path.to_owned()));
    if first_time {
        tracing::warn!("asset `{path}` is served by no asset source");
    }
}

#[cfg(test)]
#[path = "assets.test.rs"]
mod assets_test;
