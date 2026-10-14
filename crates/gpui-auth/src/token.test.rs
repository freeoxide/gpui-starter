use super::*;

fn sample_set(access: &str, refresh: Option<&str>, expires_at: Option<i64>) -> TokenSet {
    TokenSet {
        provider_id: "mock".to_string(),
        access_token: SecretString::new(access),
        refresh_token: refresh.map(SecretString::new),
        id_token: None,
        token_type: "Bearer".to_string(),
        expires_at,
        scope: Some("read".to_string()),
        account: Some(AccountInfo {
            id: Some("u1".to_string()),
            email: Some("user@example.test".to_string()),
            display_name: Some("Mock account".to_string()),
            avatar_url: Some("https://example.test/avatar.png".to_string()),
        }),
    }
}

#[test]
fn secret_debug_redacts_the_value() {
    let rendered = format!("{:?}", SecretString::new("hunter2"));
    assert!(rendered.contains("[redacted]"));
    assert!(!rendered.contains("hunter2"));
}

#[test]
fn secret_serde_round_trip_preserves_the_value() {
    let original = SecretString::new("hunter2");
    let json = serde_json::to_string(&original).unwrap();
    let restored: SecretString = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, original);
    assert_eq!(restored.expose(), "hunter2");
}

#[test]
fn secret_equality_compares_inner_values() {
    assert_eq!(SecretString::new("a"), SecretString::new("a"));
    assert_ne!(SecretString::new("a"), SecretString::new("b"));
}

#[test]
fn no_advertised_expiry_is_never_expired_or_stale() {
    let set = sample_set("a", None, None);
    assert!(!set.is_expired(1_000_000_000));
    assert!(!set.needs_refresh(1_000_000_000, 60));
}

#[test]
fn is_expired_at_and_past_the_deadline() {
    let set = sample_set("a", None, Some(100));
    assert!(!set.is_expired(99));
    assert!(set.is_expired(100));
    assert!(set.is_expired(101));
}

#[test]
fn needs_refresh_true_only_inside_the_margin() {
    let set = sample_set("a", None, Some(100));
    assert!(!set.needs_refresh(39, 60));
    assert!(set.needs_refresh(40, 60));
    assert!(set.needs_refresh(100, 60));
}

#[test]
fn reuse_until_dead_keeps_old_refresh_when_response_omits_one() {
    let old = sample_set("old", Some("refresh-old"), None);
    let new = sample_set("new", None, None);
    let merged = RefreshPolicy::ReuseUntilDead.merge(&old, new);
    assert_eq!(merged.access_token, SecretString::new("new"));
    assert_eq!(merged.refresh_token, Some(SecretString::new("refresh-old")));
}

#[test]
fn reuse_until_dead_takes_the_new_refresh_when_present() {
    let old = sample_set("old", Some("refresh-old"), None);
    let new = sample_set("new", Some("refresh-new"), None);
    let merged = RefreshPolicy::ReuseUntilDead.merge(&old, new);
    assert_eq!(merged.refresh_token, Some(SecretString::new("refresh-new")));
}

#[test]
fn rotated_drops_the_old_refresh_token() {
    let old = sample_set("old", Some("refresh-old"), None);
    let new = sample_set("new", None, None);
    let merged = RefreshPolicy::Rotated.merge(&old, new);
    assert_eq!(merged.refresh_token, None);
}

#[test]
fn token_set_serde_round_trip_keeps_account_info() {
    let set = sample_set("access-1", Some("refresh-1"), Some(1234567890));
    let json = serde_json::to_string(&set).unwrap();
    assert!(json.contains("access-1"));
    assert!(json.contains("https://example.test/avatar.png"));
    let restored: TokenSet = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, set);
}

#[test]
fn account_info_without_avatar_url_parses_as_none() {
    let json = r#"{
        "provider_id": "mock",
        "access_token": "a",
        "token_type": "Bearer",
        "account": { "id": "u1", "email": "user@example.test", "display_name": "Mock account" }
    }"#;
    let set: TokenSet = serde_json::from_str(json).unwrap();
    let account = set.account.unwrap();
    assert_eq!(account.avatar_url, None);
    assert_eq!(account.display_name.as_deref(), Some("Mock account"));
}

#[test]
fn redacted_summary_omits_every_secret_value() {
    let set = sample_set("access-1", Some("refresh-1"), Some(1234567890));
    let summary = set.redacted_summary();
    assert!(!summary.contains("access-1"));
    assert!(!summary.contains("refresh-1"));
    assert!(summary.contains("mock"));
    assert!(summary.contains("1234567890"));
    assert!(summary.contains("user@example.test"));
}
