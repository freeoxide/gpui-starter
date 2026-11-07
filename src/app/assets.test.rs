use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use gpui_kit::assets::IconName;

use super::*;

#[cfg(any(target_os = "windows", target_os = "linux"))]
mod app_icon {
    use super::*;

    #[test]
    fn embedded_app_icons_decode_at_advertised_dimensions() {
        for size in [16u16, 32, 48] {
            let png = app_icon_png(size).expect("size is in the committed hicolor set");
            assert_eq!(png.dimensions(), (u32::from(size), u32::from(size)));
            assert!(png.pixels().any(|px| px.0[3] > 0));
        }
    }

    #[test]
    fn app_icon_size_outside_the_set_is_none() {
        assert!(app_icon_png(20).is_none());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn window_icon_is_the_48px_brand_png() {
        let icon = window_icon().expect("48px brand png in the embedded set");
        assert_eq!(icon.dimensions(), (48, 48));
        assert!(icon.pixels().any(|px| px.0[3] > 0));
    }
}

#[test]
fn a_missing_asset_is_none_and_reported_once() {
    let assets = CombinedAssets::new();
    let before = REPORTED_MISSING.with(|reported| reported.borrow().len());
    assert!(
        assets
            .load("icons/definitely-not-here.svg")
            .unwrap()
            .is_none()
    );
    REPORTED_MISSING.with(|reported| {
        let reported = reported.borrow();
        assert!(reported.contains("icons/definitely-not-here.svg"));
        assert_eq!(reported.len(), before + 1);
    });
    assert!(
        assets
            .load("icons/definitely-not-here.svg")
            .unwrap()
            .is_none()
    );
    REPORTED_MISSING.with(|reported| assert_eq!(reported.borrow().len(), before + 1));
}

#[test]
fn a_resolved_asset_is_not_reported_missing() {
    let assets = CombinedAssets::new();
    let before = REPORTED_MISSING.with(|reported| reported.borrow().len());
    let path = IconName::Check.path();
    assert!(assets.load(&path).unwrap().is_some());
    REPORTED_MISSING.with(|reported| assert_eq!(reported.borrow().len(), before));
}

#[test]
fn missing_asset_report_history_is_bounded() {
    let mut reports = MissingAssetReports::default();
    for index in 0..=MAX_REPORTED_MISSING_ASSETS {
        assert!(reports.insert(format!("missing-{index}.svg")));
    }
    assert!(!reports.insert(format!("missing-{MAX_REPORTED_MISSING_ASSETS}.svg")));
    assert!(reports.insert("missing-0.svg".to_owned()));
    assert!(
        reports.len() <= MAX_REPORTED_MISSING_ASSETS,
        "kept {} paths",
        reports.len()
    );
}

#[test]
fn traversal_paths_load_none_and_are_not_reported() {
    let assets = CombinedAssets::new();
    let before = REPORTED_MISSING.with(|reported| reported.borrow().len());
    // None alone cannot catch a dropped guard: rust-embed blocks `..` on the
    // live fs and a kit miss already maps to Ok(None). The report set can.
    for path in ["../Cargo.toml", "icons/../../Cargo.toml", ""] {
        assert_eq!(assets.load(path).unwrap(), None, "{path:?}");
    }
    REPORTED_MISSING.with(|reported| assert_eq!(reported.borrow().len(), before));
}

#[test]
fn project_assets_win_on_lookup() {
    let assets = CombinedAssets::new();
    let before = REPORTED_MISSING.with(|reported| reported.borrow().len());
    let theme = "gruvbox.json";
    let expected = ProjectAssets::get(theme).expect("theme is embedded").data;
    assert_eq!(assets.load(theme).unwrap(), Some(expected));
    // The report set pins the order: consulted later, the kit source files a
    // missing-asset report before the theme resolves.
    REPORTED_MISSING.with(|reported| assert_eq!(reported.borrow().len(), before));
}

#[test]
fn list_merges_project_and_component_entries() {
    let assets = CombinedAssets::new();
    let entries: HashSet<String> = assets
        .list("")
        .expect("list both sources")
        .into_iter()
        .map(|path| path.to_string())
        .collect();
    assert!(
        entries.contains("gruvbox.json"),
        "project themes missing from list"
    );
    assert!(
        entries.contains("icons/search.svg"),
        "kit default icons missing from list"
    );
    assert!(assets.list("../").unwrap().is_empty());
}

// The kit's wasm source is a CDN fallback with no /assets tree behind it,
// so every icon non-test src uses must ship embedded on each target.
fn used_icon_names() -> Vec<(String, PathBuf)> {
    let mut used = Vec::new();
    let mut stack = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("read source dir") {
            let path = entry.expect("read dir entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs")
                && !path.to_string_lossy().ends_with(".test.rs")
            {
                let source = fs::read_to_string(&path).expect("read source file");
                for (at, _) in source.match_indices("IconName::") {
                    let name: String = source[at + "IconName::".len()..]
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect();
                    if !name.is_empty() {
                        used.push((name, path.clone()));
                    }
                }
            }
        }
    }
    used
}

#[test]
fn every_icon_used_in_src_resolves_in_combined_assets() {
    let catalog: HashMap<String, String> = IconName::ALL
        .iter()
        .map(|icon| (format!("{icon:?}"), icon.path().to_string()))
        .collect();
    let assets = CombinedAssets::new();
    let mut used = HashSet::new();
    for (name, file) in used_icon_names() {
        let path = catalog
            .get(&name)
            .unwrap_or_else(|| panic!("{file:?}: IconName::{name} is not in the kit catalog"));
        let bytes = assets
            .load(path)
            .unwrap_or_else(|err| panic!("{file:?}: {path} failed to load: {err}"))
            .unwrap_or_else(|| panic!("{file:?}: {path} is served by no asset source"));
        assert!(!bytes.is_empty(), "{file:?}: {path} loaded empty");
        used.insert(path.clone());
    }
    assert!(!used.is_empty(), "icon scan found no IconName uses in src/");

    // Wasm embeds the overlay plus assets/icons; anything else only exists
    // behind the CDN fallback, which 404s.
    let overlay: HashSet<String> = AppIcons
        .list("icons/")
        .expect("list overlay icons")
        .iter()
        .map(|p| p.to_string())
        .collect();
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    for path in &used {
        assert!(
            overlay.contains(path) || folder.join(path).is_file(),
            "{path} is in neither AppIcons nor assets/: the wasm build would 404 it"
        );
    }
    for path in &overlay {
        assert!(used.contains(path), "{path} is embedded but unused in src/");
    }
}
