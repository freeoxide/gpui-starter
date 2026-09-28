//! Kit 0.7 migration invariants the compiler cannot check: manifest pins,
//! the single-kit lock graph (the round-1 links-conflict failure mode), and
//! the vendored gpui-form patch shape.

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
// 3. Vendored gpui-form patch shape
// ---------------------------------------------------------------------------

#[test]
fn gpui_form_stays_path_patched_from_vendor() {
    let manifest = repo_file("Cargo.toml");
    assert!(
        manifest.contains("[patch.\"https://github.com/stayhydated/gpui-form\"]"),
        "the gpui-form patch table must stay keyed by the upstream git URL"
    );
    for crate_path in [
        "vendor/gpui-form/crates/gpui-form",
        "vendor/gpui-form/crates/gpui-form-collection",
    ] {
        assert!(
            manifest.contains(&format!("path = \"{crate_path}\"")),
            "patch table must map to {crate_path}"
        );
    }

    let lock = repo_file("Cargo.lock");
    for package in ["gpui-form", "gpui-form-collection"] {
        let blocks = lock_blocks(&lock, package);
        assert_eq!(
            blocks.len(),
            1,
            "expected exactly one {package} in Cargo.lock"
        );
        assert!(
            blocks[0].contains("\nversion = \"0.6.0\""),
            "{package} keeps its patched 0.6.0 version"
        );
        assert!(
            !blocks[0].contains("source = "),
            "{package} must be path-sourced from vendor/, not registry/git"
        );
    }
}
