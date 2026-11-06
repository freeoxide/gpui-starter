use gpui_auth::EMAIL_PROVIDER_ID;
use gpui_kit::App;
use gpui_kit::component::IconName;

use crate::capabilities::Capability;

pub(crate) enum ProviderKind {
    Email { capability: Capability },
    Browser,
}

pub(crate) struct ProviderSpec {
    pub(crate) id: &'static str,
    pub(crate) kind: ProviderKind,
    pub(crate) label_key: &'static str,
    pub(crate) icon: IconName,
}

const PROVIDERS: [ProviderSpec; 3] = [
    ProviderSpec {
        id: EMAIL_PROVIDER_ID,
        kind: ProviderKind::Email {
            capability: Capability::AUTH_EMAIL,
        },
        label_key: "sign_in_provider_email",
        icon: IconName::User,
    },
    ProviderSpec {
        id: "google",
        kind: ProviderKind::Browser,
        label_key: "sign_in_provider_google",
        icon: IconName::Globe,
    },
    ProviderSpec {
        id: "github",
        kind: ProviderKind::Browser,
        label_key: "sign_in_provider_github",
        icon: IconName::Github,
    },
];

pub(crate) fn providers() -> &'static [ProviderSpec] {
    &PROVIDERS
}

pub(crate) fn is_available(spec: &ProviderSpec, cx: &App) -> bool {
    match spec.kind {
        ProviderKind::Email { capability } => crate::capabilities::get(capability, cx)
            // wasm keeps password auth enabled but unsupported; the row
            // must not offer a form no provider can answer.
            .is_some_and(|status| status.supported && status.enabled),
        ProviderKind::Browser => browser_configured(spec.id, cx),
    }
}

fn browser_configured(id: &str, cx: &App) -> bool {
    #[cfg(target_family = "wasm")]
    {
        let _ = (id, cx);
        false
    }
    #[cfg(not(target_family = "wasm"))]
    {
        crate::services::auth_social::snapshot(cx)
            .providers
            .iter()
            .find(|(kind, _)| kind.id() == id)
            .is_some_and(|(_, status)| status.configured)
    }
}

pub(crate) fn start_browser_flow(spec: &ProviderSpec, cx: &mut App) {
    #[cfg(not(target_family = "wasm"))]
    if let Some(kind) = crate::services::auth_social::ProviderKind::parse(spec.id) {
        crate::services::auth_social::start(kind, cx);
        return;
    }
    tracing::warn!(
        target: "gpui_starter::sign_in",
        provider_id = spec.id,
        "no browser sign-in flow for provider"
    );
}

#[cfg(test)]
#[path = "providers.test.rs"]
mod providers_test;
