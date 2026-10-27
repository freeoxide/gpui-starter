use gpui_kit::TestAppContext;

use super::{ProviderKind, is_available, providers};

#[gpui_kit::test]
async fn registry_is_static_with_email_first(cx: &mut TestAppContext) {
    let all = providers();
    assert_eq!(all.len(), 3);
    let ids: Vec<&str> = all.iter().map(|spec| spec.id).collect();
    assert_eq!(ids, vec!["email", "google", "github"]);
    assert!(matches!(all[0].kind, ProviderKind::Email));
    assert!(matches!(all[1].kind, ProviderKind::Browser));
    assert!(matches!(all[2].kind, ProviderKind::Browser));
}

#[gpui_kit::test]
async fn email_row_follows_the_capability_entry(cx: &mut TestAppContext) {
    cx.update(|cx| crate::capabilities::initialize(cx));
    let email = &providers()[0];
    cx.update(|cx| {
        crate::capabilities::set(
            "auth_email",
            crate::capabilities::CapabilityStatus::supported_enabled(),
            cx,
        )
    });
    cx.read(|cx| assert!(is_available(email, cx)));

    cx.update(|cx| {
        crate::capabilities::set(
            "auth_email",
            crate::capabilities::CapabilityStatus {
                supported: true,
                enabled: false,
                degraded: false,
                reason: None,
                last_error: None,
            },
            cx,
        )
    });
    cx.read(|cx| assert!(!is_available(email, cx)));
}

#[gpui_kit::test]
async fn browser_rows_stay_unavailable_without_social_config(cx: &mut TestAppContext) {
    cx.update(|cx| crate::capabilities::initialize(cx));
    let google = &providers()[1];
    let github = &providers()[2];
    cx.update(|cx| {
        crate::capabilities::set(
            "auth_social",
            crate::capabilities::CapabilityStatus::supported_enabled(),
            cx,
        )
    });
    cx.read(|cx| {
        assert!(!is_available(google, cx));
        assert!(!is_available(github, cx));
    });
}

#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
async fn browser_row_lights_up_for_its_configured_provider(cx: &mut TestAppContext) {
    use std::sync::Arc;

    use gpui_auth::InMemoryStore;

    cx.update(|cx| {
        crate::capabilities::initialize(cx);
        crate::services::session::initialize(cx);
    });
    let config = crate::services::auth_social::SocialConfig {
        google: Some(crate::services::auth_social::ProviderConfig {
            client_id: "google-client".to_string(),
            client_secret: None,
            redirect: None,
            scopes: None,
        }),
        github: None,
        oidc: None,
    };
    cx.update(|cx| {
        crate::services::auth_social::flows::install_with(
            config,
            Vec::new(),
            Some(Arc::new(InMemoryStore::new())),
            None,
            cx,
        );
    });
    cx.run_until_parked();
    let google = &providers()[1];
    let github = &providers()[2];
    cx.read(|cx| {
        assert!(is_available(google, cx));
        assert!(!is_available(github, cx));
    });
}
