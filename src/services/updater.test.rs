use super::types::{
    DEFAULT_MANIFEST_URL, PERIODIC_CHECK_INTERVAL_SECS, PLACEHOLDER_MANIFEST_URL,
    STARTUP_CHECK_DELAY_SECS, current_app_version, pending_swap_path, platform_key,
};
use super::*;
use crate::app_state::{AppConfig, install_for_tests};
use gpui_kit::TestAppContext;

#[test]
fn platform_key_format() {
    let key = platform_key();
    let parts: Vec<&str> = key.split('-').collect();
    assert!(
        parts.len() >= 2,
        "platform_key should contain at least os and arch separated by dash, got: {key}"
    );
    let arch = std::env::consts::ARCH;
    assert!(
        key.contains(arch),
        "platform_key should contain arch '{arch}', got: {key}"
    );
}

#[test]
fn current_app_version_matches_cargo_pkg() {
    let version = current_app_version();
    assert_eq!(version, env!("CARGO_PKG_VERSION"));
}

#[test]
fn manifest_parse_valid_full() {
    let json = r#"{
        "version": "2.0.0",
        "release_notes": "Big update",
        "platforms": {
            "macos-aarch64": {
                "url": "https://example.com/app.tar.gz",
                "signature": "abc123",
                "size": 12345678
            }
        }
    }"#;
    let manifest: UpdateManifest = serde_json::from_str(json).expect("parse full manifest");
    assert_eq!(manifest.version, "2.0.0");
    assert_eq!(manifest.release_notes, "Big update");
    assert_eq!(manifest.platforms.len(), 1);
    let asset = manifest
        .platforms
        .get("macos-aarch64")
        .expect("platform key");
    assert_eq!(asset.url, "https://example.com/app.tar.gz");
    assert_eq!(asset.signature, "abc123");
    assert_eq!(asset.size, 12345678);
}

#[test]
fn manifest_parse_minimal_version_only() {
    let json = r#"{ "version": "1.0.0" }"#;
    let manifest: UpdateManifest = serde_json::from_str(json).expect("parse minimal manifest");
    assert_eq!(manifest.version, "1.0.0");
    assert!(manifest.release_notes.is_empty());
    assert!(manifest.platforms.is_empty());
}

#[test]
fn manifest_parse_invalid_json() {
    let json = r#"not valid json"#;
    assert!(serde_json::from_str::<UpdateManifest>(json).is_err());
}

#[test]
fn semver_newer_available() {
    let manifest_ver = semver::Version::parse("2.0.0").unwrap();
    let cur_ver = semver::Version::parse("1.0.0").unwrap();
    assert!(manifest_ver > cur_ver);
}

#[test]
fn semver_equal() {
    let manifest_ver = semver::Version::parse("1.0.0").unwrap();
    let cur_ver = semver::Version::parse("1.0.0").unwrap();
    assert!(manifest_ver == cur_ver);
    assert!(manifest_ver <= cur_ver);
}

#[test]
fn semver_older_manifest() {
    let manifest_ver = semver::Version::parse("0.9.0").unwrap();
    let cur_ver = semver::Version::parse("1.0.0").unwrap();
    assert!(manifest_ver < cur_ver);
}

#[test]
fn semver_invalid_version() {
    assert!(semver::Version::parse("not-a-version").is_err());
}

#[test]
fn update_status_default_is_idle() {
    assert_eq!(UpdateStatus::default(), UpdateStatus::Idle);
}

#[test]
fn pending_swap_path_is_consistent() {
    let a = pending_swap_path();
    let b = pending_swap_path();
    assert_eq!(
        a, b,
        "pending_swap_path should return the same path on every call"
    );
}

#[test]
fn default_manifest_url_not_empty() {
    assert!(!DEFAULT_MANIFEST_URL.is_empty());
}

#[test]
fn auto_checks_need_optin_and_a_real_manifest_url() {
    let real_url = "https://updates.example.org/manifest.json";
    assert!(
        !auto_checks_enabled(false, real_url),
        "default config must keep automatic checks off even on a real endpoint"
    );
    assert!(
        !auto_checks_enabled(true, PLACEHOLDER_MANIFEST_URL),
        "a placeholder endpoint must never be auto-polled"
    );
    assert!(
        auto_checks_enabled(true, real_url),
        "an opted-in user on a real endpoint keeps automatic checks"
    );
}

#[gpui_kit::test]
async fn initialize_runs_no_auto_check_under_default_config(cx: &TestAppContext) {
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
    cx.update(|cx| initialize(cx));

    cx.executor()
        .advance_clock(std::time::Duration::from_secs(STARTUP_CHECK_DELAY_SECS + 1));
    cx.executor().run_until_parked();
    assert_eq!(
        cx.update(|cx| snapshot(cx).status),
        UpdateStatus::Idle,
        "the startup check must not fire under the default config"
    );

    cx.executor()
        .advance_clock(std::time::Duration::from_secs(PERIODIC_CHECK_INTERVAL_SECS));
    cx.executor().run_until_parked();
    assert_eq!(
        cx.update(|cx| snapshot(cx).status),
        UpdateStatus::Idle,
        "the periodic loop must not fire under the default config"
    );
}

#[gpui_kit::test]
async fn placeholder_build_runs_no_auto_check_even_when_opted_in(cx: &TestAppContext) {
    // This test pins placeholder-build behavior; the test binary bakes the
    // placeholder URL because CI never sets GPUI_UPDATE_MANIFEST_URL.
    assert_eq!(DEFAULT_MANIFEST_URL, PLACEHOLDER_MANIFEST_URL);
    cx.update(|cx| {
        install_for_tests(
            AppConfig {
                update_check_on_startup: true,
                ..AppConfig::default()
            },
            cx,
        )
    });
    cx.update(|cx| initialize(cx));

    cx.executor().advance_clock(std::time::Duration::from_secs(
        STARTUP_CHECK_DELAY_SECS + PERIODIC_CHECK_INTERVAL_SECS + 1,
    ));
    cx.executor().run_until_parked();

    assert_eq!(
        cx.update(|cx| snapshot(cx).status),
        UpdateStatus::Idle,
        "a placeholder manifest url must keep the auto-check loops off even with opt-in"
    );
}

#[gpui_kit::test]
async fn manual_check_still_runs_under_default_config(cx: &TestAppContext) {
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
    cx.update(|cx| initialize(cx));

    cx.update(|cx| check_for_updates(cx));

    // No connectivity global is installed, so the deterministic outcome of
    // the manual pipeline is the offline short-circuit error.
    let status = cx.update(|cx| snapshot(cx).status);
    match &status {
        UpdateStatus::Error(err) => assert!(
            err.contains("connectivity"),
            "expected the offline short-circuit error, got: {err}"
        ),
        other => panic!("manual check must leave Idle, got: {other:?}"),
    }
    assert_eq!(
        cx.update(|cx| snapshot(cx).check_retry_count),
        1,
        "the first failed manual check schedules its first retry"
    );
}
