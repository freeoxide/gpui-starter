//! Project assets win on lookup, shadowing same-named component assets.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

/// The folder is deliberately relative: an absolute `$CARGO_MANIFEST_DIR`
/// path breaks embed-at-compile-time builds.
#[derive(rust_embed::RustEmbed)]
#[folder = "themes"]
struct ProjectAssets;

/// Wasm-only embedded assets (fonts, default icons): browser tabs have no
/// system fonts and no icon CDN, so the icon set is embedded for parity.
#[cfg(target_family = "wasm")]
#[derive(rust_embed::RustEmbed)]
#[folder = "assets"]
struct WasmAssets;

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
    component: gpui_kit_assets::Assets,
}

impl CombinedAssets {
    pub fn new() -> Self {
        Self {
            // Native: unit struct (icons embedded). Wasm: on-demand CDN source.
            #[cfg(not(target_family = "wasm"))]
            component: gpui_kit_assets::Assets,
            #[cfg(target_family = "wasm")]
            component: gpui_kit_assets::Assets::default(),
        }
    }
}

impl Default for CombinedAssets {
    fn default() -> Self {
        Self::new()
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
        // Wasm: embedded fonts + icons, checked before the CDN fallback.
        #[cfg(target_family = "wasm")]
        if let Some(file) = WasmAssets::get(path) {
            return Ok(Some(file.data));
        }
        self.component.load(path)
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
