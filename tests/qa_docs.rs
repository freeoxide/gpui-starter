#[allow(dead_code)]
mod common;

use common::repo_file;

#[test]
fn qa_matrix_contains_core_cases() {
    let content = repo_file("docs/qa-matrix.md");
    let normalized = content.to_lowercase();
    assert!(normalized.contains("second-instance forwarding"));
    assert!(normalized.contains("open logs folder"));
    assert!(normalized.contains("secure storage unavailable path"));
}

#[test]
fn accessibility_checklist_exists() {
    let content = repo_file("docs/accessibility-checklist.md");
    assert!(content.contains("Keyboard And Focus"));
    assert!(content.contains("Done Criteria"));
}
