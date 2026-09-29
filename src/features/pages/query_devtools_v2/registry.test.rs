// No `use super::*` here: registry.rs globs `gpui_kit::*`, which would drag
// the kit's `test` macro into scope and make `#[test]` resolve to it
// recursively.
use super::{QuerySort, RegistryRowCache, cached_registry_rows};
use gpui_query::client::{ClientDiagnostic, QueryDiagnostic};
use gpui_query::core::QueryStatus;
use std::rc::Rc;

fn query(key: &str, status: QueryStatus, cache_hits: u64) -> QueryDiagnostic {
    QueryDiagnostic {
        key: key.to_string(),
        status,
        cache_policy: "NoCache".to_string(),
        cache_age_ms: None,
        cache_hits,
        retry_count: 0,
    }
}

fn diagnostic(queries: Vec<QueryDiagnostic>) -> ClientDiagnostic {
    ClientDiagnostic {
        query_count: queries.len(),
        mutation_count: 0,
        queries,
        mutations: Vec::new(),
    }
}

#[test]
fn registry_rows_sort_by_key_and_cache_hits_return_the_same_rows() {
    let mut cache = RegistryRowCache::default();
    let d = diagnostic(vec![
        query("playground::b", QueryStatus::Success, 2),
        query("playground::a", QueryStatus::Idle, 9),
    ]);

    let rows = cached_registry_rows(&mut cache, &Some(d.clone()), QuerySort::Key, &None);
    assert_eq!(rows[0].query.key, "playground::a");
    assert_eq!(rows[0].cache_hits_str.as_ref(), "9");

    let hit = cached_registry_rows(&mut cache, &Some(d), QuerySort::Key, &None);
    assert!(
        Rc::ptr_eq(&rows, &hit),
        "an unchanged signature must reuse the cached rows"
    );
}

#[test]
fn sort_or_filter_changes_rebuild_the_cache() {
    let mut cache = RegistryRowCache::default();
    let d = diagnostic(vec![
        query("playground::a", QueryStatus::Success, 1),
        query("playground::b", QueryStatus::Failure, 5),
    ]);

    let by_hits = cached_registry_rows(&mut cache, &Some(d.clone()), QuerySort::CacheHits, &None);
    assert_eq!(by_hits[0].query.key, "playground::b", "most hits first");

    let filtered = cached_registry_rows(
        &mut cache,
        &Some(d),
        QuerySort::CacheHits,
        &Some("Failure".to_string()),
    );
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].query.key, "playground::b");
    assert!(
        !Rc::ptr_eq(&by_hits, &filtered),
        "a changed filter must not reuse the cached rows"
    );
}
