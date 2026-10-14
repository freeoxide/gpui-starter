use std::io::{Read as _, Write as _};
use std::sync::Arc;
use std::time::Duration;

use gpui_auth::{AuthEvent, AuthIo, AuthState, InMemoryStore, TokenStore};
use gpui_kit::{AppContext as _, Entity, Subscription, TestAppContext};

use super::*;
use crate::services::auth_social::providers::fake_idp::FakeIdP;
use crate::services::auth_social::{OidcConfig, ProviderConfig, SocialConfig};
use crate::services::session::SessionState;

fn scratch_runtime() -> Arc<tokio::runtime::Runtime> {
    Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("scratch runtime"),
    )
}

fn base(cx: &mut TestAppContext) {
    // The real providers finish on tokio worker threads; the deterministic
    // scheduler otherwise rejects those wakes (gpui executor.rs allow_parking).
    cx.executor().allow_parking();
    cx.update(|cx| {
        crate::capabilities::initialize(cx);
        crate::services::session::initialize(cx);
        crate::services::secure_storage::initialize(cx);
    });
}

fn fake_io(rt: &Arc<tokio::runtime::Runtime>) -> AuthIo {
    AuthIo {
        http: reqwest::Client::new(),
        runtime: Some(rt.clone()),
    }
}

fn oidc_config_for(origin: &str, redirect: Option<&str>) -> SocialConfig {
    SocialConfig {
        google: None,
        github: None,
        oidc: Some(OidcConfig {
            issuer: origin.to_string(),
            revoke_url: None,
            base: ProviderConfig {
                client_id: "oidc-client".to_string(),
                client_secret: None,
                redirect: redirect.map(str::to_string),
                scopes: None,
            },
        }),
    }
}

fn sync_http_get(url: &str) -> Option<String> {
    let mut stream = std::net::TcpStream::connect(url_socket_addr(url)?).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    let request = format!("GET {url} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).ok()?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response).ok()?;
    Some(String::from_utf8_lossy(&response).to_string())
}

fn url_socket_addr(url: &str) -> Option<std::net::SocketAddr> {
    let rest = url.strip_prefix("http://")?;
    let (authority, _) = rest.split_once('/')?;
    let (host, port) = authority.rsplit_once(':')?;
    format!("{host}:{port}").parse().ok()
}

fn capture_authorization_urls(
    cx: &mut TestAppContext,
    controller: &Entity<AuthController>,
) -> (std::rc::Rc<std::cell::RefCell<Vec<String>>>, Subscription) {
    let urls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let sink = urls.clone();
    let subscription = cx.update(|cx| {
        cx.subscribe(controller, move |_, event: &AuthEvent, _| {
            if let AuthEvent::AuthorizationRequested { url } = event {
                sink.borrow_mut().push(url.clone());
            }
        })
    });
    (urls, subscription)
}

fn wait_until(cx: &mut TestAppContext, mut condition: impl FnMut(&mut TestAppContext) -> bool) {
    for _ in 0..300 {
        cx.run_until_parked();
        if condition(cx) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("condition not met before the deadline");
}

fn loopback_redirect_url(authorize_url: &str) -> String {
    url::Url::parse(authorize_url)
        .expect("authorize url parses")
        .query_pairs()
        .find(|(name, _)| name == "redirect_uri")
        .map(|(_, value)| value.to_string())
        .expect("redirect_uri param")
}

#[gpui_kit::test]
async fn empty_config_leaves_every_provider_unconfigured(cx: &mut TestAppContext) {
    base(cx);
    cx.update(|cx| super::install_with(SocialConfig::default(), Vec::new(), None, None, cx));
    let snapshot = cx.read(|cx| super::super::snapshot(cx));
    assert_eq!(snapshot.providers.len(), 3);
    assert!(!snapshot.any_configured());
    assert_eq!(snapshot.session_owner(), None);
    for (kind, status) in &snapshot.providers {
        assert!(!status.configured, "{}", kind.id());
        assert_eq!(status.state, None);
    }
    let capabilities = cx.read(|cx| crate::capabilities::snapshot(cx));
    let aggregate = capabilities.get("auth_social").expect("aggregate entry");
    assert!(!aggregate.enabled && aggregate.degraded);
    assert!(
        aggregate
            .reason
            .as_ref()
            .is_some_and(|reason| reason.contains("no providers configured"))
    );
    let provider_row = capabilities.get("auth_social_oidc").expect("oidc row");
    assert!(
        provider_row
            .reason
            .as_ref()
            .is_some_and(|reason| reason.contains("not configured"))
    );
}

#[gpui_kit::test]
async fn configured_providers_get_controllers_and_degraded_rows(cx: &mut TestAppContext) {
    base(cx);
    let config = SocialConfig {
        google: Some(ProviderConfig {
            client_id: "google-id".to_string(),
            client_secret: None,
            redirect: None,
            scopes: None,
        }),
        github: Some(ProviderConfig {
            client_id: "gh-id".to_string(),
            client_secret: None,
            redirect: None,
            scopes: None,
        }),
        oidc: None,
    };
    cx.update(|cx| super::install_with(config, Vec::new(), None, None, cx));
    let snapshot = cx.read(|cx| super::super::snapshot(cx));
    assert!(snapshot.any_configured());
    assert_eq!(
        snapshot
            .providers
            .iter()
            .filter(|(_, status)| status.configured)
            .count(),
        2
    );
    cx.update(|cx| {
        assert!(super::super::controller(ProviderKind::Google, cx).is_some());
        assert!(super::super::controller(ProviderKind::GitHub, cx).is_some());
        assert!(super::super::controller(ProviderKind::Oidc, cx).is_none());
    });
    let capabilities = cx.read(|cx| crate::capabilities::snapshot(cx));
    let aggregate = capabilities.get("auth_social").expect("aggregate");
    assert!(aggregate.enabled && !aggregate.degraded);
    let google_row = capabilities.get("auth_social_google").expect("google row");
    assert!(google_row.degraded);
    assert!(
        google_row
            .reason
            .as_ref()
            .is_some_and(|reason| reason.contains("configured"))
    );
}

#[gpui_kit::test]
async fn start_on_an_unconfigured_provider_is_a_logged_no_op(cx: &mut TestAppContext) {
    base(cx);
    cx.update(|cx| super::install_with(SocialConfig::default(), Vec::new(), None, None, cx));
    cx.update(|cx| super::super::start(ProviderKind::Google, cx));
    let snapshot = cx.read(|cx| super::super::snapshot(cx));
    assert_eq!(snapshot.providers[0].1.state, None);
}

#[gpui_kit::test]
async fn full_loopback_flow_through_the_real_controller(cx: &mut TestAppContext) {
    base(cx);
    let rt = scratch_runtime();
    let fake = FakeIdP::spawn(rt.handle());
    let store = Arc::new(InMemoryStore::new());
    let shared_store: Arc<dyn TokenStore> = store.clone();
    let config = oidc_config_for(&fake.origin(), None);
    cx.update(|cx| {
        super::install_with(
            config,
            Vec::new(),
            Some(shared_store),
            Some(fake_io(&rt)),
            cx,
        );
    });
    let controller = cx
        .update(|cx| super::super::controller(ProviderKind::Oidc, cx))
        .expect("oidc controller");
    let (urls, _subscription) = capture_authorization_urls(cx, &controller);

    cx.update(|cx| super::super::start(ProviderKind::Oidc, cx));
    wait_until(cx, |_| !urls.borrow().is_empty());
    let authorize_url = urls.borrow()[0].clone();
    assert!(authorize_url.starts_with(&format!("{}/authorize", fake.origin())));

    let redirect_base = loopback_redirect_url(&authorize_url);
    let state = authorize_url
        .split("state=")
        .nth(1)
        .and_then(|rest| rest.split('&').next())
        .expect("state param")
        .to_string();
    let response = sync_http_get(&format!("{redirect_base}?code=fake-code&state={state}"))
        .expect("loopback capture");
    assert!(response.contains("200 OK"));
    assert!(response.contains("close this tab"));
    wait_until(cx, |cx| {
        matches!(
            cx.read(|cx| controller.read(cx).state().clone()),
            AuthState::SignedIn { .. }
        )
    });

    let controller_state = cx.read(|cx| controller.read(cx).state().clone());
    assert!(
        matches!(controller_state, AuthState::SignedIn { .. }),
        "{controller_state:?}"
    );
    let stored = cx.update(|cx| store.load("oidc", cx).unwrap());
    assert!(stored.is_some());
    let session = cx.read(|cx| crate::services::session::snapshot(cx).state);
    assert_eq!(
        session,
        SessionState::SignedIn {
            account_label: "Fake User".to_string()
        }
    );
    let snapshot = cx.read(|cx| super::super::snapshot(cx));
    assert_eq!(snapshot.session_owner(), Some(ProviderKind::Oidc));
    let capabilities = cx.read(|cx| crate::capabilities::snapshot(cx));
    let oidc_row = capabilities.get("auth_social_oidc").expect("oidc row");
    assert!(oidc_row.supported && oidc_row.enabled && !oidc_row.degraded);
}

#[gpui_kit::test]
async fn wrong_state_redirect_errors_without_a_store_row(cx: &mut TestAppContext) {
    base(cx);
    let rt = scratch_runtime();
    let fake = FakeIdP::spawn(rt.handle());
    let store = Arc::new(InMemoryStore::new());
    let shared_store: Arc<dyn TokenStore> = store.clone();
    let config = oidc_config_for(&fake.origin(), None);
    cx.update(|cx| {
        super::install_with(
            config,
            Vec::new(),
            Some(shared_store),
            Some(fake_io(&rt)),
            cx,
        )
    });
    let controller = cx
        .update(|cx| super::super::controller(ProviderKind::Oidc, cx))
        .expect("oidc controller");
    let (urls, _subscription) = capture_authorization_urls(cx, &controller);

    cx.update(|cx| super::super::start(ProviderKind::Oidc, cx));
    wait_until(cx, |_| !urls.borrow().is_empty());
    let redirect_base = loopback_redirect_url(&urls.borrow()[0]);
    sync_http_get(&format!("{redirect_base}?code=fake-code&state=bogus"))
        .expect("loopback capture still answers");
    wait_until(cx, |cx| {
        matches!(
            cx.read(|cx| controller.read(cx).state().clone()),
            AuthState::Error(_)
        )
    });

    let controller_state = cx.read(|cx| controller.read(cx).state().clone());
    assert!(
        matches!(
            controller_state,
            AuthState::Error(ref message) if message.contains("state mismatch")
        ),
        "got: {controller_state:?}"
    );
    let stored = cx.update(|cx| store.load("oidc", cx).unwrap());
    assert_eq!(stored, None);
}

#[gpui_kit::test]
async fn scheme_mode_redirect_uri_is_the_deep_link(cx: &mut TestAppContext) {
    base(cx);
    let rt = scratch_runtime();
    let fake = FakeIdP::spawn(rt.handle());
    let config = oidc_config_for(&fake.origin(), Some("scheme"));
    cx.update(|cx| super::install_with(config, Vec::new(), None, Some(fake_io(&rt)), cx));
    let controller = cx
        .update(|cx| super::super::controller(ProviderKind::Oidc, cx))
        .expect("oidc controller");
    let (urls, _subscription) = capture_authorization_urls(cx, &controller);

    cx.update(|cx| super::super::start(ProviderKind::Oidc, cx));
    wait_until(cx, |_| !urls.borrow().is_empty());
    let authorize_url = urls.borrow()[0].clone();
    assert!(authorize_url.contains("redirect_uri=gpui-starter%3A%2F%2Fauth%2Fcallback"));

    cx.update(|cx| {
        assert!(super::super::route_callback(
            "gpui-starter://auth/callback?code=scheme-code&state=wrong",
            cx
        ))
    });
    wait_until(cx, |cx| {
        matches!(
            cx.read(|cx| controller.read(cx).state().clone()),
            AuthState::Error(_)
        )
    });
    let controller_state = cx.read(|cx| controller.read(cx).state().clone());
    assert!(
        matches!(
            controller_state,
            AuthState::Error(ref message) if message.contains("state mismatch")
        ),
        "{controller_state:?}"
    );
}

#[gpui_kit::test]
async fn route_callback_truth_table(cx: &mut TestAppContext) {
    base(cx);
    cx.update(|cx| super::install_with(SocialConfig::default(), Vec::new(), None, None, cx));
    cx.update(|cx| {
        assert!(!super::super::route_callback(
            "gpui-starter://auth/callback?code=x&state=y",
            cx
        ));
        assert!(!super::super::route_callback("gpui-starter://settings", cx));
        assert!(!super::super::route_callback("not-a-url://x", cx));
    });
}

#[gpui_kit::test]
async fn restarting_a_flow_cancels_the_previous_one(cx: &mut TestAppContext) {
    base(cx);
    let rt = scratch_runtime();
    let fake = FakeIdP::spawn(rt.handle());
    let config = oidc_config_for(&fake.origin(), None);
    cx.update(|cx| super::install_with(config, Vec::new(), None, Some(fake_io(&rt)), cx));
    let controller = cx
        .update(|cx| super::super::controller(ProviderKind::Oidc, cx))
        .expect("oidc controller");
    let (urls, _subscription) = capture_authorization_urls(cx, &controller);

    cx.update(|cx| super::super::start(ProviderKind::Oidc, cx));
    wait_until(cx, |_| !urls.borrow().is_empty());
    assert!(matches!(
        cx.read(|cx| controller.read(cx).state().clone()),
        AuthState::SigningIn
    ));
    let first_url = urls.borrow()[0].clone();

    cx.update(|cx| super::super::start(ProviderKind::Oidc, cx));
    wait_until(cx, |_| urls.borrow().len() >= 2);
    assert!(matches!(
        cx.read(|cx| controller.read(cx).state().clone()),
        AuthState::SigningIn
    ));
    assert_eq!(urls.borrow().len(), 2);
    assert_ne!(urls.borrow()[1], first_url);
    assert_ne!(
        loopback_redirect_url(&urls.borrow()[1]),
        loopback_redirect_url(&first_url),
        "the replacement flow binds its own listener"
    );
}

#[gpui_kit::test]
async fn flow_timeout_cancels_the_controller(cx: &mut TestAppContext) {
    base(cx);
    let rt = scratch_runtime();
    let fake = FakeIdP::spawn(rt.handle());
    let config = oidc_config_for(&fake.origin(), None);
    cx.update(|cx| {
        super::install_with(config, Vec::new(), None, Some(fake_io(&rt)), cx);
        let handle = cx
            .try_global::<SocialFlowsHandle>()
            .expect("flows handle")
            .0
            .clone();
        handle.update(cx, |flows, _cx| {
            flows.set_flow_timeout(Duration::from_millis(50))
        });
    });
    let controller = cx
        .update(|cx| super::super::controller(ProviderKind::Oidc, cx))
        .expect("oidc controller");

    cx.update(|cx| super::super::start(ProviderKind::Oidc, cx));
    wait_until(cx, |cx| {
        matches!(
            cx.read(|cx| controller.read(cx).state().clone()),
            AuthState::SignedOut
        )
    });
}

#[gpui_kit::test]
async fn production_install_resolves_the_shared_tokio_runtime(cx: &mut TestAppContext) {
    let _guard = super::super::ENV_OVERRIDE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    base(cx);
    let runtime = scratch_runtime();
    let fake = FakeIdP::spawn(runtime.handle());
    let dir = tempfile::TempDir::new().expect("tempdir");
    let config_path = dir.path().join("auth-providers.json");
    std::fs::write(
        &config_path,
        format!(
            r#"{{ "oidc": {{ "issuer": "{origin}", "client_id": "oidc-client" }} }}"#,
            origin = fake.origin()
        ),
    )
    .expect("write provider config");

    cx.update(|cx| {
        cx.set_global(crate::services::tokio_runtime::TokioRuntimeGlobal(
            crate::services::tokio_runtime::TokioRuntime {
                runtime: runtime.clone(),
                http_client: reqwest::Client::new(),
            },
        ));
    });
    let previous = std::env::var_os("GPUI_AUTH_PROVIDERS");
    unsafe { std::env::set_var("GPUI_AUTH_PROVIDERS", &config_path) };
    cx.update(|cx| super::super::install(cx));
    unsafe {
        match previous {
            Some(value) => std::env::set_var("GPUI_AUTH_PROVIDERS", value),
            None => std::env::remove_var("GPUI_AUTH_PROVIDERS"),
        }
    }

    let controller = cx
        .update(|cx| super::super::controller(ProviderKind::Oidc, cx))
        .expect("oidc controller from production install");
    let (urls, _subscription) = capture_authorization_urls(cx, &controller);
    cx.update(|cx| super::super::start(ProviderKind::Oidc, cx));
    wait_until(cx, |_| !urls.borrow().is_empty());
    assert!(urls.borrow()[0].starts_with(&format!("{}/authorize", fake.origin())));

    let redirect_base = loopback_redirect_url(&urls.borrow()[0]);
    let state = urls.borrow()[0]
        .split("state=")
        .nth(1)
        .and_then(|rest| rest.split('&').next())
        .expect("state param")
        .to_string();
    let response = sync_http_get(&format!("{redirect_base}?code=fake-code&state={state}"))
        .expect("loopback capture through the shared runtime");
    assert!(response.contains("close this tab"));
    wait_until(cx, |cx| {
        !matches!(
            cx.read(|cx| controller.read(cx).state().clone()),
            AuthState::SigningIn
        )
    });
    if matches!(
        cx.read(|cx| controller.read(cx).state().clone()),
        AuthState::SignedIn { .. }
    ) {
        cx.update(|cx| super::super::sign_out(ProviderKind::Oidc, cx));
        cx.run_until_parked();
    }
}

#[test]
fn open_failure_detail_omits_the_authorize_url() {
    use crate::services::desktop_actions::DesktopActionError;
    let error = DesktopActionError::OpenUrlFailed {
        url: "https://auth.example.test/authorize?state=secret-state-value".to_string(),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "xdg-open command not found"),
    };
    let detail = super::open_failure_detail(&error);
    assert_eq!(detail, "xdg-open command not found");
    assert!(!detail.contains("auth.example.test"));
    assert!(!detail.contains("secret-state-value"));
}

#[gpui_kit::test]
async fn config_warnings_replace_the_aggregate_capability_reason(cx: &mut TestAppContext) {
    base(cx);
    cx.update(|cx| {
        super::install_with(
            SocialConfig::default(),
            vec!["/tmp/auth-providers.json is not valid JSON (expected value); social login stays unconfigured".to_string()],
            None,
            None,
            cx,
        );
    });
    let capabilities = cx.read(|cx| crate::capabilities::snapshot(cx));
    let aggregate = capabilities.get("auth_social").expect("aggregate entry");
    assert!(!aggregate.enabled && aggregate.degraded);
    assert!(
        aggregate
            .reason
            .as_ref()
            .is_some_and(|reason| reason.contains("not valid JSON"))
    );
    assert!(
        !aggregate.reason.as_ref().is_some_and(
            |reason| reason.contains("see auth-providers.json in the config directory")
        )
    );
}

#[gpui_kit::test]
async fn production_install_carries_a_broken_config_into_the_reason(cx: &mut TestAppContext) {
    let _guard = super::super::ENV_OVERRIDE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    base(cx);
    let dir = tempfile::TempDir::new().expect("tempdir");
    let config_path = dir.path().join("auth-providers.json");
    std::fs::write(&config_path, "{ not json").expect("write broken config");
    let previous = std::env::var_os("GPUI_AUTH_PROVIDERS");
    unsafe { std::env::set_var("GPUI_AUTH_PROVIDERS", &config_path) };
    cx.update(|cx| super::super::install(cx));
    unsafe {
        match previous {
            Some(value) => std::env::set_var("GPUI_AUTH_PROVIDERS", value),
            None => std::env::remove_var("GPUI_AUTH_PROVIDERS"),
        }
    }
    let capabilities = cx.read(|cx| crate::capabilities::snapshot(cx));
    let aggregate = capabilities.get("auth_social").expect("aggregate entry");
    assert!(
        aggregate
            .reason
            .as_ref()
            .is_some_and(|reason| reason.contains("not valid JSON"))
    );
}
