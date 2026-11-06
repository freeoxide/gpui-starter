use super::types::{
    DEFAULT_MANIFEST_URL, MAX_UPDATE_RETRIES, PERIODIC_CHECK_INTERVAL_SECS,
    PLACEHOLDER_MANIFEST_URL, RETRY_BASE_DELAY_SECS, STARTUP_CHECK_DELAY_SECS, current_app_version,
    pending_swap_path,
};
use super::*;
use crate::app_state::{AppConfig, install_for_tests};
use gpui_kit::TestAppContext;

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
async fn direct_check_runs_under_default_config(cx: &TestAppContext) {
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
    cx.update(|cx| initialize(cx));

    cx.update(|cx| super::check::check_for_updates(cx));

    // No connectivity global is installed, so the deterministic outcome of
    // a direct check is the offline short-circuit error.
    let status = cx.update(|cx| snapshot(cx).status);
    match &status {
        UpdateStatus::Error(err) => assert!(
            err.contains("connectivity"),
            "expected the offline short-circuit error, got: {err}"
        ),
        other => panic!("expected the offline short-circuit error, got: {other:?}"),
    }
    assert_eq!(
        cx.update(|cx| snapshot(cx).check_retry_count),
        1,
        "the first failed check schedules its first retry"
    );
}

/// Installs what check_for_updates needs to reach its decision branch:
/// online connectivity, a tokio runtime global, and the updater snapshot.
fn install_online_check_fixture(cx: &mut gpui_kit::App) {
    install_for_tests(AppConfig::default(), cx);
    let runtime = std::sync::Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("scratch runtime"),
    );
    cx.set_global(crate::services::tokio_runtime::TokioRuntimeGlobal(
        crate::services::tokio_runtime::TokioRuntime {
            runtime,
            http_client: reqwest::Client::new(),
        },
    ));
    crate::connectivity::initialize(cx);
    crate::connectivity::ConnectivitySnapshot::update_global(cx, |next, _| {
        next.state = crate::connectivity::ConnectivityState::Online;
    });
    initialize(cx);
}

/// Seeds a cache hit fresh enough that check_for_updates skips the fetch and
/// runs only the version comparison — the decision branch stays offline.
fn prime_cached_manifest(json: &str, cx: &mut gpui_kit::App) {
    let manifest: UpdateManifest = serde_json::from_str(json).expect("parse cached manifest");
    UpdateSnapshot::update_global(cx, |snap, _| {
        snap.cached_manifest = Some(manifest);
        snap.last_check = Some(chrono::Utc::now().to_rfc3339());
    });
}

#[gpui_kit::test]
async fn check_reports_available_for_a_newer_manifest(cx: &TestAppContext) {
    cx.update(|cx| {
        install_online_check_fixture(cx);
        prime_cached_manifest(
            r#"{ "version": "9999.0.0", "release_notes": "big release" }"#,
            cx,
        );
        UpdateSnapshot::update_global(cx, |snap, _| {
            snap.check_retry_count = MAX_UPDATE_RETRIES - 1
        });
    });

    cx.update(|cx| super::check::check_for_updates(cx));
    cx.executor().run_until_parked();

    match cx.update(|cx| snapshot(cx).status) {
        UpdateStatus::Available { version, notes } => {
            assert_eq!(version, "9999.0.0");
            assert_eq!(notes, "big release");
        }
        other => panic!("a newer manifest must surface as Available, got: {other:?}"),
    }
    assert_eq!(
        cx.update(|cx| snapshot(cx)).check_retry_count,
        0,
        "a successful check resets the retry budget"
    );
}

#[gpui_kit::test]
async fn check_reports_uptodate_for_equal_and_older_manifests(cx: &TestAppContext) {
    cx.update(|cx| {
        install_online_check_fixture(cx);
        let current = current_app_version();
        prime_cached_manifest(&format!(r#"{{ "version": "{current}" }}"#), cx);
    });

    cx.update(|cx| super::check::check_for_updates(cx));
    cx.executor().run_until_parked();
    assert_eq!(
        cx.update(|cx| snapshot(cx).status),
        UpdateStatus::UpToDate,
        "a manifest at the running version means up to date"
    );

    cx.update(|cx| prime_cached_manifest(r#"{ "version": "0.0.1" }"#, cx));
    cx.update(|cx| super::check::check_for_updates(cx));
    cx.executor().run_until_parked();
    assert_eq!(
        cx.update(|cx| snapshot(cx).status),
        UpdateStatus::UpToDate,
        "an older manifest version also means up to date"
    );
}

#[gpui_kit::test]
async fn check_errors_on_an_unparseable_manifest_version(cx: &TestAppContext) {
    cx.update(|cx| {
        install_online_check_fixture(cx);
        prime_cached_manifest(r#"{ "version": "not-a-version" }"#, cx);
    });

    cx.update(|cx| super::check::check_for_updates(cx));
    cx.executor().run_until_parked();

    match cx.update(|cx| snapshot(cx).status) {
        UpdateStatus::Error(err) => assert!(
            err.starts_with("version parse error"),
            "expected a version parse error, got: {err}"
        ),
        other => panic!("an unparseable manifest version must error, got: {other:?}"),
    }
}

#[gpui_kit::test]
async fn failed_checks_back_off_until_retries_are_exhausted(cx: &TestAppContext) {
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
    cx.update(|cx| initialize(cx));

    // No connectivity global is installed, so every check fails offline.
    cx.update(|cx| super::check::check_for_updates(cx));
    assert_eq!(cx.update(|cx| snapshot(cx)).check_retry_count, 1);

    cx.executor()
        .advance_clock(std::time::Duration::from_secs(RETRY_BASE_DELAY_SECS));
    cx.executor().run_until_parked();
    assert_eq!(
        cx.update(|cx| snapshot(cx)).check_retry_count,
        2,
        "the first retry fires after the base backoff"
    );

    cx.executor()
        .advance_clock(std::time::Duration::from_secs(RETRY_BASE_DELAY_SECS * 2));
    cx.executor().run_until_parked();
    assert_eq!(
        cx.update(|cx| snapshot(cx)).check_retry_count,
        3,
        "the second retry fires after the doubled backoff"
    );

    cx.executor()
        .advance_clock(std::time::Duration::from_secs(RETRY_BASE_DELAY_SECS * 4));
    cx.executor().run_until_parked();
    assert_eq!(
        cx.update(|cx| snapshot(cx)).check_retry_count,
        MAX_UPDATE_RETRIES,
        "the last allowed retry fires but schedules no further check"
    );

    cx.executor()
        .advance_clock(std::time::Duration::from_secs(RETRY_BASE_DELAY_SECS * 8));
    cx.executor().run_until_parked();
    assert_eq!(
        cx.update(|cx| snapshot(cx)).check_retry_count,
        MAX_UPDATE_RETRIES,
        "an exhausted retry budget never rearms"
    );
    assert!(matches!(
        cx.update(|cx| snapshot(cx).status),
        UpdateStatus::Error(_)
    ));
    assert_eq!(
        cx.update(|cx| crate::notifications::inbox::unread_count(cx)),
        1,
        "only the permanent failure reaches the notification inbox"
    );
}

/// Writes `bytes` to a temp dir and returns (path, base64 Ed25519 signature
/// over the SHA-256 of `bytes`, matching public key) — the swap's scheme.
fn write_signed_update(
    dir_name: &str,
    bytes: &[u8],
    seed: u8,
) -> (std::path::PathBuf, String, [u8; 32]) {
    use base64::Engine as _;
    use ed25519_dalek::{Signer as _, SigningKey};
    use sha2::Digest as _;

    let dir = std::env::temp_dir().join(dir_name);
    std::fs::create_dir_all(&dir).expect("create temp update dir");
    let path = dir.join("update.bin");
    std::fs::write(&path, bytes).expect("write update bytes");

    let signing_key = SigningKey::from_bytes(&[seed; 32]);
    let mut hasher = sha2::Sha256::new();
    hasher.update(bytes);
    let signature = signing_key.sign(&hasher.finalize());
    let signature_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());
    let public_key = ed25519_dalek::VerifyingKey::from(&signing_key).to_bytes();
    (path, signature_b64, public_key)
}

#[test]
fn signature_verify_accepts_a_valid_signature() {
    let (path, signature_b64, public_key) =
        write_signed_update("updater-sig-valid", b"update payload", 1);
    assert_eq!(
        super::apply::verify_ed25519_signature(&path, &signature_b64, &public_key),
        Ok(()),
        "a correctly signed file must verify"
    );
}

#[test]
fn signature_verify_rejects_tampered_file_bytes() {
    let (path, signature_b64, public_key) =
        write_signed_update("updater-sig-tampered", b"original payload", 1);
    std::fs::write(&path, b"tampered payload").expect("rewrite update bytes");
    let err =
        super::apply::verify_ed25519_signature(&path, &signature_b64, &public_key).unwrap_err();
    assert!(
        err.contains("verification failed"),
        "expected a verification failure, got: {err}"
    );
}

#[test]
fn signature_verify_rejects_signature_from_wrong_key() {
    let (path, signature_b64, _) =
        write_signed_update("updater-sig-wrong-key", b"update payload", 1);
    let wrong_key =
        ed25519_dalek::VerifyingKey::from(&ed25519_dalek::SigningKey::from_bytes(&[2; 32]))
            .to_bytes();
    let err =
        super::apply::verify_ed25519_signature(&path, &signature_b64, &wrong_key).unwrap_err();
    assert!(
        err.contains("verification failed"),
        "expected a verification failure, got: {err}"
    );
}

#[test]
fn signature_verify_rejects_short_signature() {
    let (path, _, public_key) = write_signed_update("updater-sig-short", b"update payload", 1);
    use base64::Engine as _;
    let short = base64::engine::general_purpose::STANDARD.encode([0u8; 32]);
    let err = super::apply::verify_ed25519_signature(&path, &short, &public_key).unwrap_err();
    assert!(
        err.contains("invalid signature length"),
        "expected a length rejection, got: {err}"
    );
}

#[test]
fn signature_verify_rejects_bad_base64() {
    let (path, _, public_key) = write_signed_update("updater-sig-bad-b64", b"update payload", 1);
    let err =
        super::apply::verify_ed25519_signature(&path, "not base64 !!!", &public_key).unwrap_err();
    assert!(
        err.contains("base64"),
        "expected a base64 decode failure, got: {err}"
    );
}

#[test]
fn signature_verify_fails_closed_on_empty_signature() {
    let (path, _, public_key) = write_signed_update("updater-sig-empty", b"update payload", 1);
    let err = super::apply::verify_ed25519_signature(&path, "", &public_key).unwrap_err();
    assert!(
        err.contains("invalid signature length"),
        "an absent signature must never verify, got: {err}"
    );
}

/// Every marker test must refuse before swap_installed_app: the accepted
/// path renames the source over the running test binary.
fn write_marker(dir_name: &str, marker_json: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(dir_name);
    std::fs::create_dir_all(&dir).expect("create marker dir");
    let path = dir.join("pending-swap.json");
    std::fs::write(&path, marker_json).expect("write pending-swap marker");
    path
}

#[test]
fn pending_swap_drops_a_corrupt_marker() {
    let marker = write_marker("updater-marker-corrupt", "not json {{{");
    assert_eq!(super::apply::run_pending_swap(marker.clone()), None);
    assert!(
        !marker.exists(),
        "a corrupt marker is deleted so startup stops retrying it"
    );
}

#[test]
fn pending_swap_drops_a_marker_without_source_path() {
    let marker = write_marker("updater-marker-no-source", r#"{ "version": "9999.0.0" }"#);
    assert_eq!(super::apply::run_pending_swap(marker.clone()), None);
    assert!(!marker.exists(), "a marker without a source is deleted");
}

#[test]
fn pending_swap_refuses_a_marker_without_a_signature() {
    let (source, _, _) = write_signed_update("updater-marker-unsigned", b"new binary", 7);
    let marker = write_marker(
        "updater-marker-unsigned",
        &serde_json::json!({ "source_path": source, "version": "9999.0.0" }).to_string(),
    );
    match super::apply::run_pending_swap(marker.clone()) {
        Some(UpdateStatus::Error(err)) => {
            assert_eq!(err, "pending swap refused: no signature in marker");
        }
        other => panic!("a signature-less marker must be refused, got: {other:?}"),
    }
    assert!(!marker.exists(), "a refused marker is removed");
}

#[test]
fn pending_swap_refuses_a_source_failing_reverification() {
    let (source, signature_b64, _) =
        write_signed_update("updater-marker-bad-sig", b"new binary", 7);
    std::fs::write(&source, b"tampered binary").expect("rewrite source bytes");
    let marker = write_marker(
        "updater-marker-bad-sig",
        &serde_json::json!({
            "source_path": source,
            "version": "9999.0.0",
            "signature": signature_b64,
        })
        .to_string(),
    );
    match super::apply::run_pending_swap(marker.clone()) {
        Some(UpdateStatus::Error(err)) => assert!(
            err.starts_with("pending swap refused: "),
            "expected a verification refusal, got: {err}"
        ),
        other => panic!("a tampered source must be refused, got: {other:?}"),
    }
    assert!(!marker.exists(), "a refused marker is removed");
}

#[test]
fn pending_swap_removes_markers_not_strictly_newer() {
    let current = current_app_version();
    let (source, signature_b64, _) =
        write_signed_update("updater-marker-not-newer", b"new binary", 7);
    for version in [current.as_str(), "0.0.1", "not-a-version"] {
        let marker = write_marker(
            "updater-marker-not-newer",
            &serde_json::json!({
                "source_path": &source,
                "version": version,
                "signature": &signature_b64,
            })
            .to_string(),
        );
        assert_eq!(
            super::apply::run_pending_swap(marker.clone()),
            None,
            "a marker at version {version} must never swap in"
        );
        assert!(!marker.exists(), "a non-newer marker is removed at startup");
    }
}
