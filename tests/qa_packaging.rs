//! macOS bundle packaging invariants: every Info.plist-producing script must
//! stamp the TCC usage-description keys for capabilities the app touches
//! today (Bluetooth, microphone). A missing key makes tccd kill the app on
//! first access instead of showing the permission prompt (RCA §2).

use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn repo_file(name: &str) -> String {
    let path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Returns the `<<PLIST` heredoc body a script cats into Info.plist, so
/// assertions target the emitted plist rather than incidental mentions.
fn plist_heredoc(script: &str) -> String {
    let source = repo_file(script);
    let mut lines = source.lines();
    let start = lines
        .position(|l| l.contains("<<PLIST"))
        .unwrap_or_else(|| panic!("{script} has no <<PLIST heredoc"));
    let body: Vec<&str> = source
        .lines()
        .skip(start + 1)
        .take_while(|l| *l != "PLIST")
        .collect();
    assert!(
        !body.is_empty(),
        "{script} has a <<PLIST heredoc but no terminating PLIST line"
    );
    body.join("\n")
}

/// Returns the `<string>` value that follows `<key>key</key>` in a plist
/// fragment, or None when the key is absent / not followed by a string.
fn plist_string_value<'a>(plist: &'a str, key: &str) -> Option<&'a str> {
    let mut lines = plist.lines().peekable();
    while let Some(line) = lines.next() {
        if line.trim() == format!("<key>{key}</key>") {
            let value = lines.next()?.trim();
            return value
                .strip_prefix("<string>")?
                .strip_suffix("</string>")
                .map(str::trim);
        }
    }
    None
}

fn assert_usage_description(plist: &str, script: &str, key: &str, subject: &str) {
    let value = plist_string_value(plist, key).unwrap_or_else(|| {
        panic!("{script} Info.plist lacks {key}; tccd kills the app on first use")
    });
    assert!(
        value.contains(subject),
        "{script} {key} must be user-visible prose mentioning {subject}, got: {value:?}"
    );
}

/// Returns every scripts/*.sh that writes an Info.plist — the complete set of
/// macOS bundle producers, so a new script cannot ship an unstamped plist.
fn infoplist_writers() -> Vec<String> {
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("scripts");
    let mut writers: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read dir {}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "sh"))
        .filter(|p| {
            repo_file(&p.display().to_string())
                .lines()
                .any(|l| l.contains("Info.plist") && l.contains("cat >"))
        })
        .map(|p| {
            p.strip_prefix(dir.parent().unwrap())
                .unwrap_or(&p)
                .display()
                .to_string()
        })
        .collect();
    writers.sort();
    writers
}

const PLIST_SCRIPTS: [&str; 2] = ["scripts/create-release-app.sh", "scripts/macos-dev-app.sh"];

// ---------------------------------------------------------------------------
// 1. TCC usage descriptions
// ---------------------------------------------------------------------------

#[test]
fn app_plists_carry_bluetooth_usage_description() {
    for script in PLIST_SCRIPTS {
        let plist = plist_heredoc(script);
        assert_usage_description(
            &plist,
            script,
            "NSBluetoothAlwaysUsageDescription",
            "Bluetooth",
        );
    }
}

#[test]
fn app_plists_keep_microphone_usage_description() {
    for script in PLIST_SCRIPTS {
        let plist = plist_heredoc(script);
        assert_usage_description(&plist, script, "NSMicrophoneUsageDescription", "microphone");
    }
}

/// Documents the RCA §2 audit: keys for capabilities the app does not use
/// (camera, location, AppleEvents) stay out until a capability needs them.
#[test]
fn app_plists_skip_unused_tcc_keys() {
    for script in PLIST_SCRIPTS {
        let plist = plist_heredoc(script);
        for key in [
            "NSCameraUsageDescription",
            "NSLocationWhenInUseUsageDescription",
            "NSAppleEventsUsageDescription",
        ] {
            assert!(
                plist_string_value(&plist, key).is_none(),
                "{script} declares {key} without a capability that uses it"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 2. Bundle completeness
// ---------------------------------------------------------------------------

/// Every script that writes an Info.plist must stamp both TCC keys; this
/// catches future bundle paths, not just the two known scripts.
#[test]
fn every_infoplist_writer_stamps_tcc_keys() {
    let writers = infoplist_writers();
    assert_eq!(
        writers,
        PLIST_SCRIPTS.to_vec(),
        "scripts/ gained or lost an Info.plist writer; update PLIST_SCRIPTS \
         and keep both TCC keys in the new path"
    );
    for script in &writers {
        let source = repo_file(script);
        for key in [
            "NSBluetoothAlwaysUsageDescription",
            "NSMicrophoneUsageDescription",
        ] {
            assert!(
                source.contains(&format!("<key>{key}</key>")),
                "{script} writes an Info.plist without {key}"
            );
        }
    }
}

#[test]
fn release_plist_stamps_version_from_placeholder() {
    let plist = plist_heredoc("scripts/create-release-app.sh");
    assert_eq!(
        plist_string_value(&plist, "CFBundleShortVersionString"),
        Some("$VERSION"),
        "the release plist must keep deriving its version from $VERSION so \
         callers control CFBundleShortVersionString per artifact"
    );
    for script in PLIST_SCRIPTS {
        let plist = plist_heredoc(script);
        assert!(
            plist_string_value(&plist, "CFBundleShortVersionString").is_some(),
            "{script} Info.plist lost CFBundleShortVersionString"
        );
    }
}
