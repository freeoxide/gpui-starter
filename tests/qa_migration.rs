//! Kit 0.7 migration invariants the compiler cannot check: manifest pins,
//! the single-kit lock graph (the round-1 links-conflict failure mode), and
//! the gpui-form re-pin to the migrated freeoxide repo.

use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn repo_file(name: &str) -> String {
    let path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Returns every `[[package]]` block whose package name matches.
fn lock_blocks<'a>(lock: &'a str, package: &str) -> Vec<&'a str> {
    lock.split("[[package]]")
        .skip(1)
        .filter(|block| {
            block
                .lines()
                .any(|l| l.trim() == format!("name = \"{package}\""))
        })
        .collect()
}

fn block_name(block: &str) -> &str {
    block
        .lines()
        .find_map(|l| l.trim().strip_prefix("name = "))
        .map(|n| n.trim_matches('"'))
        .unwrap_or_default()
}

fn assert_single_version(lock: &str, package: &str, version: &str) {
    let blocks = lock_blocks(lock, package);
    assert_eq!(
        blocks.len(),
        1,
        "expected exactly one {package} in Cargo.lock, found {} \
         (a second copy reintroduces the gpui-kit-assets links conflict)",
        blocks.len()
    );
    assert!(
        blocks[0].contains(&format!("\nversion = \"{version}\"")),
        "{package} must resolve to {version}"
    );
}

// ---------------------------------------------------------------------------
// 1. Manifest pins
// ---------------------------------------------------------------------------

#[test]
fn manifest_pins_kit_070_and_gpui_pre_037() {
    let manifest = repo_file("Cargo.toml");

    assert!(
        manifest.contains("package = \"gpui-pre\", version = \"0.3.7\""),
        "gpui (gpui-pre) must stay on the kit-0.7 snapshot"
    );
    assert!(
        manifest.contains("package = \"gpui-pre-platform\", version = \"0.3.7\""),
        "gpui_platform must stay on the kit-0.7 snapshot"
    );
    for pin in ["gpui-kit", "gpui-component", "gpui-kit-assets"] {
        assert!(
            manifest.contains(&format!("\n{pin} = \"0.7.0\"")),
            "Cargo.toml must pin {pin} to 0.7.0"
        );
    }
}

// ---------------------------------------------------------------------------
// 2. Single-kit lock graph
// ---------------------------------------------------------------------------

#[test]
fn lock_graph_holds_one_kit_family() {
    let lock = repo_file("Cargo.lock");

    for (package, version) in [
        ("gpui-kit", "0.7.0"),
        ("gpui-component", "0.7.0"),
        ("gpui-kit-assets", "0.7.0"),
        ("gpui-pre", "0.3.7"),
        ("gpui-pre-platform", "0.3.7"),
    ] {
        assert_single_version(&lock, package, version);
    }
}

#[test]
fn gpui_pre_snapshot_family_is_unified() {
    let lock = repo_file("Cargo.lock");

    let family: Vec<&str> = lock
        .split("[[package]]")
        .skip(1)
        .filter(|block| block_name(block).starts_with("gpui-pre"))
        .collect();
    assert!(
        family.len() >= 20,
        "expected the full gpui-pre snapshot family, found {}",
        family.len()
    );

    for block in &family {
        let name = block_name(block);
        if name == "gpui-pre-reqwest" {
            continue;
        }
        assert!(
            block.contains("\nversion = \"0.3.7\""),
            "{name} must sit on the unified 0.3.7 snapshot"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. gpui-form re-pin (freeoxide git rev, resolved unpatched)
// ---------------------------------------------------------------------------

#[test]
fn gpui_form_pins_migrated_freeoxide_repo() {
    let manifest = repo_file("Cargo.toml");
    assert!(
        manifest.contains(
            "gpui-form = { git = \"https://github.com/freeoxide/gpui-form\", \
             rev = \"f7e2fb0b30c1285638f1a877489dc03ad084319a\" }"
        ),
        "gpui-form must pin the freeoxide migration-branch HEAD byte-exact"
    );
    assert!(
        !manifest.contains("stayhydated"),
        "the retired stayhydated fork must not reappear in Cargo.toml"
    );
    assert!(
        !manifest.contains("gpui-form-collection"),
        "gpui-form-collection has no counterpart in the migrated repo"
    );
    assert!(
        manifest.contains(
            "gpui-query = { git = \"https://github.com/hmziqagent/gpui-query\", rev = \"1449ef2\" }"
        ),
        "the gpui-query [patch.crates-io] override must stay untouched"
    );

    let lock = repo_file("Cargo.lock");
    assert!(
        !lock.contains("stayhydated"),
        "Cargo.lock must carry zero stayhydated URLs"
    );
    assert!(
        lock_blocks(&lock, "gpui-form-collection").is_empty(),
        "gpui-form-collection must stay absent from the lock"
    );
    assert_single_version(&lock, "gpui-form", "0.5.2");
    let query = lock_blocks(&lock, "gpui-query");
    assert_eq!(
        query.len(),
        1,
        "expected exactly one gpui-query in Cargo.lock"
    );
    assert!(
        query[0].contains("source = \"git+https://github.com/hmziqagent/gpui-query?rev=1449ef2"),
        "gpui-query must resolve from the hmziqagent git source"
    );
}

#[test]
fn satellites_stay_on_form_repo_registry_lines() {
    let lock = repo_file("Cargo.lock");
    for (prefix, version) in [("koruma", "0.9.0"), ("es-fluent", "0.16.0")] {
        let family: Vec<&str> = lock
            .split("[[package]]")
            .skip(1)
            .filter(|block| {
                let name = block_name(block);
                name == prefix || name.starts_with(&format!("{prefix}-"))
            })
            .collect();
        assert!(
            family.len() >= 2,
            "expected the {prefix} family in Cargo.lock, found {}",
            family.len()
        );
        for block in &family {
            let name = block_name(block);
            assert_eq!(
                lock_blocks(&lock, name).len(),
                1,
                "{name} must exist as exactly one copy in Cargo.lock"
            );
            assert!(
                block.contains(&format!("\nversion = \"{version}\"")),
                "{name} must resolve to {version}"
            );
            assert!(
                block
                    .contains("source = \"registry+https://github.com/rust-lang/crates.io-index\""),
                "{name} must resolve from the registry, not a git/path duplicate"
            );
        }
    }
}

#[test]
fn gpui_form_resolves_from_freeoxide_git_unpatched() {
    let manifest = repo_file("Cargo.toml");
    assert!(
        !manifest.contains("[patch.\"https://github.com/freeoxide/gpui-form\"]"),
        "the retired gpui-form [patch] table must not reappear; the pinned \
         rev is reachable upstream and resolves from git without it"
    );

    let lock = repo_file("Cargo.lock");
    let blocks = lock_blocks(&lock, "gpui-form");
    assert_eq!(
        blocks.len(),
        1,
        "expected exactly one gpui-form in Cargo.lock"
    );
    assert!(
        blocks[0].contains(
            "source = \"git+https://github.com/freeoxide/gpui-form?rev=f7e2fb0b30c1285638f1a877489dc03ad084319a"
        ),
        "gpui-form must resolve from the freeoxide git source at the pinned rev"
    );
}
