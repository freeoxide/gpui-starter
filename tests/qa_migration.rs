//! Kit 0.7 migration invariants the compiler cannot check: manifest pins,
//! the single-kit lock graph (the round-1 links-conflict failure mode), the
//! gpui-form re-pin to the migrated freeoxide repo, and the kit-facade
//! import surface.

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

/// Returns every `.rs` file the crate compiles or runs as a test target —
/// the src/ tree, build.rs, and tests/*.rs — minus this file, whose own
/// assertions spell the banned paths they check for. Sorted so failures name
/// files stably.
fn app_sources() -> Vec<(String, String)> {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let mut files = Vec::new();
    let mut stack = vec![root.join("src")];
    while let Some(dir) = stack.pop() {
        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("read dir {}: {e}", dir.display()))
            .map(|e| e.unwrap().path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let content = std::fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
                files.push((path.display().to_string(), content));
            }
        }
    }
    let build = root.join("build.rs");
    files.push((
        build.display().to_string(),
        std::fs::read_to_string(&build).expect("read build.rs"),
    ));
    let mut tests: Vec<_> = std::fs::read_dir(root.join("tests"))
        .unwrap_or_else(|e| panic!("read dir {}: {e}", root.join("tests").display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    tests.sort();
    for path in tests {
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        files.push((path.display().to_string(), content));
    }
    files.retain(|(path, _)| !path.ends_with("qa_migration.rs"));
    files
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
fn manifest_declares_only_justified_kit_deps() {
    let manifest = repo_file("Cargo.toml");

    // gpui stays beside gpui-kit for one reason: the pinned gpui-form rev's
    // derives emit ::gpui::Entity/Window/Context into this crate.
    assert!(
        manifest.contains("package = \"gpui-pre\", version = \"0.3.7\""),
        "gpui (gpui-pre) must stay on the kit-0.7 snapshot"
    );
    assert!(
        manifest.contains("\ngpui-kit = \"0.7.0\""),
        "Cargo.toml must pin gpui-kit to 0.7.0"
    );
    // The layer crates are reached through gpui-kit's own re-exports; the
    // anchor is line start so Cargo.toml's comment prose cannot trip this.
    for dropped in ["gpui_platform", "gpui-component", "gpui-kit-assets"] {
        assert!(
            !manifest.contains(&format!("\n{dropped}")),
            "Cargo.toml must not declare {dropped}; gpui-kit already provides \
             it and qa_migration bans its import paths"
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
        ("gpui-component-macros", "0.7.0"),
        ("gpui-base", "0.7.0"),
        ("gpui-kit-assets", "0.7.0"),
        ("gpui-pre", "0.3.7"),
        ("gpui-pre-platform", "0.3.7"),
    ] {
        assert_single_version(&lock, package, version);
    }
}

/// Reads a file from the committed HEAD blob. Cargo rewrites a stale
/// working-tree Cargo.lock during dependency resolution, so only the
/// committed copy shows what a `cargo build --locked` of this commit sees.
fn committed_file(path: &str) -> String {
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let output = std::process::Command::new("git")
        .arg("show")
        .arg(format!("HEAD:{path}"))
        .current_dir(&dir)
        .output()
        .unwrap_or_else(|e| panic!("run `git show HEAD:{path}`: {e}"));
    assert!(
        output.status.success(),
        "`git show HEAD:{path}` failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap_or_else(|_| panic!("HEAD:{path} is not utf-8"))
}

#[test]
fn committed_lock_carries_no_removed_dependency() {
    let committed_lock = committed_file("Cargo.lock");
    let own = lock_blocks(&committed_lock, "gpui-starter");
    assert_eq!(
        own.len(),
        1,
        "expected one gpui-starter block in HEAD's Cargo.lock"
    );
    for package in ["gpui-pre-platform", "gpui-component", "gpui-kit-assets"] {
        assert!(
            !own[0].contains(&format!("\"{package}\"")),
            "HEAD's Cargo.lock still lists {package} as a direct dependency of \
             gpui-starter while Cargo.toml has dropped it. Cargo rewrites a \
             stale working-tree lock during resolution, so the gates stay \
             green while `cargo build --locked` on the commit fails; stage the \
             regenerated Cargo.lock together with the Cargo.toml change"
        );
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
        manifest.contains("gpui-query = { version = \"0.3\", features = [\"hook\"] }"),
        "the gpui-query requirement must keep matching the bridge's stamped version"
    );
    assert!(
        manifest.contains(
            "gpui-query = { git = \"https://github.com/hmziqagent/gpui-query\", rev = \"69a071f\" }"
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
        query[0].contains("source = \"git+https://github.com/hmziqagent/gpui-query?rev=69a071f"),
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

// ---------------------------------------------------------------------------
// 4. Kit facade import surface
// ---------------------------------------------------------------------------

// The `gpui` crate stays only for gpui-form's derives; hand-written imports
// go through the kit, and the compiler now rejects the layer crates.
#[test]
fn sources_reach_kit_layers_only_through_the_facade() {
    let bans = [
        // (banned path prefix, remedy)
        (
            "use gpui::",
            "import from gpui_kit (root glob or named items)",
        ),
        ("gpui_platform::", "import gpui_kit::platform"),
        ("gpui_component::", "import gpui_kit::component"),
        ("gpui_kit_assets::", "import gpui_kit::assets"),
    ];
    for (path, content) in app_sources() {
        for (banned, remedy) in bans {
            assert!(
                !content.contains(banned),
                "{path} bypasses the kit facade with `{banned}`; {remedy}"
            );
        }
    }
}
