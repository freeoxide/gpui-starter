use std::sync::Arc;

use super::*;

const NOW: i64 = 10_000;

fn fast_backend() -> DemoBackend {
    DemoBackend::with_params(argon_params(8, 1, 1))
}

fn password(value: &str) -> SecretString {
    SecretString::new(value)
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("test runtime")
        .block_on(future)
}

fn sign_up_user(backend: &DemoBackend, email: &str, pwd: &str) {
    block_on(backend.sign_up(reqwest::Client::new(), email, &password(pwd), NOW)).unwrap();
}

fn sign_in_user(backend: &DemoBackend, email: &str, pwd: &str, now: i64) -> AuthenticatedSession {
    block_on(backend.sign_in(reqwest::Client::new(), email, &password(pwd), now)).unwrap()
}

#[test]
fn hash_round_trips_for_correct_and_wrong_passwords() {
    let backend = fast_backend();
    let stored = block_on(backend.hash_password(password("correct-horse"))).unwrap();
    assert!(block_on(backend.verify_password(password("correct-horse"), stored.clone())).unwrap());
    assert!(!block_on(backend.verify_password(password("wrong-horse"), stored)).unwrap());
}

#[test]
fn stored_hash_never_contains_the_password_and_salts_differ() {
    let backend = fast_backend();
    let first = block_on(backend.hash_password(password("shared-password"))).unwrap();
    let second = block_on(backend.hash_password(password("shared-password"))).unwrap();
    assert!(!first.contains("shared-password"));
    assert_ne!(first, second);
    assert!(first.starts_with("v1$") && first.matches('$').count() == 2);
}

#[test]
fn malformed_emails_and_short_passwords_are_rejected() {
    let backend = fast_backend();
    for bad in [
        "",
        "a@b",
        "no-at-sign",
        "double@@at.io",
        "x@nodot",
        "@example.com",
        "user@",
    ] {
        assert!(
            matches!(
                block_on(backend.sign_up(
                    reqwest::Client::new(),
                    bad,
                    &password("password-8"),
                    NOW
                )),
                Err(AuthError::InvalidEmail)
            ),
            "{bad} should be rejected"
        );
    }
    assert!(matches!(
        block_on(backend.sign_up(
            reqwest::Client::new(),
            "ok@example.test",
            &password("short"),
            NOW
        )),
        Err(AuthError::WeakPassword)
    ));
    assert!(matches!(
        block_on(backend.sign_up(
            reqwest::Client::new(),
            "ok@example.test",
            &password(&"x".repeat(129)),
            NOW
        )),
        Err(AuthError::WeakPassword)
    ));
}

#[test]
fn duplicate_sign_up_reports_email_taken() {
    let backend = fast_backend();
    sign_up_user(&backend, "dup@example.test", "password-8");
    assert!(matches!(
        block_on(backend.sign_up(
            reqwest::Client::new(),
            "DUP@example.test",
            &password("password-8"),
            NOW
        )),
        Err(AuthError::EmailTaken)
    ));
}

#[test]
fn fresh_sessions_mint_random_tokens() {
    let first = fast_backend();
    let second = fast_backend();
    sign_up_user(&first, "a@example.test", "password-8");
    sign_up_user(&second, "a@example.test", "password-8");
    let one = sign_in_user(&first, "a@example.test", "password-8", NOW);
    let two = sign_in_user(&second, "a@example.test", "password-8", NOW);
    let first_access = one.tokens.access_token.expose();
    assert!(first_access.starts_with("demo-access-"));
    assert_ne!(two.tokens.access_token.expose(), first_access);
    let first_refresh = one.tokens.refresh_token.unwrap();
    assert!(first_refresh.expose().starts_with("demo-refresh-"));
    assert_ne!(
        first_refresh.expose(),
        two.tokens.refresh_token.unwrap().expose()
    );
    assert_eq!(one.tokens.expires_at, Some(NOW + ACCESS_TTL_SECS));
}

#[test]
fn wrong_password_reports_invalid_credentials() {
    let backend = fast_backend();
    sign_up_user(&backend, "a@example.test", "password-8");
    assert!(matches!(
        block_on(backend.sign_in(
            reqwest::Client::new(),
            "a@example.test",
            &password("password-9"),
            NOW
        )),
        Err(AuthError::InvalidCredentials)
    ));
    assert!(matches!(
        block_on(backend.sign_in(
            reqwest::Client::new(),
            "ghost@example.test",
            &password("password-8"),
            NOW
        )),
        Err(AuthError::InvalidCredentials)
    ));
}

#[test]
fn account_ids_are_stable_per_user_across_sessions() {
    let backend = fast_backend();
    sign_up_user(&backend, "a@example.test", "password-8");
    sign_up_user(&backend, "b@example.test", "password-8");
    let first = sign_in_user(&backend, "a@example.test", "password-8", NOW);
    let second = sign_in_user(&backend, "a@example.test", "password-8", NOW + 10);
    let other = sign_in_user(&backend, "b@example.test", "password-8", NOW);
    assert_ne!(
        first.tokens.access_token.expose(),
        second.tokens.access_token.expose()
    );
    assert_eq!(
        first.tokens.account.as_ref().unwrap().id,
        second.tokens.account.as_ref().unwrap().id
    );
    assert_ne!(
        first.tokens.account.as_ref().unwrap().id,
        other.tokens.account.as_ref().unwrap().id
    );
}

#[test]
fn needs_refresh_flips_past_the_margin_from_returned_expiry() {
    let backend = fast_backend();
    sign_up_user(&backend, "a@example.test", "password-8");
    let session = sign_in_user(&backend, "a@example.test", "password-8", NOW);
    let expires_at = session.tokens.expires_at.unwrap();
    assert!(!session.tokens.needs_refresh(expires_at - 121, 120));
    assert!(session.tokens.needs_refresh(expires_at - 120, 120));
}

#[test]
fn refresh_keeps_the_refresh_token_and_rotates_the_access_token() {
    let backend = fast_backend();
    sign_up_user(&backend, "a@example.test", "password-8");
    let session = sign_in_user(&backend, "a@example.test", "password-8", NOW);
    let first_access = session.tokens.access_token.expose().to_string();
    let refresh = session.tokens.refresh_token.unwrap();
    let account_id = session.tokens.account.as_ref().unwrap().id.clone();

    let refreshed = backend.refresh_impl(refresh.expose(), NOW + 100).unwrap();
    let second_access = refreshed.access_token.expose().to_string();
    assert_ne!(second_access, first_access);
    assert!(second_access.starts_with("demo-access-"));
    assert_eq!(refreshed.refresh_token.unwrap().expose(), refresh.expose());
    assert_eq!(refreshed.expires_at, Some(NOW + 100 + ACCESS_TTL_SECS));
    assert_eq!(refreshed.account.as_ref().unwrap().id, account_id);

    // Revoking the pre-rotation access token is a no-op; the live one kills
    // the session.
    backend.revoke_impl(&first_access);
    let still_alive = backend.refresh_impl(refresh.expose(), NOW + 200).unwrap();
    backend.revoke_impl(still_alive.access_token.expose());
    assert!(matches!(
        backend.refresh_impl(refresh.expose(), NOW + 300),
        Err(AuthError::RefreshRejected)
    ));
}

#[test]
fn expired_sessions_are_rejected_swept_and_purged_from_by_access() {
    let backend = fast_backend();
    sign_up_user(&backend, "a@example.test", "password-8");
    sign_up_user(&backend, "b@example.test", "password-8");
    sign_up_user(&backend, "c@example.test", "password-8");
    let stale = sign_in_user(&backend, "a@example.test", "password-8", NOW);
    let fresh = sign_in_user(&backend, "b@example.test", "password-8", NOW + 1000);
    let abandoned = sign_in_user(&backend, "c@example.test", "password-8", NOW);
    let late = NOW + SESSION_MAX_AGE_SECS;

    assert!(matches!(
        backend.refresh_impl(stale.tokens.refresh_token.unwrap().expose(), late),
        Err(AuthError::RefreshRejected)
    ));

    // Creating a session past the max age sweeps the abandoned one out.
    let swept_in = sign_in_user(&backend, "c@example.test", "password-8", late);
    assert!(matches!(
        backend.refresh_impl(abandoned.tokens.refresh_token.unwrap().expose(), late),
        Err(AuthError::RefreshRejected)
    ));

    // The stale session's access token was purged: revoking it must not
    // touch the still-live session.
    backend.revoke_impl(stale.tokens.access_token.expose());
    let live = backend
        .refresh_impl(fresh.tokens.refresh_token.as_ref().unwrap().expose(), late)
        .unwrap();
    backend.revoke_impl(live.access_token.expose());
    assert!(matches!(
        backend.refresh_impl(fresh.tokens.refresh_token.as_ref().unwrap().expose(), late),
        Err(AuthError::RefreshRejected)
    ));
    assert!(
        backend
            .refresh_impl(swept_in.tokens.refresh_token.unwrap().expose(), late)
            .is_ok()
    );
}

#[test]
fn revoke_then_refresh_is_rejected_and_revoke_is_idempotent() {
    let backend = fast_backend();
    sign_up_user(&backend, "a@example.test", "password-8");
    let session = sign_in_user(&backend, "a@example.test", "password-8", NOW);
    let refresh = session.tokens.refresh_token.unwrap();
    let access = session.tokens.access_token.expose().to_string();
    backend.revoke_impl(&access);
    assert!(matches!(
        backend.refresh_impl(refresh.expose(), NOW),
        Err(AuthError::RefreshRejected)
    ));
    backend.revoke_impl(&access);
    backend.revoke_impl("never-minted");
}

#[test]
fn concurrent_sign_ins_never_panic_and_mint_unique_tokens() {
    let backend = Arc::new(fast_backend());
    sign_up_user(&backend, "a@example.test", "password-8");
    let mut tokens = Vec::new();
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for _ in 0..8 {
            let backend = Arc::clone(&backend);
            handles.push(scope.spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .build()
                    .expect("test runtime");
                let mut minted = Vec::new();
                runtime.block_on(async {
                    for _ in 0..20 {
                        let session = backend
                            .sign_in(
                                reqwest::Client::new(),
                                "a@example.test",
                                &password("password-8"),
                                NOW,
                            )
                            .await
                            .unwrap();
                        minted.push(session.tokens.access_token.expose().to_string());
                    }
                });
                minted
            }));
        }
        for handle in handles {
            tokens.extend(handle.join().unwrap());
        }
    });
    let mut unique = tokens.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(tokens.len(), unique.len());
    assert_eq!(tokens.len(), 160);
}
