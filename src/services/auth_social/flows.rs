use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use gpui_auth::{AuthController, AuthEvent, AuthIo, AuthProvider, AuthState, TokenStore};
use gpui_kit::{App, AppContext as _, Context, Entity, Global, Subscription};

use crate::services::auth_social::config::SocialConfig;
use crate::services::auth_social::providers::github::GitHubProvider;
use crate::services::auth_social::providers::oidc::OidcProvider;
use crate::services::auth_social::{ProviderKind, RedirectMode, SocialProviderStatus};
use crate::services::session::SessionState;

const FLOW_TIMEOUT: Duration = Duration::from_secs(300);
const NOT_CONFIGURED: &str = "not configured: set client_id in auth-providers.json";
const CONFIGURED_NOT_CONNECTED: &str = "configured — not connected";
const NONE_CONFIGURED: &str =
    "no providers configured; see auth-providers.json in the config directory";

struct ActiveFlow {
    kind: ProviderKind,
    _task: Option<gpui_kit::Task<()>>,
    _abort: Option<tokio::task::AbortHandle>,
}

pub(crate) struct SocialFlows {
    controllers: BTreeMap<ProviderKind, Entity<AuthController>>,
    google: Option<Arc<OidcProvider>>,
    github: Option<Arc<GitHubProvider>>,
    generic: Option<Arc<OidcProvider>>,
    redirect_modes: BTreeMap<ProviderKind, RedirectMode>,
    config: SocialConfig,
    config_warnings: Vec<String>,
    runtime: Option<Arc<tokio::runtime::Runtime>>,
    active_flow: Option<ActiveFlow>,
    session_owner: Option<ProviderKind>,
    flow_timeout: Duration,
    _subscriptions: Vec<Subscription>,
}

pub(crate) struct SocialFlowsHandle(pub(crate) Entity<SocialFlows>);

impl Global for SocialFlowsHandle {}

pub(crate) fn install_with(
    config: SocialConfig,
    config_warnings: Vec<String>,
    store: Option<Arc<dyn TokenStore>>,
    io: Option<AuthIo>,
    cx: &mut App,
) {
    let io = io.unwrap_or_else(fallback_io);
    let store = store.unwrap_or_else(|| Arc::new(crate::services::auth::KeyringStore));
    let mut controllers = BTreeMap::new();
    let mut redirect_modes = BTreeMap::new();
    let mut google = None;
    let mut github = None;
    let mut generic = None;

    for kind in ProviderKind::ALL {
        let redirect = crate::services::auth_social::config::effective_redirect(kind, &config);
        let provider_config = crate::services::auth_social::config_for_kind(&config, kind);
        let Some(provider_config) = provider_config else {
            continue;
        };
        let dyn_provider: Arc<dyn AuthProvider> = match kind {
            ProviderKind::Google => {
                let provider = Arc::new(OidcProvider::google(provider_config, redirect));
                google = Some(provider.clone());
                provider
            }
            ProviderKind::GitHub => {
                let provider = Arc::new(GitHubProvider::new(provider_config, redirect));
                github = Some(provider.clone());
                provider
            }
            ProviderKind::Oidc => {
                let oidc_config = config
                    .oidc
                    .as_ref()
                    .expect("config_for_kind returned Some for oidc");
                let provider = Arc::new(OidcProvider::generic(oidc_config, redirect));
                generic = Some(provider.clone());
                provider
            }
        };
        redirect_modes.insert(kind, redirect);
        controllers.insert(
            kind,
            cx.new(|cx| AuthController::new(dyn_provider, store.clone(), io.clone(), cx)),
        );
    }

    let runtime = io.runtime.clone();
    let entity = cx.new(|cx| {
        let mut subscriptions = Vec::new();
        for (kind, controller) in controllers.clone() {
            subscriptions.push(cx.subscribe(
                &controller,
                move |this: &mut SocialFlows,
                      _: Entity<AuthController>,
                      event: &AuthEvent,
                      cx: &mut Context<SocialFlows>| {
                    this.on_controller_event(kind, event, cx);
                },
            ));
            subscriptions.push(cx.observe(&controller, move |this, _, cx| {
                this.sync_session(cx);
                this.refresh_capabilities(cx);
            }));
        }
        SocialFlows {
            controllers,
            google,
            github,
            generic,
            redirect_modes,
            config,
            config_warnings,
            runtime,
            active_flow: None,
            session_owner: None,
            flow_timeout: FLOW_TIMEOUT,
            _subscriptions: subscriptions,
        }
    });
    cx.set_global(SocialFlowsHandle(entity.clone()));
    let restore_list: Vec<_> = entity.read(cx).controllers.values().cloned().collect();
    for controller in restore_list {
        controller.update(cx, |controller, cx| controller.restore(cx));
    }
    entity.update(cx, |flows, cx| {
        flows.sync_session(cx);
        flows.refresh_capabilities(cx);
    });
}

pub(crate) fn fallback_io() -> AuthIo {
    tracing::warn!(
        target: "gpui_starter::auth_social",
        "tokio runtime global absent; loopback sign-in flows cannot bind a listener"
    );
    AuthIo {
        http: reqwest::Client::new(),
        runtime: None,
    }
}

impl SocialFlows {
    pub(crate) fn start(&mut self, kind: ProviderKind, cx: &mut Context<Self>) {
        let Some(controller) = self.controllers.get(&kind).cloned() else {
            tracing::warn!(
                target: "gpui_starter::auth_social",
                provider = kind.id(),
                "sign-in requested for an unconfigured provider"
            );
            return;
        };
        if let Some(active) = self.active_flow.take() {
            if let Some(previous) = self.controllers.get(&active.kind) {
                previous.update(cx, |previous, cx| previous.cancel(cx));
            }
            if let Some(abort) = active._abort {
                abort.abort();
            }
        }
        self.active_flow = Some(ActiveFlow {
            kind,
            _task: None,
            _abort: None,
        });
        controller.update(cx, |controller, cx| controller.start_sign_in(cx));
    }

    pub(crate) fn sign_out(&mut self, kind: ProviderKind, cx: &mut Context<Self>) {
        if let Some(controller) = self.controllers.get(&kind) {
            controller.update(cx, |controller, cx| controller.sign_out(cx));
        }
    }

    pub(crate) fn snapshot(&self, cx: &App) -> super::SocialAuthSnapshot {
        super::SocialAuthSnapshot {
            providers: ProviderKind::ALL
                .iter()
                .map(|kind| {
                    (
                        *kind,
                        SocialProviderStatus {
                            configured: self.config_for(kind).is_some(),
                            redirect: self
                                .redirect_modes
                                .get(kind)
                                .copied()
                                .unwrap_or(RedirectMode::Loopback),
                            state: self
                                .controllers
                                .get(kind)
                                .map(|controller| controller.read(cx).state().clone()),
                        },
                    )
                })
                .collect(),
            owner: self.session_owner,
        }
    }

    fn config_for(
        &self,
        kind: &ProviderKind,
    ) -> Option<&crate::services::auth_social::ProviderConfig> {
        crate::services::auth_social::config_for_kind(&self.config, *kind)
    }

    #[cfg(test)]
    pub(crate) fn set_flow_timeout(&mut self, timeout: Duration) {
        self.flow_timeout = timeout;
    }

    fn on_controller_event(
        &mut self,
        kind: ProviderKind,
        event: &AuthEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            AuthEvent::AuthorizationRequested { url } => {
                let flow_is_active = self
                    .active_flow
                    .as_ref()
                    .is_some_and(|active| active.kind == kind);
                if !flow_is_active {
                    return;
                }
                if let Err(err) = crate::services::desktop_actions::open_url(url, cx) {
                    tracing::warn!(
                        target: "gpui_starter::auth_social",
                        provider = kind.id(),
                        error = %open_failure_detail(&err),
                        "failed to open authorization URL"
                    );
                }
                if self.redirect_modes.get(&kind) == Some(&RedirectMode::Loopback) {
                    if let Some((receiver, abort)) = self.take_pending_redirect(kind) {
                        self.spawn_drain(kind, receiver, abort, cx);
                    }
                }
            }
            AuthEvent::FlowCompleted => {
                self.session_owner = Some(kind);
                self.sync_session(cx);
                self.refresh_capabilities(cx);
            }
            AuthEvent::FlowFailed { .. } => {
                self.active_flow = None;
                self.sync_session(cx);
                self.refresh_capabilities(cx);
            }
            AuthEvent::SignedOut => {
                if self.session_owner == Some(kind) {
                    self.session_owner = None;
                }
                self.sync_session(cx);
                self.refresh_capabilities(cx);
            }
            AuthEvent::AccountCreated { .. } => {}
        }
    }

    fn take_pending_redirect(
        &self,
        kind: ProviderKind,
    ) -> Option<(
        tokio::sync::oneshot::Receiver<String>,
        tokio::task::AbortHandle,
    )> {
        match kind {
            ProviderKind::Google => self.google.as_ref()?.take_pending_redirect(),
            ProviderKind::GitHub => self.github.as_ref()?.take_pending_redirect(),
            ProviderKind::Oidc => self.generic.as_ref()?.take_pending_redirect(),
        }
    }

    fn spawn_drain(
        &mut self,
        kind: ProviderKind,
        receiver: tokio::sync::oneshot::Receiver<String>,
        abort: tokio::task::AbortHandle,
        cx: &mut Context<Self>,
    ) {
        let Some(runtime) = self.runtime.clone() else {
            abort.abort();
            return;
        };
        let timeout = self.flow_timeout;
        let task = cx.spawn(async move |this, cx| {
            let joined = runtime.spawn(async move {
                match tokio::time::timeout(timeout, receiver).await {
                    Ok(Ok(raw_url)) => Some(raw_url),
                    Ok(Err(_)) | Err(_) => None,
                }
            });
            match joined.await {
                Ok(Some(raw_url)) => {
                    let _ = this.update(cx, |flows, cx| {
                        flows.active_flow = None;
                        flows.deliver_redirect(kind, &raw_url, cx);
                    });
                }
                Ok(None) => {
                    let _ = this.update(cx, |flows, cx| flows.timeout_flow(kind, cx));
                }
                Err(_join_error) => {}
            }
        });
        self.active_flow = Some(ActiveFlow {
            kind,
            _task: Some(task),
            _abort: Some(abort),
        });
    }

    fn deliver_redirect(&mut self, kind: ProviderKind, raw_url: &str, cx: &mut Context<Self>) {
        if let Some(controller) = self.controllers.get(&kind) {
            controller.update(cx, |controller, cx| {
                controller.complete_callback(raw_url, cx)
            });
        }
    }

    fn timeout_flow(&mut self, kind: ProviderKind, cx: &mut Context<Self>) {
        tracing::warn!(
            target: "gpui_starter::auth_social",
            provider = kind.id(),
            "sign-in flow timed out without a browser redirect"
        );
        self.active_flow = None;
        if let Some(controller) = self.controllers.get(&kind) {
            controller.update(cx, |controller, cx| controller.cancel(cx));
        }
    }

    fn sync_session(&mut self, cx: &mut Context<Self>) {
        if let Some(owner) = self.session_owner
            && !self.is_signed_in(owner, cx)
        {
            self.session_owner = None;
        }
        if self.session_owner.is_none() {
            self.session_owner = ProviderKind::ALL
                .into_iter()
                .find(|kind| self.is_signed_in(*kind, cx));
        }
        let state = match self.session_owner {
            Some(owner) => map_auth_state(self.controllers[&owner].read(cx).state()),
            None => SessionState::SignedOut,
        };
        crate::services::session::set_state(state, cx);
    }

    fn is_signed_in(&self, kind: ProviderKind, cx: &Context<Self>) -> bool {
        self.controllers.get(&kind).is_some_and(|controller| {
            matches!(controller.read(cx).state(), AuthState::SignedIn { .. })
        })
    }

    fn refresh_capabilities(&self, cx: &mut Context<Self>) {
        let mut any_configured = false;
        for kind in ProviderKind::ALL {
            let configured = self.config_for(&kind).is_some();
            any_configured |= configured;
            let signed_in = self.is_signed_in(kind, cx);
            let status = if !configured {
                crate::capabilities::CapabilityStatus::error(NOT_CONFIGURED)
            } else if signed_in {
                crate::capabilities::CapabilityStatus::supported_enabled()
            } else {
                crate::capabilities::CapabilityStatus::degraded(CONFIGURED_NOT_CONNECTED)
            };
            crate::capabilities::set(format!("auth_social_{}", kind.id()), status, cx);
        }
        let aggregate = if any_configured {
            crate::capabilities::CapabilityStatus::supported_enabled()
        } else if !self.config_warnings.is_empty() {
            crate::capabilities::CapabilityStatus::error(self.config_warnings.join("; "))
        } else {
            crate::capabilities::CapabilityStatus::error(NONE_CONFIGURED)
        };
        crate::capabilities::set("auth_social", aggregate, cx);
    }

    pub(crate) fn controller(&self, kind: ProviderKind) -> Option<Entity<AuthController>> {
        self.controllers.get(&kind).cloned()
    }

    pub(crate) fn route_callback(&mut self, link: &str, cx: &mut Context<Self>) -> bool {
        let Some(active) = self.active_flow.take() else {
            return false;
        };
        let Some(controller) = self.controllers.get(&active.kind) else {
            return false;
        };
        controller.update(cx, |controller, cx| controller.complete_callback(link, cx));
        true
    }
}

fn open_failure_detail(error: &crate::services::desktop_actions::DesktopActionError) -> String {
    match error {
        crate::services::desktop_actions::DesktopActionError::OpenUrlFailed { source, .. } => {
            source.to_string()
        }
        _ => error.to_string(),
    }
}

fn map_auth_state(state: &AuthState) -> SessionState {
    match state {
        AuthState::SignedOut => SessionState::SignedOut,
        AuthState::SigningIn | AuthState::SigningUp | AuthState::AwaitingChallenge { .. } => {
            SessionState::SigningIn
        }
        AuthState::SignedIn { account_label } => SessionState::SignedIn {
            account_label: account_label.clone(),
        },
        AuthState::Error(message) => SessionState::Error(message.clone()),
    }
}

#[cfg(test)]
#[path = "flows.test.rs"]
mod flows_test;
