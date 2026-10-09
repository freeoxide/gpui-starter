use std::fs;
use std::path::{Path, PathBuf};

use super::*;

fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("gpui-starter-notify-icon-{tag}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn stage_png(path: &Path) {
    fs::create_dir_all(path.parent().expect("png parent")).expect("icon dir");
    fs::write(path, b"stub png").expect("stub png");
}

#[test]
fn candidates_probe_share_hicolor_then_local_hicolor_then_resources() {
    let exe_dir = Path::new("/opt/app/bin");

    let candidates = icon_candidates(exe_dir);

    assert_eq!(candidates.len(), 11);
    assert_eq!(
        candidates[0],
        Path::new("/opt/app/share/icons/hicolor/48x48/apps/gpui-starter.png")
    );
    assert_eq!(
        candidates[4],
        Path::new("/opt/app/share/icons/hicolor/512x512/apps/gpui-starter.png")
    );
    assert_eq!(
        candidates[5],
        Path::new("/opt/app/bin/hicolor/48x48/apps/gpui-starter.png")
    );
    assert_eq!(
        candidates[9],
        Path::new("/opt/app/bin/hicolor/512x512/apps/gpui-starter.png")
    );
    assert_eq!(
        candidates[10],
        Path::new("/opt/app/bin/resources/gpui-starter.png")
    );
}

#[test]
fn probe_finds_share_layout_beside_bin() {
    let root = temp_root("share-layout");
    let png = root.join("share/icons/hicolor/48x48/apps/gpui-starter.png");
    stage_png(&png);

    assert_eq!(probe_app_icon(&root.join("bin")), png.display().to_string());
}

#[test]
fn probe_prefers_install_layout_over_local_and_resources() {
    let exe_dir = temp_root("layout-order").join("bin");
    stage_png(&exe_dir.join("hicolor/64x64/apps/gpui-starter.png"));
    stage_png(&exe_dir.join("resources/gpui-starter.png"));

    assert_eq!(
        probe_app_icon(&exe_dir),
        exe_dir
            .join("hicolor/64x64/apps/gpui-starter.png")
            .display()
            .to_string()
    );
}

#[test]
fn probe_prefers_smaller_size_within_a_layout() {
    let exe_dir = temp_root("size-order").join("bin");
    stage_png(&exe_dir.join("hicolor/256x256/apps/gpui-starter.png"));
    stage_png(&exe_dir.join("hicolor/64x64/apps/gpui-starter.png"));

    assert_eq!(
        probe_app_icon(&exe_dir),
        exe_dir
            .join("hicolor/64x64/apps/gpui-starter.png")
            .display()
            .to_string()
    );
}

#[test]
fn probe_falls_back_to_theme_name_when_nothing_is_bundled() {
    let exe_dir = temp_root("fallback").join("bin");

    assert_eq!(probe_app_icon(&exe_dir), APP_ICON_NAME);
}
