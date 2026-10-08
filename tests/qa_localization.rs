//! Localization catalog guards: every `localize`/`localize_with_args` literal
//! in src/ resolves in both locale files, and en/zh-CN stay in key lockstep.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

// i18n.test.rs asserts this sentinel renders as the key itself, so it must
// stay out of the catalogs.
const MISSING_BY_DESIGN: &[&str] = &["i18n_no_such_key"];

fn manifest_path(relative: &str) -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join(relative)
}

fn read_file(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn catalog_keys(path: &Path) -> BTreeSet<String> {
    let text = read_file(path);
    let mut keys = BTreeSet::new();
    let mut parent = String::new();
    for (index, raw_line) in text.lines().enumerate() {
        let line_number = index + 1;
        if raw_line.trim().is_empty() || raw_line.trim_start().starts_with('#') {
            continue;
        }
        let line = raw_line.trim();
        let colon = line.find(':').unwrap_or_else(|| {
            panic!(
                "{}:{line_number}: missing key colon: {raw_line}",
                path.display()
            )
        });
        let name = &line[..colon];
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            panic!(
                "{}:{line_number}: malformed key {name:?}: {raw_line}",
                path.display()
            );
        }
        let value = line[colon + 1..].trim();
        if !parent.is_empty() && !raw_line.starts_with(' ') {
            parent.clear();
        }
        let location = format!("{}:{line_number}", path.display());
        if value.is_empty() {
            assert!(
                !raw_line.starts_with(' '),
                "{location}: nested map inside a nested map: {raw_line}"
            );
            parent = name.to_string();
            continue;
        }
        assert!(
            value.starts_with('"') && value.ends_with('"') && value.len() >= 2,
            "{location}: value must be a double-quoted scalar: {raw_line}"
        );
        if raw_line.starts_with(' ') {
            assert!(
                !parent.is_empty(),
                "{location}: indented key outside a map: {raw_line}"
            );
            keys.insert(format!("{parent}.{name}"));
        } else {
            keys.insert(name.to_string());
        }
    }
    keys
}

fn is_ident(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn localize_literals(content: &str) -> Vec<(usize, String)> {
    let bytes = content.as_bytes();
    let mut found = Vec::new();
    let mut search_from = 0;
    while let Some(offset) = content[search_from..].find("localize") {
        let start = search_from + offset;
        let mut cursor = start + "localize".len();
        let preceded_by_ident = start > 0 && is_ident(bytes[start - 1]);
        if !preceded_by_ident {
            if content[cursor..].starts_with("_with_args") {
                cursor += "_with_args".len();
            }
            if !bytes.get(cursor).is_some_and(|byte| is_ident(*byte)) {
                while bytes
                    .get(cursor)
                    .is_some_and(|byte| byte.is_ascii_whitespace())
                {
                    cursor += 1;
                }
                if bytes.get(cursor) == Some(&b'(') {
                    cursor += 1;
                    while bytes
                        .get(cursor)
                        .is_some_and(|byte| byte.is_ascii_whitespace())
                    {
                        cursor += 1;
                    }
                    if bytes.get(cursor) == Some(&b'"') {
                        cursor += 1;
                        let mut literal: Vec<u8> = Vec::new();
                        while let Some(byte) = bytes.get(cursor).copied() {
                            cursor += 1;
                            match byte {
                                b'"' => break,
                                b'\\' => {
                                    if let Some(escaped) = bytes.get(cursor).copied() {
                                        literal.push(escaped);
                                        cursor += 1;
                                    }
                                }
                                _ => literal.push(byte),
                            }
                        }
                        found.push((start, String::from_utf8_lossy(&literal).into_owned()));
                    }
                }
            }
        }
        search_from = cursor.max(start + 1);
    }
    found
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
        .map(|entry| entry.expect("read_dir entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn localize_literals_resolve_in_both_catalogs() {
    let en = catalog_keys(&manifest_path("locales/en.yml"));
    let zh = catalog_keys(&manifest_path("locales/zh-CN.yml"));
    let mut sources = Vec::new();
    rust_sources(&manifest_path("src"), &mut sources);
    assert!(!sources.is_empty(), "no rust sources found under src/");

    let mut failures = Vec::new();
    let mut checked = 0;
    for path in &sources {
        let content = read_file(path);
        for (offset, key) in localize_literals(&content) {
            if MISSING_BY_DESIGN.contains(&key.as_str()) {
                continue;
            }
            checked += 1;
            let line_number = content[..offset].matches('\n').count() + 1;
            if !en.contains(&key) {
                failures.push(format!(
                    "{}:{line_number}: \"{key}\" missing from locales/en.yml",
                    path.display()
                ));
            }
            if !zh.contains(&key) {
                failures.push(format!(
                    "{}:{line_number}: \"{key}\" missing from locales/zh-CN.yml",
                    path.display()
                ));
            }
        }
    }
    assert!(
        checked > 0,
        "scanner matched no localize literals; facade renamed?"
    );
    assert!(
        failures.is_empty(),
        "unresolved localize keys:\n{}",
        failures.join("\n")
    );
}

#[test]
fn locale_catalogs_carry_identical_key_sets() {
    let en = catalog_keys(&manifest_path("locales/en.yml"));
    let zh = catalog_keys(&manifest_path("locales/zh-CN.yml"));
    let en_only: Vec<_> = en.difference(&zh).collect();
    let zh_only: Vec<_> = zh.difference(&en).collect();
    assert!(
        en_only.is_empty() && zh_only.is_empty(),
        "catalog key sets diverge: en-only {en_only:?}, zh-CN-only {zh_only:?}"
    );
}
