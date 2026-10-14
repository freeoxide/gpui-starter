use std::sync::Arc;

use super::*;
use crate::token::SecretString;

const NOW: i64 = 10_000;

fn fast_backend() -> DemoBackend {
    DemoBackend::with_params(argon_params(8, 1, 1))
}

fn password(value: &str) -> SecretString {
    SecretString::new(value)
}

fn sign_up_user(backend: &DemoBackend, email: &str, pwd: &str) {
    backend.sign_up_impl(email, &password(pwd), NOW).unwrap();
}

#[test]
fn hash_round_trips_for_correct_and_wrong_passwords() {
    let backend = fast_backend();
    let stored = backend.hash_password(&password("correct-horse")).unwrap();
    assert!(backend.verify_password(&password("correct-horse"), &stored));
    assert!(!backend.verify_password(&password("wrong-horse"), &stored));
}

#[test]
fn stored_hash_never_contains_the_password_and_salts_differ() {
    let backend = fast_backend();
    let first = backend.hash_password(&password("shared-password")).unwrap();
    let second = backend.hash_password(&password("shared-password")).unwrap();
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
                backend.sign_up_impl(bad, &password("password-8"), NOW),
                Err(AuthError::InvalidEmail)
            ),
            "{bad} should be rejected"
        );
    }
    assert!(matches!(
        backend.sign_up_impl("ok@example.test", &password("short"), NOW),
        Err(AuthError::WeakPassword)
    ));
    assert!(matches!(
        backend.sign_up_impl("ok@example.test", &password(&"x".repeat(129)), NOW),
        Err(AuthError::WeakPassword)
    ));
}

#[test]
fn duplicate_sign_up_reports_email_taken() {
    let backend = fast_backend();
    sign_up_user(&backend, "dup@example.test", "password-8");
    assert!(matches!(
        backend.sign_up_impl("DUP@example.test", &password("password-8"), NOW),
        Err(AuthError::EmailTaken)
    ));
}

#[test]
fn fresh_sessions_mint_deterministic_tokens_across_backends() {
    let first = fast_backend();
    let second = fast_backend();
    sign_up_user(&first, "a@example.test", "password-8");
    sign_up_user(&second, "a@example.test", "password-8");
    let one = first
        .sign_in_impl("a@example.test", &password("password-8"), NOW)
        .unwrap();
    let two = second
        .sign_in_impl("a@example.test", &password("password-8"), NOW)
        .unwrap();
    let SignInOutcome::Authenticated(one) = one else {
        panic!("expected an authenticated session");
    };
    let SignInOutcome::Authenticated(two) = two else {
        panic!("expected an authenticated session");
    };
    assert_eq!(one.tokens.access_token.expose(), "demo-access-0");
    assert_eq!(one.tokens.refresh_token.unwrap().expose(), "demo-refresh-0");
    assert_eq!(one.tokens.expires_at, Some(NOW + ACCESS_TTL_SECS));
    assert_eq!(
        two.tokens.access_token.expose(),
        one.tokens.access_token.expose()
    );
}

#[test]
fn wrong_password_reports_invalid_credentials() {
    let backend = fast_backend();
    sign_up_user(&backend, "a@example.test", "password-8");
    assert!(matches!(
        backend.sign_in_impl("a@example.test", &password("password-9"), NOW),
        Err(AuthError::InvalidCredentials)
    ));
    assert!(matches!(
        backend.sign_in_impl("ghost@example.test", &password("password-8"), NOW),
        Err(AuthError::InvalidCredentials)
    ));
}

#[test]
fn needs_refresh_flips_past_the_margin_from_returned_expiry() {
    let backend = fast_backend();
    sign_up_user(&backend, "a@example.test", "password-8");
    let outcome = backend
        .sign_in_impl("a@example.test", &password("password-8"), NOW)
        .unwrap();
    let SignInOutcome::Authenticated(session) = outcome else {
        panic!("expected an authenticated session");
    };
    let expires_at = session.tokens.expires_at.unwrap();
    assert!(!session.tokens.needs_refresh(expires_at - 121, 120));
    assert!(session.tokens.needs_refresh(expires_at - 120, 120));
}

#[test]
fn refresh_keeps_the_refresh_token_and_rotates_the_access_token() {
    let backend = fast_backend();
    sign_up_user(&backend, "a@example.test", "password-8");
    backend
        .sign_in_impl("a@example.test", &password("password-8"), NOW)
        .unwrap();
    let refreshed = backend.refresh_impl("demo-refresh-0", NOW + 100).unwrap();
    assert_eq!(refreshed.access_token.expose(), "demo-access-1");
    assert_eq!(refreshed.refresh_token.unwrap().expose(), "demo-refresh-0");
    assert_eq!(refreshed.expires_at, Some(NOW + 100 + ACCESS_TTL_SECS));
}

#[test]
fn revoke_then_refresh_is_rejected_and_revoke_is_idempotent() {
    let backend = fast_backend();
    sign_up_user(&backend, "a@example.test", "password-8");
    backend
        .sign_in_impl("a@example.test", &password("password-8"), NOW)
        .unwrap();
    backend.revoke_impl("demo-access-0");
    assert!(matches!(
        backend.refresh_impl("demo-refresh-0", NOW),
        Err(AuthError::RefreshRejected)
    ));
    backend.revoke_impl("demo-access-0");
    backend.revoke_impl("never-minted");
}

#[test]
fn concurrent_sign_ins_never_panic_and_mint_unique_sequences() {
    let backend = Arc::new(fast_backend());
    backend
        .sign_up_impl("a@example.test", &password("password-8"), NOW)
        .unwrap();
    let mut tokens = Vec::new();
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for _ in 0..8 {
            let backend = Arc::clone(&backend);
            handles.push(scope.spawn(move || {
                let mut minted = Vec::new();
                for _ in 0..20 {
                    let outcome = backend
                        .sign_in_impl("a@example.test", &password("password-8"), NOW)
                        .unwrap();
                    let SignInOutcome::Authenticated(session) = outcome else {
                        panic!("expected an authenticated session");
                    };
                    minted.push(session.tokens.access_token.expose().to_string());
                }
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

#[test]
fn rfc6238_vectors_match_the_hand_rolled_generator() {
    let secret = b"12345678901234567890";
    for (unix, expected) in [
        (59, "287082"),
        (1_111_111_109, "081804"),
        (1_234_567_890, "005924"),
        (2_000_000_000, "279037"),
        (20_000_000_000, "353130"),
    ] {
        assert_eq!(totp_code(secret, unix), expected);
    }
}

#[test]
fn base32_encoding_matches_rfc4648_vectors() {
    assert_eq!(base32_encode(b""), "");
    assert_eq!(base32_encode(b"f"), "MY");
    assert_eq!(base32_encode(b"fo"), "MZXQ");
    assert_eq!(base32_encode(b"foo"), "MZXW6");
    assert_eq!(base32_encode(b"foob"), "MZXW6YQ");
    assert_eq!(base32_encode(b"fooba"), "MZXW6YTB");
    assert_eq!(base32_encode(b"foobar"), "MZXW6YTBOI");
}

#[test]
fn totp_enrollment_round_trips_with_a_generated_code() {
    let backend = fast_backend();
    sign_up_user(&backend, "t@example.test", "password-8");
    let enrollment = backend
        .begin_totp_enrollment_impl("t@example.test")
        .unwrap();
    assert_eq!(enrollment.secret_base32.len(), 32);
    assert!(
        enrollment
            .otpauth_url
            .starts_with("otpauth://totp/gpui-starter:t@example.test?")
    );
    assert!(enrollment.otpauth_url.contains("secret="));
    let secret = decode_base32(&enrollment.secret_base32);
    let code = totp_code(&secret, NOW);
    let wrong = wrong_code_for(&secret, NOW);
    assert!(matches!(
        backend.confirm_totp_enrollment_impl("t@example.test", &wrong, NOW),
        Err(AuthError::TotpFailed)
    ));
    backend
        .confirm_totp_enrollment_impl("t@example.test", &code, NOW)
        .unwrap();
    assert!(matches!(
        backend.begin_totp_enrollment_impl("t@example.test"),
        Err(AuthError::InvalidChallenge)
    ));
}

#[test]
fn sign_in_after_enrollment_requires_totp_and_consumes_challenges() {
    let backend = fast_backend();
    sign_up_user(&backend, "t@example.test", "password-8");
    let enrollment = backend
        .begin_totp_enrollment_impl("t@example.test")
        .unwrap();
    let secret = decode_base32(&enrollment.secret_base32);
    backend
        .confirm_totp_enrollment_impl("t@example.test", &totp_code(&secret, NOW), NOW)
        .unwrap();
    let outcome = backend
        .sign_in_impl("t@example.test", &password("password-8"), NOW)
        .unwrap();
    let SignInOutcome::TotpRequired(challenge) = outcome else {
        panic!("expected a TOTP challenge");
    };
    assert_eq!(challenge.email, "t@example.test");

    let wrong = wrong_code_for(&secret, NOW);
    assert!(matches!(
        backend.verify_totp_impl(&challenge, &wrong, NOW),
        Err(AuthError::TotpFailed)
    ));
    let session = backend
        .verify_totp_impl(&challenge, &totp_code(&secret, NOW), NOW)
        .unwrap();
    assert_eq!(session.tokens.access_token.expose(), "demo-access-1");
    assert!(matches!(
        backend.verify_totp_impl(&challenge, &totp_code(&secret, NOW), NOW),
        Err(AuthError::InvalidChallenge)
    ));
}

#[test]
fn expired_challenges_and_unknown_ids_are_invalid() {
    let backend = fast_backend();
    sign_up_user(&backend, "t@example.test", "password-8");
    let enrollment = backend
        .begin_totp_enrollment_impl("t@example.test")
        .unwrap();
    let secret = decode_base32(&enrollment.secret_base32);
    backend
        .confirm_totp_enrollment_impl("t@example.test", &totp_code(&secret, NOW), NOW)
        .unwrap();
    let outcome = backend
        .sign_in_impl("t@example.test", &password("password-8"), NOW)
        .unwrap();
    let SignInOutcome::TotpRequired(challenge) = outcome else {
        panic!("expected a TOTP challenge");
    };
    assert!(matches!(
        backend.verify_totp_impl(
            &challenge,
            &totp_code(&secret, NOW),
            NOW + CHALLENGE_TTL_SECS
        ),
        Err(AuthError::InvalidChallenge)
    ));
    let unknown = TotpChallenge {
        id: "demo-ch-999".to_string(),
        email: "t@example.test".to_string(),
    };
    assert!(matches!(
        backend.verify_totp_impl(&unknown, "000000", NOW),
        Err(AuthError::InvalidChallenge)
    ));
}

#[test]
fn enrollment_requires_an_existing_user_and_pending_round() {
    let backend = fast_backend();
    assert!(matches!(
        backend.begin_totp_enrollment_impl("ghost@example.test"),
        Err(AuthError::InvalidCredentials)
    ));
    sign_up_user(&backend, "t@example.test", "password-8");
    assert!(matches!(
        backend.confirm_totp_enrollment_impl("t@example.test", "123456", NOW),
        Err(AuthError::InvalidChallenge)
    ));
}

fn decode_base32(encoded: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0u32;
    for char in encoded.chars() {
        let value = BASE32_ALPHABET
            .iter()
            .position(|candidate| *candidate as char == char)
            .unwrap_or(usize::MAX);
        assert_ne!(value, usize::MAX, "non-base32 char {char}");
        buffer = (buffer << 5) | value as u32;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(((buffer >> bits) & 0xff) as u8);
        }
    }
    out
}

fn wrong_code_for(secret: &[u8], unix: i64) -> String {
    let valid: Vec<String> = (-1..=1)
        .map(|delta| totp_code(secret, unix + delta * 30))
        .collect();
    (0..1_000_000u32)
        .map(|candidate| format!("{candidate:06}"))
        .find(|candidate| !valid.contains(candidate))
        .unwrap()
}
