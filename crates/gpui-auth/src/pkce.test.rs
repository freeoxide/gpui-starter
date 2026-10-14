use super::*;

const BASE64URL_ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn is_base64url_43(value: &str) -> bool {
    value.len() == 43 && value.chars().all(|c| BASE64URL_ALPHABET.contains(c))
}

#[test]
fn authorize_challenge_matches_the_rfc_7636_appendix_b_vector() {
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    assert_eq!(
        authorize_challenge(verifier),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
}

#[test]
fn generated_pair_is_43_char_base64url() {
    let pair = PkcePair::generate().unwrap();
    assert!(is_base64url_43(pair.expose_verifier()));
    assert!(is_base64url_43(pair.challenge()));
    assert_eq!(
        pair.challenge(),
        authorize_challenge(pair.expose_verifier())
    );
}

#[test]
fn two_generated_pairs_differ() {
    let a = PkcePair::generate().unwrap();
    let b = PkcePair::generate().unwrap();
    assert_ne!(a.expose_verifier(), b.expose_verifier());
    assert_ne!(a.challenge(), b.challenge());
}

#[test]
fn random_states_are_43_char_base64url_and_unique() {
    let mut seen = std::collections::HashSet::new();
    for _ in 0..100 {
        let state = random_state().unwrap();
        assert!(is_base64url_43(state.expose()));
        assert!(seen.insert(state.expose().to_string()));
    }
}
