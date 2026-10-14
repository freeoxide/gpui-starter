use std::sync::Arc;

use super::*;

fn tokens(tag: &str) -> TokenSet {
    TokenSet {
        provider_id: "mock".to_string(),
        access_token: crate::SecretString::new(format!("access-{tag}")),
        refresh_token: Some(crate::SecretString::new(format!("refresh-{tag}"))),
        id_token: None,
        token_type: "Bearer".to_string(),
        expires_at: None,
        scope: None,
        account: None,
    }
}

#[gpui::test]
async fn save_then_load_round_trips(cx: &mut gpui::TestAppContext) {
    let store = InMemoryStore::new();
    cx.update(|cx| {
        store.save("mock", &tokens("1"), cx).unwrap();
        let loaded = store.load("mock", cx).unwrap().unwrap();
        assert_eq!(loaded, tokens("1"));
    });
}

#[gpui::test]
async fn save_overwrites_the_previous_set(cx: &mut gpui::TestAppContext) {
    let store = InMemoryStore::new();
    cx.update(|cx| {
        store.save("mock", &tokens("1"), cx).unwrap();
        store.save("mock", &tokens("2"), cx).unwrap();
        assert_eq!(store.load("mock", cx).unwrap(), Some(tokens("2")));
    });
}

#[gpui::test]
async fn delete_then_load_misses_and_double_delete_is_ok(cx: &mut gpui::TestAppContext) {
    let store = InMemoryStore::new();
    cx.update(|cx| {
        store.save("mock", &tokens("1"), cx).unwrap();
        store.delete("mock", cx).unwrap();
        assert_eq!(store.load("mock", cx).unwrap(), None);
        store.delete("mock", cx).unwrap();
    });
}

#[test]
fn concurrent_saves_never_panic_and_the_last_write_wins() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<InMemoryStore>();

    let store = Arc::new(InMemoryStore::new());
    let mut valid: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut handles = Vec::new();
    for thread in 0..8 {
        let store = Arc::clone(&store);
        handles.push(std::thread::spawn(move || {
            for i in 0..50 {
                let tag = format!("{thread}-{i}");
                store.put("mock", tokens(&tag));
            }
        }));
        for i in 0..50 {
            valid.insert(format!("access-{thread}-{i}"));
        }
    }
    for handle in handles {
        handle.join().unwrap();
    }
    let final_set = store.get("mock").unwrap();
    assert!(valid.contains(final_set.access_token.expose()));

    store.put("mock", tokens("final"));
    assert_eq!(store.get("mock"), Some(tokens("final")));
}
